use bytes::{BufMut, BytesMut};
use tokio::net::TcpStream;
use tokio_stream::StreamExt;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use ursa_pod::{client::UfdpClient, codec::UrsaCodecError};

const SERVER_ADDRESS: &str = "127.0.0.1:8080";
const PUB_KEY: [u8; 48] = [2u8; 48];
const CID: [u8; 32] = [1u8; 32];

#[tokio::main]
async fn main() -> Result<(), UrsaCodecError> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let stream = TcpStream::connect(SERVER_ADDRESS).await?;
    let mut client = UfdpClient::new(stream, PUB_KEY, None).await?;

    let mut res = client.request(CID).await?;

    let mut buf = BytesMut::new();
    loop {
        match res.next().await {
            Some(Ok(bytes)) => buf.put_slice(&bytes),
            None => break,
            Some(Err(e)) => panic!("{e:?}"),
        }
    }

    info!("recieved {} bytes", buf.len());
    Ok(())
}
