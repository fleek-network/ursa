use bytes::BytesMut;
use futures::SinkExt;
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;
use tracing::{debug, info, Level};
use tracing_subscriber::FmtSubscriber;
use ursa_pod::codec::{Blake3CID, UrsaCodec, UrsaCodecError, UrsaFrame};

#[tokio::main]
async fn main() -> Result<(), UrsaCodecError> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let mut client = UfdpClient::new("127.0.0.1:8080").await?;

    let bytes = client.request([1; 32]).await?;
    println!("{}", String::from_utf8_lossy(&bytes));

    Ok(())
}

pub struct UfdpClient {
    transport: Framed<TcpStream, UrsaCodec>,
}

impl UfdpClient {
    pub async fn new<T: ToSocketAddrs>(dest: T) -> Result<Self, UrsaCodecError> {
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
                f => return Err(UrsaCodecError::UnexpectedFrame(f.tag().unwrap())),
            }
        }

        Ok(Self { transport })
    }

    pub async fn request(&mut self, hash: Blake3CID) -> Result<BytesMut, UrsaCodecError> {
        self.transport
            .send(UrsaFrame::ContentRequest { hash })
            .await
            .expect("content request");

        match self.transport.next().await.expect("content response")? {
            UrsaFrame::ContentResponse {
                content_len,
                proof_len,
                ..
            } => {
                info!("received content response");

                debug!("streaming proof ({proof_len})");
                if proof_len != 0 {
                    unimplemented!()
                }

                debug!("streaming content ({content_len})");
                if content_len != 0 {
                    self.transport
                        .codec_mut()
                        .read_buffer(content_len as usize, 16384);
                    match self.transport.next().await.expect("content buffer")? {
                        UrsaFrame::Buffer(bytes) => Ok(bytes),
                        f => Err(UrsaCodecError::UnexpectedFrame(f.tag().unwrap())),
                    }
                } else {
                    Err(UrsaCodecError::Unknown)
                }
            }
            f => Err(UrsaCodecError::UnexpectedFrame(f.tag().unwrap())),
        }
    }
}
