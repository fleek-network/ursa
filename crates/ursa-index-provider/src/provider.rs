use crate::advertisement::{self, EntryChunk};

use advertisement::Advertisement;
use anyhow::{anyhow, Error, Result};
use db::Store;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use cid::{multihash::Code, Cid};
use forest_encoding::Cbor;
use forest_ipld::Ipld;
use fvm_ipld_blockstore::Blockstore;
use libipld::codec::Encode;
use libipld_cbor::DagCborCodec;
use libp2p::{identity::Keypair, Multiaddr, PeerId};
use rand;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::Write,
    str::FromStr,
    sync::{Arc, RwLock},
};
use tracing::{info, trace};
use ursa_store::{BlockstoreExt, UrsaStore};
use ursa_utils::convert_cid;

pub const HEAD_KEY: &str = "head";

pub struct Provider<S> {
    head: Arc<RwLock<Option<Cid>>>,
    keypair: Keypair,
    store: Arc<UrsaStore<S>>,
    temp_ads: HashMap<usize, Advertisement>,
}

impl<S> Provider<S>
where
    S: Blockstore + Store + Sync + Send + 'static,
{
    pub fn new(keypair: Keypair, store: Arc<UrsaStore<S>>) -> Self {
        let head = store
            .blockstore()
            .read(HEAD_KEY)
            .expect("reading from store should not fail")
            .map(|h| Cid::try_from(h).unwrap());
        Provider {
            keypair,
            store,
            head: Arc::new(RwLock::new(head)),
            temp_ads: HashMap::new(),
        }
    }

    pub fn store(&self) -> Arc<UrsaStore<S>> {
        Arc::clone(&self.store)
    }

    pub fn keypair(&self) -> &Keypair {
        &self.keypair
    }

    pub fn head(&self) -> Option<Cid> {
        let head_lock = self.head.read().unwrap();
        *head_lock
    }
}

impl<S> Clone for Provider<S>
where
    S: Blockstore + Store + Sync + Send + 'static,
{
    fn clone(&self) -> Self {
        Self {
            head: Arc::clone(&self.head),
            keypair: self.keypair.clone(),
            store: Arc::clone(&self.store),
            temp_ads: self.temp_ads.clone(),
        }
    }
}

pub enum ProviderError {
    NotFoundError(Error),
    InternalError(Error),
}

impl IntoResponse for ProviderError {
    fn into_response(self) -> Response {
        match self {
            ProviderError::NotFoundError(e) => {
                (StatusCode::NOT_FOUND, e.to_string()).into_response()
            }
            ProviderError::InternalError(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
            }
        }
    }
}

pub trait ProviderInterface: Sync + Send + 'static {
    fn create(&mut self, ad: Advertisement) -> Result<usize>;
    fn add_chunk(&mut self, bytes: Vec<u8>, id: usize) -> Result<()>;
    fn publish(&mut self, id: usize) -> Result<()>;
    fn create_announce_message(&mut self, peer_id: PeerId, domain: String) -> Result<Vec<u8>>;
}

impl<S> ProviderInterface for Provider<S>
where
    S: Blockstore + Store + Sync + Send + 'static,
{
    fn create(&mut self, mut ad: Advertisement) -> Result<usize> {
        let id: usize = rand::thread_rng().gen();
        ad.Entries = None;
        self.temp_ads.insert(id, ad);

        trace!("ad created with id : {}", id);
        Ok(id)
    }

    fn add_chunk(&mut self, bytes: Vec<u8>, id: usize) -> Result<()> {
        let entries = forest_encoding::from_slice(&bytes).unwrap();

        if let Some(ad) = self.temp_ads.get_mut(&id) {
            let entry_head_clone = ad.Entries.clone();
            let chunk = EntryChunk::new(entries, entry_head_clone);
            return match self.store.db.put_obj(&chunk, Code::Blake2b256) {
                Ok(cid) => {
                    ad.Entries = Some(Ipld::Link(convert_cid(cid.to_bytes())));
                    Ok(())
                }
                Err(e) => Err(anyhow!(format!("{e}"))),
            };
        }

        Err(anyhow!("ad not found"))
    }

    fn publish(&mut self, id: usize) -> Result<()> {
        let mut head = self.head.write().unwrap();
        let keypair = self.keypair.clone();
        let current_head = head.take();
        if let Some(mut ad) = self.temp_ads.remove(&id) {
            ad.PreviousID = current_head.map(|h| Ipld::Link(convert_cid(h.to_bytes())));
            let sig = ad.sign(&keypair)?;
            ad.Signature = Ipld::Bytes(sig.into_protobuf_encoding());
            let ipld_ad = forest_ipld::to_ipld(&ad)?;
            let cid = self
                .store
                .blockstore()
                .put_obj(&ipld_ad, Code::Blake2b256)?;
            self.store.db.write(HEAD_KEY, cid.to_bytes())?;
            *head = Some(cid);
            return Ok(());
        }
        Err(anyhow!("ad not found"))
    }

    fn create_announce_message(&mut self, peer_id: PeerId, mut domain: String) -> Result<Vec<u8>> {
        if domain.is_empty() {
            domain = "/ip4/127.0.0.1/tcp/8070".to_string();
        }
        let mut multiaddrs = Multiaddr::from_str(&domain)?;
        multiaddrs = Multiaddr::try_from(format!("{multiaddrs}/http/p2p/{peer_id}"))?;
        let message_addrs = [multiaddrs].to_vec();
        if let Some(head_cid) = *self.head.read().unwrap() {
            let message = Message {
                Cid: head_cid,
                Addrs: message_addrs,
                ExtraData: *b"",
            };

            info!("Announcing the advertisement with the message {message:?}");
            Ok(message.marshal_cbor().unwrap())
        } else {
            Err(anyhow!("No head found for announcement!"))
        }
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub Cid: Cid,
    pub Addrs: Vec<Multiaddr>,
    pub ExtraData: [u8; 0],
}
impl Cbor for Message {
    fn marshal_cbor(&self) -> Result<Vec<u8>, forest_encoding::Error> {
        const MESSAGE_BUFFER_LENGTH: [u8; 1] = [131];
        let mut bytes = Vec::new();
        let _ = bytes.write_all(&MESSAGE_BUFFER_LENGTH);
        let _encoded_cid = self.Cid.encode(DagCborCodec, &mut bytes);

        let encoded_addrs =
            forest_encoding::to_vec(&self.Addrs).expect("addresses serialization cannot fail");
        bytes
            .write_all(&encoded_addrs)
            .expect("writing encoded address to bytes should not fail");

        let _encoded_data = self.ExtraData.encode(DagCborCodec, &mut bytes);

        Ok(bytes)
    }
}

#[cfg(test)]
#[path = "tests/provider_tests.rs"]
mod provider_tests;
