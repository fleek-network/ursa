use std::error::Error;

use futures::SinkExt;
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use ursa_pod::codec::{UrsaCodec, UrsaFrame};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    UfdpClient::new("127.0.0.1:8080").await?;
    Ok(())
}

pub struct UfdpClient {
    _transport: Framed<TcpStream, UrsaCodec>,
}

impl UfdpClient {
    pub async fn new<T: ToSocketAddrs>(dest: T) -> Result<Self, Box<dyn Error>> {
        let codec = UrsaCodec::default();
        let stream = TcpStream::connect(dest).await?;
        let mut transport = Framed::new(stream, codec);

        // send handshake
        transport
            .send(UrsaFrame::HandshakeRequest {
                version: 0,
                supported_compression_bitmap: 0,
                lane: 0xFF,
                pubkey: [1; 48],
            })
            .await
            .expect("handshake request");

        // receive handshake
        if let Ok(frame) = transport.next().await.expect("handshake response") {
            match frame {
                UrsaFrame::HandshakeResponse { .. } => {
                    info!("received handshake response from server: {frame:?}");
                }
                _ => panic!("unexpected frame"),
            }
        }

        Ok(Self {
            _transport: transport,
        })
    }
}
