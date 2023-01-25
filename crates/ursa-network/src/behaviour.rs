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

use anyhow::Result;
use db::Store;
use fvm_ipld_blockstore::Blockstore;
use graphsync::GraphSync;
use libipld::{store::StoreParams, Cid};
use libp2p::swarm::behaviour::toggle::Toggle;
use libp2p::{
    autonat::{Behaviour as Autonat, Config as AutonatConfig},
    dcutr::behaviour::Behaviour as Dcutr,
    gossipsub::{
        error::{PublishError, SubscriptionError},
        Gossipsub, IdentTopic as Topic, MessageId, PeerScoreParams, PeerScoreThresholds,
    },
    identify::{Behaviour as Identify, Config as IdentifyConfig},
    identity::Keypair,
    kad::{store::MemoryStore, Kademlia, KademliaConfig},
    mdns::tokio::Behaviour as Mdns,
    multiaddr::Protocol,
    ping::Behaviour as Ping,
    relay::v2::{
        client::Client as RelayClient,
        relay::{Config as RelayConfig, Relay as RelayServer},
    },
    request_response::{ProtocolSupport, RequestResponse, RequestResponseConfig},
    swarm::NetworkBehaviour,
    Multiaddr, PeerId,
};
use libp2p_bitswap::{Bitswap, BitswapConfig, BitswapStore};
use std::borrow::Cow;
use std::num::NonZeroUsize;
use std::time::Duration;
use std::{collections::HashSet, iter};

use tracing::{info, warn};
use ursa_metrics::BITSWAP_REGISTRY;
use ursa_store::GraphSyncStorage;

use crate::gossipsub::build_gossipsub;
use crate::{
    codec::protocol::{UrsaExchangeCodec, UrsaProtocol},
    config::NetworkConfig,
};

pub const IPFS_PROTOCOL: &str = "ipfs/0.1.0";
pub const KAD_PROTOCOL: &[u8] = b"/ursa/kad/0.0.1";

fn ursa_agent() -> String {
    format!("ursa/{}", env!("CARGO_PKG_VERSION"))
}

/// Composes protocols for the behaviour of the node in the network.
#[derive(NetworkBehaviour)]
pub struct Behaviour<P, S>
where
    P: StoreParams,
    S: Blockstore + Clone + Store + Send + Sync + 'static,
{
    /// Alive checks.
    ping: Ping,

    /// Identify and exchange info with other peers.
    identify: Identify,

    /// autonat
    autonat: Toggle<Autonat>,

    /// Relay client. Used to listen on a relay for incoming connections.
    pub(crate) relay_client: Toggle<RelayClient>,

    /// Relay server. Used to allow other peers to route through the node
    relay_server: Toggle<RelayServer>,

    /// DCUtR
    dcutr: Toggle<Dcutr>,

    /// mDNS LAN peer discovery
    mdns: Toggle<Mdns>,

    /// Kademlia peer discovery
    pub(crate) kad: Kademlia<MemoryStore>,

    /// Bitswap for exchanging data between blocks between peers.
    pub(crate) bitswap: Bitswap<P>,

    /// Ursa's gossiping protocol for message propagation.
    pub(crate) gossipsub: Gossipsub,

    /// request/response protocol implementation for [`UrsaProtocol`]
    pub(crate) request_response: RequestResponse<UrsaExchangeCodec>,

    /// Graphsync for efficiently exchanging data between blocks between peers.
    pub(crate) graphsync: GraphSync<GraphSyncStorage<S>>,
}

impl<P, S> Behaviour<P, S>
where
    P: StoreParams,
    S: Blockstore + Clone + Store + Send + Sync + 'static,
{
    pub fn new<B: BitswapStore<Params = P>>(
        keypair: &Keypair,
        config: &NetworkConfig,
        bitswap_store: B,
        graphsync_store: GraphSyncStorage<S>,
        relay_client: Option<libp2p::relay::v2::client::Client>,
        peers: &mut HashSet<PeerId>,
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

        // Setup the bitswap behaviour
        let bitswap = Bitswap::new(BitswapConfig::default(), bitswap_store);

        if let Err(e) = bitswap.register_metrics(&BITSWAP_REGISTRY) {
            // cargo tests will attempt to register duplicate registries, can ignore safely
            warn!("Failed to register bitswap metrics: {}", e);
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
                Dcutr::new()
            })
            .into();

        let mdns = if config.mdns {
            Some(Mdns::new(Default::default()).expect("mDNS start"))
        } else {
            None
        }
        .into();

        // setup the kademlia behaviour
        let mut kad = {
            let store = MemoryStore::new(local_peer_id);
            let replication_factor = NonZeroUsize::new(config.kad_replication_factor).unwrap();
            let mut kad_config = KademliaConfig::default();
            kad_config
                .set_protocol_names(vec![Cow::from(KAD_PROTOCOL)])
                .set_replication_factor(replication_factor);

            Kademlia::with_config(local_peer_id, store, kad_config.clone())
        };

        // Set up the Graphsync behaviour.
        let graphsync = GraphSync::new(graphsync_store);

        // init bootstraps
        for addr in config.bootstrap_nodes.iter() {
            if let Some(Protocol::P2p(mh)) = addr.to_owned().pop() {
                let peer_id = PeerId::from_multihash(mh).unwrap();
                info!("Adding bootstrap node: {peer_id} - {addr}");
                kad.add_address(&peer_id, addr.clone());
                peers.insert(peer_id);
            } else {
                warn!("Could not parse bootstrap addr {addr}");
            }
        }

        if !config.bootstrapper && !config.bootstrap_nodes.is_empty() {
            if let Err(e) = kad.bootstrap() {
                warn!("Failed to bootstrap: {}", e);
            } else {
                info!("Bootstrapping into the network...");
            }
        } else {
            warn!("Skipping bootstrap");
        }

        Behaviour {
            ping,
            autonat,
            relay_server,
            relay_client: relay_client.into(),
            dcutr,
            bitswap,
            identify,
            gossipsub,
            kad,
            mdns,
            request_response,
            graphsync,
        }
    }

    pub fn add_address(&mut self, peer_id: &PeerId, addr: Multiaddr) {
        self.bitswap.add_address(peer_id, addr.clone());
        self.kad.add_address(peer_id, addr.clone());
        self.request_response.add_address(peer_id, addr.clone());
        self.graphsync.add_address(peer_id, addr);
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
    ) -> Result<libp2p_bitswap::QueryId> {
        Ok(self.bitswap.get(cid, providers))
    }

    pub fn sync_block(
        &mut self,
        cid: Cid,
        providers: Vec<PeerId>,
    ) -> Result<libp2p_bitswap::QueryId> {
        Ok(self.bitswap.sync(cid, providers, iter::once(cid)))
    }
}
