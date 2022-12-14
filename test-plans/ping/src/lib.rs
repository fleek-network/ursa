use anyhow::Result;
use env_logger::Env;
use futures::future::ready;
use futures::{FutureExt, StreamExt};
use log::info;
use rand::Rng;
use std::borrow::Cow;
use std::time::Duration;
use testground::network_conf::{
    FilterAction, LinkShape, NetworkConfiguration, RoutingPolicyType, DEFAULT_DATA_NETWORK,
};

const LISTENING_PORT: u16 = 1234;

#[async_trait::async_trait]
pub trait PingSwarm: Sized {
    async fn listen_on(&mut self, address: &str) -> Result<()>;

    fn dial(&mut self, address: &str) -> Result<()>;

    async fn await_connections(&mut self, number: usize);

    async fn await_pings(&mut self, number: usize);

    async fn loop_on_next(&mut self);

    fn local_peer_id(&self) -> String;
}

pub async fn run_ping<S>(mut swarm: S) -> Result<()>
    where
        S: PingSwarm,
{
    info!("Running ping test: {}", swarm.local_peer_id());

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let client = testground::client::Client::new_and_init().await.unwrap();

    let local_addr = match if_addrs::get_if_addrs()
        .unwrap()
        .into_iter()
        .find(|iface| iface.name == "eth1")
        .unwrap()
        .addr
        .ip()
    {
        std::net::IpAddr::V4(addr) => format!("/ip4/{addr}/tcp/{LISTENING_PORT}"),
        std::net::IpAddr::V6(_) => unimplemented!(),
    };

    info!(
        "Test instance, listening for incoming connections on: {:?}.",
        local_addr
    );

    swarm.listen_on(&local_addr).await?;

    let test_instance_count = client.run_parameters().test_instance_count as usize;
    let mut address_stream = client
        .subscribe("peers", test_instance_count)
        .await
        .take(test_instance_count)
        .map(|a| {
            let value = a.unwrap();
            value["Addrs"][0].as_str().unwrap().to_string()
        })
        .take_while(|a| ready(a != &local_addr));

    let payload = serde_json::json!({
        "ID": swarm.local_peer_id(),
        "Addrs": [
            local_addr
        ],
    });

    client.publish("peers", Cow::Owned(payload)).await?;

    while let Some(addr) = address_stream.next().await {
        swarm.dial(&addr).unwrap();
    }


    drop(address_stream);

    info!("Wait to connect to each peer.");

    swarm
        .await_connections(client.run_parameters().test_instance_count as usize - 1)
        .await;

    signal_wait_and_drive_swarm(&client, &mut swarm, "connected".to_string()).await?;

    ping(&client, &mut swarm, "initial".to_string()).await?;

    let iterations: usize = client
        .run_parameters()
        .test_instance_params
        .get("iterations")
        .unwrap()
        .parse()
        .unwrap();
    let max_latency_ms: u64 = client
        .run_parameters()
        .test_instance_params
        .get("max_latency_ms")
        .unwrap()
        .parse()
        .unwrap();

    for i in 1..iterations + 1 {
        client.record_message(format!("⚡️  ITERATION ROUND {}", i));

        let latency = Duration::from_millis(rand::thread_rng().gen_range(0..max_latency_ms))
            .as_nanos()
            .try_into()
            .unwrap();

        let network_conf = NetworkConfiguration {
            network: DEFAULT_DATA_NETWORK.to_owned(),
            ipv4: None,
            ipv6: None,
            enable: true,
            default: LinkShape {
                latency,
                jitter: 0,
                bandwidth: 0,
                filter: FilterAction::Accept,
                loss: 0.0,
                corrupt: 0.0,
                corrupt_corr: 0.0,
                reorder: 0.0,
                reorder_corr: 0.0,
                duplicate: 0.0,
                duplicate_corr: 0.0,
            },
            rules: None,
            callback_state: format!("network-configured-{}", i),
            callback_target: Some(client.run_parameters().test_instance_count),
            routing_policy: RoutingPolicyType::AllowAll,
        };

        client.configure_network(network_conf).await.unwrap();

        ping(&client, &mut swarm, format!("done-{}", i)).await?;
    }

    client.record_success().await?;

    Ok(())
}

async fn ping<S>(client: &testground::client::Client, swarm: &mut S, tag: String) -> Result<()>
    where
        S: PingSwarm,
{
    info!("Wait to receive ping from each peer.");

    swarm
        .await_pings(client.run_parameters().test_instance_count as usize - 1)
        .await;

    signal_wait_and_drive_swarm(client, swarm, tag).await
}

async fn signal_wait_and_drive_swarm<S>(
    client: &testground::client::Client,
    swarm: &mut S,
    tag: String,
) -> Result<()>
    where
        S: PingSwarm,
{
    info!(
        "Signal and wait for all peers to signal being done with \"{}\".",
        tag
    );

    futures::future::select(
        swarm.loop_on_next(),
        client
            .signal_and_wait(tag, client.run_parameters().test_instance_count)
            .boxed_local(),
    )
        .await;

    Ok(())
}