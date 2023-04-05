use tokio::net::TcpStream;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use ursa_pod::{client::UfdpClient, connection::UrsaCodecError};

const SERVER_ADDRESS: &str = "127.0.0.1:6969";
const PUB_KEY: [u8; 48] = [2u8; 48];
const CID: [u8; 32] = [1u8; 32];

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), UrsaCodecError> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let mut handles = vec![];

    for _ in 0..1 {
        #[cfg(not(feature = "bench-hyper"))]
        handles.push(tokio::spawn(download_ufdp()));

        #[cfg(feature = "bench-hyper")]
        handles.push(tokio::spawn(hyper::download()));
    }

    futures::future::join_all(handles).await;

    Ok(())
}

async fn download_ufdp() -> Result<(), UrsaCodecError> {
    let time = std::time::Instant::now();
    let stream = TcpStream::connect(SERVER_ADDRESS).await?;
    let mut client = UfdpClient::new(stream, PUB_KEY, None).await?;
    let size = client.request(CID).await?;

    let took = time.elapsed().as_millis();
    info!("received {} bytes in {took}ms", size);
    Ok(())
}

#[cfg(feature = "bench-hyper")]
mod hyper {
    use crate::SERVER_ADDRESS;
    use bytes::Bytes;
    use http_body_util::{BodyExt, Empty};
    use hyper::Request;
    use tokio::net::TcpStream;
    use tracing::info;

    pub async fn download() {
        let time = std::time::Instant::now();
        // Open a TCP connection to the remote host
        let stream = TcpStream::connect(SERVER_ADDRESS).await.unwrap();
        // Perform a TCP handshake
        let (mut sender, conn) = hyper::client::conn::http1::handshake(stream).await.unwrap();
        // Spawn a task to poll the connection, driving the HTTP state
        tokio::task::spawn(async move {
            if let Err(err) = conn.await {
                println!("Connection failed: {:?}", err);
            }
        });
        // Create an HTTP request with an empty body and a HOST header
        let req = Request::builder()
            .uri(SERVER_ADDRESS)
            .header(hyper::header::HOST, "127.0.0.1")
            .body(Empty::<Bytes>::new())
            .unwrap();

        // Send it
        let mut res = sender.send_request(req).await.unwrap();
        let mut len = 0;
        // Stream the body, dropping each chunk immediately
        while let Some(frame) = res.frame().await {
            match frame {
                Ok(bytes) => {
                    if let Some(bytes) = bytes.data_ref() {
                        len += bytes.len();
                    }
                }
                Err(e) => panic!("{e:?}"),
            }
        }

        let took = time.elapsed().as_millis();
        info!("received {} bytes in {took}ms", len);
    }
}
