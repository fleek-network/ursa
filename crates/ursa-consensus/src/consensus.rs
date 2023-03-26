use anyhow::{Context, Result};
use arc_swap::ArcSwap;
use narwhal_node::NodeStorage;
use narwhal_types::Batch;
use resolve_path::PathResolveExt;
use std::cell::RefCell;
use std::sync::Arc;
use std:: path::PathBuf;
use tendermint_proto::abci::ResponseQuery;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::Notify;
use narwhal_config::{Committee, Epoch, Parameters, WorkerCache};

use ursa_utils::transactions::{build_transaction, decode_committee};
use ursa_application::types::Query;
use crate::config::GenesisCommittee;
use crate::{
    config::ConsensusConfig,
    execution::Execution,
    narwhal::{NarwhalArgs, NarwhalService},
};
use crate::{AbciQueryQuery};
// what do we need for this file to work and be complete?
// - A mechanism to dynamically move the epoch forward and changing the committee dynamically.
//    Each epoch has a fixed committee. The committee only changes per epoch.
// - Manage different stores for each epoch.
// - Restore the latest committee and epoch information from a persistent database.
// - Restart the narwhal service for each new epoch.
// - Execution engine with mpsc or a normal channel to deliver the transactions to abci.
//
// TBD:
// - Do we need a catch up process here in this file?
// - Where will we be doing the communication with execution engine from this file?
//
// But where should the config come from?

// Dalton Notes:
// - Epoch time should be gathered from application layer along with new committee on epoch change

/// The consensus layer, which wraps a narwhal service and moves the epoch forward.

/// The default channel capacity.

pub struct Consensus {
    /// The state of the current Narwhal epoch
    epoch_state: RefCell<EpochState>,
    /// The narwhal configuration.
    narwhal_args: NarwhalArgs,
    /// This should not change ever so should be held in the outer layer
    parameters: Parameters,
    /// Narwhal execution state
    execution_state: Arc<Execution>,
    /// Path to the database used by the narwhal implementation
    store_path: PathBuf,
    /// Path to the genesis committee json
    genesis_committee: PathBuf,
    /// Timestamp of the narwhal certificate that caused an epoch change
    /// is sent through this channel to notify that epoch chould change
    reconfigure_notify: Arc<Notify>,
    /// Called from the shutdown function to notify the start event loop to
    /// exit.
    shutdown_notify: Notify,
    /// Used to query application state.
    tx_abci_queries: mpsc::Sender<(oneshot::Sender<ResponseQuery>, AbciQueryQuery)>
}

struct EpochState {
    /// The current epoch
    epoch: Epoch,
    /// The length of the epoch
    epoch_time: u64,
    /// The Narwhal service for the current epoch
    narwhal: Option<NarwhalService>,
}

impl Consensus {
    pub fn new(
        config: ConsensusConfig,
        tx_abci_queries: mpsc::Sender<(oneshot::Sender<ResponseQuery>, AbciQueryQuery)>,
        tx_certificates : mpsc::Sender<Vec<Batch>>,
        reconfigure_notify: Arc<Notify>
    ) -> Result<Self> {
        let narwhal_args = NarwhalArgs::load(config.clone())?;
        //TODO(dalton): Genesis epoch time needs to come from a genesis file.
        let epoch_time = 86400000;
        //TODO(dalton): Checkpoint system instead of starting from epoch 0 everytime
        let epoch = 0;

        //TODO(dalton): Should the ABCI engine also become ExecutionState? Is there value in keeping them seperated?
        let execution_state = Execution::new(epoch, tx_certificates);

        let epoch_state = EpochState {
            epoch,
            epoch_time,
            narwhal: None,
        };

        let store_path = config.store_path.resolve().into_owned();
        std::fs::create_dir_all(&store_path).context("Could not create the store directory.")?;

        Ok(Consensus {
            epoch_state: RefCell::new(epoch_state),
            narwhal_args,
            parameters: config.parameters,
            execution_state: Arc::new(execution_state),
            store_path,
            genesis_committee: config.genesis_committee,
            reconfigure_notify,
            shutdown_notify: Notify::new(),
            tx_abci_queries
        })
    }

    async fn start_current_epoch(&self) {
        let epoch = { self.epoch_state.borrow().epoch };
        //make or open store specific to current epoch
        let mut store_path = self.store_path.clone();
        store_path.set_file_name(format!("narwhal-store-{}", epoch));
        let store = NodeStorage::reopen(store_path);

        //Pull Epoch Info
        let (committee, worker_cache) = if epoch == 0 {
            let bytes = std::fs::read(self.genesis_committee.clone())
                .with_context(|| {
                    format!(
                        "Count not read the genesis committee file at: {:?}",
                        self.genesis_committee
                    )
                })
                .unwrap();
            let genesis_committee: GenesisCommittee = serde_json::from_slice(bytes.as_slice())
                .with_context(|| {
                    format!(
                        "Could not deserialize the genesis committee file at: {:?}",
                        self.genesis_committee
                    )
                })
                .unwrap();
            let committee = Arc::new(Committee::from(&genesis_committee));
            let worker_cache = Arc::new(WorkerCache::from(&genesis_committee));
            (committee, worker_cache)
        } else {
            ////TEMPORARY
            let bytes = std::fs::read(self.genesis_committee.clone())
                .with_context(|| {
                    format!(
                        "Count not read the genesis committee file at: {:?}",
                        self.genesis_committee
                    )
                })
                .unwrap();
            let genesis_committee: GenesisCommittee = serde_json::from_slice(bytes.as_slice())
                .with_context(|| {
                    format!(
                        "Could not deserialize the genesis committee file at: {:?}",
                        self.genesis_committee
                    )
                })
                .unwrap();
            let committee = Arc::new(Committee::from(&genesis_committee));
            let worker_cache = Arc::new(WorkerCache::from(&genesis_committee));
            (committee, worker_cache)
            ////TEMPORARY
        };

        let service = NarwhalService::new(
            self.narwhal_args.clone(),
            store,
            Arc::new(ArcSwap::new(committee)),
            Arc::new(ArcSwap::new(worker_cache)),
            self.parameters.clone(),
        );

        service.start(self.execution_state.clone()).await;

        {
            self.epoch_state.borrow_mut().narwhal = Some(service);
        }
    }

    async fn move_to_next_epoch(&self) {
        {
            let mut epoch_state_mut = self.epoch_state.borrow_mut();

            if let Some(narwhal) = epoch_state_mut.narwhal.take() {
                narwhal.shutdown().await;
                epoch_state_mut.epoch += 1;
            } else if epoch_state_mut.epoch > 0 {
                //if narwhal is None only increment if its not epoch 0 because that would be genesis
                epoch_state_mut.epoch += 1;
            }
        };
        self.start_current_epoch().await
    }

    pub async fn start(&mut self) {
        self.start_current_epoch().await;
        loop {
            let reconfigure_future = self.reconfigure_notify.notified();
            let shutdown_future = self.shutdown_notify.notified();
            tokio::pin!(shutdown_future);
            tokio::pin!(reconfigure_future);
            tokio::select! {
                _ = shutdown_future => {
                    break
                }
                _ = reconfigure_future => {
                    self.move_to_next_epoch().await;
                    continue
                }
                // reconfigure event shoud continue instead of breaking
            }
        }
    }

    pub async fn shutdown(&mut self) {
        self.shutdown_notify.notify_waiters();
    }
}

// TODO(dalton): make this pullable from genesis file
const epoch_address: &str = "0xAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAC";
const registry_address: &str = "0xCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC";

//Application Query Helpers
impl Consensus {
    async fn get_epoch_info(&self) -> Result<()> {
        //Build transaction
        let (function, txn) = build_transaction(epoch_address, "getCurrentEpochInfo():(uint256 epoch, uint256 currentEpochEndStamp, tuple[](string publicKey, string primaryAddress, string workerAddress,string workerMempool,string workerPublicKey,string networkKey))", &[])?;
        let query = Query::EthCall(txn);

        let query_string = serde_json::to_string(&query)?;

        let abci_query = AbciQueryQuery {
        data: query_string,
        path: "".to_string(),
        height: None,
        prove: None,
        };

        // Construct one shot channel to recieve response
        let (tx, rx) = oneshot::channel();

        // Send and wait for response
        self.tx_abci_queries.send((tx, abci_query)).await?;
        let response = rx.await.with_context(|| "Failure querying abci")?;

        // decode response
        let decoded_response = function.decode_output(&response.value)?;

        // Safe unwrap. But will panic if epoch ever gets passed max u64.Which is 584942417 years with 1 millisecond epochs
        let epoch = decoded_response[0].clone().into_int().unwrap().as_u64();
        let epoch_timestamp = decoded_response[1].clone().into_int().unwrap().as_u64();
        //safe unwrap
        let (committee, worker_cache) = decode_committee(decoded_response[3].clone().into_array().unwrap(), epoch);

        Ok(())
    }
}
