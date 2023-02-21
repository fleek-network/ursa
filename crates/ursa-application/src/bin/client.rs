use anyhow::Result;
use bytes::Bytes;
use ethers::abi::parse_abi;
use ethers::contract::BaseContract;
use ethers::prelude::{Address, TransactionRequest, U256};
use revm::primitives::{ExecutionResult, Output};
use std::env;
use ursa_application::types::{Query, QueryResponse};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    let contract_addr = "0xAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        .parse::<Address>()
        .unwrap();

    // generate abi for the calldata from the human readable interface
    let abi = BaseContract::from(parse_abi(&[
        "function helloWorld() public pure returns (string)",
        "function get() public view returns (uint256)",
        "function add() external returns (uint256)",
    ])?);

    let token_abi = BaseContract::from(parse_abi(&[
        "function totalSupply() external view returns (uint256)",
        "function decimals() public returns (uint8)",
        "function name() external view returns (string)",
        "function symbol() external view returns (string)",
        "function controller() external view returns (address)",
    ])?);

    match args[1].as_str() {
        "get" => {
            let encoded = abi.encode("get", ())?;
            let transaction_request = TransactionRequest::new()
                .to(contract_addr)
                .data(Bytes::from(hex::decode(hex::encode(&encoded))?));
            let query = Query::EthCall(transaction_request);

            let query = serde_json::to_string(&query)?;
            let client = reqwest::Client::new();
            let res = client
                .get(format!("{}/abci_query", "http://192.168.1.237:3005"))
                .query(&[("data", query), ("path", "".to_string())])
                .send()
                .await?;

            let val = res.bytes().await?;

            let val: QueryResponse = serde_json::from_slice(&val)?;

            let val = match val {
                QueryResponse::Tx(res) => match res {
                    ExecutionResult::Success { output, .. } => match output {
                        Output::Call(bytes) => bytes,
                        _ => panic!("Output wrong"),
                    },
                    _ => panic!("Txn was not succesful"),
                },
                _ => panic!("Error"),
            };

            let readable_output: u64 = match abi.decode_output("get", val) {
                Ok(output) => output,
                Err(e) => panic!("{:?}", e),
            };
            println!("Counter is currently at: {}", readable_output);
        }
        "inc" => {
            let encoded = abi.encode("add", ())?;
            let transaction_request = TransactionRequest::new()
                .to(contract_addr)
                .data(Bytes::from(hex::decode(hex::encode(&encoded))?))
                .gas(21000000)
                .from(
                    "0xDAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
                        .parse::<Address>()
                        .unwrap(),
                );

            let tx = serde_json::to_string(&transaction_request)?;

            let client = reqwest::Client::new();
            client
                .get(format!("{}/broadcast_tx", "http://127.0.0.1:8003"))
                .query(&[("tx", tx)])
                .send()
                .await?;

            println!("transaction sent to consensus");
        }
        _ => {
            println!("{}", args[1].as_str());
            let encoded = token_abi.encode(args[1].as_str(), ()).unwrap();

            let transaction_request = TransactionRequest::new()
                .to(contract_addr)
                .data(Bytes::from(hex::decode(hex::encode(&encoded))?));
            let query = Query::EthCall(transaction_request);

            let query = serde_json::to_string(&query)?;
            let client = reqwest::Client::new();
            let res = client
                .get(format!("{}/abci_query", "http://127.0.0.1:8003"))
                .query(&[("data", query), ("path", "".to_string())])
                .send()
                .await?;

            let val = res.bytes().await?;

            let val: QueryResponse = serde_json::from_slice(&val)?;
            println!("{:?}", val);

            let val = match val {
                QueryResponse::Tx(res) => {
                    println!("{:?}", res);
                    match res {
                        ExecutionResult::Success { output, .. } => match output {
                            Output::Call(bytes) => bytes,
                            _ => panic!("Output wrong"),
                        },
                        _ => panic!("Txn was not succesful"),
                    }
                }
                _ => panic!("Error"),
            };

            let readable_output: String = match token_abi.decode_output(args[1].as_str(), val) {
                Ok(output) => output,
                Err(e) => panic!("{:?}", e),
            };
            println!("contract returned: {:?}", readable_output);
            println!("U256: {:?}", U256::from_dec_str("1000000000"))
        }
    }
    Ok(())
}
