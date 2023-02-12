// Copyright 2022-2023 Fleek Network
// SPDX-License-Identifier: Apache-2.0, MIT

use multiaddr::Multiaddr;
use narwhal_config::{
    Authority, Committee, Parameters, Stake, WorkerCache, WorkerId, WorkerIndex, WorkerInfo,
};
use narwhal_crypto::{NetworkPublicKey, PublicKey};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug, Clone, Deserialize, Serialize)]
// When deserializing the config file, use the default from the Default instance
// to fill any missing field.
#[serde(default)]
pub struct ConsensusConfig {
    /// The address in which the primary will listen for incoming requests on. This MUST
    /// be a UDP address.
    pub address: Multiaddr,
    /// Path to the BLS12381 private key for the primary.
    pub keypair: PathBuf,
    /// Path to the Ed25519 networking private key for the primary.
    // TODO(qti3e) We should probably use the same Ed25519 key that ursa/identity.rs provides.
    pub network_keypair: PathBuf,
    /// Path to the database used by the narwhal implementation.
    pub store_path: PathBuf,
    /// Path to the JSON file containing the committee information for genesis.
    // Note(qti3e): In a perfect world, I wanted to be able to have the config embedded
    // here and not just having a path to a JSON file, but creating the default value would
    // not be really a trivial flow during the initial load of the program.
    pub genesis_committee: PathBuf,
    /// Narwhal parameters used for the consensus.
    pub parameters: Parameters,
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
    /// UDP address which the worker is using to connect with the other workers and the
    /// primary.
    pub address: Multiaddr,
    /// UDP address which the worker is listening on to receive transactions from user space.
    pub transaction: Multiaddr,
    /// The path to the network key pair (Ed25519) for the worker.
    pub keypair: PathBuf,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GenesisCommittee {
    pub authorities: BTreeMap<PublicKey, GenesisAuthority>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GenesisAuthority {
    /// The voting power of this authority.
    pub stake: Stake,
    /// The network address of the primary.
    pub primary_address: Multiaddr,
    /// Network key of the primary.
    pub network_key: NetworkPublicKey,
    /// Worker information for this authority.
    pub workers: [WorkerInfo; 1],
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        // TODO(qti3e) We should decide on the default ports. I used the following format:
        // reserve 6xxx for consensus layer in the entire ursa project.
        // 8000 for primary
        // 8x01 for worker `x` address
        // 8x02 for worker `x` transaction address
        Self {
            address: "/ip4/0.0.0.0/udp/8000".parse().unwrap(),
            keypair: "~/.ursa/keystore/consensus/primary.key".into(),
            network_keypair: "~/.ursa/keystore/consensus/network.key".into(),
            store_path: "~/.ursa/data/narwhal_store".into(),
            // default the committee.json location relative to the cwd of the current process.
            genesis_committee: "./committee.json".into(),
            parameters: Parameters::default(),
            worker: [WorkerConfig {
                address: "/ip4/0.0.0.0/udp/8101".parse().unwrap(),
                transaction: "/ip4/0.0.0.0/udp/8102".parse().unwrap(),
                keypair: "~/.ursa/keystore/consensus/worker-01.key".into(),
            }],
        }
    }
}

impl From<&GenesisCommittee> for Committee {
    fn from(genesis_committee: &GenesisCommittee) -> Self {
        Committee {
            epoch: 0,
            authorities: genesis_committee
                .authorities
                .iter()
                .map(|(key, authority)| (key.clone(), authority.into()))
                .collect(),
        }
    }
}

impl From<&GenesisAuthority> for Authority {
    fn from(authority: &GenesisAuthority) -> Self {
        Authority {
            stake: authority.stake,
            primary_address: authority.primary_address.clone(),
            network_key: authority.network_key.clone(),
        }
    }
}

impl From<&GenesisCommittee> for WorkerCache {
    fn from(genesis_committee: &GenesisCommittee) -> Self {
        WorkerCache {
            epoch: 0,
            workers: genesis_committee
                .authorities
                .iter()
                .map(|(key, authority)| {
                    (
                        key.clone(),
                        WorkerIndex(
                            authority
                                .workers
                                .iter()
                                .enumerate()
                                .map(|(id, info)| (id as WorkerId, info.clone()))
                                .collect(),
                        ),
                    )
                })
                .collect(),
        }
    }
}
