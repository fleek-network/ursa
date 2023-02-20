// Copyright 2022-2023 Fleek Network
// SPDX-License-Identifier: Apache-2.0, MIT

pub mod config;
pub mod execution;
pub mod keys;
pub mod service;
pub mod validator;

mod abci_engine;
pub use abci_engine::{Engine};

mod server;
pub use server::AbciApi;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BroadcastTxQuery {
    tx: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AbciQueryQuery {
    path: String,
    data: String,
    height: Option<usize>,
    prove: Option<bool>,
}