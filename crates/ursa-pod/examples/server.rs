use tokio::net::TcpListener;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use ursa_pod::{
    codec::{consts::MAX_BLOCK_SIZE, UrsaCodecError},
    server::{Backend, UfdpServer},
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

    #[cfg(feature = "bench-hyper")]
    hyper::serve(addr).await;

    #[cfg(not(feature = "bench-hyper"))]
    run_ufdp(addr).await;

    info!("Listening on port 6969");

    Ok(())
}

async fn run_ufdp(addr: &str) {
    let listener = TcpListener::bind(addr).await.unwrap();
    let server = UfdpServer::new(DummyBackend {}).unwrap();
    loop {
        let (stream, _) = listener.accept().await.unwrap();
        server.handle(stream).unwrap();
    }
}

#[cfg(feature = "bench-hyper")]
mod hyper {
    use std::io::Error;

    use bytes::Bytes;
    use http_body_util::Full;
    use hyper::{server::conn::http1, service::service_fn, Response};
    use tokio::net::TcpListener;

    pub async fn serve(addr: &str) {
        let listener = TcpListener::bind(addr).await.unwrap();

        loop {
            let (stream, _) = listener.accept().await.unwrap();

            let service = service_fn(move |_req| async move {
                Ok::<_, Error>(Response::new(Full::new(Bytes::from(
                    &[0u8; 100 * 1024 * 1024] as &[u8],
                ))))
            });

            if let Err(err) = http1::Builder::new()
                .serve_connection(stream, service)
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        }
    }
}
