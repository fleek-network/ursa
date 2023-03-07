use bytes::Bytes;
use ethers::core::types::TransactionRequest;
use jsonrpc_v2::{Data, Error, Params};
use narwhal_types::TransactionProto;
use std::sync::Arc;
use ursa_application::types::Query;
use ursa_consensus::AbciQueryQuery;

use crate::api::NetworkInterface;
use crate::routes::network::Result;

pub async fn eth_send_raw_transaction<I>(
    data: Data<Arc<I>>,
    Params(params): Params<TransactionRequest>,
) -> Result<()>
where
    I: NetworkInterface,
{
    //Todo(dalton) to support actually ethereum RPC we should return the transaction hash here. Keccack256(txn)
    let request = TransactionProto {
        transaction: Bytes::from(serde_json::to_vec(&params).unwrap()),
    };
    if let Err(e) = data.0.submit_narwhal_txn(request).await {
        Err(Error::internal(e))
    } else {
        Ok(())
    }
}

pub async fn eth_call<I>(
    data: Data<Arc<I>>,
    Params(params): Params<TransactionRequest>,
) -> Result<Vec<u8>>
where
    I: NetworkInterface,
{
    let query = Query::EthCall(params);
    let query = serde_json::to_string(&query).unwrap();

    let abci_query = AbciQueryQuery {
        data: query,
        path: "".to_string(),
        height: None,
        prove: None,
    };

    match data.0.query_abci(abci_query).await {
        Err(e) => Err(Error::internal(e)),
        Ok(res) => Ok(res.value),
    }
}
