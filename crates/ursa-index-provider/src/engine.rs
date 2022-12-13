use db::{rocks_config::RocksDbConfig, rocks::RocksDb};
use tokio::{
    select,
    sync::mpsc::{ UnboundedReceiver as Receiver, UnboundedSender as Sender, unbounded_channel},
};
use db::Store as Store_;
use forest_ipld::Ipld;
use crate::{
    advertisement::{self, EntryChunk},
    config::ProviderConfig,
    signed_head::SignedHead, provider::Provider,
};

use anyhow::{anyhow, Error, Result};

use axum::{
    body::Body,
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Extension, Json, Router,
};
use cid::Cid;

use fvm_ipld_blockstore::Blockstore;
use libp2p::identity::Keypair;
use std::{
    collections::VecDeque,
    str::FromStr,
    sync::{Arc, RwLock},
};
use tracing::{error, info, trace};
use ursa_store::{Store, BlockstoreExt};

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
// handlers
async fn head<S: Blockstore + Store_ + Sync + Send + 'static>(
    Extension(state): Extension<ProviderEngine<S>>,
) -> Result<Json<SignedHead>, ProviderError> {
    
    if let Some(head) = *state.head.read().unwrap() {
        let signed_head = SignedHead::new(&state.keypair, head)
            .map_err(|e| ProviderError::InternalError(anyhow!(e.to_string())))?;
        Ok(Json(signed_head))
    } else {
        Err(ProviderError::NotFoundError(anyhow!("No head found")))
    }
}

async fn get_block< S: Blockstore + Store_ + Sync + Send + 'static,>(
    Extension(state): Extension<ProviderEngine<S>>,
    Path(cid): Path<String>,
) -> Result<Response<Body>, ProviderError> {
    let cid =
        Cid::from_str(&cid).map_err(|e| ProviderError::InternalError(anyhow!(e.to_string())))?;
    match state.provider.store().blockstore().get_obj::<Vec<u8>>(&cid) {
        Ok(Some(d)) => Ok(Response::builder().body(Body::from(d)).unwrap()),
        Ok(None) => Err(ProviderError::NotFoundError(anyhow!("Block not found"))),
        Err(e) => Err(ProviderError::InternalError(anyhow!(format!("{}", e)))),
    }
}

async fn get_head<S: Blockstore + Sync + Send + 'static>(
) -> Result<String, ProviderError> {
//     if let Some(head) = *state.head.read().unwrap() {
//         let signed_head = SignedHead::new(&state.keypair, head)
//             .map_err(|e| ProviderError::InternalError(anyhow!(e.to_string())))?;
//         Ok(Json(signed_head))
//     } else {
//         Err(ProviderError::NotFoundError(anyhow!("No head found")))
//     }
    Ok("head".to_string())
}



pub enum ProviderCommand {
    /// put multihashes when node start caching new contenct
    Put { context_id: Vec<u8>, sender: Result<()> },
    /// remove multihashes from advertisment when evicted by a node
    Remove { context_id: Vec<u8>, sender: Result<()>, },
}

pub struct ProviderEngine<S> {
    /// list of cids that needs to be published to indexers
    root_cids: Arc<RwLock<VecDeque<Cid>>>,
    /// provider config
    config: ProviderConfig,
    /// index provider
    provider: Provider<S>,
    /// used by other processes to send message to provider engine
    command_sender: Sender<ProviderCommand>,
    /// Handles inbound messages to the provider engine
    command_receiver: Receiver<ProviderCommand>,
}

impl <S> ProviderEngine <S>
where
    S: Blockstore + Store_ + Sync + Send + 'static,
{
    pub fn new(keypair: Keypair, store: Arc<Store<S>>, index_store: Arc<Store<S>>, config: ProviderConfig) -> Self {
        // let provider_db_name = config.database_path.clone();
        // let provider_db = RocksDb::open(provider_db_name, &RocksDbConfig::default())
        //     .expect("Opening RocksDB must succeed");
        // let index_store = Arc::new(Store::new(Arc::clone(&Arc::new(provider_db))));

        
        let (command_sender, command_receiver) = unbounded_channel();
        ProviderEngine {
            root_cids: Arc::new(RwLock::new(VecDeque::new())),
            config,
            provider: Provider::new(keypair, index_store),
            command_receiver,
            command_sender,
        }
    }

    pub async fn start(self, provider_config: &ProviderConfig) -> Result<()> {
        info!("Index provider engine starting up!");

        let state = Arc::new(self);
        let app_router = Router::new()
            .route("/", get(get_head::<S>))
            .route("/head", get(head::<S>))
            .route("/:cid", get(get_block::<S>))
            .layer(Extension(state));

        let app_address = format!("{}:{}", provider_config.local_address, provider_config.port)
            .parse()
            .unwrap();

        info!("index provider listening on: {:?}", &app_address);
        let server = axum::Server::bind(&app_address)
            .serve(app_router.into_make_service())
            .await;

        Ok(())
    }
}


#[cfg(test)]
mod tests {


    use super::*;
    use db:: MemoryDB;
    use libp2p::PeerId;
    use multihash::{Code, MultihashDigest};
    use crate::{advertisement::Advertisement, provider::ProviderInterface};
    use simple_logger::SimpleLogger;
    use tokio::task;
    use tracing::log::LevelFilter;

    fn get_store() -> Arc<Store<MemoryDB>> {
        let db = Arc::new(MemoryDB::default());
        Arc::new(Store::new(Arc::clone(&db)))
    }

    #[tokio::test]
    async fn test_create_ad() -> Result<(), Box<dyn std::error::Error>> {
        SimpleLogger::new()
            .with_level(LevelFilter::Debug)
            .with_utc_timestamps()
            .init()
            .unwrap();

        let keypair = Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());

        let store = get_store();
        let index_store = get_store();
        let provider_config = ProviderConfig::default();

        let provider_engine = ProviderEngine::new(keypair.clone(), store, index_store, provider_config.clone());

        let mut provider_interface = provider_engine.provider.clone();
        let provider_interface_copy = provider_engine.provider.clone();

        task::spawn(async move {
            if let Err(err) = provider_engine.start(&provider_config).await {
                error!("[provider_task] - {:?}", err);
            }
        });

        let _ = task::spawn(async move {
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

            let id = provider_interface.create(ad).unwrap();

            let mut entries: Vec<Ipld> = vec![];
            let count = 10;

            for i in 0..count {
                let b = Into::<i32>::into(i).to_ne_bytes();
                let mh = Code::Blake2b256.digest(&b);
                entries.push(Ipld::Bytes(mh.to_bytes()))
            }
            let bytes = forest_encoding::to_vec(&entries)?;
            provider_interface.add_chunk(bytes, id)?;
            provider_interface.publish(id)?;

            Ok::<_, Error>(())
        })
        .await?;

        let signed_head = surf::get("http://0.0.0.0:8070/").recv_string().await?;
        // assert_eq!(
        //     signed_head.open()?.1,
        //     provider_interface_copy.head.read().unwrap().unwrap()
        // );
        println!("{:?}", signed_head);

        Ok(())
    }
}
