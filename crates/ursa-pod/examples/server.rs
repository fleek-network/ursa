use blake3::Hash;
use tokio::net::TcpListener;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;
use ursa_pod::{
    connection::UrsaCodecError,
    server::{Backend, UfdpServer},
    types::{BlsSignature, Secp256k1PublicKey},
};

const CONTENT: &[u8] = &[0; 256 * 1024];

#[derive(Clone)]
struct DummyBackend {
    tree: Vec<[u8; 32]>,
}

fn raw_block(block: u64) -> Option<&'static [u8]> {
    // serve 10GB
    if block < 4 * 1024 * 10 {
        Some(CONTENT)
    } else {
        None
    }
}

impl Backend for DummyBackend {
    fn raw_block(&self, _cid: &Hash, block: u64) -> Option<&[u8]> {
        raw_block(block)
    }

    fn decryption_key(&self, _request_id: u64) -> (ursa_pod::types::Secp256k1AffinePoint, u64) {
        let key = [1; 33];
        let key_id = 0;
        (key, key_id)
    }

    fn get_balance(&self, _pubkey: Secp256k1PublicKey) -> u128 {
        9001
    }

    fn save_batch(&self, _batch: BlsSignature) -> Result<(), String> {
        Ok(())
    }

    fn get_tree(&self, _cid: &Hash) -> Option<Vec<[u8; 32]>> {
        Some(self.tree.clone())
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), UrsaCodecError> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // build a 10GB blake3 tree
    let mut tree_builder = blake3::ursa::HashTreeBuilder::new();
    (0..4 * 1024 * 10).for_each(|_i| tree_builder.update(CONTENT));
    let output = tree_builder.finalize();
    let hash = output.hash;

    let backend = DummyBackend { tree: output.tree };
    let addr = "127.0.0.1:6969";
    info!("Serving content on port 6969, for: {hash}");

    let listener = TcpListener::bind(addr).await.unwrap();
    let server = UfdpServer::new(backend.into());

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        info!("handling connection");

        if let Err(e) = server.serve(stream).await {
            error!("UFDP Session failed: {e:?}");
        }
    }
}
