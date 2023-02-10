// Copyright 2022-2023 Fleek Network
// SPDX-License-Identifier: Apache-2.0, MIT

use anyhow::Result;
use fastcrypto::traits::KeyPair as PrimaryKeyPair;
use multiaddr::Multiaddr;
use narwhal_crypto::{KeyPair, NetworkKeyPair as NarwhalNetworkKeyPair};
use rand::{rngs::OsRng, SeedableRng};
use tracing::info;
use ursa_consensus::{
    config::{NetworkKeyPair, NodeConfig, ValidatorKeyPair},
    Service,
};

#[tokio::main]
async fn main() -> Result<()> {
    let listen_address: Multiaddr = "/ip4/0.0.0.0/tcp/5678".parse().unwrap();

    // todo(botch): Abstract
    let mut rng = rand::rngs::StdRng::from_rng(OsRng).unwrap();

    let dir = tempfile::TempDir::new().unwrap().into_path();
    let keypair = KeyPair::generate(&mut rng);
    let worker_keypair = NarwhalNetworkKeyPair::generate(&mut rng);
    let account_keypair = NarwhalNetworkKeyPair::generate(&mut rng);
    let network_keypair = NarwhalNetworkKeyPair::generate(&mut rng);

    let primary_address: Multiaddr = "/ip4/127.0.0.1/udp/0".parse().unwrap();
    let network_address: Multiaddr = "/ip4/127.0.0.1/udp/0/http".parse().unwrap();

    let config = NodeConfig {
        keypair: ValidatorKeyPair::new(keypair),
        worker_keypair: NetworkKeyPair::new(worker_keypair),
        account_keypair: NetworkKeyPair::new(account_keypair),
        network_keypair: NetworkKeyPair::new(network_keypair),
        network_address,
        db_path: dir,
    };

    info!("Started narwhal listening on {}", listen_address);

    Service::start(&config).await?;

    Ok(())
}
