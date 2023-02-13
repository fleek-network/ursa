// Copyright 2022-2023 Fleek Network
// SPDX-License-Identifier: Apache-2.0, MIT

use crate::config::{GenesisAuthority, GenesisCommittee};
use crate::keys::LoadOrCreate;
use crate::{config::ConsensusConfig, validator::Validator};
use anyhow::Context;
use arc_swap::ArcSwap;
use fastcrypto::traits::KeyPair as _;
use multiaddr::Multiaddr;
use mysten_metrics::RegistryService;
use narwhal_config::{Committee, Parameters, WorkerCache, WorkerInfo};
use narwhal_crypto::{KeyPair, NetworkKeyPair};
use narwhal_executor::ExecutionState;
use narwhal_node::{primary_node::PrimaryNode, worker_node::WorkerNode, NodeStorage};
use prometheus::Registry;
use rand::thread_rng;
use resolve_path::PathResolveExt;
use serde::de::DeserializeOwned;
use serde::Serialize;
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
    pub fn load(config: ConsensusConfig) -> anyhow::Result<Self> {
        // Load or create all of the keys.
        let mut rng = thread_rng();
        let primary_keypair = KeyPair::load_or_create(&mut rng, &config.keypair.resolve())?;
        let primary_network_keypair =
            NetworkKeyPair::load_or_create(&mut rng, &config.network_keypair.resolve())?;
        let worker_keypair =
            NetworkKeyPair::load_or_create(&mut rng, &config.worker[0].keypair.resolve())?;

        // Load the committee.json file.
        let genesis_committee: GenesisCommittee =
            load_or_create_json(config.genesis_committee.resolve(), || GenesisCommittee {
                authorities: [(
                    primary_keypair.public().clone(),
                    GenesisAuthority {
                        stake: 1,
                        primary_address: config.address.clone(),
                        network_key: primary_network_keypair.public().clone(),
                        workers: [WorkerInfo {
                            name: worker_keypair.public().clone(),
                            transactions: config.worker[0].transaction.clone(),
                            worker_address: config.worker[0].address.clone(),
                        }],
                    },
                )]
                .into_iter()
                .collect(),
            })
            .context("Could not load the genesis committee.")?;

        let committee = Arc::new(Committee::from(&genesis_committee));
        let worker_cache = Arc::new(WorkerCache::from(&genesis_committee));

        // create the directory for the store.
        let store_path = config.store_path.resolve().into_owned();
        std::fs::create_dir_all(&store_path).context("Could not create the store directory.")?;

        Ok(Self {
            parameters: config.parameters,
            primary_keypair,
            primary_network_keypair,
            worker_keypair,
            primary_address: config.address,
            worker_address: config.worker[0].address.clone(),
            store_path,
            committee: Arc::new(ArcSwap::new(committee)),
            worker_cache: Arc::new(ArcSwap::new(worker_cache)),
            registry_service: RegistryService::new(Registry::new()),
        })
    }
}

// TODO(qti3e) Move this to somewhere else.
fn load_or_create_json<T, P: AsRef<std::path::Path>, F>(path: P, default_fn: F) -> anyhow::Result<T>
where
    F: Fn() -> T,
    T: Serialize + DeserializeOwned,
{
    let path = path.as_ref();

    if path.exists() {
        let bytes = std::fs::read(path)
            .with_context(|| format!("Could not read the file: '{:?}'", path))?;

        serde_json::from_slice(bytes.as_slice())
            .with_context(|| format!("Could not deserialize the file: '{:?}'", path))
    } else {
        let value = default_fn();
        let bytes = serde_json::to_vec_pretty(&value).context("Serialization failed.")?;

        let parent = path
            .parent()
            .context("Could not resolve the parent directory.")?;

        std::fs::create_dir_all(parent)
            .with_context(|| format!("Could not create the directory: '{:?}'", parent))?;

        std::fs::write(path, bytes).with_context(|| format!("Could not write to '{:?}'.", path))?;

        Ok(value)
    }
}
