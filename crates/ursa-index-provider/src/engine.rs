use db::{rocks_config::RocksDbConfig, rocks::RocksDb};
use tokio::{
    select,
    sync::{
        mpsc::{ UnboundedReceiver as Receiver, UnboundedSender as Sender, unbounded_channel},
        oneshot,
    },
};
use db::Store as Store_;
use forest_ipld::Ipld;
use futures::{stream::{self, StreamExt}, Future};
use crate::{
    advertisement::{self, EntryChunk},
    config::ProviderConfig,
    signed_head::SignedHead, provider::{Provider, HEAD_KEY},
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
    sync::{Arc, RwLock}, task::{Context, Poll}, pin::Pin,
};
use tracing::{error, info, trace};
use ursa_store::{Store, BlockstoreExt};

type CommandOneShotSender<T> = oneshot::Sender<Result<T, Error>>;
type CommandOneShotReceiver<T> = oneshot::Receiver<Result<T, Error>>;

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
    Extension(state): Extension<Provider<S>>,
) -> Result<Json<SignedHead>, ProviderError> {
    if let Ok(Some(head))  = state.store().blockstore().read(HEAD_KEY) {
        let signed_head = SignedHead::new(state.keypair(), Cid::try_from(head).unwrap())
            .map_err(|e| ProviderError::InternalError(anyhow!(e.to_string())))?;
        Ok(Json(signed_head))
    } else {
        Err(ProviderError::NotFoundError(anyhow!("No head found")))
    }
}

async fn get_block<S: Blockstore + Store_ + Sync + Send + 'static>(
    Extension(state): Extension<Provider<S>>,
    Path(cid): Path<String>,
) -> Result<Response<Body>, ProviderError> {
    let cid =
        Cid::from_str(&cid).map_err(|e| ProviderError::InternalError(anyhow!(e.to_string())))?;
    match state.store().blockstore().get_obj::<Vec<u8>>(&cid) {
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


#[derive(Debug)]
pub enum ProviderCommand {
    /// put multihashes when node start caching new contenct
    Put { context_id: Vec<u8>, sender: CommandOneShotSender<()> },
    /// remove multihashes from advertisment when evicted by a node
    Remove { context_id: Vec<u8>, sender: CommandOneShotReceiver<()>, },
}

pub struct ProviderEngine<S> {
    /// index provider
    provider: Provider<S>,
    /// list of cids that needs to be published to indexers
    cids_queue: CidQueue,
    /// provider config
    config: ProviderConfig,
    /// used by other processes to send message to provider engine
    command_sender: Sender<ProviderCommand>,
    /// Handles inbound messages to the provider engine
    command_receiver: Receiver<ProviderCommand>,
}

impl <S> ProviderEngine <S>
where
    S: Blockstore + Store_ + Sync + Send + 'static,
{
    pub fn new(keypair: Keypair, store: Arc<Store<S>>, provider_store: Arc<Store<S>>, config: ProviderConfig) -> Self {
        let (command_sender, command_receiver) = unbounded_channel();
        ProviderEngine {
            cids_queue: CidQueue {root_cids: VecDeque::new()},
            config,
            provider: Provider::new(keypair, provider_store),
            command_receiver,
            command_sender,
        }
    }
    pub fn command_sender(&self) -> Sender<ProviderCommand>{
        self.command_sender.clone()
    }

    pub async fn start(mut self, provider_config: &ProviderConfig) -> Result<()> {
        info!("Index provider engine starting up!");

        let app_router = Router::new()
            .route("/head", get(head::<S>))
            .route("/:cid", get(get_block::<S>))
            .layer(Extension(self.provider.clone()));

        let app_address = format!("{}:{}", provider_config.local_address, provider_config.port)
            .parse()
            .unwrap();

        info!("index provider listening on: {:?}", &app_address);
        
        // let engine = self.engine();
        let (server, engine) = tokio::join!(
            axum::Server::bind(&app_address).serve(app_router.into_make_service()),
            self.poll_command());
        engine.expect("failed to start the engine");
        server.expect("failed to start the server");
        Ok(())
    }

    pub async fn poll_command(&mut self) -> Result<()> {
        // loop {
        //     if let Some(command) = self.command_receiver.recv().await {
        //         match command{
        //             ProviderCommand::Put{ context_id, sender } => {
        //                 let cid = Cid::try_from(context_id).unwrap();
        //                 self.cids_queue.push_back(cid);
        //                 info!("the {cid:?} is queued to be published to the index provider");
        //                 if let Err(e) = sender.send(Ok(())) {
        //                     error!("Provider Engine: {:?}", e);
        //                 }
        //             },
        //             // TODO: implement when cache eviction is implemented
        //             ProviderCommand::Remove{ .. } => todo!(),
        //         }         
        //     }
        // }
        loop {
            select! {
                event = self.cids_queue.await => {
                    let root_cid = event;
                    
                },
                command = self.command_receiver.recv() => {
                    let command = command.ok_or_else(|| anyhow!("Command invalid!"))?;
                    match command{
                        ProviderCommand::Put{ context_id, sender } => {
                            let cid = Cid::try_from(context_id).unwrap();
                            self.cids_queue.root_cids.push_back(cid);
                            info!("the {cid:?} is queued to be published to the index provider");
                            if let Err(e) = sender.send(Ok(())) {
                                error!("Provider Engine: {:?}", e);
                            }
                        },
                        // TODO: implement when cache eviction is implemented
                        ProviderCommand::Remove{ .. } => todo!(),
                    } 
                    
                },
            }
        }
    }

    // pub async fn poll_cid_queue(&mut self) -> Result<()> {
    //     loop {
    //         let read_lock = self.cids_queue.read().unwrap();
    //         if !read_lock.is_empty(){
    //             let write_lock = self.cids_queue.write().unwrap();
    //             let next_cid = write_lock.pop_front().unwrap();


    //         }
    //         drop(read_lock);
    //     }
    // }
}

#[derive(Debug)]
pub struct CidQueue {
    pub root_cids: VecDeque<Cid>
}
impl Future for CidQueue {
    type Output = CidQueue;
    
    fn poll(mut self: Pin<&mut CidQueue>, cx: &mut Context<'_>) ->  Poll<Self::Output> {
        if !self.root_cids.is_empty(){
            return Poll::Ready(self);
        }
        Poll::Pending
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
    use surf::Error as SurfError;
    use tokio::task;
    use tracing::log::LevelFilter;

    fn setup_logger(level: LevelFilter) {
        if let Err(err) = SimpleLogger::new()
            .with_level(level)
            .with_utc_timestamps()
            .init() {
                info!("Logger already set. Ignore.")
            }
    }

    fn get_store() -> Arc<Store<MemoryDB>> {
        let db = Arc::new(MemoryDB::default());
        Arc::new(Store::new(Arc::clone(&db)))
    }

    fn provider_engine_init(config: &ProviderConfig) -> (ProviderEngine<MemoryDB>, PeerId) {
        setup_logger(LevelFilter::Debug);
        let keypair = Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());

        let store = get_store();
        let index_store = get_store();
        let provider_engine = ProviderEngine::new(keypair.clone(), store, index_store, config.clone());
        (provider_engine, peer_id)
    }

    #[tokio::test]
    async fn test_create_ad() -> Result<(), Box<dyn std::error::Error>> {
        let mut config = ProviderConfig::default();
        config.port = 0;

        let (provider_engine, peer_id) = provider_engine_init(&config);

        let mut provider_interface = provider_engine.provider.clone();

        task::spawn(async move {
            if let Err(err) = provider_engine.start(&config).await {
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


            let signed_head: SignedHead = surf::get("http://0.0.0.0:8070/head").recv_json().await
                .map_err(|e| SurfError::into_inner(e))?;
            assert_eq!(
                signed_head.open()?.1,
                provider_interface.head().unwrap()
            );

            Ok::<_, Error>(())
        })
        .await?;


        Ok(())
    }

    #[tokio::test]
    async fn test_events() -> Result<(), Box<dyn std::error::Error>>{
        let mut config = ProviderConfig::default();
        config.port = 0;

        let (provider_engine, peer_id) = provider_engine_init(&config);

        let (sender, receiver) = oneshot::channel();
        let msg = ProviderCommand::Put { context_id: b"some test root cid".to_vec(), sender };
        let provider_sender = provider_engine.command_sender.clone();

        task::spawn(async move {
            if let Err(err) = provider_engine.start(&config).await {
                error!("[provider_task] - {:?}", err);
            }
        });

        let _ = provider_sender.send(msg);
        let res = receiver.await?;

        Ok(())
    }
}
