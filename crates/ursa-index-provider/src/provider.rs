use crate::{
    advertisement::{self, EntryChunk},
    config::ProviderConfig,
    signed_head::SignedHead,
};
use db::Store as Store_;
use advertisement::Advertisement;
use anyhow::{anyhow, Error, Result};

use async_trait::async_trait;
use axum::{
    body::Body,
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Extension, Json, Router,
};
use cid::Cid;
use forest_encoding::Cbor;
use forest_ipld::Ipld;
use fvm_ipld_blockstore::Blockstore;
use libipld::codec::Encode;
use libipld_cbor::DagCborCodec;
use libp2p::{identity::Keypair, Multiaddr, PeerId};
use multihash::Code;
use rand;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    io::Write,
    str::FromStr,
    sync::{Arc, RwLock},
};
use tracing::{error, info, trace};
use ursa_store::{BlockstoreExt, Store};
use ursa_utils::convert_cid;

pub const HEAD_KEY: &str = "head";
// handlers
async fn head<S: Blockstore + Sync + Send + 'static>(
    Extension(state): Extension<Provider<S>>,
) -> Result<Json<SignedHead>, ProviderError> {
    if let Some(head) = *state.head.read().unwrap() {
        let signed_head = SignedHead::new(&state.keypair, head)
            .map_err(|e| ProviderError::InternalError(anyhow!(e.to_string())))?;
        Ok(Json(signed_head))
    } else {
        Err(ProviderError::NotFoundError(anyhow!("No head found")))
    }
}

async fn get_block<S: Blockstore + Sync + Send + 'static>(
    Extension(state): Extension<Provider<S>>,
    Path(cid): Path<String>,
) -> Result<Response<Body>, ProviderError> {
    let cid =
        Cid::from_str(&cid).map_err(|e| ProviderError::InternalError(anyhow!(e.to_string())))?;
    match state.store.db.get(&cid) {
        Ok(Some(d)) => Ok(Response::builder().body(Body::from(d)).unwrap()),
        Ok(None) => Err(ProviderError::NotFoundError(anyhow!("Block not found"))),
        Err(e) => Err(ProviderError::InternalError(anyhow!(format!("{}", e)))),
    }
}

pub struct Provider<S> {
    head: Arc<RwLock<Option<Cid>>>,
    keypair: Keypair,
    store: Arc<Store<S>>,
    temp_ads: HashMap<usize, Advertisement>,
}

impl<S> Provider<S>
where
    S: Blockstore + Store_ + Sync + Send + 'static,
{
    pub fn new(keypair: Keypair, store: Arc<Store<S>>) -> Self {
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

    pub fn store(&self) -> Arc<Store<S>> {
        Arc::clone(&self.store)
    }

    pub fn keypair(&self) -> &Keypair {
        &self.keypair
    }

    pub fn head(&self) -> Option<Cid> {
        let head_lock = self.head.read().unwrap();
        *head_lock
    }

    // pub async fn start(self, provider_config: &ProviderConfig) -> Result<()> {
    //     info!("Index provider starting up!");

    //     let app_router = Router::new()
    //         .route("/head", get(head::<S>))
    //         .route("/:cid", get(get_block::<S>))
    //         .layer(Extension(self.clone()));

    //     let app_address = format!("{}:{}", provider_config.local_address, provider_config.port)
    //         .parse()
    //         .unwrap();

    //     info!("index provider listening on: {:?}", &app_address);
    //     let _server = axum::Server::bind(&app_address)
    //         .serve(app_router.into_make_service())
    //         .await;
    //     Ok(())
    // }
}

impl<S> Clone for Provider<S>
where
    S: Blockstore + Store_ + Sync + Send + 'static,
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

#[async_trait]
pub trait ProviderInterface: Sync + Send + 'static {
    fn create(&mut self, ad: Advertisement) -> Result<usize>;
    fn add_chunk(&mut self, bytes: Vec<u8>, id: usize) -> Result<()>;
    fn publish(&mut self, id: usize) -> Result<()>;
    // fn create_announce_msg(&mut self, peer_id: PeerId) -> Result<Vec<u8>>;
    // async fn announce_http_message(&self, announce_msg: Vec<u8>);
}

#[async_trait]
impl<S> ProviderInterface for Provider<S>
where
    S: Blockstore + Store_ + Sync + Send + 'static,
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
                Err(e) => Err(anyhow!(format!("{}", e))),
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
            self.store.db.write(HEAD_KEY, cid.to_bytes());
            *head = Some(cid);
            return Ok(());
        }
        Err(anyhow!("ad not found"))
    }

    // fn create_announce_msg(&mut self, peer_id: PeerId) -> Result<Vec<u8>> {
    //     let mut multiaddrs = Multiaddr::from_str(&self.config.domain)?;
    //     multiaddrs = Multiaddr::try_from(format!("{}/http/p2p/{}", multiaddrs, peer_id))?;
    //     let msg_addrs = [multiaddrs].to_vec();
    //     if let Some(head_cid) = *self.head.read().unwrap() {
    //         let message = Message {
    //             Cid: head_cid,
    //             Addrs: msg_addrs,
    //             ExtraData: *b"",
    //         };

    //         info!(
    //             "Announcing the advertisement with the message {:?}",
    //             message
    //         );
    //         Ok(message.marshal_cbor().unwrap())
    //     } else {
    //         Err(anyhow!("No head found for announcement!"))
    //     }
    // }

    // async fn announce_http_message(&self, announce_msg: Vec<u8>) {
    //     let res = surf::put(format!("{}/ingest/announce", self.config.indexer_url))
    //         .body(announce_msg)
    //         .await;
    //     match res {
    //         Ok(r) => info!("http announce successful {:?}", r.status()),
    //         Err(e) => error!("error: http announce failed {:?}", e),
    //     };
    // }
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

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use db::{rocks::RocksDb, rocks_config::RocksDbConfig};
//     use libp2p::PeerId;
//     use multihash::MultihashDigest;
//     use simple_logger::SimpleLogger;
//     use tokio::task;
//     use tracing::log::LevelFilter;

//     async fn init(keypair: Keypair) -> Provider<RocksDb> {
//         let provider_config = ProviderConfig::default();
//         let provider_db = RocksDb::open("index_provider_db", &RocksDbConfig::default())
//             .expect("Opening RocksDB must succeed");
//         let index_store = Arc::new(Store::new(Arc::clone(&Arc::new(provider_db))));
//         let index_provider = Provider::new(keypair.clone(), index_store);

//         let provider_interface = index_provider.clone();
//         if let Err(err) = index_provider.start(&provider_config).await {
//             error!("[provider_task] - {:?}", err);
//         }

//         provider_interface
//     }

//     #[tokio::test]
//     async fn test_create_ad() -> Result<(), Box<dyn std::error::Error>> {
//         SimpleLogger::new()
//             .with_level(LevelFilter::Debug)
//             .with_utc_timestamps()
//             .init()
//             .unwrap();

//         let keypair = Keypair::generate_ed25519();
//         let peer_id = PeerId::from(keypair.public());

//         let provider_config = ProviderConfig::default();
//         let provider_db = RocksDb::open("index_provider_db", &RocksDbConfig::default())
//             .expect("Opening RocksDB must succeed");
//         let index_store = Arc::new(Store::new(Arc::clone(&Arc::new(provider_db))));
//         let index_provider = Provider::new(keypair.clone(), index_store, provider_config.clone());

//         let mut provider_interface = index_provider.clone();
//         let provider_interface_copy = index_provider.clone();

//         task::spawn(async move {
//             if let Err(err) = index_provider.start(&provider_config).await {
//                 error!("[provider_task] - {:?}", err);
//             }
//         });

//         let _ = task::spawn(async move {
//             let ad = Advertisement {
//                 PreviousID: None,
//                 Provider: peer_id.to_base58(),
//                 Addresses: vec!["/ip4/127.0.0.1/tcp/6009".into()],
//                 Signature: Ipld::Bytes(vec![]),
//                 Entries: None,
//                 Metadata: Ipld::Bytes(vec![]),
//                 ContextID: Ipld::Bytes("ursa".into()),
//                 IsRm: false,
//             };

//             let id = provider_interface.create(ad).unwrap();

//             let mut entries: Vec<Ipld> = vec![];
//             let count = 10;

//             for i in 0..count {
//                 let b = Into::<i32>::into(i).to_ne_bytes();
//                 let mh = Code::Blake2b256.digest(&b);
//                 entries.push(Ipld::Bytes(mh.to_bytes()))
//             }
//             let bytes = forest_encoding::to_vec(&entries)?;
//             provider_interface.add_chunk(bytes, id)?;
//             provider_interface.publish(id)?;

//             Ok::<_, Error>(())
//         })
//         .await?;

//         let signed_head: SignedHead = surf::get("http://0.0.0.0:8070/head").recv_json().await?;
//         assert_eq!(
//             signed_head.open()?.1,
//             provider_interface_copy.head.read().unwrap().unwrap()
//         );

//         Ok(())
//     }
// }
