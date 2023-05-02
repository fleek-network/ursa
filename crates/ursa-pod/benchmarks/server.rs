use std::{
    cmp::min,
    collections::{BTreeMap, HashMap},
    io::Write,
};

use arrayref::array_ref;
use tokio::net::TcpListener;
use ursa_pod::{
    blake3::Hash,
    instrument,
    server::{Backend, UfdpHandler},
    types::{BlsSignature, Secp256k1PublicKey},
};

const ADDRESS: &str = "0.0.0.0:6969";
// content for a block
const BLOCK: &[u8] = &[0; 256 * 1024];
const SIZES: [usize; 17] = [
    // MB
    1 * 1024 * 1024,
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
    1 * 1024 * 1024 * 1024,
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

        for (i, size) in SIZES.iter().enumerate() {
            eprint!("\rBuilding blake3 trees ... ({i}/{})", SIZES.len() - 1);
            std::io::stdout().flush().unwrap();
            let mut tree_builder = blake3::ursa::HashTreeBuilder::new();
            let mut i = 0;
            while let Some(block) = Self::raw_block(*size, i) {
                tree_builder.update(block);
                i += 1
            }
            let output = tree_builder.finalize();
            sizes.insert(output.hash, *size);
            trees.insert(output.hash, output.tree);
        }
        eprintln!("\rBuilding blake3 trees ... (done)   ");
        let mut arr = sizes.iter().collect::<Vec<(&Hash, &usize)>>();
        arr.sort_by(|(_, a), (_, b)| a.cmp(b));
        eprintln!("{arr:?}");

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

    let listener = TcpListener::bind(ADDRESS).await.unwrap();
    eprintln!("Listening on {ADDRESS}");

    let mut session_id = 0;
    loop {
        let (stream, _) = listener.accept().await.unwrap();

        let backend = backend.clone();
        tokio::spawn(async move {
            let handler = UfdpHandler::new(stream, backend, session_id);
            instrument!(
                handler.serve().await.unwrap(),
                "sid={session_id},tag=session"
            );
        });
        session_id += 1;
    }
}
