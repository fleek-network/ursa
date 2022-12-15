#[cfg(test)]
mod tests {
    use crate::behaviour::BehaviourEvent;
    use crate::{
        codec::protocol::{RequestType, UrsaExchangeRequest},
        discovery::DiscoveryEvent,
        NetworkCommand, NetworkConfig, UrsaService, URSA_GLOBAL,
    };
    use anyhow::Result;
    use bytes::Bytes;
    use db::MemoryDB;
    use futures::StreamExt;
    use fvm_ipld_car::{load_car, CarReader};
    use libipld::{cbor::DagCborCodec, ipld, multihash::Code, Block, DefaultParams, Ipld};
    use libp2p::{
        gossipsub::IdentTopic as Topic, identity::Keypair, multiaddr::Protocol, swarm::SwarmEvent,
        Multiaddr, PeerId,
    };
    use libp2p_bitswap::BitswapStore;
    use simple_logger::SimpleLogger;
    use tracing::warn;
    use std::{sync::Arc, time::Duration, vec};
    use tokio::{select, sync::oneshot, time::timeout};
    use tracing::{error, info, log::LevelFilter};
    use ursa_index_provider::{config::ProviderConfig, provider::Provider};
    use ursa_store::{BitswapStorage, Store};
    use ursa_utils::convert_cid;

    fn create_block(ipld: Ipld) -> Block<DefaultParams> {
        Block::encode(DagCborCodec, Code::Blake3_256, &ipld).unwrap()
    }

    async fn run_bootstrap(
        config: &mut NetworkConfig,
        port: u16,
    ) -> Result<(UrsaService<MemoryDB>, String)> {
        let keypair = Keypair::generate_ed25519();
        let swarm_addr = format!("/ip4/127.0.0.1/tcp/{}", port);
        config.swarm_addrs = vec![swarm_addr.clone().parse().unwrap()];
        let addr = format!("{}/p2p/{}", swarm_addr, PeerId::from(keypair.public()));
        let (bootstrap, ..) = network_init(config, Some(addr.clone()), Some(keypair)).await?;
        Ok((bootstrap, addr))
    }

    async fn network_init(
        config: &mut NetworkConfig,
        bootstrap_addr: Option<String>,
        bootstrap_keypair: Option<Keypair>,
    ) -> Result<(
        UrsaService<MemoryDB>,
        Multiaddr,
        PeerId,
        Arc<Store<MemoryDB>>,
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
        let index_store = get_store();

        if let Some(addr) = bootstrap_addr {
            config.bootstrap_nodes = [addr].iter().map(|node| node.parse().unwrap()).collect();
        }
        let provider_config = ProviderConfig::default();
        let index_provider = Provider::new(keypair.clone(), index_store, provider_config.clone());

        let mut service = UrsaService::new(keypair, &config, Arc::clone(&store), index_provider)?;

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

    fn setup_logger(level: LevelFilter) {
        if let Err(err) = SimpleLogger::new()
            .with_level(level)
            .with_utc_timestamps()
            .init()
        {
            info!("Logger already set. Ignore.")
        }
    }

    fn get_store() -> Arc<Store<MemoryDB>> {
        let db = Arc::new(MemoryDB::default());
        Arc::new(Store::new(Arc::clone(&db)))
    }

    fn get_block(content: &[u8]) -> Block<DefaultParams> {
        create_block(ipld!(&content[..]))
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

    #[tokio::test]
    async fn test_network_start() -> Result<()> {
        setup_logger(LevelFilter::Info);

        let mut config = NetworkConfig::default();
        let (mut service, ..) = network_init(&mut config, None, None).await?;

        loop {
            if let SwarmEvent::NewListenAddr { address, .. } =
                timeout(Duration::from_secs(20), service.swarm.select_next_some())
                    .await
                    .expect("event to be received")
            {
                info!("SwarmEvent::NewListenAddr: {:?}:", address);
                break;
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_network_gossip() -> Result<()> {
        setup_logger(LevelFilter::Info);
        let mut config = NetworkConfig::default();

        let (mut node_1, node_1_addrs, peer_id_1, ..) =
            network_init(&mut config, None, None).await?;
        let (mut node_2, ..) =
            network_init(&mut config, Some(node_1_addrs.to_string()), None).await?;

        loop {
            select! {
                event_1 = node_1.swarm.select_next_some() => {
                    if let SwarmEvent::ConnectionEstablished { .. } = event_1 {
                        // Construct gossip messsage
                        let topic = Topic::new(URSA_GLOBAL);
                        if let Err(error) = node_1.swarm.behaviour_mut().publish(topic, Bytes::from_static(b"hello world!")) {
                            warn!("Failed to send with error: {:?}", error);
                        };
                        // let (sender, receiver) = oneshot::channel();
                        // let msg = NetworkCommand::GossipsubMessage {
                        //     peer_id: peer_id_1,
                        //     message: GossipsubMessage::Publish {
                        //         topic: topic.hash(),
                        //         data: Bytes::from_static(b"hello world!"),
                        //         sender,
                        //     },
                        // };

                        // let res = command_sender.send(msg);
                        // assert!(res.is_ok());

                        // let res = receiver.await;
                        // assert!(res.is_ok());
                        // assert!(res.unwrap().is_ok());
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
                            "peer: {:?}, id: {:?}, messsage: {:?}",
                            propagation_source, message_id, message
                        );
                        assert_eq!(Bytes::from_static(b"hello world!"), message.data);
                        break;
                    }
                }
            };
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_network_mdns() -> Result<()> {
        setup_logger(LevelFilter::Info);
        let mut config = NetworkConfig {
            mdns: true,
            ..Default::default()
        };

        let (mut node_1, _, peer_id_1, ..) = network_init(&mut config, None, None).await?;
        let (mut node_2, ..) = network_init(&mut config, None, None).await?;

        loop {
            select! {
                event_2 = node_2.swarm.select_next_some() => {
                    if let SwarmEvent::Behaviour(BehaviourEvent::Discovery(DiscoveryEvent::Connected(peer_id))) = event_2 {
                        info!("[BehaviourEvent::Discovery(event)]: {:?}, {:?}: ", peer_id, peer_id_1);
                        if peer_id == peer_id_1 {
                            break
                        }
                    }
                }
                _ = node_1.swarm.select_next_some() => {}
            };
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_network_discovery() -> Result<()> {
        setup_logger(LevelFilter::Info);
        let mut config = NetworkConfig::default();

        let (mut node_1, _, peer_id_1, ..) = network_init(&mut config, None, None).await?;
        let (mut node_2, ..) = network_init(&mut config, None, None).await?;

        loop {
            select! {
                event_2 = node_2.swarm.select_next_some() => {
                    if let SwarmEvent::Behaviour(BehaviourEvent::Discovery(DiscoveryEvent::Connected(peer_id))) = event_2 {
                        info!("[BehaviourEvent::Discovery(event)]: {:?}, {:?}: ", peer_id, peer_id_1);
                        if peer_id == peer_id_1 {
                            break
                        }
                    }
                }
                _ = node_1.swarm.select_next_some() => {}
            };
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_network_req_res() -> Result<()> {
        setup_logger(LevelFilter::Debug);
        let mut config = NetworkConfig::default();

        let (mut node_1, node_1_addrs, peer_id_1, ..) =
            network_init(&mut config, None, None).await?;
        let (mut node_2, _, peer_id_2, ..) =
            network_init(&mut config, Some(node_1_addrs.to_string()), None).await?;

        let node_1_sender = node_1.command_sender();

        // tokio::task::spawn(async move { node_1.start().await.unwrap() });

        let (sender, receiver) = oneshot::channel();
        let request = UrsaExchangeRequest(RequestType::CarRequest("Qm".to_string()));
        let msg = NetworkCommand::SendRequest {
            peer_id: peer_id_2,
            request,
            channel: sender,
        };

        loop {
            select! {
                event_2 = node_2.swarm.select_next_some() => {

                    if let SwarmEvent::ConnectionEstablished { peer_id, .. } = event_2 {
                        info!("[SwarmEvent::ConnectionEstablished]: {:?}, {:?}: ", peer_id, peer_id_1);
                        if peer_id == peer_id_1 {
                            break
                        }
                    }
                }
                _ = node_1.swarm.select_next_some() => {}
            };
        }

        let (sender, receiver) = oneshot::channel();
        let request = UrsaExchangeRequest(RequestType::CarRequest("Qm".to_string()));
        let msg = NetworkCommand::SendRequest {
            peer_id: peer_id_2,
            request,
            channel: sender,
        };

        node_1_sender.send(msg)?;
        let value = receiver
            .await
            .expect("Unable to receive from peers channel");

        println!("Req/Res: {:?}: ", value);

        // wait for the peers to get connected before requesting bitswap
        // loop {
        //     let (peers_sender, peers_receiver) = oneshot::channel();
        //     let peers_msg = NetworkCommand::GetPeers { sender: peers_sender };
        //     node_1_sender.send(peers_msg)?;
        //     let value = peers_receiver.await.expect("Unable to receive from peers channel");
        //     println!("Req/Res: {:?}: ", value);
        //     break;
        // }

        // loop {
        //     if let Some(SwarmEvent::Behaviour(BehaviourEvent::RequestResponse(
        //         RequestResponseEvent::Message { peer, message },
        //     ))) = swarm_2.next().await
        //     {
        //         info!("Node 2 RequestMessage: {:?}", message);
        //         break;
        //     }
        // }

        Ok(())
    }

    #[tokio::test]
    async fn test_bitswap_get() -> Result<()> {
        setup_logger(LevelFilter::Info);
        let mut config = NetworkConfig {
            mdns: true,
            ..Default::default()
        };

        let (mut node_1, _, _, store_1) = network_init(&mut config, None, None).await?;
        let (mut node_2, _, _, store_2) = network_init(&mut config, None, None).await?;

        let bitswap_store_1 = BitswapStorage(store_1.clone());
        let mut bitswap_store_2 = BitswapStorage(store_2.clone());
        let block = get_block(&b"hello world"[..]);
        info!("inserting block into bitswap store for node 1");
        insert_block(bitswap_store_1, &block);

        let node_2_sender = node_2.command_sender();

        // tokio::task::spawn(async move { node_1.start().await.unwrap() });

        // wait for the peers to get connected before requesting bitswap
        // loop {
        //     let (peers_sender, peers_receiver) = oneshot::channel();
        //     let peers_msg = NetworkCommand::GetPeers {
        //         sender: peers_sender,
        //     };
        //     node_2_sender.send(peers_msg);
        //     let value = peers_receiver
        //         .await
        //         .expect("Unable to receive from peers channel");
        //     println!("PEERS: {:?}: ", value);
        //     break;
        // }

        let (sender, receiver) = oneshot::channel();
        let msg = NetworkCommand::GetBitswap {
            cid: convert_cid(block.cid().to_bytes()),
            sender,
        };

        tokio::task::spawn(async move {
            let send = node_2_sender.send(msg);
            if let Err(error) = send {
                error!("failed to send bitswap request: {:?}: ", error)
            } else {
                info!("send bitswap request!")
            }

            // let res = timeout(Duration::from_secs(10), receiver).await;

            // info!("RESPONSE: {:?}: ", res);

            let value = receiver
                .await
                .expect("Unable to receive from bitswap channel");

            info!("VALUE: {:?}: ", value);

            match value {
                Ok(_) => {
                    let store_1_block = bitswap_store_2
                        .get(&convert_cid(block.cid().to_bytes()))
                        .unwrap();
                    assert_eq!(store_1_block, Some(block.data().to_vec()));
                }
                Err(e) => panic!("{:?}", e),
            }
        })
        .await?;

        Ok(())
    }

    // #[tokio::test]
    // async fn test_bitswap_get_block_not_found() -> Result<()> {
    //     setup_logger(LevelFilter::Info);
    //     let mut config = NetworkConfig::default();

    //     let store1 = get_store("test_db1");
    //     let store2 = get_store("test_db2");
    //     let provider_db = RocksDb::open("index_provider_db", &RocksDbConfig::default())
    //         .expect("Opening RocksDB must succeed");
    //     let index_store = Arc::new(Store::new(Arc::clone(&Arc::new(provider_db))));

    //     let (node_1, _) = network_init(&mut config, Arc::clone(&store1), Arc::clone(&index_store));

    //     let block = get_block(&b"hello world"[..]);

    //     config.swarm_addr = "/ip4/0.0.0.0/tcp/6010".parse().unwrap();
    //     let (node_2, _) = network_init(&mut config, Arc::clone(&store2), Arc::clone(&index_store));

    //     let node_2_sender = node_2.command_sender.clone();

    //     tokio::task::spawn(async move { node_1.start().await.unwrap() });

    //     tokio::task::spawn(async move { node_2.start().await.unwrap() });

    //     let (sender, receiver) = oneshot::channel();

    //     let msg = NetworkCommand::GetBitswap {
    //         cid: convert_cid(block.cid().to_bytes()),
    //         sender,
    //     };

    //     node_2_sender.send(msg)?;

    //     futures::executor::block_on(async {
    //         info!("waiting for msg on block receive channel...");
    //         let value = receiver.await.expect("Unable to receive from channel");
    //         // TODO: fix the assertion for this test
    //         match value {
    //             Err(val) => assert_eq!(
    //                 val.to_string(),
    //                 format!(
    //                     "The requested block with cid {:?} is not found with any peers",
    //                     *block.cid()
    //                 )
    //             ),
    //             _ => {}
    //         }
    //     });

    //     Ok(())
    // }

    // #[tokio::test]
    // async fn add_block() -> Result<()> {
    //     setup_logger(LevelFilter::Info);
    //     let db = Arc::new(
    //         RocksDb::open("../test_db", &RocksDbConfig::default())
    //             .expect("Opening RocksDB must succeed"),
    //     );
    //     let store = Arc::new(Store::new(Arc::clone(&db)));

    //     let mut bitswap_store = BitswapStorage(store.clone());

    //     let block = get_block(&b"hello world"[..]);
    //     info!("inserting block into bitswap store for node");
    //     let cid = convert_cid(block.cid().to_bytes());
    //     let string_cid = Cid::to_string(&cid);
    //     info!("block cid to string : {:?}", string_cid);

    //     if let Err(err) = bitswap_store.insert(&block) {
    //         error!(
    //             "there was an error while inserting into the blockstore {:?}",
    //             err
    //         );
    //     } else {
    //         info!("block inserted successfully");
    //     }
    //     info!("{:?}", bitswap_store.contains(&convert_cid(cid.to_bytes())));

    //     Ok(())
    // }

    // #[tokio::test]
    // async fn get_block_local() -> Result<()> {
    //     setup_logger(LevelFilter::Info);
    //     let db1 = Arc::new(
    //         RocksDb::open("test_db2", &RocksDbConfig::default())
    //             .expect("Opening RocksDB must succeed"),
    //     );

    //     let store1 = Arc::new(Store::new(Arc::clone(&db1)));
    //     let mut bitswap_store_1 = BitswapStorage(store1.clone());

    //     let cid =
    //         Cid::from_str("bafkreif2opfibjypwkjzzry3jbibcjqcjwnpoqpeiqw75eu3s3u3zbdszq").unwrap();

    //     if let Ok(res) = bitswap_store_1.contains(&convert_cid(cid.to_bytes())) {
    //         println!("block exists in current db: {:?}", res);
    //     }

    //     Ok(())
    // }

    // #[tokio::test]
    // #[ignore]
    // async fn test_bitswap_sync() -> Result<()> {
    //     setup_logger(LevelFilter::Info);
    //     let mut config = NetworkConfig::default();

    //     let store1 = get_store("test_db1");
    //     let store2 = get_store("test_db2");

    //     let mut bitswap_store2 = BitswapStorage(store2.clone());
    //     let provider_db = RocksDb::open("index_provider_db", &RocksDbConfig::default())
    //         .expect("Opening RocksDB must succeed");
    //     let index_store = Arc::new(Store::new(Arc::clone(&Arc::new(provider_db))));

    //     let path = "../car_files/text_mb.car";

    //     // put the car file in store 1
    //     // patch fix blocking io is not good
    //     let file = File::open(path).await?;
    //     let reader = BufReader::new(file);
    //     let cids = load_car(store1.blockstore(), reader).await?;

    //     let file_h = File::open(path).await?;
    //     let reader_h = BufReader::new(file_h);
    //     let mut car_reader = CarReader::new(reader_h).await?;

    //     let mut cids_vec = Vec::<Cid>::new();
    //     while let Some(block) = car_reader.next_block().await? {
    //         cids_vec.push(block.cid);
    //     }

    //     let (node_1, _) = network_init(&mut config, Arc::clone(&store1), Arc::clone(&index_store));

    //     config.swarm_addr = "/ip4/0.0.0.0/tcp/6010".parse().unwrap();
    //     let (node_2, _) = network_init(&mut config, Arc::clone(&store2), Arc::clone(&index_store));

    //     let node_2_sender = node_2.command_sender.clone();

    //     tokio::task::spawn(async move { node_1.start().await.unwrap() });

    //     tokio::task::spawn(async move { node_2.start().await.unwrap() });

    //     let (sender, receiver) = oneshot::channel();

    //     let msg = NetworkCommand::GetBitswap {
    //         cid: cids[0],
    //         sender,
    //     };

    //     node_2_sender.send(msg)?;

    //     futures::executor::block_on(async {
    //         info!("waiting for msg on block receive channel...");
    //         let value = receiver.await.expect("Unable to receive from channel");
    //         if let Ok(_val) = value {
    //             for cid in cids_vec {
    //                 assert!(bitswap_store2
    //                     .contains(&convert_cid(cid.to_bytes()))
    //                     .unwrap());
    //             }
    //         }
    //     });
    //     Ok(())
    // }
}
