use crate::behaviour::BehaviourEvent;
use crate::utils::bloom_filter::CountingBloomFilter;
use crate::{
    codec::protocol::{RequestType, UrsaExchangeRequest},
    NetworkCommand, NetworkConfig, UrsaService, URSA_GLOBAL,
};
use anyhow::Result;
use async_fs::File;
use bytes::Bytes;
use cid::{multihash::Code, Cid};
use db::MemoryDB;
use futures::io::BufReader;
use futures::StreamExt;
use fvm_ipld_car::{load_car, CarReader};
use ipld_traversal::blockstore::Blockstore;
use libipld::{cbor::DagCborCodec, ipld, Block, DefaultParams, Ipld};
use libp2p::kad::{BootstrapOk, KademliaEvent, QueryResult};
use libp2p::request_response::RequestResponseEvent;
use libp2p::{
    gossipsub::IdentTopic as Topic, identity::Keypair, multiaddr::Protocol, swarm::SwarmEvent,
    Multiaddr, PeerId,
};
use libp2p_bitswap::BitswapStore;
use simple_logger::SimpleLogger;
use std::path::Path;
use std::{sync::Arc, time::Duration, vec};
use tokio::{select, sync::oneshot, time::timeout};
use tracing::warn;
use tracing::{error, info, log::LevelFilter};
use ursa_store::{BitswapStorage, GraphSyncStorage, UrsaStore};

fn create_block(ipld: Ipld) -> Block<DefaultParams> {
    Block::encode(DagCborCodec, Code::Blake3_256, &ipld).unwrap()
}

fn setup_logger(level: LevelFilter) {
    if let Err(err) = SimpleLogger::new()
        .with_level(level)
        .with_utc_timestamps()
        .init()
    {
        error!("Logger already set {:?}:", err)
    }
}

fn get_store() -> Arc<UrsaStore<MemoryDB>> {
    let db = Arc::new(MemoryDB::default());
    Arc::new(UrsaStore::new(Arc::clone(&db)))
}

fn get_block(content: &[u8]) -> Block<DefaultParams> {
    create_block(ipld!(content))
}

fn insert_block(mut s: BitswapStorage<MemoryDB>, b: &Block<DefaultParams>) {
    match s.insert(b) {
        Err(err) => error!(
            "there was an error while inserting into the blockstore {:?}",
            err
        ),
        Ok(()) => info!("block inserted successfully"),
    }
}

async fn run_bootstrap(
    config: &mut NetworkConfig,
) -> Result<(UrsaService<MemoryDB>, Multiaddr, PeerId)> {
    let keypair = Keypair::generate_ed25519();
    config.swarm_addrs = vec!["/ip4/127.0.0.1/tcp/0".parse().unwrap()];
    config.bootstrapper = true;
    config.bootstrap_nodes = vec![];
    let (bootstrap, addr, peer_id, ..) = network_init(config, None, Some(keypair)).await?;
    Ok((bootstrap, addr, peer_id))
}

async fn network_init(
    config: &mut NetworkConfig,
    bootstrap_addr: Option<Multiaddr>,
    bootstrap_keypair: Option<Keypair>,
) -> Result<(
    UrsaService<MemoryDB>,
    Multiaddr,
    PeerId,
    Arc<UrsaStore<MemoryDB>>,
)> {
    let keypair = match bootstrap_keypair {
        Some(k) => k,
        None => {
            config.swarm_addrs = vec!["/ip4/0.0.0.0/tcp/0".parse().unwrap()];
            Keypair::generate_ed25519()
        }
    };
    let peer_id = PeerId::from(keypair.clone().public());
    let store = get_store();

    if let Some(addr) = bootstrap_addr {
        config.bootstrap_nodes = vec![addr];
    }

    let mut service = UrsaService::new(keypair, config, Arc::clone(&store))?;

    let node_addrs = async {
        loop {
            if let SwarmEvent::NewListenAddr { mut address, .. } =
                timeout(Duration::from_secs(5), service.swarm.select_next_some())
                    .await
                    .expect("received some event")
            {
                address.push(Protocol::P2p(peer_id.into()));
                return address;
            }
        }
    }
    .await;

    Ok((service, node_addrs, peer_id, store))
}

#[tokio::test]
async fn test_network_start() -> Result<()> {
    setup_logger(LevelFilter::Info);

    let mut config = NetworkConfig::default();
    let (mut service, ..) = network_init(&mut config, None, None).await?;

    loop {
        if let SwarmEvent::NewListenAddr { address, .. } =
            timeout(Duration::from_secs(5), service.swarm.select_next_some())
                .await
                .expect("event to be received")
        {
            info!("SwarmEvent::NewListenAddr: {address:?}");
            break;
        }
    }
    Ok(())
}

#[tokio::test]
async fn test_network_gossip() -> Result<()> {
    setup_logger(LevelFilter::Info);
    let mut config = NetworkConfig::default();

    let (mut node_1, node_1_addrs, ..) = network_init(&mut config, None, None).await?;
    let (mut node_2, ..) = network_init(&mut config, Some(node_1_addrs), None).await?;

    loop {
        select! {
            event_1 = node_1.swarm.select_next_some() => {
                if let SwarmEvent::ConnectionEstablished { .. } = event_1 {
                    let topic = Topic::new(URSA_GLOBAL);
                    if let Err(error) = node_1.swarm.behaviour_mut().publish(topic, Bytes::from_static(b"hello world!")) {
                        warn!("Failed to send with error: {error:?}");
                    };
                }
            }
            event_2 = node_2.swarm.select_next_some() => {
                if let SwarmEvent::Behaviour(BehaviourEvent::Gossipsub(
                    libp2p::gossipsub::GossipsubEvent::Message {
                        propagation_source,
                        message_id,
                        message,
                    },
                )) = event_2
                {
                    info!(
                        "peer: {propagation_source:?}, id: {message_id:?}, message: {message:?}"
                    );
                    assert_eq!(Bytes::from_static(b"hello world!"), message.data);
                    break;
                }
            }
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_network_mdns() -> Result<()> {
    setup_logger(LevelFilter::Info);
    let mut config = NetworkConfig {
        mdns: true,
        bootstrap_nodes: vec![],
        ..Default::default()
    };

    let (node_1, _, peer_id_1, ..) = network_init(&mut config, None, None).await?;
    tokio::task::spawn(async move { node_1.start().await.unwrap() });

    let (mut node_2, ..) = network_init(&mut config, None, None).await?;

    loop {
        let event = node_2.swarm.select_next_some().await;
        if let SwarmEvent::ConnectionEstablished { peer_id, .. } = event {
            info!("[SwarmEvent::ConnectionEstablished]: {peer_id:?}, {peer_id_1:?}");
            if peer_id == peer_id_1 {
                break;
            }
        };
    }
    Ok(())
}

#[tokio::test]
async fn test_network_kad() -> Result<()> {
    setup_logger(LevelFilter::Info);

    let (bootstrap, bootstrap_addr, bootstrap_id) =
        run_bootstrap(&mut NetworkConfig::default()).await?;

    tokio::task::spawn(async move { bootstrap.start().await.unwrap() });

    let (mut node_1, _, peer_id_1, ..) = network_init(
        &mut NetworkConfig::default(),
        Some(bootstrap_addr.clone()),
        None,
    )
    .await?;

    // wait for node 1 to identify with bootstrap
    loop {
        if let SwarmEvent::Behaviour(BehaviourEvent::Identify(libp2p::identify::Event::Sent {
            peer_id,
            ..
        })) = node_1.swarm.select_next_some().await
        {
            info!("[SwarmEvent::Identify::Sent]: {peer_id:?}, {bootstrap_id:?}");
            if peer_id == bootstrap_id {
                break;
            }
        }
    }

    tokio::task::spawn(async move { node_1.start().await.unwrap() });

    let (mut node_2, ..) =
        network_init(&mut NetworkConfig::default(), Some(bootstrap_addr), None).await?;

    // wait for node 2 to connect with node 1 through kad peer discovery
    loop {
        if let SwarmEvent::ConnectionEstablished { peer_id, .. } =
            node_2.swarm.select_next_some().await
        {
            info!("[SwarmEvent::ConnectionEstablished]: {peer_id:?}, {peer_id_1:?}");
            if peer_id == peer_id_1 {
                break;
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn test_network_req_res() -> Result<()> {
    setup_logger(LevelFilter::Info);
    let mut config = NetworkConfig::default();

    let (mut node_1, node_1_addrs, peer_id_1, ..) = network_init(&mut config, None, None).await?;
    let (mut node_2, _, peer_id_2, ..) =
        network_init(&mut config, Some(node_1_addrs), None).await?;

    // Wait for at least one connection
    loop {
        if let SwarmEvent::ConnectionEstablished { peer_id, .. } =
            node_1.swarm.select_next_some().await
        {
            info!("[SwarmEvent::ConnectionEstablished]: {peer_id:?}, {peer_id_1:?}: ");
            break;
        }
    }

    let node_1_sender = node_1.command_sender();
    tokio::task::spawn(async move { node_1.start().await.unwrap() });

    let (sender, _) = oneshot::channel();
    let request = UrsaExchangeRequest(RequestType::CarRequest("Qm".to_string()));
    let msg = NetworkCommand::SendRequest {
        peer_id: peer_id_2,
        request,
        channel: sender,
    };

    assert!(node_1_sender.send(msg).is_ok());

    loop {
        if let SwarmEvent::Behaviour(BehaviourEvent::RequestResponse(
            RequestResponseEvent::Message { peer, message },
        )) = timeout(Duration::from_secs(5), node_2.swarm.select_next_some())
            .await
            .expect("event to be received")
        {
            info!("[RequestResponseEvent::Message]: {peer:?}, {message:?}");
            break;
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_bitswap_get() -> Result<()> {
    setup_logger(LevelFilter::Info);
    let mut config = NetworkConfig {
        mdns: true,
        ..Default::default()
    };

    let (mut node_1, node_1_addrs, peer_id_1, store_1) =
        network_init(&mut config, None, None).await?;
    let (node_2, _, _, store_2) = network_init(&mut config, Some(node_1_addrs), None).await?;

    let bitswap_store_1 = BitswapStorage(store_1.clone());
    let mut bitswap_store_2 = BitswapStorage(store_2.clone());

    let block = get_block(&b"hello world"[..]);
    info!("inserting block into bitswap store for node 1");
    insert_block(bitswap_store_1, &block);

    // Wait for at least one connection
    loop {
        if let SwarmEvent::ConnectionEstablished { peer_id, .. } =
            node_1.swarm.select_next_some().await
        {
            info!(
                "[SwarmEvent::ConnectionEstablished]: {:?}, {:?}: ",
                peer_id, peer_id_1
            );
            break;
        }
    }

    let node_2_sender = node_2.command_sender();

    // Start nodes
    tokio::task::spawn(async move { node_1.start().await.unwrap() });
    tokio::task::spawn(async move { node_2.start().await.unwrap() });

    let (sender, receiver) = oneshot::channel();
    let msg = NetworkCommand::GetBitswap {
        cid: *block.cid(),
        sender,
    };

    assert!(node_2_sender.send(msg).is_ok());

    let res = receiver
        .await
        .expect("Unable to receive from bitswap channel");

    match res {
        Ok(_) => {
            let store_1_block = bitswap_store_2.get(block.cid()).unwrap();

            info!(
                "inserting block into bitswap store for node 1, {:?}",
                store_1_block
            );
            assert_eq!(store_1_block, Some(block.data().to_vec()));
        }
        Err(e) => panic!("{e:?}"),
    }

    Ok(())
}

#[tokio::test]
async fn test_bitswap_sync() -> Result<()> {
    setup_logger(LevelFilter::Info);
    let mut config = NetworkConfig {
        mdns: true,
        ..Default::default()
    };

    let (mut node_1, node_1_addrs, peer_id_1, store_1) =
        network_init(&mut config, None, None).await?;
    let (node_2, _, _, store_2) = network_init(&mut config, Some(node_1_addrs), None).await?;

    let mut bitswap_store_2 = BitswapStorage(store_2.clone());

    // Wait for at least one connection
    loop {
        if let SwarmEvent::ConnectionEstablished { peer_id, .. } =
            node_1.swarm.select_next_some().await
        {
            info!("[SwarmEvent::ConnectionEstablished]: {peer_id:?}, {peer_id_1:?}: ");
            break;
        }
    }

    let node_2_sender = node_2.command_sender();

    // Start nodes
    tokio::task::spawn(async move { node_1.start().await.unwrap() });
    tokio::task::spawn(async move { node_2.start().await.unwrap() });

    // put the car file in store 1
    let path = Path::new("../../test_files/test.car");
    let file = File::open(path).await?;
    let reader = BufReader::new(file);
    let cids = load_car(store_1.blockstore(), reader).await?;

    let file_h = File::open(path).await?;
    let reader_h = BufReader::new(file_h);
    let mut car_reader = CarReader::new(reader_h).await?;

    let mut cids_vec = Vec::<Cid>::new();
    while let Some(block) = car_reader.next_block().await? {
        cids_vec.push(block.cid);
    }

    let (sender, receiver) = oneshot::channel();
    let msg = NetworkCommand::GetBitswap {
        cid: cids[0],
        sender,
    };

    assert!(node_2_sender.send(msg).is_ok());

    let res = receiver
        .await
        .expect("Unable to receive from bitswap channel");

    match res {
        Ok(_) => {
            for cid in cids_vec {
                assert!(bitswap_store_2.contains(&cid).unwrap());
            }
        }
        Err(e) => panic!("{e:?}"),
    }

    Ok(())
}

#[tokio::test]
async fn test_put_command() -> Result<()> {
    setup_logger(LevelFilter::Info);

    // Set up bootstrap.
    let (bootstrap, bootstrap_addr, bootstrap_id) =
        run_bootstrap(&mut NetworkConfig::default()).await?;
    tokio::task::spawn(async move { bootstrap.start().await.unwrap() });

    // Set up node 1.
    let (mut node_1, _, peer_id_1, .., store_1) = network_init(
        &mut NetworkConfig::default(),
        Some(bootstrap_addr.clone()),
        None,
    )
    .await?;

    // Store some data in node 1's store.
    let mut graphsync_store_1 = GraphSyncStorage(store_1.clone());
    let block = get_block(&b"hello world"[..]);
    info!("inserting block into Graphsync store for node 1");
    graphsync_store_1.insert(&block).unwrap();
    assert!(graphsync_store_1.has(block.cid()).unwrap());

    let node_1_sender = node_1.command_sender();

    // Wait for node 1 to identify with bootstrap then start it up.
    loop {
        if let SwarmEvent::Behaviour(BehaviourEvent::Identify(libp2p::identify::Event::Sent {
            peer_id,
            ..
        })) = node_1.swarm.select_next_some().await
        {
            info!("[SwarmEvent::Identify::Sent]: {peer_id:?}, {bootstrap_id:?}");
            if peer_id == bootstrap_id {
                break;
            }
        }
    }
    tokio::task::spawn(async move { node_1.start().await.unwrap() });

    // Set up node 2.
    let (mut node_2, .., store_2) =
        network_init(&mut NetworkConfig::default(), Some(bootstrap_addr), None).await?;

    let graphsync_store_2 = GraphSyncStorage(store_2.clone());
    // Node 2 does not have blocks in its store.
    assert!(!graphsync_store_2.has(block.cid()).unwrap());

    // Wait for node 2 to connect with node 1 through kad peer discovery then start it up.
    loop {
        if let SwarmEvent::ConnectionEstablished { peer_id, .. } =
            node_2.swarm.select_next_some().await
        {
            info!("[SwarmEvent::ConnectionEstablished]: {peer_id:?}, {peer_id_1:?}");
            if peer_id == peer_id_1 {
                break;
            }
        }
    }
    // Wait for node 2 to finish bootstrapping.
    loop {
        if let SwarmEvent::Behaviour(BehaviourEvent::Kad(
            KademliaEvent::OutboundQueryProgressed {
                result: QueryResult::Bootstrap(Ok(BootstrapOk { num_remaining, .. })),
                ..
            },
        )) = node_2.swarm.select_next_some().await
        {
            if num_remaining == 0 {
                info!("[KademliaEvent::Bootstrap]: Node 2 is done bootstrapping");
                break;
            }
        }
    }
    tokio::task::spawn(async move { node_2.start().await.unwrap() });

    // Send node 1 a PUT command.
    let (sender, receiver) = oneshot::channel();
    let request = NetworkCommand::Put {
        cid: *block.cid(),
        sender,
    };
    assert!(node_1_sender.send(request).is_ok());
    assert!(receiver.await.is_ok());

    // Wait for node 1 to send cache request to node 2.
    // Wait for node 2 to pull content from node 1.
    for s in (3..5).rev() {
        tokio::time::sleep(Duration::from_secs(s)).await;

        let store_1_block = graphsync_store_2.get(block.cid()).unwrap();
        info!("Block received {store_1_block:?}");

        if store_1_block.is_some() {
            assert_eq!(store_1_block, Some(block.data().to_vec()));
            return Ok(());
        }
    }

    panic!("Failed to replicate content")
}

#[tokio::test]
async fn test_send_cache_summary() -> Result<()> {
    setup_logger(LevelFilter::Info);
    let mut config = NetworkConfig::default();

    let (mut node_1, node_1_addrs, peer_id_1, ..) = network_init(&mut config, None, None).await?;
    let (mut node_2, _, peer_id_2, ..) =
        network_init(&mut config, Some(node_1_addrs), None).await?;

    loop {
        select! {
            event_1 = node_1.swarm.select_next_some() => {
                if let SwarmEvent::ConnectionEstablished { .. } = event_1 {
                    let mut cached_content = CountingBloomFilter::default();
                    cached_content.insert(&Cid::default().to_bytes());
                    let request = UrsaExchangeRequest(RequestType::StoreSummary(
                        cached_content,
                    ));
                    node_1.swarm.behaviour_mut().request_response.send_request(&peer_id_2, request);
                }
            }
            event_2 = node_2.swarm.select_next_some() => {
                if let SwarmEvent::Behaviour(BehaviourEvent::RequestResponse(
                                         RequestResponseEvent::Message { peer, message },
                )) = event_2
                {
                    let event = SwarmEvent::Behaviour(BehaviourEvent::RequestResponse(
                        RequestResponseEvent::Message { peer, message }
                    ));
                    node_2.handle_swarm_event(event).unwrap();
                    break;
                }
            }
        }
    }

    // check if cid exists
    let cached_content = node_2
        .peer_cached_content
        .get(&peer_id_1)
        .expect("Peer id not contained in peer content.");
    assert!(
        cached_content.contains(&Cid::default().to_bytes()),
        "CID not contained in cache summary."
    );

    Ok(())
}
