use bytes::BytesMut;
use tokio::net::TcpListener;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use ursa_pod::{
    codec::UrsaCodecError,
    server::{Backend, UfdpServer},
    types::{Blake3Cid, BlsSignature, Secp256k1PublicKey},
};

#[derive(Clone, Copy)]
struct DummyBackend {}

impl Backend for DummyBackend {
    fn raw_content(&self, _cid: Blake3Cid) -> BytesMut {
        BytesMut::from("hello world!")
    }

    fn get_balance(&self, _pubkey: Secp256k1PublicKey) -> u128 {
        10
    }

    fn save_tx(
        &self,
        _pubkey: Secp256k1PublicKey,
        _acknowledgment: BlsSignature,
    ) -> Result<(), String> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), UrsaCodecError> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    info!("Listening on port 8080");

    let mut server = UfdpServer::new(DummyBackend {})?;
    loop {
        let (stream, _) = listener.accept().await?;
        server.handle(stream)?;
    }
}
