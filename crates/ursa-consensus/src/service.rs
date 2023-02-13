// Copyright 2022-2023 Fleek Network
// SPDX-License-Identifier: Apache-2.0, MIT

use crate::{config::ConsensusConfig, validator::Validator};
use arc_swap::ArcSwap;
use fastcrypto::traits::KeyPair as _;
use multiaddr::Multiaddr;
use mysten_metrics::RegistryService;
use narwhal_config::{Committee, Parameters, WorkerCache};
use narwhal_crypto::{KeyPair, NetworkKeyPair};
use narwhal_executor::ExecutionState;
use narwhal_node::{primary_node::PrimaryNode, worker_node::WorkerNode, NodeStorage};
use std::{path::PathBuf, sync::Arc};
use tokio::{sync::Mutex, time::Instant};
use tracing::{error, info};

/// Maximum number of times we retry to start the primary or the worker, before we panic.
const MAX_RETRIES: u32 = 2;

/// Manages running the narwhal and bullshark as a service.
pub struct ConsensusService {
    arguments: ServiceArgs,
    store: NodeStorage,
    primary: PrimaryNode,
    worker_node: WorkerNode,
    status: Mutex<Status>,
}

/// Arguments used to run a consensus service.
pub struct ServiceArgs {
    pub parameters: Parameters,
    pub primary_keypair: KeyPair,
    pub primary_network_keypair: NetworkKeyPair,
    pub worker_keypair: NetworkKeyPair,
    pub primary_address: Multiaddr,
    pub worker_address: Multiaddr,
    pub store_path: PathBuf,
    pub committee: Arc<ArcSwap<Committee>>,
    pub worker_cache: Arc<ArcSwap<WorkerCache>>,
    pub registry_service: RegistryService,
}

#[derive(PartialEq)]
enum Status {
    Running,
    Stopped,
}

impl ConsensusService {
    /// Create a new consensus service using the provided arguments.
    pub fn new(arguments: ServiceArgs) -> Self {
        let store = NodeStorage::reopen(&arguments.store_path);

        let primary = PrimaryNode::new(
            arguments.parameters.clone(),
            true,
            arguments.registry_service.clone(),
        );

        let worker_node = WorkerNode::new(
            0,
            arguments.parameters.clone(),
            arguments.registry_service.clone(),
        );

        Self {
            arguments,
            store,
            primary,
            worker_node,
            status: Mutex::new(Status::Stopped),
        }
    }

    /// Start the consensus process by starting the Narwhal's primary and worker.
    ///
    /// # Panics
    ///
    /// This function panics if it can not start either the Primary or the Worker.
    pub async fn start<State>(&self, state: State)
    where
        State: ExecutionState + Send + Sync + 'static,
    {
        let mut status = self.status.lock().await;
        if *status == Status::Running {
            error!("ConsensusService is already running.");
            return;
        }

        let name = self.arguments.primary_keypair.public().clone();
        let execution_state = Arc::new(state);

        let epoch = self.arguments.committee.load().epoch();
        info!("Starting ConsensusService for epoch {}", epoch);

        let mut running = false;
        for i in 0..MAX_RETRIES {
            info!("Trying to start the Narwhal Primary...");
            if i > 0 {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }

            if let Err(e) = self
                .primary
                .start(
                    self.arguments.primary_keypair.copy(),
                    self.arguments.primary_network_keypair.copy(),
                    self.arguments.committee.clone(),
                    self.arguments.worker_cache.clone(),
                    &self.store,
                    execution_state.clone(),
                )
                .await
            {
                error!("Unable to start Narwhal Primary: {:?}", e);
            } else {
                running = true;
                break;
            }
        }
        if !running {
            panic!("Failed to start the Narwhal Primary after {MAX_RETRIES} tries",);
        }

        let mut running = false;
        for i in 0..MAX_RETRIES {
            info!("Trying to start the Narwhal Worker...");
            if i > 0 {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }

            if let Err(e) = self
                .worker_node
                .start(
                    name.clone(),
                    self.arguments.worker_keypair.copy(),
                    self.arguments.committee.clone(),
                    self.arguments.worker_cache.clone(),
                    &self.store,
                    Validator::new(),
                    None,
                )
                .await
            {
                error!("Unable to start Narwhal Worker: {:?}", e);
            } else {
                running = true;
                break;
            }
        }

        if !running {
            panic!("Failed to start the Narwhal Worker after {MAX_RETRIES} tries",);
        }

        *status = Status::Running;
    }

    /// Shutdown the primary and the worker and waits until nodes have shutdown.
    pub async fn shutdown(&self) {
        let mut status = self.status.lock().await;
        if *status == Status::Stopped {
            error!("Narwhal shutdown was called but node is not running.");
            return;
        }

        let now = Instant::now();
        let epoch = self.arguments.committee.load().epoch();
        info!("Shutting down Narwhal epoch {:?}", epoch);

        self.worker_node.shutdown().await;
        self.worker_node.shutdown().await;

        info!(
            "Narwhal shutdown for epoch {:?} is complete - took {} seconds",
            epoch,
            now.elapsed().as_secs_f64()
        );

        *status = Status::Stopped;
    }
}

impl ServiceArgs {
    /// Load a service arguments from a raw configuration.
    pub fn load(_config: ConsensusConfig) -> anyhow::Result<Self> {
        todo!()
    }
}
