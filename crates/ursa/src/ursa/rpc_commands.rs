use structopt::StructOpt;
use tracing::{error, info};
use ursa_rpc_client::functions::put_file;
use ursa_rpc_server::api::NetworkPutFileParams;

#[derive(Debug, StructOpt)]
pub enum RpcCommands {
    #[structopt(about = "put the file on the node")]
    Put {
        #[structopt(about = "The path to the file")]
        path: String,
    },
}

impl RpcCommands {
    pub async fn run(&self) {
        match self {
            Self::Put { path } => {
                let params = NetworkPutFileParams {
                    path: path.to_string(),
                };
                match put_file(params).await {
                    Ok(v) => {
                        info!("Put car file done: {v:?}");
                    }
                    Err(_e) => {
                        error!("There was an error while calling the rpc server. Please Check Server Logs")
                    }
                };
            }
        }
    }
}
