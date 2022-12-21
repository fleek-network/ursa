//! # Ursa Behaviour implementation.
//!
//! Ursa custom behaviour implements [`NetworkBehaviour`] with the following options:
//!
//! - [`Ping`] A `NetworkBehaviour` that responds to inbound pings and
//!   periodically sends outbound pings on every established connection.
//! - [`Identify`] A `NetworkBehaviour` that automatically identifies nodes periodically, returns information
//!   about them, and answers identify queries from other nodes.
//! - [`Bitswap`] A `NetworkBehaviour` that handles sending and receiving blocks.
//! - [`Gossipsub`] A `NetworkBehaviour` that handles the gossipsub protocol.
//! - [`DiscoveryBehaviour`]
//! - [`RequestResponse`] A `NetworkBehaviour` that implements a generic
//!   request/response protocol or protocol family, whereby each request is
//!   sent over a new substream on a connection.

use anyhow::{Error, Result};
use cid::Cid;
use libipld::store::StoreParams;
use libp2p::dcutr;
use libp2p::swarm::behaviour::toggle::Toggle;
use libp2p::{
    autonat::{Behaviour as Autonat, Config as AutonatConfig},
    gossipsub::{
        error::{PublishError, SubscriptionError},
        Gossipsub, IdentTopic as Topic, MessageId, PeerScoreParams, PeerScoreThresholds,
    },
    identify::{Behaviour as Identify, Config as IdentifyConfig},
    identity::Keypair,
    kad,
    ping::Behaviour as Ping,
    relay::v2::{
        client::Client as RelayClient,
        relay::{Config as RelayConfig, Relay as RelayServer},
    },
    request_response::{ProtocolSupport, RequestResponse, RequestResponseConfig},
    swarm::NetworkBehaviour,
    Multiaddr, PeerId,
};
use libp2p_bitswap::{Bitswap, BitswapConfig, BitswapStore, QueryId};
use std::time::Duration;
use std::{collections::HashSet, iter};
use tracing::error;
use ursa_metrics::BITSWAP_REGISTRY;

use crate::gossipsub::build_gossipsub;
use crate::{
    codec::protocol::{UrsaExchangeCodec, UrsaProtocol},
    config::NetworkConfig,
    discovery::DiscoveryBehaviour,
};

pub const IPFS_PROTOCOL: &str = "ipfs/0.1.0";

fn ursa_agent() -> String {
    format!("ursa/{}", env!("CARGO_PKG_VERSION"))
}

/// Composes protocols for the behaviour of the node in the network.
#[derive(NetworkBehaviour)]
pub struct Behaviour<P: StoreParams> {
    /// Alive checks.
    ping: Ping,

    /// Identify and exchange info with other peers.
    identify: Identify,

    /// autonat
    autonat: Toggle<Autonat>,

    /// Relay client. Used to listen on a relay for incoming connections.
    relay_client: Toggle<RelayClient>,

    /// Relay server. Used to allow other peers to route through the node
    relay_server: Toggle<RelayServer>,

    /// DCUtR
    dcutr: Toggle<dcutr::behaviour::Behaviour>,

    /// Bitswap for exchanging data between blocks between peers.
    pub(crate) bitswap: Bitswap<P>,

    /// Ursa's gossiping protocol for message propagation.
    pub(crate) gossipsub: Gossipsub,

    /// Kademlia discovery and bootstrap.
    pub(crate) discovery: DiscoveryBehaviour,

    /// request/response protocol implementation for [`UrsaProtocol`]
    pub(crate) request_response: RequestResponse<UrsaExchangeCodec>,
}

impl<P: StoreParams> Behaviour<P> {
    pub fn new<S: BitswapStore<Params = P>>(
        keypair: &Keypair,
        config: &NetworkConfig,
        bitswap_store: S,
        relay_client: Option<libp2p::relay::v2::client::Client>,
    ) -> Self {
        let local_public_key = keypair.public();
        let local_peer_id = PeerId::from(local_public_key.clone());

        // Setup the ping behaviour
        let ping = Ping::default();

        // Setup the gossip behaviour
        let mut gossipsub = build_gossipsub(keypair, config);
        gossipsub
            .with_peer_score(PeerScoreParams::default(), PeerScoreThresholds::default())
            .expect("PeerScoreParams and PeerScoreThresholds");

        // Setup the discovery behaviour
        let discovery = DiscoveryBehaviour::new(keypair, config);

        // Setup the bitswap behaviour
        let bitswap = Bitswap::new(BitswapConfig::default(), bitswap_store);

        if let Err(e) = bitswap.register_metrics(&BITSWAP_REGISTRY) {
            // cargo tests will attempt to register duplicate registries, can ignore safely
            error!("Failed to register bitswap metrics: {}", e);
        }

        // Setup the identify behaviour
        let identify = Identify::new(
            IdentifyConfig::new(IPFS_PROTOCOL.into(), keypair.public())
                .with_agent_version(ursa_agent()),
        );

        let request_response = {
            let mut cfg = RequestResponseConfig::default();

            // todo(botch): calculate an upper limit to allow for large files
            cfg.set_request_timeout(Duration::from_secs(60));

            let protocols = iter::once((UrsaProtocol, ProtocolSupport::Full));

            RequestResponse::new(UrsaExchangeCodec, protocols, cfg)
        };

        let autonat = config
            .autonat
            .then(|| {
                let config = AutonatConfig {
                    throttle_server_period: Duration::from_secs(30),
                    ..AutonatConfig::default()
                };

                Autonat::new(local_peer_id, config)
            })
            .into();

        let relay_server = config
            .relay_server
            .then(|| RelayServer::new(local_public_key.into(), RelayConfig::default()))
            .into();

        let dcutr = config
            .relay_client
            .then(|| {
                if relay_client.is_none() {
                    panic!("relay client not instantiated");
                }
                dcutr::behaviour::Behaviour::new()
            })
            .into();

        Behaviour {
            ping,
            autonat,
            relay_server,
            relay_client: relay_client.into(),
            dcutr,
            bitswap,
            identify,
            gossipsub,
            discovery,
            request_response,
        }
    }

    pub fn publish(
        &mut self,
        topic: Topic,
        data: impl Into<Vec<u8>>,
    ) -> Result<MessageId, PublishError> {
        self.gossipsub.publish(topic, data)
    }

    pub fn public_address(&self) -> Option<&Multiaddr> {
        self.autonat.as_ref().and_then(|a| a.public_address())
    }

    pub fn peers(&self) -> HashSet<PeerId> {
        self.discovery.peers().clone()
    }

    pub fn is_relay_client_enabled(&self) -> bool {
        self.relay_client.is_enabled()
    }

    pub fn discovery(&mut self) -> &mut DiscoveryBehaviour {
        &mut self.discovery
    }

    pub fn bootstrap(&mut self) -> Result<kad::QueryId, Error> {
        self.discovery.bootstrap()
    }

    pub fn subscribe(&mut self, topic: &Topic) -> Result<bool, SubscriptionError> {
        self.gossipsub.subscribe(topic)
    }

    pub fn unsubscribe(&mut self, topic: &Topic) -> Result<bool, PublishError> {
        self.gossipsub.unsubscribe(topic)
    }

    pub fn get_block(
        &mut self,
        cid: Cid,
        providers: impl Iterator<Item = PeerId>,
    ) -> Result<QueryId> {
        let cid = cid;
        Ok(self.bitswap.get(cid, providers))
    }

    pub fn sync_block(&mut self, cid: Cid, providers: Vec<PeerId>) -> Result<QueryId> {
        let cid = cid;
        Ok(self.bitswap.sync(cid, providers, iter::once(cid)))
    }
}
