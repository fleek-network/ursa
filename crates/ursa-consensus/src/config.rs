// Copyright 2022-2023 Fleek Network
// SPDX-License-Identifier: Apache-2.0, MIT

use std::{path::PathBuf, sync::Arc};

use fastcrypto::{bls12381::min_sig::BLS12381KeyPair, ed25519::Ed25519KeyPair};
use multiaddr::Multiaddr;
use mysten_metrics::RegistryService;
use narwhal_config::{Parameters, WorkerId};
use narwhal_crypto::NetworkKeyPair as NarwhalNetworkKeyPair;
use serde::{Deserialize, Serialize};
use tokio::sync::OnceCell;

pub type KeyPair = Ed25519KeyPair;
pub type AuthorityKeyPair = BLS12381KeyPair;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ValidatorKeyPair {
    #[serde(skip)]
    keypair: OnceCell<Arc<AuthorityKeyPair>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct NetworkKeyPair {
    #[serde(skip)]
    keypair: OnceCell<Arc<KeyPair>>,
}

impl ValidatorKeyPair {
    pub fn new(keypair: AuthorityKeyPair) -> Self {
        let cell = OnceCell::new();
        cell.set(Arc::new(keypair))
            .expect("Failed to set authority keypair");
        Self { keypair: cell }
    }

    pub fn authority_keypair(&self) -> &AuthorityKeyPair {
        self.keypair.get().as_ref().unwrap()
    }
}

impl NetworkKeyPair {
    pub fn new(keypair: KeyPair) -> Self {
        let cell = OnceCell::new();
        cell.set(Arc::new(keypair))
            .expect("Failed to set authority keypair");
        Self { keypair: cell }
    }

    pub fn keypair(&self) -> &KeyPair {
        self.keypair.get().as_ref().unwrap()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodeConfig {
    pub keypair: ValidatorKeyPair,
    pub worker_keypair: NetworkKeyPair,
    pub account_keypair: NetworkKeyPair,
    pub network_keypair: NetworkKeyPair,
    pub network_address: Multiaddr,
    pub db_path: PathBuf,
}

pub struct NarwhalConfig {
    pub keypair: ValidatorKeyPair,
    pub network_keypair: NetworkKeyPair,
    pub registry_service: RegistryService,
    pub ids_and_keypairs: Vec<(WorkerId, NarwhalNetworkKeyPair)>,
    pub internal_consensus: bool,
    pub parameters: Parameters,
    pub storage_base_path: PathBuf,
}
