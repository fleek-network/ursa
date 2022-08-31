mod ursa;

use std::sync::Arc;

use db::{rocks::RocksDb, rocks_config::RocksDbConfig};
use dotenv::dotenv;
use libp2p::identity::Keypair;
use network::UrsaService;
use rpc_server::{api::NodeNetworkInterface, config::ServerConfig, server::Server};
use service_metrics::{config::MetricsServiceConfig, metrics};
use store::Store;
use structopt::StructOpt;
use tracing::{error, info};
use ursa::{cli_error_and_die, wait_until_ctrlc, Cli, Subcommand};

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::fmt::init();

    // Capture Cli inputs
    let Cli { opts, cmd } = Cli::from_args();

    match opts.to_config() {
        Ok(config) => {
            if let Some(command) = cmd {
                match command {
                    Subcommand::Rpc(cmd) => {
                        cmd.run().await;
                    }
                }
            } else {
                info!("Starting up with config {:?}", config);

                // todo(botch): check local for a stored keypair
                let keypair = Keypair::generate_ed25519();
                let config_db_path = config.database_path.as_ref().unwrap().as_path().to_str();
                let db_path = opts
                    .database_path
                    .unwrap_or(config_db_path.unwrap().to_string());
                info!("Using {} as database path", db_path);

                let db = RocksDb::open(db_path, &RocksDbConfig::default())
                    .expect("Opening RocksDB must succeed");
                let db = Arc::new(db);
                let store = Arc::new(Store::new(Arc::clone(&db)));
                let service = UrsaService::new(keypair, &config, Arc::clone(&store));
                let rpc_sender = service.command_sender().clone();

                // Start libp2p service
                let service_task = tokio::spawn(async {
                    if let Err(err) = service.start().await {
                        error!("[service_task] - {:?}", err);
                    }
                });

                let ServerConfig { addr, port } = ServerConfig::default();
                let port = opts.rpc_port.unwrap_or(port);
                let server_config = ServerConfig::new(port, addr);

                let interface = Arc::new(NodeNetworkInterface {
                    store,
                    network_send: rpc_sender,
                });
                let server = Server::new(&server_config, interface);

                let metrics_config = MetricsServiceConfig::default();

                // Start multiplex server service(rpc and http)
                let rpc_task = tokio::spawn(async move {
                    if let Err(err) = server.start(server_config).await {
                        error!("[server] - {:?}", err);
                    }
                });

                // Start metrics service
                let metrics_task = tokio::spawn(async move {
                    if let Err(err) = metrics::start(&metrics_config).await {
                        error!("[metrics_task] - {:?}", err);
                    }
                });

                wait_until_ctrlc();

                // Gracefully shutdown node & rpc
                rpc_task.abort();
                service_task.abort();
                metrics_task.abort();
            }
        }
        Err(e) => {
            cli_error_and_die(&format!("Error parsing config. Error was: {}", e), 1);
        }
    };
}
