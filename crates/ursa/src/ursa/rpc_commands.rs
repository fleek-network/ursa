use structopt::StructOpt;
use tracing::{error, info};
use ursa_rpc_service::{
    api::{NetworkGetFileParams, NetworkPutFileParams},
    client::functions::{get_file, put_file},
};

#[derive(Debug, StructOpt)]
pub enum RpcCommands {
    #[structopt(about = "put the file on the node")]
    Put {
        #[structopt(about = "The path to the file")]
        path: String,
    },
    #[structopt(
        about = "get the file from network for a given root cid and store it on given path"
    )]
    Get {
        #[structopt(about = "root cid to get the file")]
        cid: String,
        #[structopt(about = "The path to store the file")]
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
                    Ok(file) => {
                        info!("Put car file done: {:?}", file);
                    }
                    Err(_e) => {
                        error!("There was an error while calling the rpc server. Please Check Server Logs")
                    }
                };
            }
            Self::Get { cid, path } => {
                let params = NetworkGetFileParams {
                    path: path.to_string(),
                    cid: cid.to_string(),
                };
                match get_file(params).await {
                    Ok(_result) => {
                        info!("file stored at {path:?}");
                    }
                    Err(_e) => {
                        error!("There was an error while calling the rpc server. Please Check Server Logs")
                    }
                };
            }
        }
    }
}
