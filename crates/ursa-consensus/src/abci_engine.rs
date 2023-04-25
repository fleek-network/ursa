use anyhow::{bail, Result};
use resolve_path::PathResolveExt;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Notify};
use tokio::{pin, select, time};
use tracing::warn;
use ursa_utils::shutdown::ShutdownController;

use narwhal_types::{Batch, Transaction};

// Tendermint Types
use crate::AbciQueryQuery;
//use tendermint_abci::{Client as AbciClient, ClientBuilder};
use tm_protos::abci::{
    RequestBeginBlock, RequestDeliverTx, RequestEndBlock, RequestInfo, RequestInitChain,
    RequestQuery, ResponseQuery,
};
use tm_protos::types::Header;
use ursa_application::{Client as AbciClient, ClientBuilder, ExecutionResponse};

pub const CHANNEL_CAPACITY: usize = 1_000;

pub struct Engine {
    /// The path to the UDS of the ABCI app.
    abci_uds: PathBuf,
    /// The blocking Abci client connected to the application layer, for executing certificates.
    client: AbciClient,
    /// The blocking abci client for used only for querys, holds info connection only.
    req_client: AbciClient,
    /// The last block height, initialized to the application's latest block by default.
    last_block_height: i64,
    tx_abci_queries: mpsc::Sender<(oneshot::Sender<ResponseQuery>, AbciQueryQuery)>,
    tx_certificates: mpsc::Sender<Vec<Batch>>,
    rx_abci_queries: mpsc::Receiver<(oneshot::Sender<ResponseQuery>, AbciQueryQuery)>,
    rx_certificates: mpsc::Receiver<Vec<Batch>>,
    reconfigure_notifier: Arc<Notify>,
}

impl Engine {
    pub async fn new(abci_uds: PathBuf) -> Self {
        //Todo(dalton): handle his elegently. We are getting here too fast and application server
        // is not starting in time.
        time::sleep(time::Duration::from_millis(500)).await;
        let resolved_path = abci_uds.resolve();
        let mut client = ClientBuilder::default()
            .connect(resolved_path.clone())
            .await
            .unwrap();

        let last_block_height = client
            .info(RequestInfo::default())
            .await
            .map(|res| res.last_block_height)
            .unwrap_or_default();

        let (tx_abci_queries, rx_abci_queries) = mpsc::channel(CHANNEL_CAPACITY);
        let (tx_certificates, rx_certificates) = mpsc::channel(CHANNEL_CAPACITY);
        let reconfigure_notifier = Arc::new(Notify::new());

        // Instantiate a new client to not be locked in an Info connection.
        let client = ClientBuilder::default()
            .connect(resolved_path.clone())
            .await
            .unwrap();

        let req_client = ClientBuilder::default()
            .connect(resolved_path)
            .await
            .unwrap();

        Self {
            abci_uds,
            client,
            req_client,
            last_block_height,
            tx_abci_queries,
            tx_certificates,
            rx_abci_queries,
            rx_certificates,
            reconfigure_notifier,
        }
    }

    pub async fn start(&mut self, shutdown_controller: ShutdownController) -> Result<()> {
        self.init_chain().await?;

        loop {
            let shutdown_future = shutdown_controller.notify.notified();
            pin!(shutdown_future);
            select! {
                Some(batches) = self.rx_certificates.recv() => {
                    self.handle_cert(batches).await?;
                },
                Some((tx, req)) = self.rx_abci_queries.recv() => {
                    self.handle_abci_query(tx, req).await?;
                }
                _ = shutdown_future => break,
                else => break,
            }
        }

        Ok(())
    }

    pub fn get_abci_queries_sender(
        &self,
    ) -> mpsc::Sender<(oneshot::Sender<ResponseQuery>, AbciQueryQuery)> {
        self.tx_abci_queries.clone()
    }

    pub fn get_certificates_sender(&self) -> mpsc::Sender<Vec<Batch>> {
        self.tx_certificates.clone()
    }

    pub fn get_reconfigure_notify(&self) -> Arc<Notify> {
        self.reconfigure_notifier.clone()
    }

    /// On each new certificate, increment the block height to proposed and run through the
    /// BeginBlock -> DeliverTx for each tx in the certificate -> EndBlock -> Commit event loop.
    async fn handle_cert(&mut self, batch: Vec<Batch>) -> Result<()> {
        // Increment block.
        let proposed_block_height = self.last_block_height + 1;

        // Save it for next time.
        self.last_block_height = proposed_block_height;

        // Drive the app through the event loop.
        self.begin_block(proposed_block_height).await?;
        // If the results of execution are to change the epoch wait until after block is committed.
        let change_epoch = self.deliver_batch(batch).await?;
        self.end_block(proposed_block_height).await?;
        self.commit().await?;

        if change_epoch {
            self.reconfigure_notifier.notify_waiters();
        }

        Ok(())
    }

    /// Handles ABCI queries coming to the primary and forwards them to the ABCI App. Each
    /// handle call comes with a Sender channel which is used to send the response back to the
    /// Primary and then to the client.
    ///
    /// Client => Primary => handle_cert => ABCI App => Primary => Client
    async fn handle_abci_query(
        &mut self,
        tx: oneshot::Sender<ResponseQuery>,
        req: AbciQueryQuery,
    ) -> Result<()> {
        let req_height = req.height.unwrap_or(0);
        let req_prove = req.prove.unwrap_or(false);

        let resp = self
            .req_client
            .query(RequestQuery {
                data: req.data.into(),
                path: req.path,
                height: req_height as i64,
                prove: req_prove,
            })
            .await?;
        if let Err(err) = tx.send(resp) {
            bail!("{:?}", err);
        }
        Ok(())
    }

    /// Reconstructs the batch corresponding to the provided Primary's certificate from the Workers' stores
    /// and proceeds to deliver each tx to the App over ABCI's DeliverTx endpoint.
    /// Returns true if the epoch should change based on the results of execution.
    async fn deliver_batch(&mut self, batches: Vec<Batch>) -> Result<bool> {
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

// Tendermint Lifecycle Helpers.
impl Engine {
    /// Calls the `InitChain` hook on the app, ignores "already initialized" errors.
    pub async fn init_chain(&mut self) -> Result<()> {
        let mut client = ClientBuilder::default()
            .connect(self.abci_uds.clone().resolve())
            .await?;
        if let Err(err) = client.init_chain(RequestInitChain::default()).await {
            // Ignore errors about the chain being already initialized.
            if err.to_string().contains("already initialized") {
                warn!("{}", err);
                return Ok(());
            }
            bail!(err)
        };
        Ok(())
    }

    /// Calls the `BeginBlock` hook on the ABCI app. For now, it just makes a request with
    /// the new block height.
    // If we wanted to, we could add additional arguments to be forwarded from the Consensus
    // to the App logic on the beginning of each block.
    async fn begin_block(&mut self, height: i64) -> Result<()> {
        let req = RequestBeginBlock {
            header: Some(Header {
                height,
                ..Default::default()
            }),
            ..Default::default()
        };

        self.client.begin_block(req).await?;
        Ok(())
    }

    /// Calls the `DeliverTx` hook on the ABCI app. Returns true if the result of the tx says the epoch should change.
    async fn deliver_tx(&mut self, tx: Transaction) -> Result<bool> {
        let response = self.client.deliver_tx(RequestDeliverTx { tx }).await?;

        if let Ok(ExecutionResponse::ChangeEpoch) = serde_json::from_slice(&response.data) {
            return Ok(true);
        }
        Ok(false)
    }

    /// Calls the `EndBlock` hook on the ABCI app. For now, it just makes a request with
    /// the proposed block height.
    // If we wanted to, we could add additional arguments to be forwarded from the Consensus
    // to the App logic on the end of each block.
    async fn end_block(&mut self, height: i64) -> Result<()> {
        let req = RequestEndBlock { height };
        self.client.end_block(req).await?;
        Ok(())
    }

    /// Calls the `Commit` hook on the ABCI app.
    async fn commit(&mut self) -> Result<()> {
        self.client.commit().await?;
        Ok(())
    }
}
