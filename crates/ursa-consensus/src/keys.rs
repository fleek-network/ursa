// Copyright 2022-2023 Fleek Network
// SPDX-License-Identifier: Apache-2.0, MIT

use anyhow::Context;
use fastcrypto::traits::{AllowedRng, EncodeDecodeBase64, KeyPair};
use tracing::info;

pub trait Generate {
    fn generate_random<R: AllowedRng>(rng: &mut R) -> Self;
}

pub trait LoadOrCreate: Generate + Sized {
    fn load_or_create<P: AsRef<std::path::Path>, R: AllowedRng>(
        rng: &mut R,
        path: P,
    ) -> anyhow::Result<Self>;
}

// implement the generate from random logic for KeyPair objects.
impl<T: KeyPair> Generate for T {
    #[inline]
    fn generate_random<R: AllowedRng>(rng: &mut R) -> Self {
        Self::generate(rng)
    }
}

impl<T: EncodeDecodeBase64 + Generate> LoadOrCreate for T {
    fn load_or_create<P: AsRef<std::path::Path>, R: AllowedRng>(
        rng: &mut R,
        path: P,
    ) -> anyhow::Result<Self> {
        let path = path.as_ref();

        if path.exists() {
            info!("Reading '{:?}' from file system.", path);

            let content = std::fs::read_to_string(path)
                .with_context(|| format!("Could not read the file: '{:?}'", path))?;

            Self::decode_base64(content.as_str())
                .map_err(|e| anyhow::anyhow!("Could not decode the file as base64: {:?}", e))
        } else {
            info!("Creating '{:?}' because it doesn't exists.", path);

            let value = Self::generate_random(rng);

            let parent = path
                .parent()
                .context("Could not resolve the parent directory.")?;

            std::fs::create_dir_all(parent)
                .with_context(|| format!("Could not create the directory: '{:?}'", parent))?;

            std::fs::write(path, value.encode_base64())
                .with_context(|| format!("Could not write to '{:?}'.", path))?;

            Ok(value)
        }
    }
}
