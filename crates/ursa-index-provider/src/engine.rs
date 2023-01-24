use crate::{
    advertisement::{Advertisement, MAX_ENTRIES},
    config::ProviderConfig,
    provider::{Provider, ProviderInterface},
    signed_head::SignedHead,
};
use bytes::Bytes;
use db::Store;
use libipld_core::ipld::Ipld;
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver as Receiver, UnboundedSender as Sender},
    oneshot,
};
use ursa_network::{GossipsubMessage, NetworkCommand};

use anyhow::{anyhow, Error, Result};

use axum::{body::Body, extract::Path, response::Response, routing::get, Extension, Json, Router};

use crate::provider::ProviderError;
use fvm_ipld_blockstore::Blockstore;
use libipld::Cid;
use libp2p::{gossipsub::TopicHash, identity::Keypair, multiaddr::Protocol, Multiaddr, PeerId};
use std::{collections::VecDeque, str::FromStr, sync::Arc};
use tracing::{error, info, warn};
use ursa_store::UrsaStore;

type CommandOneShotSender<T> = oneshot::Sender<Result<T, Error>>;
type CommandOneShotReceiver<T> = oneshot::Receiver<Result<T, Error>>;

// handlers
async fn head<S: Blockstore + Store + Sync + Send + 'static>(
    Extension(state): Extension<Provider<S>>,
) -> Result<Json<SignedHead>, ProviderError> {
    if let Some(head) = state.head() {
        let signed_head = SignedHead::new(state.keypair(), head)
            .map_err(|e| ProviderError::InternalError(anyhow!(e.to_string())))?;
        Ok(Json(signed_head))
    } else {
        Err(ProviderError::NotFoundError(anyhow!("No head found")))
    }
}

async fn get_block<S: Blockstore + Store + Sync + Send + 'static>(
    Extension(state): Extension<Provider<S>>,
    Path(cid): Path<String>,
) -> Result<Response<Body>, ProviderError> {
    let cid =
        Cid::from_str(&cid).map_err(|e| ProviderError::InternalError(anyhow!(e.to_string())))?;
    match state.store().blockstore().get(&cid) {
        Ok(Some(d)) => Ok(Response::builder().body(Body::from(d)).unwrap()),
        Ok(None) => Err(ProviderError::NotFoundError(anyhow!("Block not found"))),
        Err(e) => {
            error!("{}", e);
            Err(ProviderError::InternalError(anyhow!(format!("{e}"))))
        }
    }
}

#[derive(Debug)]
pub enum ProviderCommand {
    /// put multihashes when node start caching new contenct
    Put {
        context_id: Vec<u8>,
        size: u64,
        sender: CommandOneShotSender<()>,
    },
    /// remove multihashes from advertisment when evicted by a node
    Remove {
        context_id: Vec<u8>,
        sender: CommandOneShotReceiver<()>,
    },
}

#[derive(Debug)]
pub struct CidQueue {
    pub root_cids: VecDeque<Cid>,
    pub receiver: oneshot::Receiver<Result<()>>,
}

pub struct ProviderEngine<S> {
    /// index provider
    provider: Provider<S>,
    /// main cache node store to get all the cids in a dag
    store: Arc<UrsaStore<S>>,
    /// provider config
    config: ProviderConfig,
    /// used by other processes to send message to provider engine
    command_sender: Sender<ProviderCommand>,
    /// Handles inbound messages to the provider engine
    command_receiver: Receiver<ProviderCommand>,
    /// network command sender for communication with libp2p node
    network_command_sender: Sender<NetworkCommand>,
    /// Server from which advertised content is retrievable.
    server_address: Multiaddr,
    domain: Multiaddr,
}

impl<S> ProviderEngine<S>
where
    S: Blockstore + Store + Sync + Send + 'static,
{
    pub fn new(
        keypair: Keypair,
        store: Arc<UrsaStore<S>>,
        provider_store: Arc<UrsaStore<S>>,
        config: ProviderConfig,
        network_command_sender: Sender<NetworkCommand>,
        server_address: Multiaddr,
        domain: Multiaddr,
    ) -> Self {
        let (command_sender, command_receiver) = unbounded_channel();
        ProviderEngine {
            command_receiver,
            command_sender,
            config,
            network_command_sender,
            provider: Provider::new(keypair, provider_store),
            store,
            server_address,
            domain,
        }
    }
    pub fn command_sender(&self) -> Sender<ProviderCommand> {
        self.command_sender.clone()
    }

    pub fn command_receiver(&mut self) -> &mut Receiver<ProviderCommand> {
        &mut self.command_receiver
    }

    pub fn provider(&self) -> Provider<S> {
        self.provider.clone()
    }

    pub fn store(&self) -> Arc<UrsaStore<S>> {
        Arc::clone(&self.store)
    }

    pub fn router(&self) -> Router {
        Router::new()
            .route("/head", get(head::<S>))
            .route("/:cid", get(get_block::<S>))
            .layer(Extension(self.provider()))
    }

    pub async fn start(mut self) -> Result<()> {
        info!("Index provider engine starting up!");

        loop {
            if let Some(command) = self.command_receiver.recv().await {
                match command {
                    ProviderCommand::Put {
                        context_id,
                        sender,
                        size,
                    } => {
                        let cid = Cid::try_from(context_id).unwrap();
                        if let Err(e) = sender.send(Ok(())) {
                            error!("Provider Engine: {:?}", e);
                        }
                        let peer_id = PeerId::from(self.provider.keypair().public());

                        if let Err(e) = self.publish_local(cid, size).await {
                            error!("Error while publishing the advertisement locally: {:?}", e)
                        } else {
                            match self
                                .provider
                                .create_announce_message(peer_id, self.domain.clone())
                            {
                                Ok(announce_message) => {
                                    if let Err(e) = self
                                        .gossip_announce(announce_message.clone(), peer_id)
                                        .await
                                    {
                                        warn!("there was an error while gossiping the announcement, will try to announce via http {:?}", e);
                                        self.http_announce(announce_message).await;
                                    }
                                }
                                Err(e) => warn!(
                                    "There was a problem parsing announcement message: {:?}",
                                    e
                                ),
                            }
                        }
                    }
                    // TODO: implement when cache eviction is implemented
                    ProviderCommand::Remove { .. } => todo!(),
                }
            }
        }
    }

    pub async fn publish_local(&mut self, root_cid: Cid, file_size: u64) -> Result<()> {
        let (listener_addresses_sender, listener_addresses_receiver) = oneshot::channel();
        self.network_command_sender
            .send(NetworkCommand::GetListenerAddresses {
                sender: listener_addresses_sender,
            })?;

        let context_id = root_cid.to_bytes();
        info!(
            "Creating advertisement for cids under root cid: {:?}.",
            root_cid
        );
        let peer_id = PeerId::from(self.provider.keypair().public());

        let listener_addresses = listener_addresses_receiver.await?;
        let mut addresses = vec![self.server_address.to_string()];
        for la in listener_addresses {
            let mut address = Multiaddr::empty();
            for protocol in la.into_iter() {
                match protocol {
                    Protocol::Ip6(ip) => address.push(Protocol::Ip6(ip)),
                    Protocol::Ip4(ip) => address.push(Protocol::Ip4(ip)),
                    Protocol::Tcp(port) => address.push(Protocol::Tcp(port)),
                    _ => {}
                }
            }
            addresses.push(address.to_string())
        }
        let advertisement = Advertisement::new(
            context_id.clone(),
            peer_id,
            addresses.clone(),
            false,
            file_size,
        );
        let provider_id = self.provider.create(advertisement)?;

        let dag = self.store.dag_traversal(&(root_cid))?;
        let entries = dag
            .iter()
            .map(|d| return Ipld::Bytes(d.0.hash().to_bytes()))
            .collect::<Vec<Ipld>>();
        let chunks: Vec<&[Ipld]> = entries.chunks(MAX_ENTRIES).collect();

        info!("Inserting Index chunks.");
        for chunk in chunks.iter() {
            let entries_bytes = fvm_ipld_encoding::to_vec(&chunk)?;
            self.provider
                .add_chunk(entries_bytes, provider_id)
                .expect(" adding chunk to advertisement should not fail!");
        }
        info!("Publishing the advertisement now");
        self.provider
            .publish(provider_id)
            .expect("publishing the ad should not fail");

        Ok(())
    }

    pub async fn gossip_announce(&mut self, data: Vec<u8>, peer_id: PeerId) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        let topic = TopicHash::from_raw("indexer/ingest/mainnet");
        let message = NetworkCommand::GossipsubMessage {
            peer_id,
            message: GossipsubMessage::Publish {
                topic,
                data: Bytes::from(data),
                sender,
            },
        };
        self.network_command_sender.send(message)?;
        receiver.await?.map_err(|e| anyhow!(e))?;
        Ok(())
    }

    pub async fn http_announce(&mut self, data: Vec<u8>) {
        if let Err(e) = surf::put(format!("{}/ingest/announce", self.config.indexer_url))
            .body(data)
            .await
        {
            error!("failed to announce to the indexer via http: {:?}", e);
        };
    }
}

#[cfg(test)]
#[path = "tests/engine_tests.rs"]
mod engine_tests;
