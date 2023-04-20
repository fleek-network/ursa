pub mod epoch_manager;
pub mod node_registry;
pub mod reputation;

use anyhow::{Context, Result};
use bytes::Bytes;
use ethers::types::TransactionRequest;
use narwhal_types::{TransactionProto, TransactionsClient};
use tendermint_proto::abci::ResponseQuery;
use tokio::sync::{mpsc, oneshot};

use crate::transactions::{AbciQueryQuery, Query};

pub async fn query_application(
    tx_abci_queries: &mpsc::Sender<(oneshot::Sender<ResponseQuery>, AbciQueryQuery)>,
    req: TransactionRequest,
) -> Result<Vec<u8>> {
    let query = serde_json::to_string(&Query::EthCall(req))?;

    let abci_query = AbciQueryQuery {
        path: "".to_string(),
        data: query,
        height: None,
        prove: None,
    };

    // Construct one shot channel to recieve response.
    let (tx, rx) = oneshot::channel();

    // Send and wait for response.
    tx_abci_queries.send((tx, abci_query)).await?;
    let response = rx.await.with_context(|| "Failure querying abci")?;

    Ok(response.value)
}

pub async fn send_txn_to_application(
    mempool_address: String,
    req: TransactionRequest,
) -> Result<()> {
    let txn = serde_json::to_vec(&req)?;

    let request = TransactionProto {
        transaction: Bytes::from(txn),
    };

    let mut client = TransactionsClient::connect(mempool_address).await?;

    match client.submit_transaction(request).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e.into()),
    }
}
