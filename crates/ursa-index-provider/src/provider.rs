use crate::{
    advertisement::{self, EntryChunk},
    config::ProviderConfig,
    signed_head::SignedHead,
};

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
use ipld_blockstore::{BlockStore, BlockStoreExt};
use libipld::codec::Encode;
use libipld_cbor::DagCborCodec;
use libp2p::{identity::Keypair, Multiaddr, PeerId};
use multihash::Code;
use rand;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io::Write, str::FromStr, sync::Arc};
use tokio::sync::RwLock;
use tracing::info;
use ursa_utils::convert_cid;

// handlers
async fn head<S: BlockStore + Sync + Send + 'static>(
    Extension(state): Extension<Provider<S>>,
) -> Result<Json<SignedHead>, ProviderError> {
    if let Some(head) = *state.head.read().await {
        let signed_head = SignedHead::new(&state.keypair, head)
            .map_err(|e| return ProviderError::InternalError(anyhow!(e.to_string())))?;
        Ok(Json(signed_head))
    } else {
        Err(ProviderError::NotFoundError(anyhow!("No head found")))
    }
}

async fn get_block<S: BlockStore + Sync + Send + 'static>(
    Extension(state): Extension<Provider<S>>,
    Path(cid): Path<String>,
) -> Result<Response<Body>, ProviderError> {
    let cid = Cid::from_str(&cid)
        .map_err(|e| return ProviderError::InternalError(anyhow!(e.to_string())))?;
    let store = state.blockstore;
    match store.get_bytes(&cid) {
        Ok(Some(d)) => Ok(Response::builder().body(Body::from(d)).unwrap()),
        Ok(None) => Err(ProviderError::NotFoundError(anyhow!("Block not found"))),
        Err(e) => Err(ProviderError::InternalError(anyhow!(format!("{}", e)))),
    }
}

pub struct Provider<S> {
    head: Arc<RwLock<Option<Cid>>>,
    keypair: Keypair,
    blockstore: Arc<S>,
    temp_ads: Arc<RwLock<HashMap<usize, Advertisement>>>,
    address: Multiaddr,
}

impl<S> Provider<S>
where
    S: BlockStore + Sync + Send + 'static,
{
    pub fn new(keypair: Keypair, blockstore: Arc<S>) -> Self {
        Provider {
            keypair,
            blockstore,
            head: Arc::new(RwLock::new(None)),
            temp_ads: Arc::new(RwLock::new(HashMap::new())),
            address: Multiaddr::empty(),
        }
    }

    pub async fn start(mut self, config: &ProviderConfig) -> Result<()> {
        info!("index provider starting up");
        let ProviderConfig { addr, port } = config;
        self.address = format!("/ip4/{}/tcp/{}", *addr, *port).parse().unwrap();

        let app_router = Router::new()
            .route("/head", get(head::<S>))
            .route("/:cid", get(get_block::<S>))
            .layer(Extension(self.clone()));

        let app_address = format!("{}:{}", *addr, *port).parse().unwrap();

        info!("index provider listening on: {:?}", &app_address);
        let _server = axum::Server::bind(&app_address)
            .serve(app_router.into_make_service())
            .await;
        Ok(())
    }
}

impl<S> Clone for Provider<S>
where
    S: BlockStore + Sync + Send + 'static,
{
    fn clone(&self) -> Self {
        Self {
            head: Arc::clone(&self.head),
            keypair: self.keypair.clone(),
            blockstore: Arc::clone(&self.blockstore),
            temp_ads: Arc::clone(&self.temp_ads),
            address: self.address.clone(),
        }
    }
}

pub enum ProviderError {
    NotFoundError(Error),
    InternalError(Error),
}
impl IntoResponse for ProviderError {
    fn into_response(self) -> Response {
        return match self {
            ProviderError::NotFoundError(e) => {
                (StatusCode::NOT_FOUND, e.to_string()).into_response()
            }
            ProviderError::InternalError(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
            }
        };
    }
}

#[async_trait]
pub trait ProviderInterface: Sync + Send + 'static {
    async fn create(&self, ad: Advertisement) -> Result<usize>;
    async fn add_chunk(&self, bytes: Vec<u8>, id: usize) -> Result<()>;
    async fn publish(&self, id: usize) -> Result<()>;
    async fn announce_msg(&self, peer_id: PeerId) -> Result<Vec<u8>>;
}

#[async_trait]
impl<S> ProviderInterface for Provider<S>
where
    S: BlockStore + Sync + Send + 'static,
{
    async fn create(&self, mut ad: Advertisement) -> Result<usize> {
        let id: usize = rand::thread_rng().gen();
        ad.Entries = None;
        let mut temp_ads = self.temp_ads.write().await;
        temp_ads.insert(id, ad);
        info!("ad created with id : {}", id);

        Ok(id)
    }

    async fn add_chunk(&self, bytes: Vec<u8>, id: usize) -> Result<()> {
        let entries = forest_encoding::from_slice(&bytes).unwrap();

        let mut temp_ads = self.temp_ads.write().await;
        if let Some(ad) = temp_ads.get_mut(&id) {
            let entry_head_clone = ad.Entries.clone();
            let chunk = EntryChunk::new(entries, entry_head_clone);
            return match self.blockstore.put_obj(&chunk, Code::Blake2b256) {
                Ok(cid) => {
                    ad.Entries = Some(Ipld::Link(convert_cid(cid.to_bytes())));
                    Ok(())
                }
                Err(e) => Err(anyhow!(format!("{}", e))),
            };
        }

        Err(anyhow!("ad not found"))
    }

    async fn publish(&self, id: usize) -> Result<()> {
        let mut head = self.head.write().await;
        let keypair = self.keypair.clone();
        let current_head = head.take();
        let mut temp_ads = self.temp_ads.write().await;
        if let Some(mut ad) = temp_ads.remove(&id) {
            ad.PreviousID = current_head.map(|h| Ipld::Link(convert_cid(h.to_bytes())));
            let sig = ad.sign(&keypair)?;
            ad.Signature = Ipld::Bytes(sig.into_protobuf_encoding());
            let ipld_ad = forest_ipld::to_ipld(ad)?;
            let cid = self.blockstore.put_obj(&ipld_ad, Code::Blake2b256)?;
            *head = Some(cid);
            return Ok(());
        }
        return Err(anyhow!("ad not found"));
    }

    async fn announce_msg(&self, peer_id: PeerId) -> Result<Vec<u8>> {
        let msg_addrs = [format!("{}/http/p2p/{}", self.address, peer_id)
            .parse()
            .unwrap()]
        .to_vec();
        let head = self.head.read().await;
        let head_cid: Cid = (*head).expect("no head found for announcement");
        let message = Message {
            Cid: head_cid,
            Addrs: msg_addrs,
            ExtraData: *b"",
        };
        info!("Announcing th advertisement with the message {:?}", message);

        Ok(message.marshal_cbor().unwrap())
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
        println!("{:?}", self.Cid);
        const MESSAGE_BUFFER_LENGTH: [u8; 1] = [131];
        let mut bytes = Vec::new();
        let _ = bytes.write_all(&MESSAGE_BUFFER_LENGTH);
        let _encoded_cid = self.Cid.encode(DagCborCodec, &mut bytes);

        println!("{:?}", self.Addrs);
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
mod tests {
    use super::*;
    use db::{rocks::RocksDb, rocks_config::RocksDbConfig};
    use libp2p::PeerId;
    use multihash::MultihashDigest;
    use std::{thread, time::Duration};

    #[tokio::test]
    async fn test_create_ad() -> Result<(), Box<dyn std::error::Error>> {
        let keypair = Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());
        let provider_db = RocksDb::open("index_provider_db", &RocksDbConfig::default())
            .expect("Opening RocksDB must succeed");
        let provider = Provider::new(keypair.clone(), Arc::new(provider_db));
        let provider_config = ProviderConfig::default();

        let provider_interface = provider.clone();
        tokio::task::spawn(async move {
            let _ = provider.start(&provider_config).await;
        });

        let delay = Duration::from_millis(2000);
        thread::sleep(delay);

        let ad = Advertisement {
            PreviousID: None,
            Provider: peer_id.to_base58(),
            Addresses: vec!["/ip4/127.0.0.1/tcp/6009".into()],
            Signature: Ipld::Bytes(vec![]),
            Entries: None,
            Metadata: Ipld::Bytes(vec![]),
            ContextID: Ipld::Bytes("ursa".into()),
            IsRm: false,
        };

        let id = provider_interface.create(ad).await.unwrap();

        let mut entries: Vec<Ipld> = vec![];
        let count = 10;

        for i in 0..count {
            let b = Into::<i32>::into(i).to_ne_bytes();
            let mh = Code::Blake2b256.digest(&b);
            entries.push(Ipld::Bytes(mh.to_bytes()))
        }
        let bytes = forest_encoding::to_vec(&entries)?;
        let _ = provider_interface.add_chunk(bytes, id).await;
        let _ = provider_interface.publish(id).await;
        let t_head = provider_interface.head.read().await;

        let signed_head: SignedHead = surf::get("http://0.0.0.0:8070/head").recv_json().await?;
        assert_eq!(signed_head.open()?.1, t_head.unwrap());

        Ok(())
    }
}
