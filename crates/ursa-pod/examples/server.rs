use std::fmt::Display;

use bytes::BytesMut;
use futures::SinkExt;
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;
use tracing::{debug, error, info, Level};
use tracing_subscriber::FmtSubscriber;
use ursa_pod::codec::{Blake3CID, UrsaCodec, UrsaCodecError, UrsaFrame};

trait Backend: Copy + Send + Sync + 'static {
    fn raw_content(&self, cid: Blake3CID) -> BytesMut;
}

#[derive(Clone, Copy)]
struct DummyBackend {}

impl Backend for DummyBackend {
    fn raw_content(&self, _cid: Blake3CID) -> BytesMut {
        BytesMut::from("hello world!")
    }
}

struct UfdpServer<B: Backend> {
    backend: B,
    listener: TcpListener,
}

impl<B> UfdpServer<B>
where
    B: Backend,
{
    pub async fn new<A: Display + ToSocketAddrs>(
        addr: A,
        backend: B,
    ) -> Result<Self, UrsaCodecError> {
        let listener = TcpListener::bind(&addr).await?;
        info!("Listening on {addr}");

        Ok(Self { listener, backend })
    }

    pub async fn start(self) -> Result<(), UrsaCodecError> {
        loop {
            let (stream, _) = self.listener.accept().await?;
            tokio::spawn(async move {
                let mut transport = Framed::new(stream, UrsaCodec::default());

                match transport.next().await.expect("handshake request") {
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
                    _ => return,
                }

                while let Some(request) = transport.next().await {
                    debug!("Received frame: {request:?}");
                    match request {
                        Ok(UrsaFrame::ContentRequest { hash }) => {
                            info!("Content request received, sending response");
                            let content = self.backend.raw_content(hash);
                            transport
                                .send(UrsaFrame::ContentResponse {
                                    compression: 0,
                                    proof_len: 0,
                                    content_len: content.len() as u64,
                                    signature: [1u8; 64],
                                })
                                .await
                                .expect("content response");

                            transport
                                .send(UrsaFrame::Buffer(content))
                                .await
                                .expect("content data")
                        }
                        Ok(_) => unimplemented!(),
                        Err(e) => {
                            error!("{e:?}");
                            break;
                        }
                    }
                }

                debug!("Connection Closed");
            });
        }
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
