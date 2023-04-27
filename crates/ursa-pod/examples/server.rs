use tokio::net::TcpListener;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;
use ursa_pod::{
    connection::UrsaCodecError,
    server::{Backend, UfdpHandler},
    types::{Blake3Cid, BlsSignature, Secp256k1PublicKey},
};

const CONTENT: &[u8] = &[0; 256 * 1024];

#[derive(Clone, Copy)]
struct DummyBackend {}

impl Backend for DummyBackend {
    fn raw_block(&self, _cid: &Blake3Cid, block: u64) -> Option<&[u8]> {
        // serve 10GB
        if block < 4 * 1024 * 10 {
            Some(CONTENT)
        } else {
            None
        }
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
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), UrsaCodecError> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let addr = "127.0.0.1:6969";
    info!("Listening on port 6969");

    let listener = TcpListener::bind(addr).await.unwrap();
    loop {
        let (mut stream, _) = listener.accept().await.unwrap();
        let (read, write) = stream.split();
        info!("accepted conn");
        let handler = UfdpHandler::new(read, write, DummyBackend {}, 0);

        if let Err(e) = handler.serve().await {
            error!("UFDP Session failed: {e:?}");
        }
    }
}
