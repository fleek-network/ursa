use anyhow::{bail, Result};
use std::net::SocketAddr;
use tokio::sync::mpsc::{Receiver};
use tokio::sync::oneshot::Sender as OneShotSender;
use tracing::warn;

use narwhal_types::{Batch, Transaction};

// Tendermint Types
use tendermint_abci::{Client as AbciClient, ClientBuilder};
use tendermint_proto::abci::{
    RequestBeginBlock, RequestDeliverTx, RequestEndBlock, RequestInfo, RequestInitChain,
    RequestQuery, ResponseQuery,
};
use tendermint_proto::types::Header;
use crate::AbciQueryQuery;

pub struct Engine {
    /// The address of the ABCI app
    pub app_address: SocketAddr,
    /// Messages received from the ABCI Server to be forwarded to the engine.
    pub rx_abci_queries: Receiver<(OneShotSender<ResponseQuery>, AbciQueryQuery)>,
    /// The last block height, initialized to the application's latest block by default
    pub last_block_height: i64,
    pub client: AbciClient,
    pub req_client: AbciClient,
}

impl Engine {
    pub fn new(
        app_address: SocketAddr,
        rx_abci_queries: Receiver<(OneShotSender<ResponseQuery>, AbciQueryQuery)>,
    ) -> Self {
        let mut client = ClientBuilder::default().connect(app_address).unwrap();

        let last_block_height = client
            .info(RequestInfo::default())
            .map(|res| res.last_block_height)
            .unwrap_or_default();

        // Instantiate a new client to not be locked in an Info connection
        let client = ClientBuilder::default().connect(app_address).unwrap();
        let req_client = ClientBuilder::default().connect(app_address).unwrap();
        Self {
            app_address,
            rx_abci_queries,
            last_block_height,
            client,
            req_client,
        }
    }

    /// Receives an ordered list of certificates and apply any application-specific logic.
    pub async fn run(&mut self, mut rx_output: Receiver<Vec<Batch>>) -> Result<()> {
        self.init_chain()?;

        loop {
            tokio::select! {
                Some(batches) = rx_output.recv() => {
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

    /// On each new certificate, increment the block height to proposed and run through the
    /// BeginBlock -> DeliverTx for each tx in the certificate -> EndBlock -> Commit event loop.
    fn handle_cert(&mut self, batch: Vec<Batch>) -> Result<()> {
        // increment block
        let proposed_block_height = self.last_block_height + 1;

        // save it for next time
        self.last_block_height = proposed_block_height;

        // drive the app through the event loop
        self.begin_block(proposed_block_height)?;
        self.deliver_batch(batch)?;
        self.end_block(proposed_block_height)?;
        self.commit()?;
        Ok(())
    }

    /// Handles ABCI queries coming to the primary and forwards them to the ABCI App. Each
    /// handle call comes with a Sender channel which is used to send the response back to the
    /// Primary and then to the client.
    ///
    /// Client => Primary => handle_cert => ABCI App => Primary => Client
    fn handle_abci_query(
        &mut self,
        tx: OneShotSender<ResponseQuery>,
        req: AbciQueryQuery,
    ) -> Result<()> {
        let req_height = req.height.unwrap_or(0);
        let req_prove = req.prove.unwrap_or(false);

        let resp = self.req_client.query(RequestQuery {
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
    fn deliver_batch(&mut self, batches: Vec<Batch>) -> Result<()> {
        // Deliver
        batches.into_iter().try_for_each(|batch| {
            batch.transactions.into_iter().try_for_each(|txn| {
                self.deliver_tx(txn)
            })?;
            Ok::<_, anyhow::Error>(())
        })?;

        Ok(())
    }
}

// Tendermint Lifecycle Helpers
impl Engine {
    /// Calls the `InitChain` hook on the app, ignores "already initialized" errors.
    pub fn init_chain(&mut self) -> Result<()> {
        let mut client = ClientBuilder::default().connect(self.app_address)?;
        match client.init_chain(RequestInitChain::default()) {
            Ok(_) => {
            }
            Err(err) => {
                tracing::error!("{:?}", err);
                 // ignore errors about the chain being uninitialized
                if err.to_string().contains("already initialized") {
                    warn!("{}", err);
                    return Ok(());
                }
                bail!(err)
            }
        };
        Ok(())
    }

    /// Calls the `BeginBlock` hook on the ABCI app. For now, it just makes a request with
    /// the new block height.
    // If we wanted to, we could add additional arguments to be forwarded from the Consensus
    // to the App logic on the beginning of each block.
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

    /// Calls the `DeliverTx` hook on the ABCI app.
    fn deliver_tx(&mut self, tx: Transaction) -> Result<()> {
        self.client.deliver_tx(RequestDeliverTx { tx })?;
        Ok(())
    }

    /// Calls the `EndBlock` hook on the ABCI app. For now, it just makes a request with
    /// the proposed block height.
    // If we wanted to, we could add additional arguments to be forwarded from the Consensus
    // to the App logic on the end of each block.
    fn end_block(&mut self, height: i64) -> Result<()> {
        let req = RequestEndBlock { height };
        self.client.end_block(req)?;
        Ok(())
    }

    /// Calls the `Commit` hook on the ABCI app.
    fn commit(&mut self) -> Result<()> {
        self.client.commit()?;
        Ok(())
    }
}

