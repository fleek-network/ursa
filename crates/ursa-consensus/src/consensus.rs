use anyhow::{anyhow, Context, Result};
use bytes::Bytes;
use fastcrypto::traits::KeyPair;
use futures::lock::Mutex;
use narwhal_config::{
    Authority, Committee, Epoch, Parameters, WorkerCache, WorkerIndex, WorkerInfo,
};
use narwhal_node::NodeStorage;
use narwhal_types::{TransactionProto, TransactionsClient};
use resolve_path::PathResolveExt;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::Notify;
use tokio::{pin, select, task, time};
use tracing::error;
use ursa_application::interface::application::{
    ApplicationQuery, ApplicationUpdate, ExecutionData, Query, Transaction, TransactionResponse,
    TransactionType,
};

use crate::{
    config::ConsensusConfig,
    execution::Execution,
    narwhal::{NarwhalArgs, NarwhalService},
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
    tx_abci_queries: ApplicationQuery,
}

/// This struct contains mutable state only for the current epoch.
struct EpochState {
    /// The Narwhal service for the current epoch.
    narwhal: NarwhalService,
}

impl Consensus {
    pub fn new(
        config: ConsensusConfig,
        tx_abci_queries: ApplicationQuery,
        tx_certificates: ApplicationUpdate,
        mempool_address: String,
    ) -> Result<Self> {
        let narwhal_args = NarwhalArgs::load(config.clone())?;
        let reconfigure_notify = Arc::new(Notify::new());
        let execution_state = Execution::new(tx_certificates, reconfigure_notify.clone());

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
            .authorities
            .contains_key(self.narwhal_args.primary_keypair.public())
        {
            self.run_edge_node().await;
            return;
        }

        // Make or open store specific to current epoch.
        let mut store_path = self.store_path.clone();
        store_path.push(format!("{epoch}"));
        let store = NodeStorage::reopen(store_path);

        let service = NarwhalService::new(
            self.narwhal_args.clone(),
            store,
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
        let primary_public_key = self.narwhal_args.primary_network_keypair.public().clone();
        let mempool_address = self.mempool_address.clone();
        task::spawn(async move {
            time::sleep(time_until_change).await;
            // We shouldnt panic here lets repeatedly try.
            loop {
                time::sleep(Duration::from_secs(1)).await;
                // TODO: Get nonce and sign transaction
                let transaction = Transaction {
                    sender: primary_public_key.clone(),
                    nonce: 0,
                    transaction_type: TransactionType::ChangeEpoch,
                    signature: None,
                };

                let txn_bytes = match bincode::serialize(&transaction) {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        error!("Error decoding transaction to signal epoch change");
                        continue;
                    }
                };

                let request = TransactionProto {
                    transaction: Bytes::from(txn_bytes),
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
        let response = match self
            .tx_abci_queries
            .run(Transaction::get_query(TransactionType::Query(
                Query::CurrentEpochInfo,
            )))
            .await
        {
            Ok(TransactionResponse::Success(ExecutionData::EpochInfo(info))) => info,
            _ => return Err(anyhow!("Unable to get epoch info")),
        };

        let committee = Committee {
            epoch: response.epoch,
            authorities: response
                .committee
                .iter()
                .map(|node| {
                    let authority = Authority {
                        stake: 1,
                        primary_address: node.domain.clone(),
                        network_key: node.network_key.clone(),
                    };
                    (node.public_key.clone(), authority)
                })
                .collect(),
        };

        let worker_cache = WorkerCache {
            epoch: response.epoch,
            workers: response
                .committee
                .iter()
                .map(|node| {
                    let mut worker_index = BTreeMap::new();
                    node.workers
                        .iter()
                        .map(|worker| WorkerInfo {
                            name: worker.public_key.clone(),
                            transactions: worker.mempool.clone(),
                            worker_address: worker.address.clone(),
                        })
                        .enumerate()
                        .for_each(|(index, worker)| {
                            worker_index.insert(index as u32, worker);
                        });
                    (node.public_key.clone(), WorkerIndex(worker_index))
                })
                .collect(),
        };

        Ok((committee, worker_cache, response.epoch, response.epoch_end))
    }
}
