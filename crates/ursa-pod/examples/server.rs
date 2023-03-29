use bytes::BytesMut;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use ursa_pod::{
    codec::UrsaCodecError,
    server::{Backend, UfdpServer},
    types::{BLSSignature, Blake3CID, Secp256k1PublicKey},
};

#[derive(Clone, Copy)]
struct DummyBackend {}

impl Backend for DummyBackend {
    fn raw_content(&self, _cid: Blake3CID) -> BytesMut {
        BytesMut::from("hello world!")
    }

    fn get_balance(&self, _pubkey: Secp256k1PublicKey) -> u128 {
        10
    }

    fn save_tx(
        &self,
        _pubkey: Secp256k1PublicKey,
        _acknowledgment: BLSSignature,
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

    let server = UfdpServer::new("0.0.0.0:8080", DummyBackend {}).await?;
    server.start().await
}
