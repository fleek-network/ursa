// Copyright 2022-2023 Fleek Network
// SPDX-License-Identifier: Apache-2.0, MIT

use narwhal_worker::TransactionValidator;
use std::io::Error;

#[derive(Clone)]
pub struct Validator {}

impl Validator {
    pub fn new() -> Self {
        Self {}
    }
}

impl TransactionValidator for Validator {
    type Error = Error;

    fn validate(&self, _t: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }

    fn validate_batch(&self, _b: &narwhal_types::Batch) -> Result<(), Self::Error> {
        Ok(())
    }
}
