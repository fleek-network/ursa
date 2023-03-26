use anyhow::{bail, Result};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::{Mutex, Notify};
use tracing::{error, warn};

use narwhal_types::{Batch, Transaction};

//Tendermint Types
use crate::AbciQueryQuery;
use tendermint_abci::{Client as AbciClient, ClientBuilder};
use tendermint_proto::abci::{
    RequestBeginBlock, RequestDeliverTx, RequestEndBlock, RequestInfo, RequestInitChain,
    RequestQuery, ResponseQuery,
};
use tendermint_proto::types::Header;
use ursa_application::ExecutionResponse;

pub const CHANNEL_CAPACITY: usize = 1_000;

pub struct Engine {
    /// The address of the ABCI app
    app_address: SocketAddr,
    /// The blocking Abci client connected to the application layer
    client: AbciClient,
    /// The last block height, initialized to the application's latest block by default
    last_block_height: i64,
    tx_abci_queries: mpsc::Sender<(oneshot::Sender<ResponseQuery>, AbciQueryQuery)>,
    tx_certificates: mpsc::Sender<Vec<Batch>>,
    rx_abci_queries: mpsc::Receiver<(oneshot::Sender<ResponseQuery>, AbciQueryQuery)>,
    rx_certificates: mpsc::Receiver<Vec<Batch>>,
    reconfigure_notifier: Arc<Notify>,
}

impl Engine {
    pub fn new(app_address: SocketAddr) -> Self {
        let mut client = ClientBuilder::default().connect(app_address).unwrap();

        let last_block_height = client
            .info(RequestInfo::default())
            .map(|res| res.last_block_height)
            .unwrap_or_default();

        let (tx_abci_queries, rx_abci_queries) = mpsc::channel(CHANNEL_CAPACITY);
        let (tx_certificates, rx_certificates) = mpsc::channel(CHANNEL_CAPACITY);
        let reconfigure_notifier = Arc::new(Notify::new());

        // Instantiate a new client to not be locked in an Info connection
        let client = ClientBuilder::default().connect(app_address).unwrap();
        Self {
            app_address,
            client,
            last_block_height,
            tx_abci_queries,
            tx_certificates,
            rx_abci_queries: rx_abci_queries,
            rx_certificates: rx_certificates,
            reconfigure_notifier,
        }
    }

    pub async fn start(&mut self) -> Result<()> {                       
        self.init_chain()?;

                loop {
                    tokio::select! {
                        Some(batches) = self.rx_certificates.recv() => {
                            self.handle_cert(batches)?;
                        },
                        Some((tx, req)) = self.rx_abci_queries.recv() => {
                            self.handle_abci_query(tx, req)?;
                        }
                        else => break,
                    }
         }

         Ok(())
    }

    pub fn get_abci_queries_sender(&self) -> mpsc::Sender<(oneshot::Sender<ResponseQuery>, AbciQueryQuery)> {
        self.tx_abci_queries.clone()
    }

    pub fn get_certificates_sender(&self) -> mpsc::Sender<Vec<Batch>> {
        self.tx_certificates.clone()
    }

    pub fn get_reconfigure_notify(&self) -> Arc<Notify> {
        self.reconfigure_notifier.clone()
    }

    ///On each new certificate, increment the block height to proposed and run through the
    ///BeginBlock -> DeliverTx for each tx in the certificate -> EndBlock -> Commit event loop.
    fn handle_cert(&mut self, batch: Vec<Batch>) -> Result<()> {
        // increment block
        let proposed_block_height = self.last_block_height + 1;

        // save it for next time
        self.last_block_height = proposed_block_height;


        // drive the app through the event loop
        self.begin_block(proposed_block_height)?;
        // if the results of execution are to change the epoch wait until after block is committed
        let change_epoch = self.deliver_batch(batch)?;
        self.end_block(proposed_block_height)?;
        self.commit()?;

        if change_epoch {
            self.reconfigure_notifier.notify_waiters();
        }

        Ok(())
    }

    ///Handles ABCI queries coming to the primary and forwards them to the ABCI App. Each
    ///handle call comes with a Sender channel which is used to send the response back to the
    ///Primary and then to the client.
    ///
    ///Client => Primary => handle_cert => ABCI App => Primary => Client
    fn handle_abci_query(
        &mut self,
        tx: oneshot::Sender<ResponseQuery>,
        req: AbciQueryQuery,
    ) -> Result<()> {
        let req_height = req.height.unwrap_or(0);
        let req_prove = req.prove.unwrap_or(false);

        let resp = self.client.query(RequestQuery {
            data: req.data.into(),
            path: req.path,
            height: req_height as i64,
            prove: req_prove,
        })?;

        if let Err(err) = tx.send(resp) {
            bail!("{:?}", err);
        }
        Ok(())
    }

    /// Reconstructs the batch corresponding to the provided Primary's certificate from the Workers' stores
    /// and proceeds to deliver each tx to the App over ABCI's DeliverTx endpoint.
    /// returns true if the epoch should change based on the results of execution
    fn deliver_batch(&mut self, batches: Vec<Batch>) -> Result<bool> {
        //Deliver
        let mut change_epoch = false;

        for batch in batches {
            for txn in batch.transactions {
                let results = self.deliver_tx(txn)?;
                if results {
                    change_epoch = true;
                }
            }
        }

        Ok(change_epoch)
    }
}

//Tendermint Lifecycle Helpers
impl Engine {
    ///Calls the `InitChain` hook on the app, ignores "already initialized" errors.
    pub fn init_chain(&mut self) -> Result<()> {
        let mut client = ClientBuilder::default().connect(self.app_address)?;
        if let Err(err) = client.init_chain(RequestInitChain::default()) {
            error!("{:?}", err);
            //ignore errors about the chain being uninitialized
            if err.to_string().contains("already initialized") {
                warn!("{}", err);
                return Ok(());
            }
            bail!(err)
        };
        Ok(())
    }

    ///Calls the `BeginBlock` hook on the ABCI app. For now, it just makes a request with
    ///the new block height.
    //If we wanted to, we could add additional arguments to be forwarded from the Consensus
    //to the App logic on the beginning of each block.
    fn begin_block(&mut self, height: i64) -> Result<()> {
        let req = RequestBeginBlock {
            header: Some(Header {
                height,
                ..Default::default()
            }),
            ..Default::default()
        };

        self.client.begin_block(req)?;
        Ok(())
    }

    ///Calls the `DeliverTx` hook on the ABCI app. Returns true if the result of the tx says the epoch should change
    fn deliver_tx(&mut self, tx: Transaction) -> Result<bool> {
        let response = self
            .client
            .deliver_tx(RequestDeliverTx { tx })?;

        if let Ok(ExecutionResponse::ChangeEpoch) = serde_json::from_slice(&response.data) {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    ///Calls the `EndBlock` hook on the ABCI app. For now, it just makes a request with
    ///the proposed block height.
    //If we wanted to, we could add additional arguments to be forwarded from the Consensus
    //to the App logic on the end of each block.
    fn end_block(&mut self, height: i64) -> Result<()> {
        let req = RequestEndBlock { height };
        self.client.end_block(req)?;
        Ok(())
    }

    ///Calls the `Commit` hook on the ABCI app.
    fn commit(&mut self) -> Result<()> {
        self.client.commit()?;
        Ok(())
    }   

}

// mod vx {
//     use super::*;
//     use tokio::sync::{mpsc, oneshot};

//     struct Engine {
//         app_address: SocketAddr,
//         reconfigure_notifier: Arc<Notify>,
//         tx_abci_queries: mpsc::Sender<(oneshot::Sender<ResponseQuery>, AbciQueryQuery)>,
//         tx_certificates: mpsc::Sender<Vec<Batch>>,
//         rx_abci_queries: Option<mpsc::Receiver<(oneshot::Sender<ResponseQuery>, AbciQueryQuery)>>,
//         rx_certificates: Option<mpsc::Receiver<Vec<Batch>>>,
//         thread_handler: Option<std::thread::JoinHandle<()>>
//     }

//     impl Engine {
//         pub fn new(app_address: SocketAddr, reconfigure_notifier: Arc<Notify>) -> Self {
//             let (tx_abci_queries, mut rx_abci_queries) = mpsc::channel(1000);
//             let (tx_certificates, mut rx_certificates) = mpsc::channel(1000);

//             Self {
//                 app_address,
//                 reconfigure_notifier,
//                 tx_abci_queries,
//                 tx_certificates,
//                 rx_abci_queries: Some(rx_abci_queries),
//                 rx_certificates: Some(rx_certificates),
//                 thread_handler: None
//             }
//         }

//         pub fn start(&mut self) {
//             // if we're already running the thread, don't start anything.
//             if self.thread_handler.is_some() {
//                 return;
//             }

//             let mut rx_certificates = self.rx_certificates.take().unwrap();
//             let mut rx_abci_queries = self.rx_abci_queries.take().unwrap();

//             let thread_handler = std::thread::spawn(move || {
//                 let mut client = ClientBuilder::default().connect(self.app_address).unwrap();
//                 let mut state = EngineState::new(self.app_address.clone(), self.reconfigure_notifier.clone());

//                 futures::executor::block_on(async move {
//                     state.init_chain();
                
//                     loop {
//                         tokio::select! {
//                             Some(batches) = rx_certificates.recv() => {
//                                 state.handle_cert(batches);
//                             },
//                             Some((tx, req)) = rx_abci_queries.recv() => {
//                                 state.handle_abci_query(tx, req);
//                             }
//                             else => break,
//                         }
//                     }

//                 });
//             });

//             self.thread_handler = Some(thread_handler);
//         }

//         pub fn get_abci_queries_sender(&self) -> mpsc::Sender<(oneshot::Sender<ResponseQuery>, AbciQueryQuery)> {
//             self.tx_abci_queries
//         }

//         pub fn get_certificates_sender(&self) -> mpsc::Sender<Vec<Batch>> {
//             self.tx_certificates
//         }
//         /// Consume self to shutdown.
//         pub fn shutdown(mut self) {
//            self.thread_handler.take();
//         }
//     }

//     struct EngineState {
//         ///The address of the ABCI app
//         pub app_address: SocketAddr,
//         pub last_block_height: Mutex<i64>,
//         pub client: Mutex<AbciClient>,
//         pub req_client: Mutex<AbciClient>,
//         /// Used to notify the consensus the epoch has changed and it needs to reconfigure.
//         pub reconfigure_notify: Arc<Notify>,
//     }

//     impl EngineState {
//         pub fn new(app_address: SocketAddr, reconfigure_notify: Arc<Notify>) -> Self {
//             let mut client = ClientBuilder::default().connect(app_address).unwrap();
    
//             let last_block_height = client
//                 .info(RequestInfo::default())
//                 .map(|res| res.last_block_height)
//                 .unwrap_or_default();
    
//             //Instantiate a new client to not be locked in an Info connection
//             let client = ClientBuilder::default().connect(app_address).unwrap();
//             let req_client = ClientBuilder::default().connect(app_address).unwrap();
//             Self {
//                 app_address,
//                 last_block_height: Mutex::new(last_block_height),
//                 client: Mutex::new(client),
//                 req_client: Mutex::new(req_client),
//                 reconfigure_notify,
//             }
//         }
    
//         ///On each new certificate, increment the block height to proposed and run through the
//         ///BeginBlock -> DeliverTx for each tx in the certificate -> EndBlock -> Commit event loop.
//         async fn handle_cert(&self, batch: Vec<Batch>) -> Result<()> {
//             // increment block height
//             let block_height = {
//                 let mut lock = *self.last_block_height.lock().await;
//                 lock += 1;
//                 lock
//             };
    
//             // drive the app through the event loop
//             self.begin_block(block_height).await?;
//             // if the results of execution are to change the epoch wait until after block is committed
//             let change_epoch = self.deliver_batch(batch).await?;
//             self.end_block(block_height).await?;
//             self.commit().await?;
    
//             if change_epoch {
//                 self.reconfigure_notify.notify_waiters();
//             }
    
//             Ok(())
//         }
    
//         ///Handles ABCI queries coming to the primary and forwards them to the ABCI App. Each
//         ///handle call comes with a Sender channel which is used to send the response back to the
//         ///Primary and then to the client.
//         ///
//         ///Client => Primary => handle_cert => ABCI App => Primary => Client
//         async fn handle_abci_query(
//             &self,
//             tx: OneShotSender<ResponseQuery>,
//             req: AbciQueryQuery,
//         ) -> Result<()> {
//             let req_height = req.height.unwrap_or(0);
//             let req_prove = req.prove.unwrap_or(false);
    
//             let resp = self.req_client.lock().await.query(RequestQuery {
//                 data: req.data.into(),
//                 path: req.path,
//                 height: req_height as i64,
//                 prove: req_prove,
//             })?;
    
//             if let Err(err) = tx.send(resp) {
//                 bail!("{:?}", err);
//             }
//             Ok(())
//         }
    
//         /// Reconstructs the batch corresponding to the provided Primary's certificate from the Workers' stores
//         /// and proceeds to deliver each tx to the App over ABCI's DeliverTx endpoint.
//         /// returns true if the epoch should change based on the results of execution
//         async fn deliver_batch(&self, batches: Vec<Batch>) -> Result<bool> {
//             //Deliver
//             let mut change_epoch = false;
    
//             for batch in batches {
//                 for txn in batch.transactions {
//                     let results = self.deliver_tx(txn).await?;
//                     if results {
//                         change_epoch = true;
//                     }
//                 }
//             }
    
//             Ok(change_epoch)
//         }
//     }
// }