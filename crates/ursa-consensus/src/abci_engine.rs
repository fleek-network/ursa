use anyhow::{bail, Result};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tokio::sync::oneshot::Sender as OneShotSender;
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

pub struct Engine {
    ///The address of the ABCI app
    pub app_address: SocketAddr,
    ///Messages received from the ABCI Server to be forwarded to the engine.
    //  pub rx_abci_queries: Receiver<(OneShotSender<ResponseQuery>, AbciQueryQuery)>,
    ///Certificates recieved from consensus to pass to application
    //  pub rx_certificates: Receiver<Vec<Batch>>,
    ///The last block height, initialized to the application's latest block by default
    pub last_block_height: Mutex<i64>,
    pub client: Mutex<AbciClient>,
    pub req_client: Mutex<AbciClient>,
    /// Used to notify the consensus the epoch has changed and it needs to reconfigure.
    pub reconfigure_notify: Arc<Notify>,
}

impl Engine {
    pub fn new(app_address: SocketAddr, reconfigure_notify: Arc<Notify>) -> Self {
        let mut client = ClientBuilder::default().connect(app_address).unwrap();

        let last_block_height = client
            .info(RequestInfo::default())
            .map(|res| res.last_block_height)
            .unwrap_or_default();

        //Instantiate a new client to not be locked in an Info connection
        let client = ClientBuilder::default().connect(app_address).unwrap();
        let req_client = ClientBuilder::default().connect(app_address).unwrap();
        Self {
            app_address,
            last_block_height: Mutex::new(last_block_height),
            client: Mutex::new(client),
            req_client: Mutex::new(req_client),
            reconfigure_notify,
        }
    }

    ///Receives an ordered list of certificates and apply any application-specific logic.
    pub async fn run(
        &self,
        mut rx_abci_queries: Receiver<(OneShotSender<ResponseQuery>, AbciQueryQuery)>,
        mut rx_certificates: Receiver<Vec<Batch>>,
    ) -> Result<()> {
        self.init_chain()?;
        loop {
            tokio::select! {
                Some(batches) = rx_certificates.recv() => {
                    self.handle_cert(batches).await?;
                },
                Some((tx, req)) = rx_abci_queries.recv() => {
                    self.handle_abci_query(tx, req).await?;
                }
                else => break,
            }
        }
        Ok(())
    }

    ///On each new certificate, increment the block height to proposed and run through the
    ///BeginBlock -> DeliverTx for each tx in the certificate -> EndBlock -> Commit event loop.
    async fn handle_cert(&self, batch: Vec<Batch>) -> Result<()> {
        // increment block height
        let block_height = {
            let mut lock = *self.last_block_height.lock().await;
            lock += 1;
            lock
        };

        // drive the app through the event loop
        self.begin_block(block_height).await?;
        // if the results of execution are to change the epoch wait until after block is committed
        let change_epoch = self.deliver_batch(batch).await?;
        self.end_block(block_height).await?;
        self.commit().await?;

        if change_epoch {
            self.reconfigure_notify.notify_waiters();
        }

        Ok(())
    }

    ///Handles ABCI queries coming to the primary and forwards them to the ABCI App. Each
    ///handle call comes with a Sender channel which is used to send the response back to the
    ///Primary and then to the client.
    ///
    ///Client => Primary => handle_cert => ABCI App => Primary => Client
    async fn handle_abci_query(
        &self,
        tx: OneShotSender<ResponseQuery>,
        req: AbciQueryQuery,
    ) -> Result<()> {
        let req_height = req.height.unwrap_or(0);
        let req_prove = req.prove.unwrap_or(false);

        let resp = self.req_client.lock().await.query(RequestQuery {
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
    async fn deliver_batch(&self, batches: Vec<Batch>) -> Result<bool> {
        //Deliver
        let mut change_epoch = false;

        for batch in batches {
            for txn in batch.transactions {
                let results = self.deliver_tx(txn).await?;
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
    pub fn init_chain(&self) -> Result<()> {
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
    async fn begin_block(&self, height: i64) -> Result<()> {
        let req = RequestBeginBlock {
            header: Some(Header {
                height,
                ..Default::default()
            }),
            ..Default::default()
        };

        self.client.lock().await.begin_block(req)?;
        Ok(())
    }

    ///Calls the `DeliverTx` hook on the ABCI app. Returns true if the result of the tx says the epoch should change
    async fn deliver_tx(&self, tx: Transaction) -> Result<bool> {
        let response = self
            .client
            .lock()
            .await
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
    async fn end_block(&self, height: i64) -> Result<()> {
        let req = RequestEndBlock { height };
        self.client.lock().await.end_block(req)?;
        Ok(())
    }

    ///Calls the `Commit` hook on the ABCI app.
    async fn commit(&self) -> Result<()> {
        self.client.lock().await.commit()?;
        Ok(())
    }
}
