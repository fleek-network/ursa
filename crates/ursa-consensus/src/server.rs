use crate::{AbciQueryQuery, BroadcastTxQuery};
use multiaddr::{Multiaddr, Protocol};
use narwhal_types::{TransactionProto, TransactionsClient};
use std::fmt::Debug;
use tendermint_proto::abci::ResponseQuery;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::{channel as oneshot_channel, Sender as OneShotSender};
use tracing::{debug, error};
use warp::{Filter, Rejection, Reply};

/// Simple HTTP API server which listens to messages on:
/// * `broadcast_tx`: forwards them to Narwhal's mempool/worker socket, which will proceed to put
/// it in the consensus process and eventually forward it to the application.
/// * `abci_query`: forwards them over a channel to a handler (typically the application).
pub struct AbciApi<T> {
    mempool_address: String,
    tx: Sender<(OneShotSender<T>, AbciQueryQuery)>,
}

impl<T: Send + Sync + Debug> AbciApi<T> {
    pub async fn new(
        mempool_address: Multiaddr,
        tx: Sender<(OneShotSender<T>, AbciQueryQuery)>,
    ) -> Self {
        let mempool_port = mempool_address
            .iter()
            .find_map(|proto| match proto {
                Protocol::Tcp(port) => Some(port),
                _ => None,
            })
            .expect("Expected tcp url for worker mempool");

        let mempool_address_string = format!("http://0.0.0.0:{}", mempool_port);

        Self {
            mempool_address: mempool_address_string,
            tx,
        }
    }
}

impl AbciApi<ResponseQuery> {
    pub fn routes(self) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        let route_broadcast_tx = warp::path("broadcast_tx")
            .and(warp::query::<BroadcastTxQuery>())
            .and_then(move |req: BroadcastTxQuery| {
                let address = self.mempool_address.clone();
                async move {
                    debug!("broadcast_tx: {:?}", req);
                    let mut client = TransactionsClient::connect(address).await.unwrap();
                    let request = TransactionProto {
                        transaction: req.tx.clone().into(),
                    };

                    if let Err(e) = client.submit_transaction(request).await {
                        Ok::<_, Rejection>(format!(
                            "ERROR IN: broadcast_tx: {:?}. Err: {:?}",
                            req, e
                        ))
                    } else {
                        Ok::<_, Rejection>(format!("broadcast_tx: {:?}", req))
                    }
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