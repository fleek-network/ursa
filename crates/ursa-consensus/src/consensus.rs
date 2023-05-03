use anyhow::{Context, Result};
use bytes::Bytes;
use fastcrypto::traits::KeyPair;
use futures::lock::Mutex;
use narwhal_config::{Committee, Epoch, Parameters, WorkerCache};
use narwhal_node::NodeStorage;
use narwhal_types::{Batch, TransactionProto, TransactionsClient};
use resolve_path::PathResolveExt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tendermint_proto::abci::ResponseQuery;
use tokio::sync::{mpsc, oneshot, Notify};
use tokio::{pin, select, task, time};
use tracing::error;

use crate::AbciQueryQuery;
use crate::{
    config::ConsensusConfig,
    execution::Execution,
    narwhal::{NarwhalArgs, NarwhalService},
};
use ursa_application::types::Query;
use ursa_utils::evm::epoch_manager::{
    decode_committee, decode_epoch_info_return, get_epoch_info_call, get_signal_epoch_change_call,
};

// what do we need for this file to work and be complete?
// - A mechanism to dynamically move the epoch forward and changing the committee dynamically.
//    Each epoch has a fixed committee. The committee only changes per epoch.
// - Manage different stores for each epoch.
// - Restore the latest committee and epoch information from a persistent database.
// - Restart the narwhal service for each new epoch.
// - Execution engine with mpsc or a normal channel to deliver the transactions to abci.

const STORE_NAME: &str = "narwhal-epochs";

/// The consensus layer, which wraps a narwhal service and moves the epoch forward.
pub struct Consensus {
    /// The state of the current Narwhal epoch.
    epoch_state: Mutex<Option<EpochState>>,
    /// The narwhal configuration.
    narwhal_args: NarwhalArgs,
    /// This should not change ever so should be held in the outer layer.
    parameters: Parameters,
    /// Narwhal execution state.
    execution_state: Arc<Execution>,
    /// Path to the database used by the narwhal implementation.
    store_path: PathBuf,
    /// The address to the worker mempool.
    mempool_address: String,
    /// Timestamp of the narwhal certificate that caused an epoch change
    /// is sent through this channel to notify that epoch chould change.
    reconfigure_notify: Arc<Notify>,
    /// Called from the shutdown function to notify the start event loop to
    /// exit.
    shutdown_notify: Notify,
    /// Used to query application state.
    tx_abci_queries: mpsc::Sender<(oneshot::Sender<ResponseQuery>, AbciQueryQuery)>,
}

/// This struct contains mutable state only for the current epoch.
struct EpochState {
    /// The Narwhal service for the current epoch.
    narwhal: NarwhalService,
}

impl Consensus {
    pub fn new(
        config: ConsensusConfig,
        tx_abci_queries: mpsc::Sender<(oneshot::Sender<ResponseQuery>, AbciQueryQuery)>,
        tx_certificates: mpsc::Sender<Vec<Batch>>,
        reconfigure_notify: Arc<Notify>,
        mempool_address: String,
    ) -> Result<Self> {
        let narwhal_args = NarwhalArgs::load(config.clone())?;

        let execution_state = Execution::new(tx_certificates);

        let mut store_path = config.store_path.clone();
        store_path.push(STORE_NAME);
        let absolute_store_path = store_path.resolve().into_owned();
        std::fs::create_dir_all(&absolute_store_path)
            .context("Could not create the store directory.")?;

        Ok(Consensus {
            epoch_state: Mutex::new(None),
            narwhal_args,
            parameters: config.parameters,
            execution_state: Arc::new(execution_state),
            store_path: absolute_store_path,
            mempool_address,
            reconfigure_notify,
            shutdown_notify: Notify::new(),
            tx_abci_queries,
        })
    }

    async fn start_current_epoch(&self) {
        // Pull epoch info.
        // TODO(dalton): This shouldnt ever fail but we should just retry if it does.
        let (committee, worker_cache, epoch, epoch_end_time) = self.get_epoch_info().await.unwrap();

        // If the this node is not on the committee, dont start narwhal start edge node logic.
        if !committee
            .authorities()
            .contains_key(self.narwhal_args.primary_keypair.public())
        {
            self.run_edge_node().await;
            return;
        }

        // Make or open store specific to current epoch.
        let mut store_path = self.store_path.clone();
        store_path.push(format!("{epoch}"));

        let service = NarwhalService::new(
            self.narwhal_args.clone(),
            store_path,
            committee,
            worker_cache,
            self.parameters.clone(),
        );

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        // TODO(Parsa/Dalton): We might want to add a assert requirment that ensures epoch time > now
        // This logic works now as joining in late would have you signal and continute consuming certificates
        // But may be good to put this restriction on this function to design around, So our checkpoint can hold this
        // assertion true before calling start epoch.
        let until_epoch_ends: u64 = (epoch_end_time as u128)
            .saturating_sub(now)
            .try_into()
            .unwrap();

        // Start the timer to signal when your node thinks its ready to change epochs.
        let time_until_epoch_change = Duration::from_millis(until_epoch_ends);
        self.wait_to_signal_epoch_change(time_until_epoch_change)
            .await;

        service.start(self.execution_state.clone()).await;

        *self.epoch_state.lock().await = Some(EpochState { narwhal: service })
    }

    async fn move_to_next_epoch(&self) {
        {
            let epoch_state_mut = self.epoch_state.lock().await.take();
            if let Some(state) = epoch_state_mut {
                state.narwhal.shutdown().await
            }
        }
        self.start_current_epoch().await
    }

    async fn wait_to_signal_epoch_change(&self, time_until_change: Duration) {
        let primary_public_key = self.narwhal_args.primary_keypair.public().clone();
        let mempool_address = self.mempool_address.clone();
        task::spawn(async move {
            time::sleep(time_until_change).await;
            // We shouldnt panic here lets repeatedly try.
            loop {
                time::sleep(Duration::from_secs(1)).await;

                let txn = match serde_json::to_vec(&get_signal_epoch_change_call(
                    primary_public_key.to_string(),
                )) {
                    Ok(txn) => txn,
                    Err(_) => {
                        error!("Error signaling epoch change, trying again");
                        continue;
                    }
                };

                let request = TransactionProto {
                    transaction: Bytes::from(txn),
                };

                let mut client = match TransactionsClient::connect(mempool_address.clone()).await {
                    Ok(client) => client,
                    Err(e) => {
                        error!("Error building client to signal epoch change {:?}", e);
                        continue;
                    }
                };

                if client.submit_transaction(request).await.is_ok() {
                    break;
                }
                error!("Error signaling epoch change trying again");
            }
        });
    }

    pub async fn start(&mut self) {
        self.start_current_epoch().await;
        loop {
            let reconfigure_future = self.reconfigure_notify.notified();
            let shutdown_future = self.shutdown_notify.notified();
            pin!(shutdown_future);
            pin!(reconfigure_future);
            select! {
                _ = shutdown_future => {
                    break
                }
                _ = reconfigure_future => {
                    self.move_to_next_epoch().await;
                    continue
                }
            }
        }
    }

    pub async fn shutdown(&mut self) {
        self.shutdown_notify.notify_waiters();
    }

    pub async fn run_edge_node(&self) {
        // Todo(Dalton): Edge node logic
    }
}

// Application Query Helpers.
impl Consensus {
    async fn get_epoch_info(&self) -> Result<(Committee, WorkerCache, Epoch, u64)> {
        // Build transaction.
        let txn = get_epoch_info_call();
        let query = Query::EthCall(txn);

        let query_string = serde_json::to_string(&query)?;

        let abci_query = AbciQueryQuery {
            data: query_string,
            path: "".to_string(),
            height: None,
            prove: None,
        };

        // Construct one shot channel to recieve response.
        let (tx, rx) = oneshot::channel();

        // Send and wait for response.
        self.tx_abci_queries.send((tx, abci_query)).await?;
        let response = rx.await.with_context(|| "Failure querying abci")?;

        // Decode response.
        let epoch_info = decode_epoch_info_return(response.value)?;

        let epoch = epoch_info.epoch.as_u64();
        let epoch_timestamp = epoch_info.current_epoch_end_ms.as_u64();

        let (committee, worker_cache) = decode_committee(epoch_info.committee_members, epoch);

        Ok((committee, worker_cache, epoch, epoch_timestamp))
    }
}
