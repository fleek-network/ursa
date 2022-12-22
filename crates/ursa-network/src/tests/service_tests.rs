#[cfg(test)]
mod tests {
    use crate::behaviour::BehaviourEvent;
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
    use libipld::{cbor::DagCborCodec, ipld, Block, DefaultParams, Ipld};
    use libp2p::request_response::RequestResponseEvent;
    use libp2p::{
        gossipsub::IdentTopic as Topic, identity::Keypair, multiaddr::Protocol, swarm::SwarmEvent,
        Multiaddr, PeerId,
    };
    use libp2p_bitswap::BitswapStore;
    use std::fmt::Display;
    use std::future::Future;
    use std::path::Path;
    use std::sync::Once;
    use std::{sync::Arc, time::Duration, vec};
    use tokio::task::JoinHandle;
    use tokio::{select, sync::oneshot, time::timeout};
    use tracing::warn;
    use tracing::{error, info, level_filters::LevelFilter};
    use tracing_subscriber::fmt::format::debug_fn;
    use ursa_store::{BitswapStorage, UrsaStore};

    fn create_block(ipld: Ipld) -> Block<DefaultParams> {
        Block::encode(DagCborCodec, Code::Blake3_256, &ipld).unwrap()
    }

    static INIT: Once = Once::new();

    fn setup_logger(level: LevelFilter) {
        INIT.call_once(|| {
            if let Err(err) = tracing_subscriber::fmt()
                .with_max_level(level)
                .with_thread_names(true)
                .with_target(false)
                .try_init()
            {
                error!("Logger already set: {err:?}")
            }
        })
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

    /// Creates a new subscriber for the closure with a given tag displayed for each log message.
    fn tag<S: Display, T>(tag: S, f: impl FnOnce() -> T) -> T {
        let tag = tag.to_string();
        let bootstrap_fmt = tracing_subscriber::fmt()
            .with_thread_names(true)
            .with_target(false)
            .fmt_fields(debug_fn(move |w, _, m| write!(w, "[{tag}] {m:?}")))
            .finish();
        tracing::subscriber::with_default(bootstrap_fmt, f)
    }

    pub fn spawn<T>(t: impl Display, f: T) -> JoinHandle<T::Output>
    where
        T: Future + Send + 'static,
        T::Output: Send + 'static,
    {
        tag(t, || tokio::task::spawn(f))
    }

    async fn run_bootstrap(
        config: Option<NetworkConfig>,
    ) -> Result<(UrsaService<MemoryDB>, Multiaddr, PeerId)> {
        let keypair = Keypair::generate_ed25519();
        let mut config = config.unwrap_or(NetworkConfig {
            mdns: false,
            bootstrapper: true,
            bootstrap_nodes: vec![],
            ..Default::default()
        });

        let (bootstrap, swarm_addr, peer_id, ..) =
            network_init(&mut config, None, Some(keypair)).await?;
        Ok((bootstrap, swarm_addr, peer_id))
    }

    async fn network_init(
        config: &mut NetworkConfig,
        bootstrap_addr: Option<Multiaddr>,
        keypair: Option<Keypair>,
    ) -> Result<(
        UrsaService<MemoryDB>,
        Multiaddr,
        PeerId,
        Arc<UrsaStore<MemoryDB>>,
    )> {
        // set listen addr to random port
        config.swarm_addrs = vec!["/ip4/0.0.0.0/tcp/0".parse().unwrap()];

        let keypair = keypair.unwrap_or_else(Keypair::generate_ed25519);
        let peer_id = PeerId::from(keypair.clone().public());
        let store = get_store();

        if let Some(addr) = bootstrap_addr {
            config.bootstrap_nodes = vec![addr];
        }

        let mut service = UrsaService::new(keypair, config, Arc::clone(&store))?;

        let node_addrs = async {
            loop {
                let event = timeout(Duration::from_secs(5), service.swarm.select_next_some())
                    .await
                    .expect("received some event");

                match event {
                    SwarmEvent::NewListenAddr { mut address, .. } => {
                        address.push(Protocol::P2p(peer_id.into()));
                        return address;
                    }
                    _ => service.handle_swarm_event(event).expect("new listen addr"),
                }
            }
        }
        .await;

        Ok((service, node_addrs, peer_id, store))
    }

    #[tokio::test]
    async fn test_network_start() -> Result<()> {
        setup_logger(LevelFilter::INFO);

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
        setup_logger(LevelFilter::INFO);
        let mut config = NetworkConfig {
            bootstrap_nodes: vec![],
            ..Default::default()
        };

        let (mut node1, node1_addr, ..) = network_init(&mut config, None, None).await?;
        let (mut node2, _, node2_id, ..) =
            network_init(&mut config, Some(node1_addr), None).await?;

        loop {
            select! {
                event = node1.swarm.select_next_some() => match event {
                    SwarmEvent::Behaviour(BehaviourEvent::Identify(libp2p::identify::Event::Received { peer_id, .. })) => {
                        tag("node 1", || node1.handle_swarm_event(event))?;
                        if peer_id == node2_id {
                            let topic = Topic::new(URSA_GLOBAL);
                            if let Err(error) = node1.swarm.behaviour_mut().publish(topic, Bytes::from_static(b"hello world!")) {
                                warn!("Failed to send with error: {error:?}");
                            };
                        }
                    }
                    event => tag("node 1", || node1.handle_swarm_event(event))?
                },
                event = node2.swarm.select_next_some() => match event {
                    SwarmEvent::Behaviour(BehaviourEvent::Gossipsub(
                        libp2p::gossipsub::GossipsubEvent::Message {
                            propagation_source,
                            message_id,
                            message,
                        },
                    )) => {
                        info!(
                            "peer: {propagation_source:?}, id: {message_id:?}, message: {message:?}"
                        );
                        assert_eq!(Bytes::from_static(b"hello world!"), message.data);
                        break;
                    }
                    event => tag("node 2", || node2.handle_swarm_event(event))?
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_network_mdns() -> Result<()> {
        setup_logger(LevelFilter::INFO);
        let mut config = NetworkConfig {
            mdns: true,
            bootstrap_nodes: vec![],
            ..Default::default()
        };

        let (mut node1, _, node1_id, ..) = network_init(&mut config, None, None).await?;
        let (mut node2, ..) = network_init(&mut config, None, None).await?;

        loop {
            select! {
                event = node1.swarm.select_next_some() => tag("node 1", || node1.handle_swarm_event(event))?,
                event = node2.swarm.select_next_some() => match event {
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        tag("node 2", || node2.handle_swarm_event(event))?;
                        if peer_id == node1_id {
                            break;
                        }
                    }
                    event => tag("node 2", || node2.handle_swarm_event(event))?
                }
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_network_kad() -> Result<()> {
        setup_logger(LevelFilter::INFO);

        let (mut bootstrap, bootstrap_addr, bootstrap_id) = run_bootstrap(None).await?;

        let mut config = NetworkConfig {
            bootstrap_nodes: vec![bootstrap_addr],
            ..Default::default()
        };

        let (mut node1, _, node1_id, ..) = network_init(&mut config, None, None).await?;

        // Wait for node 1 to identify with bootstrap
        loop {
            select! {
                event = bootstrap.swarm.select_next_some() => tag("bootstrap", || bootstrap.handle_swarm_event(event))?,
                event = node1.swarm.select_next_some() => match event {
                    SwarmEvent::Behaviour(BehaviourEvent::Identify(libp2p::identify::Event::Received { peer_id, .. })) => {
                        tag("node 1", || node1.handle_swarm_event(event))?;
                        if peer_id == bootstrap_id {
                            info!("[node 1] identified with bootstrap");
                            break;
                        }
                    }
                    event => tag("node 1", || node1.handle_swarm_event(event))?
                }
            }
        }

        let (mut node2, ..) = tag("node 2", || network_init(&mut config, None, None)).await?;

        // wait for node 2 to connect to node 1 automatically through kad discovery
        loop {
            select! {
                event = bootstrap.swarm.select_next_some() => tag("bootstrap", || bootstrap.handle_swarm_event(event))?,
                event = node1.swarm.select_next_some() => tag("node 1", || node1.handle_swarm_event(event))?,
                event = node2.swarm.select_next_some() => match event {
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        if peer_id == node1_id {
                            info!("[node 2] connected established with node 1");
                            break;
                        }
                    }
                    event => tag("node 2", || node2.handle_swarm_event(event))?
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_network_req_res() -> Result<()> {
        setup_logger(LevelFilter::INFO);
        let mut config = NetworkConfig::default();

        let (mut node1, node1_addrs, node1_id, ..) = network_init(&mut config, None, None).await?;
        let (mut node2, _, node2_id, ..) =
            network_init(&mut config, Some(node1_addrs), None).await?;

        // Wait for node 2 to connect to node 1
        loop {
            select! {
                event = node1.swarm.select_next_some() => tag("node 1", || node1.handle_swarm_event(event))?,
                event = node2.swarm.select_next_some() => match event {
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        if peer_id == node1_id {
                            info!("[test] connected established with node 1");
                            break;
                        }
                    }
                    event => tag("node 2", || node2.handle_swarm_event(event))?
                }
            }
        }

        let node1_sender = node1.command_sender();
        spawn("node 1", async move { node1.start().await.unwrap() });

        let (sender, _) = oneshot::channel();
        let request = UrsaExchangeRequest(RequestType::CarRequest("Qm".to_string()));
        let msg = NetworkCommand::SendRequest {
            peer_id: node2_id,
            request,
            channel: sender,
        };

        assert!(node1_sender.send(msg).is_ok());

        // wait for either node 2 to receive the message, or 5 seconds
        timeout(
            Duration::from_secs(5),
            tag("node 2", || async {
                loop {
                    match node2.swarm.select_next_some().await {
                        SwarmEvent::Behaviour(BehaviourEvent::RequestResponse(
                            RequestResponseEvent::Message { peer, message },
                        )) => {
                            info!("[RequestResponseEvent::Message]: {peer:?}, {message:?}");
                            break;
                        }
                        event => node2.handle_swarm_event(event).unwrap(),
                    }
                }
            }),
        )
        .await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_bitswap_get() -> Result<()> {
        setup_logger(LevelFilter::INFO);
        let mut config = NetworkConfig {
            bootstrap_nodes: vec![],
            ..Default::default()
        };

        let (mut node1, node1_addr, node1_id, node1_store) =
            network_init(&mut config, None, None).await?;
        let (mut node2, _, _, node2_store) =
            network_init(&mut config, Some(node1_addr), None).await?;

        let bitswap_store_1 = BitswapStorage(node1_store.clone());
        let mut bitswap_store_2 = BitswapStorage(node2_store.clone());

        let block = get_block(&b"hello world"[..]);
        info!("inserting block into bitswap store for node 1");
        insert_block(bitswap_store_1, &block);

        // Wait for node 2 to identify with node 1
        loop {
            select! {
                event = node1.swarm.select_next_some() => tag("node 1", || node1.handle_swarm_event(event))?,
                event = node2.swarm.select_next_some() => match event {
                    SwarmEvent::Behaviour(BehaviourEvent::Identify(libp2p::identify::Event::Received { peer_id, .. })) => {
                        tag("node 2", || node2.handle_swarm_event(event))?;
                        if peer_id == node1_id {
                            info!("[node 2] identified with node 1");
                            break;
                        }
                    }
                    event => tag("node 2", || node2.handle_swarm_event(event))?
                }
            }
        }

        let node2_sender = node2.command_sender();

        // Start nodes
        spawn("node 1", async move { node1.start().await.unwrap() });
        spawn("node 2", async move { node2.start().await.unwrap() });

        let (sender, receiver) = oneshot::channel();
        let msg = NetworkCommand::GetBitswap {
            cid: *block.cid(),
            sender,
        };

        assert!(node2_sender.send(msg).is_ok());

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
        setup_logger(LevelFilter::INFO);
        let mut config = NetworkConfig {
            bootstrap_nodes: vec![],
            ..Default::default()
        };

        let (mut node1, node1_addr, node1_id, node1_store) =
            network_init(&mut config, None, None).await?;
        let (mut node2, _, _, node2_store) =
            network_init(&mut config, Some(node1_addr), None).await?;

        let mut bitswap_store_2 = BitswapStorage(node2_store.clone());

        // Wait for node 2 to identify with node 1
        loop {
            select! {
                event = node1.swarm.select_next_some() => tag("node 1", || node1.handle_swarm_event(event))?,
                event = node2.swarm.select_next_some() => match event {
                    SwarmEvent::Behaviour(BehaviourEvent::Identify(libp2p::identify::Event::Received { peer_id, .. })) => {
                        tag("node 2", || node2.handle_swarm_event(event))?;
                        if peer_id == node1_id {
                            info!("[node 2] identified with node 1");
                            break;
                        }
                    }
                    event => tag("node 2", || node2.handle_swarm_event(event))?
                }
            }
        }

        let node2_sender = node2.command_sender();

        // Start nodes
        spawn("node 1", async move { node1.start().await.unwrap() });
        spawn("node 2", async move { node2.start().await.unwrap() });

        // Initialize node 1's store with the test car file
        let path = Path::new("../../test_files/test.car");
        let file = File::open(path).await?;
        let reader = BufReader::new(file);
        let cids = load_car(node1_store.blockstore(), reader).await?;

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

        assert!(node2_sender.send(msg).is_ok());

        let res = receiver
            .await
            .expect("Unable to receive from bitswap channel");

        match res {
            Ok(_) => {
                for cid in cids_vec {
                    assert!(bitswap_store_2.contains(&cid).is_ok());
                }
            }
            Err(e) => panic!("{e:?}"),
        }

        Ok(())
    }
}
