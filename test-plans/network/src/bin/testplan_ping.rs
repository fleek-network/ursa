use anyhow::{anyhow, Result};
use env_logger::Env;
use futures::future::ready;
use futures::{FutureExt, StreamExt};
use libp2p::swarm::SwarmEvent;
use log::info;
use rand::Rng;
use std::collections::HashSet;
use std::{borrow::Cow, time::Duration};
use testground::network_conf::{
    FilterAction, LinkShape, NetworkConfiguration, RoutingPolicyType, DEFAULT_DATA_NETWORK,
};
use testplan::TestSwarm;

#[tokio::main]
async fn main() -> Result<()> {
    let mut rt = TestSwarm::new().await?;

    info!("Running ping test: {}", rt.local_peer_id());

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let local_addr = rt.local_addr.to_string();

    let test_instance_count = rt.client.run_parameters().test_instance_count as usize;
    let mut address_stream = rt
        .client
        .subscribe("peers", test_instance_count)
        .await
        .take(test_instance_count)
        .map(|a| {
            let value = a.unwrap();
            value["Addrs"][0].as_str().unwrap().to_string()
        })
        // Note: we sidestep simultaneous connect issues by ONLY connecting to peers
        // who published their addresses before us (this is enough to dedup and avoid
        // two peers dialling each other at the same time).
        //
        // We can do this because sync service pubsub is ordered.
        .take_while(|a| ready(a != &local_addr));

    let payload = serde_json::json!({
        "ID": rt.local_peer_id(),
        "Addrs": [
            local_addr
        ],
    });

    rt.client.publish("peers", Cow::Owned(payload)).await?;

    let mut to_connect = 0;
    while let Some(addr) = address_stream.next().await {
        to_connect += 1;
        info!("Dialing node: {}", addr);
        rt.dial(&addr)?;
    }

    // Otherwise the testground background task gets blocked sending
    // subscription upgrades to the backpressured channel.
    drop(address_stream);

    info!("Wait to connect to each peer.");
    rt.await_connections(to_connect).await;
    rt.drive_until_signal("connected").await?;

    ping(&mut rt, "initial").await?;

    let iterations: usize = rt
        .client
        .run_parameters()
        .test_instance_params
        .get("iterations")
        .unwrap()
        .parse()
        .unwrap();
    let max_latency_ms: u64 = rt
        .client
        .run_parameters()
        .test_instance_params
        .get("max_latency_ms")
        .unwrap()
        .parse()
        .unwrap();

    for i in 1..iterations + 1 {
        rt.client
            .record_message(format!("⚡️  ITERATION ROUND {}", i));

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
            callback_target: Some(rt.client.run_parameters().test_instance_count),
            routing_policy: RoutingPolicyType::AllowAll,
        };

        rt.client.configure_network(network_conf).await.unwrap();

        ping(&mut rt, format!("done-{}", i)).await?;
    }

    rt.client
        .record_success()
        .await
        .map_err(|e| anyhow::anyhow!(e))
}

async fn ping<S: ToString>(rt: &mut TestSwarm, tag: S) -> Result<()> {
    rt.await_pings((rt.client.run_parameters().test_instance_count - 1) as usize)
        .await;
    rt.drive_until_signal(tag).await?;
    Ok(())
}
