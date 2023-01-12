mod bootstrap;
mod cache;
mod node;

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

    if let Err(e) = bootstrap::start_node(&mut client).await {
        client.record_failure(e).await.expect("Success");
    } else {
        client.record_success().await.expect("Success");
    }
}
