use std::{error::Error, fmt::Display};

use tokio::net::{TcpListener, ToSocketAddrs};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;
use tracing::info;
use ursa_pod::codec::UrsaCodec;

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
                    println!("{request:?}");
                }
            });
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let server = UfdpServer::new("0.0.0.0:8080").await?;
    server.start().await
}
