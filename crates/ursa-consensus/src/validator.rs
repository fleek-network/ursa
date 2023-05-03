// Copyright 2022-2023 Fleek Network
// SPDX-License-Identifier: Apache-2.0, MIT

use async_trait::async_trait;
use narwhal_types::Batch;
use narwhal_worker::TransactionValidator;
use std::io::Error;

#[derive(Clone)]
pub struct Validator {}

impl Validator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TransactionValidator for Validator {
    type Error = Error;

    fn validate(&self, _t: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn validate_batch(&self, _b: &Batch) -> Result<(), Self::Error> {
        Ok(())
    }
}
