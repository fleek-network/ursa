use std::{error::Error, fmt::Display};

use futures::SinkExt;
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;
use ursa_pod::codec::{UrsaCodec, UrsaFrame};

struct UfdpServer {
    listener: TcpListener,
}

impl UfdpServer {
    pub async fn new<A: Display + ToSocketAddrs>(addr: A) -> Result<Self, Box<dyn Error>> {
        let listener = TcpListener::bind(&addr).await?;
        info!("Listening on {addr}");

        Ok(Self { listener })
    }

    pub async fn start(self) -> Result<(), Box<dyn Error>> {
        loop {
            let (stream, _) = self.listener.accept().await?;
            tokio::spawn(async move {
                let mut transport = Framed::new(stream, UrsaCodec::default());

                while let Some(request) = transport.next().await {
                    info!("{request:?}");
                    match request {
                        Ok(UrsaFrame::HandshakeRequest { lane, .. }) => {
                            info!("Handshake received, sending response");
                            transport
                                .send(UrsaFrame::HandshakeResponse {
                                    pubkey: [2; 33],
                                    epoch_nonce: 1000,
                                    lane: if lane == 0xFF { 0 } else { lane },
                                    last: None,
                                })
                                .await
                                .expect("handshake response");
                        }
                        Ok(_) => {}
                        Err(e) => error!("{e:?}"),
                    }
                }
            });
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let server = UfdpServer::new("0.0.0.0:8080").await?;
    server.start().await
}
