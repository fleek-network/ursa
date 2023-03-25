// Copyright 2022-2023 Fleek Network
// SPDX-License-Identifier: Apache-2.0, MIT
use crate::keys::LoadOrCreate;
use crate::{config::ConsensusConfig, validator::Validator};
use arc_swap::ArcSwap;
use fastcrypto::traits::KeyPair as _;
use multiaddr::Multiaddr;
use mysten_metrics::RegistryService;
use narwhal_config::{Committee, Parameters, WorkerCache};
use narwhal_crypto::{KeyPair, NetworkKeyPair};
use narwhal_executor::ExecutionState;
use narwhal_node::{primary_node::PrimaryNode, worker_node::WorkerNode, NodeStorage};
use prometheus::Registry;
use rand::thread_rng;
use resolve_path::PathResolveExt;
use std::sync::Arc;
use tokio::{sync::Mutex, time::Instant};
use tracing::{error, info};

/// Maximum number of times we retry to start the primary or the worker, before we panic.
const MAX_RETRIES: u32 = 2;

/// Manages running the narwhal and bullshark as a service.
pub struct NarwhalService {
    arguments: NarwhalArgs,
    store: NodeStorage,
    primary: PrimaryNode,
    worker_node: WorkerNode,
    committee: Arc<ArcSwap<Committee>>,
    worker_cache: Arc<ArcSwap<WorkerCache>>,
    status: Mutex<Status>,
}

/// Arguments used to run a consensus service.
pub struct NarwhalArgs {
    pub primary_keypair: KeyPair,
    pub primary_network_keypair: NetworkKeyPair,
    pub worker_keypair: NetworkKeyPair,
    pub primary_address: Multiaddr,
    pub worker_address: Multiaddr,
    pub registry_service: RegistryService,
}

#[derive(PartialEq)]
enum Status {
    Running,
    Stopped,
}

impl NarwhalService {
    /// Create a new narwhal service using the provided arguments.
    pub fn new(
        arguments: NarwhalArgs,
        store: NodeStorage,
        committee: Arc<ArcSwap<Committee>>,
        worker_cache: Arc<ArcSwap<WorkerCache>>,
        parameters: Parameters,
    ) -> Self {
        let primary =
            PrimaryNode::new(parameters.clone(), true, arguments.registry_service.clone());

        let worker_node = WorkerNode::new(0, parameters, arguments.registry_service.clone());

        Self {
            arguments,
            store,
            primary,
            worker_node,
            committee,
            worker_cache,
            status: Mutex::new(Status::Stopped),
        }
    }

    /// Start the narwhal process by starting the Narwhal's primary and worker.
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
            error!("NarwhalService is already running.");
            return;
        }

        let name = self.arguments.primary_keypair.public().clone();
        let execution_state = Arc::new(state);

        let epoch = self.committee.load().epoch();
        info!("Starting NarwhalService for epoch {}", epoch);

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
                    self.committee.clone(),
                    self.worker_cache.clone(),
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
                    self.committee.clone(),
                    self.worker_cache.clone(),
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
        let epoch = self.committee.load().epoch();
        info!("Shutting down Narwhal epoch {:?}", epoch);

        self.worker_node.shutdown().await;
        self.primary.shutdown().await;

        info!(
            "Narwhal shutdown for epoch {:?} is complete - took {} seconds",
            epoch,
            now.elapsed().as_secs_f64()
        );

        *status = Status::Stopped;
    }
}

impl Drop for NarwhalService {
    fn drop(&mut self) {
        futures::executor::block_on(self.shutdown());
    }
}

impl NarwhalArgs {
    //TODO(dalton): should this be renamed, maybe load_genesis?
    /// Load a service arguments from a raw configuration.
    pub fn load(config: ConsensusConfig) -> anyhow::Result<Self> {
        // Load or create all of the keys.
        let mut rng = thread_rng();
        let primary_keypair = KeyPair::load_or_create(&mut rng, &config.keypair.resolve())?;
        let primary_network_keypair =
            NetworkKeyPair::load_or_create(&mut rng, &config.network_keypair.resolve())?;
        let worker_keypair =
            NetworkKeyPair::load_or_create(&mut rng, &config.worker[0].keypair.resolve())?;

        Ok(Self {
            primary_keypair,
            primary_network_keypair,
            worker_keypair,
            primary_address: config.address,
            worker_address: config.worker[0].address.clone(),
            registry_service: RegistryService::new(Registry::new()),
        })
    }
}

impl Clone for NarwhalArgs {
    fn clone(&self) -> Self {
        Self {
            primary_keypair: self.primary_keypair.copy(),
            primary_network_keypair: self.primary_network_keypair.copy(),
            worker_keypair: self.worker_keypair.copy(),
            primary_address: self.primary_address.clone(),
            worker_address: self.worker_address.clone(),
            registry_service: self.registry_service.clone(),
        }
    }
}
