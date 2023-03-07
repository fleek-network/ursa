use ethers::{
    abi::AbiParser,
    core::types::{Address, TransactionRequest},
};
use structopt::StructOpt;
use tracing::{error, info};
use ursa_rpc_service::{
    api::{NetworkGetFileParams, NetworkPutFileParams},
    client::functions::{eth_call, eth_send_transaction, get_file, put_file},
};
use ursa_utils::transactions::encode_params;

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

    //Example 'ursa rpc txn 0xAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAB "myFunction(string,uint256):(uint256) param1 1"
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
                let to = match address.parse::<Address>() {
                    Ok(addr) => addr,
                    Err(_) => {
                        error!("Not a valid address");
                        return;
                    }
                };

                let function_abi = match AbiParser::default().parse_function(function) {
                    Ok(func) => func,
                    Err(e) => {
                        error!("Unable to parse function: {e:?}");
                        return;
                    }
                };

                let data = match encode_params(&function_abi, args) {
                    Ok(params) => params,
                    Err(e) => {
                        error!("unable to encode params: {e:?}");
                        return;
                    }
                };

                let txn = TransactionRequest::new().to(to).data(data);

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
                let to = match address.parse::<Address>() {
                    Ok(addr) => addr,
                    Err(_) => {
                        error!("Not a valid address");
                        return;
                    }
                };

                let function_abi = match AbiParser::default().parse_function(function) {
                    Ok(func) => func,
                    Err(e) => {
                        error!("Unable to parse function: {e:?}");
                        return;
                    }
                };

                let data = match encode_params(&function_abi, args) {
                    Ok(params) => params,
                    Err(e) => {
                        error!("unable to encode params: {e:?}");
                        return;
                    }
                };

                let txn = TransactionRequest::new().to(to).data(data);

                match eth_call(txn).await {
                    Ok(result) => match function_abi.decode_output(&result) {
                        Ok(tokens) => info!("Returned data is: {tokens:?}"),
                        Err(_) => error!("Error decoding output: {result:?}"),
                    },
                    Err(_e) => {
                        error!("There was an error while calling the rpc server. Please Check Server Logs")
                    }
                };
            }
        }
    }
}
