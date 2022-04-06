# Fleek network -ing

## Bootstrapping

The premise is that all connections between peers must be authenticated encrypted, and multiplexed. Below we discuss our approach to meeting the goal.

## Transport

Libp2p supports dialing/listening on different transports in parallel. We will have base line interoperability support for QUIC for both inbound and outbound connections, while also supporting tcp and others as fallbacks in case QUIC fails. IPV6 and IPV4 both are supported, with IPV4 being the default.

- QUIC over TCP?
    - QUIC requires not initial negotiation to agree on auth or multiplexing
    - fewer round trips to establish connection
        - reducing latency
    - Native multiplexer
    - Native security with TLS1.3
    - Congestion control
    - in addition to our “sticky” links for peering in gossip, initially the handshake takes one round trip time, for any already seen peers there will be zero round trips.
    - QUIC over UDP uses fewer file descriptors, opening one UDP socket per listen address instead of one socket per connection.

- Takeaways:
    - QUIC by default
    - support for TCP
    - IPV4 default

## Identities and security

Beside TLS1.3 and SECIO, we chose libp2p Noise as our default transport security. SECIO has been the default in IPFS, but is being phased out slowly. Through a transport upgrader, Noise will create a security handshake channel between the peers `secp256k1` identities in the dialing and listening. As described in [libp2p-noise](https://github.com/libp2p/specs/tree/master/noise), the two peers can now exchange encrypted information. Multistream-select is the current transport upgrader but soon to be replaced with Multiselect 2.  

We will set the priority of transport security through our swarm as follows:

- Noise
    - priority: `100`
- TLS
    - priority: `200`
- SECIO
    - priority: `300`
    - support will be dropped eventually once IPFS and libp2p phase it out
    
- Takeaways:
    - Noise by default
    - Secp256k1 identies
    - default to multistream-select 1.0
        - paying close attention to multistream 2.0

## ****Multiplexing****

Multiplexing is native to QUIC, therefore, it will only apply to transports such as TCP. We have set the default to be mplex for any TCP connection. 

## Discovery - WIP

Before messages are sent back and forth between peers, they must first discover one another. On top of gossipsub there can be any discovery mechanism as long as they use the Peer discovery interface for libp2p. We have a couple of methods, the first of which is Peer exchange through our Fleek bootstrap nodes, these nodes are nominal, and neutral, as in they don’t interact with the mesh network. The bootstrap nodes help us in several ways. One way is to maintain score of all the nodes that connect to the bootstrap nodes; subsequently preventing nodes, malicious ones, from continuing their bootstrap process. Another use case for bootstrap nodes are gossip-only nodes and act as the orchestrators and maintainers of the network. The second method, which is exploratory, is using explicit peering agreements, to define a set of peers to which nodes should connect to when bootstrapping. This should help speed up the bootstrap process for nodes far away from Fleek’s bootstrap nodes. There are downsides to this and that is seemingly well behaved nodes turning rogue.

## GossipSub - Content routing

Here we discuss Fleek Mesh Construction, we start with the default v1.0 parameters and define a recommendation for the v1.1 peer scoring. We diverge from one default value and that is `D`, with that change we must also change `D_low` and `D_high`  degree of the network. Which boils down to how many peers can a node maintain a direct connection to in its local mesh. 

- Parameters
    - [v1.0 spec](https://github.com/libp2p/specs/blob/master/pubsub/gossipsub/gossipsub-v1.0.md#parameters)
        - `D`
            - The desired outbound degree of the network
            - fleek: 8
            - default: 6
        - `D_low`
            - Lower bound for outbound degree
            - fleek:
            - default: 4
        - `D_high`
            - Upper bound for outbound degree
            - fleek:
            - default: 12
        - `D_lazy`
            - (Optional) the outbound degree for gossip emission
            - fleek:
            - default: `D` = degree of the network
        - `heartbeat_interval`
            - Time between [heartbeats](https://github.com/libp2p/specs/blob/master/pubsub/gossipsub/gossipsub-v1.0.md#heartbeat)
            - fleek:
            - default: 1 second
        - `fanout_ttl`
            - Time-to-live for each topic's fanout state
            - fleek:
            - default: 60 seconds
        - `mcache_len`
            - Number of history windows in message cache
            - fleek:
            - default: 5
        - `mcache_gossip`
            - Number of history windows to use when emitting gossip
            - fleek:
            - default: 3
        - `seen_ttl`
            - Expiry time for cache of seen message ids
            - fleek:
            - default: 2 minutes
    - [v1.1 spec](https://github.com/libp2p/specs/blob/master/pubsub/gossipsub/gossipsub-v1.1.md#overview-of-new-parameters)
        - `PruneBackoff`
            - Time after pruning a mesh peer before we consider grafting them again.
            - fleek:
            - default: 1 minute
        - `FloodPublish`
            - Whether to enable flood publishing
            - fleek: true
            - default: true
        - `GossipFactor`
            - % of peers to send gossip to, if we have more than `D_lazy` available
            - fleek:
            - default: 0.25
        - `D_score`
            - Number of peers to retain by score when pruning because of oversubscription
            - fleek:
            - default: 4 or 5 for a `D` = 6.
        - `D_out`
            - Number of outbound connections to keep in the mesh. Must be less than `D_lo` and at most `D/2`
            - fleek:
            - default: 2 for a `D` of 6
        - v1.1 peer scoring
            
            There are no libp2p defaults, so the values are fleek specific.
            
            - **Global - apply to all peers and topics**
                - `GossipThreshold`
                    - No gossip emitted to peers below threshold; incoming gossip is ignored.
                    - value [float]:
                - `PublishThreshold`
                    - No self-published messages are sent to peers below threshold.
                    - value [float]:
                - `GraylistThreshold`
                    - All RPC messages are ignored from peers below threshold.
                    - value [float]:
                - `AcceptPXThreshold`
                    - PX information by peers below this threshold is ignored.
                    - value [float]:
                - `OpportunisticGraftThreshold`
                    - If the median score in the mesh drops below this threshold, then the router may opportunistically graft better scoring peers.
                    - value [float]:
                - `DecayInterval`
                    - Interval at which parameter decay is calculated.
                    - value [float]:
                - `DecayToZero`
                    - Limit below which we consider a decayed param to be "zero".
                    - value [float]:
                - `RetainScore`
                    - Time to remember peer scores after a peer disconnects.
                    - value [float]:
            - **Observed score based on behaviour**
                - `AppSpecificWeight`
                    - Weight of `P₅`, the application-specific score.
                    - Must be positive, however score values may be negative.
                    - type: weight
                    - value:
                - `IPColocationFactorWeight`
                    - Weight of `P₅`, the application-specific score.
                    - Must be positive, however score values may be negative.
                    - type: weight
                    - value:
                - `IPColocationFactorThreshold`
                    - Weight of `P₅`, the application-specific score.
                    - Must be positive, however score values may be negative.
                    - type: weight
                    - value:
                - `BehaviourPenaltyWeight`
                    - Weight of `P₅`, the application-specific score.
                    - Must be positive, however score values may be negative.
                    - type: weight
                    - value:
                - `BehaviourPenaltyDecay`
                    - Weight of `P₅`, the application-specific score.
                    - Must be positive, however score values may be negative.
                    - type: weight
                    - value:
            - Peer’s behaviour within a single topic
                - `TopicWeight`
                - **`P₁`**
                - `TimeInMeshWeight`
                - `TimeInMeshQuantum`
                - `TimeInMeshCap`
                - **`P₂`**
                - `FirstMessageDeliveriesWeight`
                - `FirstMessageDeliveriesDecay`
                - `FirstMessageDeliveriesCap`
                - **`P₃`**
                - `MeshMessageDeliveriesWeight`
                - `MeshMessageDeliveriesDecay`
                - `MeshMessageDeliveriesThreshold`
                - `MeshMessageDeliveriesCap`
                - `MeshMessageDeliveriesActivation`
                - `MeshMessageDeliveryWindow`
                - **`P₃b`**
                - `MeshFailurePenaltyWeight`
                - `MeshFailurePenaltyDecay`
                - **`P₄`**
                - `InvalidMessageDeliveriesWeight`
                - `InvalidMessageDeliveriesDecay`
        
- ****Recommendations for Network Operators****
    - The network can boostrap through known small set of bootstrap nodes. This can be done through Peer Exchange, as long as the peer can find one peer participating in the topic of interest.
    - Without a discovery service
        - Create and operate a set of stable bootstrapper nodes, whose addresses are known ahead of time by the application.
        - The bootstrappers should be configured without a mesh (ie set `D=D_lo=D_hi=D_out=0`) and with Peer Exchange enabled, utilizing Signed Peer Records.
        - The application should assign a high application-specific score to the bootstrappers and set `AcceptPXThreshold` to a high enough value attainable only by the bootstrappers.
        - The bootstrap nodes will only act as gossip and peer exchange nodes only.
        - Network operators may configure the application-specific scoring function such that the bootstrappers enforce further constraints into accepting new nodes (eg protocol handshakes, staked participation, and so on).
    - With discovery service
        - even if an external peer discovery service like Kademlia is used, recommended to use bootstrap nodes configured with Peer exchange and high application scores.