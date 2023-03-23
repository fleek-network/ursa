# Ursa Fair Delivery Protocol

> ℹ️ This internal document is aimed to be an initial draft of the said protocol, to gain review and
> comments from the team, and it is planned for this document to be transformed into the formal
> protocol description once the initial implementation is done and the reviews have happened.

Ursa's fair delivery protocol is a point to point protocol that different parties in Ursa use in order
to transfer content to a client.

In this version of the document we only focus on the cache node and the client, and leave the discussion
about the gateway for future, but essentially the gateway has the power to intercept the request and perform
some modifications to the data frames as it sees fit for the purposes of getting rewarded.

## Summary of the problem

Ursa is not like a traditional CDN, some of the biggest differences are 1) relying on a decentralized account
management, and 2) the decentralization of the nodes in the network which eliminates any trust assumptions
about the intentions of the said nodes.

Implementing a working and efficient CDN within these constraints has never been done before, for example the
integrity of the content being served is not guranteed in a naive implementation, luckily we don't have to deal
with that problem since we are used content addressability.

The next big problem is the decentralized account management which revolves around the delivery of content, how
do we ensure that the nodes running inside this network are getting paid for the work they do? And how do we charge
a client for the bandwidth they are using, without impacting the latency?

In the literature this problem is referred to as the **Fair Market Exchange** problem, which states that once an
exchange of goods between two parties is over, either no party should have received anything they wanted, or both
parties should be satisfied.

In our case the two parties are 1) the node, and 2) the client. They are exchanging payment for content.

## Solution

```mermaid
---
title: Optimistic Happy Path
---
sequenceDiagram
    actor Client
    participant Node
    Client->>Node: Request
    loop Every Block
      Node->>Client: Publicly Verifiable Encrypted Content
      Client->>Node: Delivery Acknowledgment
      Node->>Client: Decryption Key
    end
```

> 📝 You might have heard me using the term "Receipt of Payment" before, but now I'm using the term
> "Delivery Acknowledgment".

The above diagram depicts the main concept behind the solution, a node sends the encrypted response and only
delivers the decryption key to the client once it receives the  *Delivery Acknowledgment* from the client.

And of course, you might naturally ask what if the node doesn't deliver the decryption key? Since the client
can not unsend the "Delivery Acknowledgment" we have placed a slow-path for the client to retrieve the decryption
key from the committee.

```mermaid
---
title: Unhappy Path
---
sequenceDiagram
    actor Client
    participant Node
    participant Committee
    Client->>Node: Request
    loop Every Block
        Node->>Client: Publicly Verifiable Encrypted Content
        Client->>Node: Delivery Acknowledgmen
        Node->>Client: Decryption Key
    end
    Note over Client, Node: Last Block
    Node->>Client: Publicly Verifiable Encrypted Content
    Client->>Node: Delivery Acknowledgmen
    Note over Client, Node: Connection Hungup
    Client->>Committee: Delivery Acknowledgmen
    Committee->>Client: Decryption Key
```

## Algorithms

In this section we will go through the algorithms used for different sections.


### Key Generation

#### Node

The node has a ephemeral private key for this protocol, which is shared with the committee using Shamir Secret Sharing at
the beginning of each consensus epoch. We use curve SECP256K1, the private key is a random number $\alpha \in \mathbb{Z}$.

This private key is only used for the purposes of delivery protocol and is refreshed every epoch. The public key obtained
from this secret key should not be used to globally identify a node outside the epoch it was used at.

#### Client

Each client is identified by its public key, for clients we use BLS signatures, specifically the curve BLS12-381. This decision
is made to allow

### Encryption
### Verification
### Decryption
### Generating Delivery Acknowledgments
### Verifying Delivery Acknowledgments
### Hash request to curve

## Network Interface

### Handshake

### 
