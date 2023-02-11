// Copyright 2022-2023 Fleek Network
// SPDX-License-Identifier: Apache-2.0, MIT

use multiaddr::Multiaddr;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
// When deserializing the config file, use the default from the Default instance
// to fill any missing field.
#[serde(default)]
pub struct ConsensusConfig {
    /// The address in which the primary will listen for incoming requests on. This MUST
    /// be a UDP address.
    address: Multiaddr,
    /// Path to the BLS12381 private key for the primary.
    keypair: PathBuf,
    /// Path to the Ed25519 networking private key for the primary.
    // TODO(qti3e) We should probably use the same Ed25519 key that ursa/identity.rs provides.
    network_keypair: PathBuf,
    /// Path to the database used by the narwhal implementation.
    store_path: PathBuf,
    /// Configuration of the consensus worker.
    // Ideally we want to keep the possibility of 'allowing' future extending of the
    // implementation, so that we may support more than one worker, for this reason
    // we want the worker section of the config to be an array.
    // At the same time, currently as part of the implementation we want to enforce
    // the presence of one and only one worker.
    // This is the reason we are using a fixed size array of size one for now. So the
    // config will stay backward compatible, and at the same time we will have a verification
    // on the array size to ensure the length of one item.
    worker: [WorkerConfig; 1],
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkerConfig {
    /// UDP address which the worker is using to connect with the other workers and the primary.
    pub address: Multiaddr,
    /// UDP address which the worker is listening on to receive transactions from user space.
    pub transaction: Multiaddr,
    /// The path to the network key pair (Ed25519) for the worker.
    pub keypair: PathBuf,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        // TODO(qti3e) We should decide on the default ports. I used the following format:
        // reserve 6xxx for consensus layer in the entire ursa project.
        // 6000 for primary
        // 6x01 for worker `x` address
        // 6x02 for worker `x` transaction address
        Self {
            address: "/ip4/0.0.0.0/udp/6000".parse().unwrap(),
            keypair: "~/.ursa/keystore/consensus/primary.key".into(),
            network_keypair: "~/.ursa/keystore/consensus/network.key".into(),
            store_path: "~/.ursa/data/narwhal_db".into(),
            worker: [WorkerConfig {
                address: "/ip4/0.0.0.0/udp/6101".parse().unwrap(),
                transaction: "/ip4/0.0.0.0/udp/6102".parse().unwrap(),
                keypair: "~/.ursa/keystore/consensus/worker-01.key".into(),
            }],
        }
    }
}
