use libp2p::{identity, Swarm};

use crate::behaviour::FnetBehaviour;

/// Create the network identity
///   - secp256k1
///   - Nodes identify each other with PeerId derived from PK
/// Construct our node transport
///   - QUIC
///   - dev: we will use development_transport function [TCP, noise]
/// Network behaviour
///   - while the transport defines how to send
///   - network behaviour defines what bytes to send over
///   - it only cares about the messages sent over the network not how
/// Swarm
///   - we need something that connects the two
///   - passing commands from the NetworkBehaviour to the Transport as well as events from the Transport to the NetworkBehaviour
/// MultiAddr
///   - in libp2p instead of passing in an IP, we pass in a multiaddr
///   - poll the swarm

pub struct FnetP2PService {
    swarm: Swarm<FnetBehaviour>,
}

impl FnetP2PService {
    pub fn new() {
        // create pub/priv key pair for secp256
        let id_key = identity::Keypair::generate_secp256k1();
        // setup peer id with key pair
        // setup noise
        // create transport
        // get default limits
        // setup a global topic FNET
        // start swarm construction
        //  -
        //  - custom gossipsub
        // listen on swarm
        //
    }
}
