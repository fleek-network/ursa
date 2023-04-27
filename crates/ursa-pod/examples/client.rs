use tokio::net::TcpStream;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use ursa_pod::{client::UfdpClient, connection::UrsaCodecError, types::Blake3Cid};

const SERVER_ADDRESS: &str = "127.0.0.1:6969";
const PUB_KEY: [u8; 48] = [2u8; 48];
const CID: Blake3Cid = Blake3Cid([1u8; 32]);

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), UrsaCodecError> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let mut handles = vec![];

    for _ in 0..1 {
        #[cfg(not(feature = "bench-hyper"))]
        handles.push(tokio::spawn(async {
            let time = std::time::Instant::now();
            let mut stream = TcpStream::connect(SERVER_ADDRESS).await.unwrap();
            let (read, write) = stream.split();
            let mut client = UfdpClient::new(read, write, PUB_KEY, None).await.unwrap();
            let size = client.request(CID).await.unwrap();

            let took = time.elapsed().as_millis();
            info!("received {} bytes in {took}ms", size);
        }));
    }

    futures::future::join_all(handles).await;

    Ok(())
}
