// Copyright 2022-2023 Fleek Network
// SPDX-License-Identifier: Apache-2.0, MIT

use anyhow::Result;
use std::{path::PathBuf, sync::Arc};

use arc_swap::ArcSwap;
use async_trait::async_trait;
use config::{NarwhalConfig, NetworkKeyPair, NodeConfig, ValidatorKeyPair};
use execution::Execution;
use fastcrypto::traits::KeyPair as PrimaryKeyPair;
use multiaddr::Multiaddr;
use mysten_metrics::RegistryService;
use narwhal_config::{
    Authority, Committee, Epoch, Parameters, SharedWorkerCache, Stake, WorkerCache, WorkerId,
    WorkerIndex, WorkerInfo,
};
use narwhal_crypto::NetworkKeyPair as NarwhalNetworkKeyPair;
use narwhal_executor::ExecutionState;
use narwhal_node::{primary_node::PrimaryNode, worker_node::WorkerNodes, NodeStorage};
use narwhal_worker::TransactionValidator;
use prometheus::Registry;
use tokio::sync::mpsc::channel;
use validator::Validator;

pub mod config;
pub mod execution;
pub mod validator;

pub const URSA_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct ValidatorActor {
    stake: Stake,
    keypair: ValidatorKeyPair,
    primary_address: Multiaddr,
    worker_keypair: NetworkKeyPair,
    network_keypair: NetworkKeyPair,
}

#[async_trait]
pub trait Consensus {
    async fn start<S, V: TransactionValidator>(
        self,
        committee: Arc<Committee>,
        shared_worker_cache: SharedWorkerCache,
        execution_state: Arc<S>,
        tx_validator: V,
    ) -> Result<()>
    where
        S: ExecutionState + Send + Sync + 'static;
}

pub struct Narwhal {
    keypair: ValidatorKeyPair,
    parameters: Parameters,
    internal_consensus: bool,
    storage_base_path: PathBuf,
    // todo(botch): abstract this elsewhere
    validators: Vec<ValidatorActor>,
    network_keypair: NetworkKeyPair,
    ids_and_keypairs: Vec<(WorkerId, NarwhalNetworkKeyPair)>,
    worker_nodes: narwhal_node::worker_node::WorkerNodes,
    primary_node: narwhal_node::primary_node::PrimaryNode,
}

#[async_trait]
impl Consensus for Narwhal {
    async fn start<S, V: TransactionValidator>(
        self,
        committee: Arc<Committee>,
        shared_worker_cache: SharedWorkerCache,
        execution_state: Arc<S>,
        tx_validator: V,
    ) -> Result<()>
    where
        S: ExecutionState + Send + Sync + 'static,
    {
        let mut store_path = self.storage_base_path.clone();
        store_path.push(format!("epoch{}", committee.epoch()));
        let store = NodeStorage::reopen(store_path);
        let primary_key = self.keypair.authority_keypair().public().clone();

        self.primary_node
            .start(
                self.keypair.authority_keypair().copy(),
                self.network_keypair.keypair().copy(),
                Arc::new(ArcSwap::new(committee.clone().into())),
                shared_worker_cache.clone(),
                &store,
                execution_state,
            )
            .await?;

        self.worker_nodes
            .start(
                primary_key,
                self.ids_and_keypairs,
                Arc::new(ArcSwap::new(committee.clone().into())),
                shared_worker_cache,
                &store,
                tx_validator,
            )
            .await?;

        Ok(())
    }
}

impl Narwhal {
    pub fn new(config: NarwhalConfig, validators: Vec<ValidatorActor>) -> Self {
        let primary_node = PrimaryNode::new(
            config.parameters.clone(),
            config.internal_consensus,
            config.registry_service.clone(),
        );
        let worker_nodes =
            WorkerNodes::new(config.registry_service.clone(), config.parameters.clone());

        Self {
            parameters: config.parameters,
            keypair: config.keypair,
            internal_consensus: config.internal_consensus,
            storage_base_path: config.storage_base_path,
            network_keypair: config.network_keypair,
            ids_and_keypairs: config.ids_and_keypairs,
            worker_nodes,
            primary_node,
            validators,
        }
    }
}

pub struct Service {
    config: NodeConfig,
}

impl Service {
    pub async fn start(config: &NodeConfig) -> Result<()> {
        let validators: Vec<ValidatorActor> = vec![ValidatorActor {
            stake: 1,
            keypair: config.keypair.clone(),
            primary_address: config.network_address.clone(),
            worker_keypair: config.worker_keypair.clone(),
            network_keypair: config.network_keypair.clone(),
        }];

        let authorities = validators
            .iter()
            .map(|validator| {
                (
                    validator.keypair.authority_keypair().public().clone(),
                    Authority {
                        stake: 1,
                        primary_address: validator.primary_address.clone(),
                        network_key: validator.network_keypair.keypair().public().clone(),
                    },
                )
            })
            .collect();

        let worker_cache = WorkerCache {
            epoch: Epoch::default(),
            // BTreeMap<PublicKey, WorkerIndex>,
            workers: validators
                .iter()
                .map(|validator| {
                    let worker_address: Multiaddr = "/ip4/127.0.0.1/udp/0".parse().unwrap();
                    let worker_transactions = "/ip4/127.0.0.1/tcp/0/http".parse().unwrap();

                    (
                        validator.keypair.authority_keypair().public().clone(),
                        WorkerIndex(
                            [(
                                0,
                                WorkerInfo {
                                    name: validator.worker_keypair.keypair().public().clone(),
                                    transactions: worker_transactions,
                                    worker_address,
                                },
                            )]
                            .into_iter()
                            .collect(),
                        ),
                    )
                })
                .collect(),
        };

        let committee = Committee {
            epoch: Epoch::default(),
            authorities,
        };

        // todo(botch): handle unwrap
        // cloning much?
        let narwhal_config = NarwhalConfig {
            keypair: config.keypair.clone(),
            network_keypair: config.network_keypair.clone(),
            // pass this in from the instantiation
            registry_service: RegistryService::new(Registry::new()),
            ids_and_keypairs: vec![(0, config.worker_keypair.keypair().copy())],
            internal_consensus: Default::default(),
            parameters: Parameters::default(),
            storage_base_path: config.db_path.clone(),
        };

        let narwhal = Narwhal::new(narwhal_config, validators);
        let (tx, _rx) = channel(1);
        let shared_worker_cache = SharedWorkerCache::from(worker_cache);
        let execution_state = Arc::new(Execution::new(Epoch::default(), tx));

        let tx_validator = Validator::new();

        narwhal
            .start(
                committee.into(),
                shared_worker_cache,
                execution_state,
                tx_validator,
            )
            .await?;

        Ok(())
    }
}
