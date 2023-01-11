use std::collections::VecDeque;
use std::fmt::{Debug, Display};
use std::slice::Iter;
use std::sync::Arc;
use std::task::{Context, Poll};

use anyhow::{anyhow, Result};
use cid::Cid;
use db::Store;
use fnv::FnvHashMap;
use futures_util::FutureExt;
use fvm_ipld_blockstore::Blockstore;
use fvm_ipld_car::load_car;
use libp2p::{
    core::connection::ConnectionId,
    swarm::{
        behaviour::FromSwarm, dummy::ConnectionHandler, NetworkBehaviour, NetworkBehaviourAction,
        PollParameters,
    },
    PeerId,
};
use surf::Client;
use tokio::sync::oneshot::{channel, Receiver};
use tracing::error;
use void::Void;

use ursa_store::UrsaStore;

use crate::OriginConfig;

#[allow(dead_code)]
#[derive(Debug)]
pub enum Origin {
    Ipfs,
    Arweave,
    Filecoin,
}

impl Origin {
    pub async fn get<G: Display, C: Display>(
        &self,
        client: &Client,
        gateway: G,
        cid: C,
    ) -> Result<Vec<u8>> {
        let url = match self {
            Origin::Ipfs => format!("https://{gateway}/ipfs/{cid}"),
            _ => unimplemented!(),
        };

        client
            .get(url)
            .header("Accept", "application/vnd.ipld.car")
            .recv_bytes()
            .await
            .map_err(|e| anyhow!(e))
    }
}

pub type QueryId = u64;

pub struct PendingQuery {
    pub cid: Cid,
    pub receiver: Receiver<Result<()>>,
}

#[derive(Debug)]
pub enum OriginEvent {
    QueryCompleted(QueryId, Cid, Result<()>),
}

pub struct OriginBehavior<S> {
    store: Arc<UrsaStore<S>>,
    query_count: QueryId,
    pending: FnvHashMap<QueryId, PendingQuery>,
    client: Arc<Client>,
    config: OriginConfig,
    event_queue: VecDeque<OriginEvent>,
}

impl<S> OriginBehavior<S>
where
    S: Blockstore + Store + Send + Sync + 'static,
{
    pub fn new(config: OriginConfig, store: Arc<UrsaStore<S>>) -> Self {
        OriginBehavior {
            config,
            client: Arc::new(Client::new()),
            store,
            query_count: 0,
            pending: Default::default(),
            event_queue: Default::default(),
        }
    }

    pub fn get(&mut self, origin: Origin, cid: Cid) -> QueryId {
        let id = self.query_count;
        self.query_count += 1;
        let (sender, receiver) = channel();
        self.pending.insert(id, PendingQuery { cid, receiver });

        let gateway = match origin {
            Origin::Ipfs => self.config.ipfs_gateway.clone(),
            _ => unimplemented!(),
        };
        let (client, store) = (self.client.clone(), self.store.clone());
        tokio::task::spawn(async move {
            let res = origin.get(client.as_ref(), gateway, cid).await;

            match res {
                Ok(data) => {
                    load_car(store.db.as_ref(), data.as_slice())
                        .await
                        .expect("failed to load car from origin");
                    if let Err(e) = sender.send(Ok(())) {
                        error!("Failure sending success: {e:?}")
                    }
                }
                Err(e) => {
                    error!("Failed to get data from origin: {:?}", e);
                    if let Err(e) = sender.send(Err(e)) {
                        error!("Failure sending error response: {e:?}")
                    }
                }
            }
        });

        id
    }
}

impl<S> NetworkBehaviour for OriginBehavior<S>
where
    S: Blockstore + Store + Send + Sync + 'static,
{
    type ConnectionHandler = ConnectionHandler;
    type OutEvent = OriginEvent;

    fn new_handler(&mut self) -> Self::ConnectionHandler {
        ConnectionHandler
    }

    fn on_swarm_event(&mut self, _event: FromSwarm<Self::ConnectionHandler>) {}

    fn on_connection_handler_event(&mut self, _: PeerId, _: ConnectionId, event: Void) {
        void::unreachable(event)
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        _params: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<Self::OutEvent, Self::ConnectionHandler>> {
        self.pending
            .retain(|id, query| match query.receiver.poll_unpin(cx) {
                Poll::Ready(Ok(res)) => {
                    self.event_queue
                        .push_back(OriginEvent::QueryCompleted(*id, query.cid, res));
                    false
                }
                Poll::Ready(Err(e)) => {
                    error!("receiver error fetching from origin: {:?}", e);
                    false
                }
                Poll::Pending => true,
            });

        if let Some(event) = self.event_queue.pop_front() {
            return Poll::Ready(NetworkBehaviourAction::GenerateEvent(event));
        }

        Poll::Pending
    }
}
