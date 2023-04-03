use structopt::StructOpt;
use tracing::{error, info};
use ursa_rpc_service::{
    api::{NetworkGetFileParams, NetworkPutFileParams},
    client::functions::{eth_call, eth_send_transaction, get_file, put_file},
};
use ursa_utils::transactions::build_transaction;

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

    // Example 'ursa rpc txn 0xAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAB "myFunction(string,uint256):(uint256) param1 1"
    #[structopt(about = "Send a txn to Narwhal")]
    Txn {
        #[structopt(about = "The address of the contract")]
        address: String,
        #[structopt(
            about = "The function as human readable abi. Example: \"functionName(string, uint256):(uint256)\""
        )]
        function: String,
        #[structopt(about = "The arguments as a string")]
        args: Vec<String>,
    },
    #[structopt(about = "Make a call to application layer, does not change state")]
    Call {
        #[structopt(about = "The address of the contract")]
        address: String,
        #[structopt(
            about = "The function as human readable abi. Example: \"functionName(string, uint256):(uint256)\""
        )]
        function: String,
        #[structopt(about = "The arguments as a string")]
        args: Vec<String>,
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
            Self::Txn {
                address,
                function,
                args,
            } => {
                let txn = match build_transaction(address, function, args) {
                    Ok((_, txn)) => txn,
                    Err(e) => {
                        error!("{e:?}");
                        return;
                    }
                };

                match eth_send_transaction(txn).await {
                    Ok(_result) => {
                        info!("transaction submitted");
                    }
                    Err(_e) => {
                        error!("There was an error while calling the rpc server. Please Check Server Logs")
                    }
                };
            }
            Self::Call {
                address,
                function,
                args,
            } => {
                let (function_abi, txn) = match build_transaction(address, function, args) {
                    Ok((func, txn)) => (func, txn),
                    Err(e) => {
                        error!("{e:?}");
                        return;
                    }
                };

                match eth_call(txn).await {
                    Ok(result) => match function_abi.decode_output(&result) {
                        Ok(tokens) => info!("Returned data is: {tokens:?}"),
                        Err(err) => error!("Error decoding output: {err:?}"),
                    },
                    Err(_e) => {
                        error!("There was an error while calling the rpc server. Please Check Server Logs")
                    }
                };
            }
        }
    }
}
