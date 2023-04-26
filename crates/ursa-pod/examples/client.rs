use tokio::net::TcpStream;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use ursa_pod::{blake3::Hash, client::UfdpClient, connection::UrsaCodecError};

const SERVER_ADDRESS: &str = "127.0.0.1:6969";
const PUB_KEY: [u8; 48] = [2u8; 48];
const HASH: &str = "28960eef7d587ab6d1627b7efe30c7a07ce2dce4871d339fdfb607cb0776e064";

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), UrsaCodecError> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let hash = Hash::from_hex(HASH).unwrap();

    info!("requesting hash {hash}");
    let time = std::time::Instant::now();
    let stream = TcpStream::connect(SERVER_ADDRESS).await.unwrap();
    let mut client = UfdpClient::new(stream, PUB_KEY, None).await.unwrap();
    let size = client.request(hash).await.unwrap();

    let took = time.elapsed().as_millis();
    info!("received {} bytes in {took}ms", size);

    Ok(())
}
