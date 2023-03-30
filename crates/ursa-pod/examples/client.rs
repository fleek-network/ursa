use bytes::BytesMut;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio_util::io::StreamReader;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use ursa_pod::{client::UfdpClient, codec::UrsaCodecError};

const CID: [u8; 32] = [1u8; 32];

#[tokio::main]
async fn main() -> Result<(), UrsaCodecError> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let stream = TcpStream::connect("127.0.0.1:8080").await?;
    let mut client = UfdpClient::new(stream).await?;

    let res = client.request(CID).await?;
    let mut reader = StreamReader::new(res);
    let mut bytes = BytesMut::with_capacity(16 * 1024);
    reader.read_buf(&mut bytes).await?;

    println!("{}", String::from_utf8_lossy(&bytes));

    Ok(())
}
