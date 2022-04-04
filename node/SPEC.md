# Fleek network -ing

## Bootstrapping

The premise is that all connections between peers must authenticated encrypted, and multiplexed. Below we discuss our approach to meeting the goal.

## Transport

Libp2p support dialling/listening on different transports in parallel. We will have a base line interoperability support of QUIC for both inbound and outbound connections, while also supporting tcp and others as fallback in case QUIC fails. IPV6 and IPV4 both are supported, with IPV4 being the default. 

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

Beside TLS1.3 and SECIO, we chose libp2p Noise as our default as our transport security. SECIO gas been the default in IPFS, but is being phased out slowly. Through a transport upgrader, noise will create a security handshake channel between the peers `secp256k1` identities in the dialling and listening. As described in [libp2p-noise](https://github.com/libp2p/specs/tree/master/noise), the two peers can now exchange encrypted information. Multistream-select is the current transport upgrader but soon to be replaced with Multiselect 2.  

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

Multiplexing is native to QUIC, therefore, it will only apply transports such as TCP. We have set the default to be mplex for any TCP connection. 

## Discovery - WIP

## GossipSub - WIP

- Peer discovery
- Parameters
    - [v1.0 spec](https://github.com/libp2p/specs/blob/master/pubsub/gossipsub/gossipsub-v1.0.md#parameters)
        - `D`
            - The desired outbound degree of the network
            - fleek:
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
            
- Spam Protection Measures - WIP
- ****Recommendations for Network Operators****