use std::error::Error;
//use std::net::ToSocketAddrs;

use bytes::BytesMut;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::net::ToSocketAddrs;
use tokio_util::codec::Encoder;
use ursa_pod::codec::UrsaCodec;
use ursa_pod::codec::UrsaFrame;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    UfdpClient::new("127.0.0.1:8080").await?;
    Ok(())
}

pub struct UfdpClient {
    _transport: TcpStream,
    _codec: UrsaCodec,
}

impl UfdpClient {
    pub async fn new<T: ToSocketAddrs>(dest: T) -> Result<Self, Box<dyn Error>> {
        let mut codec = UrsaCodec::default();
        let mut transport = TcpStream::connect(dest).await?;

        // send handshake
        let mut buf = BytesMut::new();
        codec
            .encode(
                UrsaFrame::HandshakeRequest {
                    version: 0,
                    supported_compression_bitmap: 0,
                    lane: 0xFF,
                    pubkey: [1; 48],
                },
                &mut buf,
            )
            .unwrap();
        transport.writable().await?;
        transport.write_all(&buf).await?;

        Ok(Self {
            _transport: transport,
            _codec: codec,
        })
    }
}
