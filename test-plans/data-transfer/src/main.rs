mod bootstrap;
mod node;
mod pull;

use env_logger::Env;
use testground::client::Client;

#[tokio::main]
async fn main() {
    let mut client = Client::new_and_init().await.unwrap();

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // The first instance that arrives will be the bootstrapper.
    let seq = client.global_seq();
    if seq == 1 {
        return bootstrap::start_bootstrap(client).await;
    }

    let num_nodes = client.run_parameters().test_instance_count - 1;

    let node = match bootstrap::start_node(&mut client).await {
        Ok(node) => node,
        Err(e) => {
            // All nodes wait here and signal to the bootstrap node that they are done.
            client.signal_and_wait("done", num_nodes).await.unwrap();
            client.record_failure(e).await.expect("Success");
            return;
        }
    };

    let result = pull::run_test(&mut client, node).await;

    // All nodes wait here and signal to the bootstrap node that they are done.
    client.signal_and_wait("done", num_nodes).await.unwrap();

    match result {
        Ok((test_name, duration)) => {
            client.record_message(format!("{test_name}: {duration:?}"));
            client.record_success().await.expect("Success")
        }
        Err(e) => client.record_failure(e).await.expect("Success"),
    }
}
