use crate::{AbciQueryQuery, BroadcastTxQuery};
use anyhow::Context;
use futures::SinkExt;
use tendermint_proto::abci::ResponseQuery;
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::{channel as oneshot_channel, Sender as OneShotSender};
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tracing::{error, warn};
use warp::{Filter, Rejection, Reply};

use std::net::SocketAddr;

/// Simple HTTP API server which listens to messages on:
/// * `broadcast_tx`: forwards them to Narwhal's mempool/worker socket, which will proceed to put
/// it in the consensus process and eventually forward it to the application.
/// * `abci_query`: forwards them over a channel to a handler (typically the application).
pub struct AbciApi<T> {
    mempool_address: SocketAddr,
    tx: Sender<(OneShotSender<T>, AbciQueryQuery)>,
}

impl<T: Send + Sync + std::fmt::Debug> AbciApi<T> {
    pub fn new(
        mempool_address: SocketAddr,
        tx: Sender<(OneShotSender<T>, AbciQueryQuery)>,
    ) -> Self {
        Self {
            mempool_address,
            tx,
        }
    }
}

impl AbciApi<ResponseQuery> {
    pub fn routes(self) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        let route_broadcast_tx = warp::path("broadcast_tx")
            .and(warp::query::<BroadcastTxQuery>())
            .and_then(move |req: BroadcastTxQuery| async move {
                warn!("broadcast_tx: {:?}", req);

                let stream = TcpStream::connect(self.mempool_address)
                    .await
                    .with_context(|| {
                        format!(
                            "ROUTE_BROADCAST_TX failed to connect to {}",
                            self.mempool_address
                        )
                    })
                    .unwrap();
                let mut transport = Framed::new(stream, LengthDelimitedCodec::new());

                if let Err(e) = transport.send(req.tx.clone().into()).await {
                    Ok::<_, Rejection>(format!("ERROR IN: broadcast_tx: {:?}. Err: {}", req, e))
                } else {
                    Ok::<_, Rejection>(format!("broadcast_tx: {:?}", req))
                }
            });

        let route_abci_query = warp::path("abci_query")
            .and(warp::query::<AbciQueryQuery>())
            .and_then(move |req: AbciQueryQuery| {
                let tx_abci_queries = self.tx.clone();
                async move {
                    let (tx, rx) = oneshot_channel();
                    match tx_abci_queries.send((tx, req.clone())).await {
                        Ok(_) => {}
                        Err(err) => error!("Error forwarding abci query: {}", err),
                    };
                    let resp = rx.await.unwrap();

                    // Return the value
                    Ok::<_, Rejection>(resp.value)
                }
            });

        route_broadcast_tx.or(route_abci_query)
    }
}
