// Copyright 2022-2023 Fleek Network
// SPDX-License-Identifier: Apache-2.0, MIT

pub mod config;
pub mod consensus;
pub mod execution;
pub mod keys;
pub mod narwhal;
pub mod validator;

mod abci_engine;
pub use abci_engine::Engine;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BroadcastTxQuery {
    tx: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AbciQueryQuery {
    pub path: String,
    pub data: String,
    pub height: Option<usize>,
    pub prove: Option<bool>,
}

pub type Epoch = u64;
