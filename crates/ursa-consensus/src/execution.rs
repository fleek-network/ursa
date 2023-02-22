// Copyright 2022-2023 Fleek Network
// SPDX-License-Identifier: Apache-2.0, MIT

use async_trait::async_trait;
use narwhal_executor::ExecutionState;
use narwhal_types::{Batch, ConsensusOutput};
use tokio::sync::mpsc::Sender;
use tracing::error;

type Epoch = u64;

pub struct Execution {
    /// current epoch store implementation
    pub epoch: Epoch,
    /// managing certificates generated by narwhal
    pub transactions: Sender<Vec<Batch>>,
}

impl Execution {
    pub fn new(epoch: Epoch, transactions: Sender<Vec<Batch>>) -> Self {
        Self {
            epoch,
            transactions,
        }
    }
}

#[async_trait]
impl ExecutionState for Execution {
    async fn handle_consensus_output(&self, consensus_output: ConsensusOutput) {
        for (_, batches) in consensus_output.batches {
            if let Err(err) = self.transactions.send(batches).await {
                error!("Failed to send txn: {}", err);
            }
        }
    }

    async fn last_executed_sub_dag_index(&self) -> u64 {
        0
    }
}
