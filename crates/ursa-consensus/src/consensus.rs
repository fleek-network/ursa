use anyhow::{Context, Result};
use arc_swap::ArcSwap;
use narwhal_node::NodeStorage;
use narwhal_types::Batch;
use resolve_path::PathResolveExt;
use std::cell::RefCell;
use std::sync::Arc;
use std::{net::SocketAddr, path::PathBuf};
use tendermint_proto::abci::ResponseQuery;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::oneshot::Sender as OneShotSender;
use tokio::sync::Notify;
use tokio::task;
use tracing::error;

use narwhal_config::{Committee, Epoch, Parameters, WorkerCache};

use crate::config::GenesisCommittee;
use crate::{
    config::ConsensusConfig,
    execution::Execution,
    narwhal::{NarwhalArgs, NarwhalService},
};
use crate::{AbciQueryQuery, Engine};
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
pub const CHANNEL_CAPACITY: usize = 1_000;

pub struct Consensus {
    /// The state of the current Narwhal epoch
    epoch_state: RefCell<EpochState>,
    /// The narwhal configuration.
    narwhal_args: NarwhalArgs,
    /// This should not change ever so should be held in the outer layer
    parameters: Parameters,
    /// Narwhal execution state
    execution_state: Arc<Execution>,
    /// Engine for moving application layer and responding to queries
    abci_engine: Arc<Engine>,
    /// Receiver worker passes certificates too
    rx_certificates: Option<Receiver<Vec<Batch>>>,
    /// Receiver that forwards querys to application
    rx_abci_queries: Option<Receiver<(OneShotSender<ResponseQuery>, AbciQueryQuery)>>,
    /// Path to the database used by the narwhal implementation
    store_path: PathBuf,
    /// Path to the genesis committee json
    genesis_committee: PathBuf,
    /// Timestamp of the narwhal certificate that caused an epoch change
    /// is sent through this channel to notify that epoch chould change
    reconfigure_notify: Arc<Notify>,
    // Called from the shutdown function to notify the start event loop to
    // exit.
    shutdown_notify: Notify,
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
        app_domain: String,
        rx_abci_queries: Receiver<(OneShotSender<ResponseQuery>, AbciQueryQuery)>,
    ) -> Result<Self> {
        let narwhal_args = NarwhalArgs::load(config.clone())?;
        //TODO(dalton): Genesis epoch time needs to come from a genesis file.
        let epoch_time = 86400000;
        //TODO(dalton): Checkpoint system instead of starting from epoch 0 everytime
        let epoch = 0;

        let mut app_address = app_domain.parse::<SocketAddr>()?;
        app_address.set_ip("0.0.0.0".parse()?);
        let reconfigure_notify = Arc::new(Notify::new());

        let abci_engine = Engine::new(app_address, reconfigure_notify.clone());

        let (tx_certificates, rx_certificates) = channel(CHANNEL_CAPACITY);

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
            abci_engine: Arc::new(abci_engine),
            rx_certificates: Some(rx_certificates),
            rx_abci_queries: Some(rx_abci_queries),
            store_path,
            genesis_committee: config.genesis_committee,
            reconfigure_notify,
            shutdown_notify: Notify::new(),
        })
    }

    async fn start_current_epoch(&self) {
        let epoch = { self.epoch_state.borrow().epoch };
        //make or open store specific to current epoch
        let mut store_path = self.store_path.clone();
        store_path.set_file_name(format!("narwhal-store-{}", epoch));
        let store = NodeStorage::reopen(store_path);

        //Pull new committee and worker cache
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
        let query_reciever = self.rx_abci_queries.take().unwrap();
        let certificate_reciever = self.rx_certificates.take().unwrap();
        let engine_clone = self.abci_engine.clone();
        let abci_engine_task = task::spawn(async move {
            if let Err(err) = engine_clone.run(query_reciever, certificate_reciever).await {
                error!("[consensus_engine_task] - {:?}", err)
            }
        });

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

        abci_engine_task.abort();
    }

    pub async fn shutdown(&mut self) {
        self.shutdown_notify.notify_waiters();
    }
}
