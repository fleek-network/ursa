use std::{collections::HashMap, io::Write, sync::Arc};

use tokio::net::TcpListener;
use ursa_pod::{
    blake3::Hash,
    instrument,
    server::{Backend, UfdpServer},
    types::{BlsSignature, Secp256k1PublicKey},
};

const ADDRESS: &str = "0.0.0.0:6969";
// content for a block
const BLOCK: &[u8] = &[0; 256 * 1024];
const SIZES: [usize; 17] = [
    // MB
    1024 * 1024,
    2 * 1024 * 1024,
    4 * 1024 * 1024,
    8 * 1024 * 1024,
    16 * 1024 * 1024,
    32 * 1024 * 1024,
    64 * 1024 * 1024,
    128 * 1024 * 1024,
    256 * 1024 * 1024,
    512 * 1024 * 1024,
    // GB
    1024 * 1024 * 1024,
    2 * 1024 * 1024 * 1024,
    4 * 1024 * 1024 * 1024,
    8 * 1024 * 1024 * 1024,
    16 * 1024 * 1024 * 1024,
    32 * 1024 * 1024 * 1024,
    64 * 1024 * 1024 * 1024,
];

#[derive(Clone, Debug)]
struct BenchmarkBackend {
    sizes: HashMap<Hash, usize>,
    trees: HashMap<Hash, Vec<[u8; 32]>>,
}

impl BenchmarkBackend {
    fn new() -> Self {
        let mut sizes = HashMap::new();
        let mut trees = HashMap::new();
        let mut display = String::new();

        let mut builder = blake3::ursa::HashTreeBuilder::new();
        let mut b = 0;
        for (i, size) in SIZES.into_iter().enumerate() {
            eprint!("\rBuilding blake3 trees ... ({}/{})", i + 1, SIZES.len());
            std::io::stdout().flush().unwrap();

            while let Some(block) = Self::raw_block(size, b) {
                builder.update(block);
                b += 1;
            }
            // clone the builder at this state, and finalize it. The original will continue to be
            // used for the next iterations
            let output = builder.clone().finalize();
            sizes.insert(output.hash, size);
            trees.insert(output.hash, output.tree);

            display.push_str(&format!("{}: {size}\n", output.hash));
        }

        eprintln!("\rBuilding blake3 trees ... done\n{display}");

        Self { sizes, trees }
    }

    fn raw_block(len: usize, block: usize) -> Option<&'static [u8]> {
        let s = block * BLOCK.len();
        if s < len {
            let e = len.min(s + BLOCK.len());
            Some(&BLOCK[..e - s])
        } else {
            None
        }
    }
}

impl Backend for BenchmarkBackend {
    fn raw_block(&self, hash: &Hash, block: u64) -> Option<&[u8]> {
        self.sizes
            .get(hash)
            .and_then(|len| Self::raw_block(*len, block as usize))
    }

    fn get_tree(&self, cid: &Hash) -> Option<Vec<[u8; 32]>> {
        self.trees.get(cid).cloned()
    }

    fn decryption_key(&self, _request_id: u64) -> (ursa_pod::types::Secp256k1AffinePoint, u64) {
        ([3u8; 33], 0)
    }

    fn get_balance(&self, _pubkey: Secp256k1PublicKey) -> u128 {
        9001
    }

    fn save_batch(&self, _batch: BlsSignature) -> Result<(), String> {
        Ok(())
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let backend = BenchmarkBackend::new();
    let server = Arc::new(UfdpServer::new(backend.into()));

    let listener = TcpListener::bind(ADDRESS).await.unwrap();
    eprintln!("Listening on {ADDRESS}");

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        let server = server.clone();
        tokio::spawn(async move {
            instrument!(server.serve(stream).await.unwrap(), "tag=session");
        });
    }
}
