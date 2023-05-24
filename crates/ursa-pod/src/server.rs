use std::{
    io::{Error, ErrorKind},
    sync::Arc,
};

use blake3::Hash;
use dashmap::DashMap;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{
    connection::{
        consts::{
            CONTENT_RANGE_REQ_TAG, CONTENT_REQ_TAG, DECRYPTION_KEY_REQ_TAG, HANDSHAKE_REQ_TAG,
            MAX_LANES,
        },
        LastLaneData, Reason, UfdpConnection, UrsaCodecError, UrsaFrame,
    },
    instrument,
    tree::ProofBuf,
    types::{BlsPublicKey, BlsSignature, Secp256k1AffinePoint, Secp256k1PublicKey},
};

/// Backend trait used by [`UfdpServer`] to access external data
pub trait Backend: Send + Sync + 'static {
    /// Get the raw content of a block.
    fn raw_block(&self, cid: &Hash, block: u64) -> Option<&[u8]>;

    /// Get a hash tree for a cid. These ideally should be stored alongside the raw data.
    fn get_tree(&self, cid: &Hash) -> Option<Vec<[u8; 32]>>;

    /// Get a decryption_key for a block, includes a block request id.
    fn decryption_key(&self, request_id: u64) -> (Secp256k1AffinePoint, u64);

    /// Get a clients current balance.
    fn get_balance(&self, pubkey: Secp256k1PublicKey) -> u128;

    /// Save a batch of transactions to be submitted to consensus.
    fn save_batch(&self, batch: BlsSignature) -> Result<(), String>;
}

pub trait Stream: AsyncRead + AsyncWrite + Unpin {}
impl<T: AsyncRead + AsyncWrite + Unpin> Stream for T {}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VerifiedStreamState {
    Active,
    Pending,
    Disconnected,
}

#[cfg_attr(test, derive(Clone))]
pub struct VerifiedStreamHandler<B: Backend> {
    state: VerifiedStreamState,
    bytes_served: u64,
    last_ack: Option<BlsSignature>,
    backend: Arc<B>,
}

impl<B: Backend> VerifiedStreamHandler<B> {
    #[inline(always)]
    pub const fn new(backend: Arc<B>) -> Self {
        Self {
            backend,
            state: VerifiedStreamState::Active,
            bytes_served: 0,
            last_ack: None,
        }
    }

    /// Get the last data recieved
    pub fn last_data(&self) -> Option<LastLaneData> {
        self.last_ack.map(|signature| {
            LastLaneData {
                // TODO: Handle edge case for only one block sent and no last ack avail
                signature,
                bytes: self.bytes_served,
            }
        })
    }

    /// Deliver content to a client, waiting for a delivery ack and sending decryption keys for
    /// each block of the content.
    async fn deliver_content<S: Stream>(
        &mut self,
        conn: &mut UfdpConnection<S>,
        hash: Hash,
    ) -> std::io::Result<()> {
        let mut block_number = 0;

        let tree = instrument!(
            self.backend.get_tree(&hash),
            "tag=backend_get_tree,hash={hash}",
        )
        .ok_or(UrsaCodecError::Io(Error::new(
            ErrorKind::NotFound,
            "Tree not found for {hash}",
        )))?;

        let mut proof = ProofBuf::new(&tree, 0);
        let mut proof_len = proof.len() as u64;

        while let Some(block) = instrument!(
            self.backend.raw_block(&hash, block_number),
            "tag=backend_raw_block,hash={hash}",
        ) {
            if block_number != 0 {
                proof = ProofBuf::resume(&tree, block_number as usize);
                proof_len = proof.len() as u64;
            }

            let decryption_key = [0; 33];
            let block_len = block.len() as u64;

            instrument!(
                conn.write_frame(UrsaFrame::ContentResponse {
                    compression: 0,
                    proof_len,
                    block_len,
                    signature: [1u8; 64],
                })
                .await?,
                "tag=write_content_res,hash={hash}",
            );

            instrument!(
                conn.write_frame(UrsaFrame::Buffer(proof.as_slice().into()))
                    .await?,
                "tag=write_proof,hash={hash},proof_len={proof_len}",
            );
            instrument!(
                conn.write_frame(UrsaFrame::Buffer(block.into())).await?,
                "tag=write_block,hash={hash},block_len={block_len}",
            );

            self.state = VerifiedStreamState::Pending;

            // Wait for delivery acknowledgment.
            match instrument!(
                conn.read_frame(Some(DECRYPTION_KEY_REQ_TAG)).await?,
                "tag=read_da,hash={hash}",
            ) {
                Some(UrsaFrame::DecryptionKeyRequest { .. }) => {
                    // todo: transaction manager (batch and store tx)
                }
                _ => unreachable!(), // Guaranteed by frame filter
            }

            // Send decryption key
            // todo: integrate crypto
            instrument!(
                conn.write_frame(UrsaFrame::DecryptionKeyResponse { decryption_key })
                    .await?,
                "tag=write_dk,hash={hash}",
            );

            self.state = VerifiedStreamState::Active;

            block_number += 1;
        }

        instrument!(
            conn.write_frame(UrsaFrame::EndOfRequestSignal).await?,
            "tag=write_eor,hash={hash}",
        );

        Ok(())
    }

    async fn handle<S: Stream>(&mut self, conn: &mut UfdpConnection<S>) -> std::io::Result<()> {
        if self.state != VerifiedStreamState::Active {
            panic!("lane should not be pending or disconnected");
        }

        // Handle requests
        while let Some(frame) = instrument!(
            conn.read_frame(Some(CONTENT_REQ_TAG | CONTENT_RANGE_REQ_TAG))
                .await?,
            "tag=read_content_req",
        ) {
            match frame {
                UrsaFrame::ContentRequest { hash } => {
                    instrument!(
                        self.deliver_content(conn, hash).await?,
                        "tag=deliver_content,hash={hash}",
                    );
                }
                UrsaFrame::ContentRangeRequest { .. } => todo!(),
                _ => unreachable!(), // Guaranteed by frame filter
            }
        }

        Ok(())
    }

    fn needs_resumption(&self) -> bool {
        self.state == VerifiedStreamState::Disconnected
    }

    // Await a single delivery acknowledgment before handling requests.
    async fn resume<S: Stream>(&mut self, conn: &mut UfdpConnection<S>) -> std::io::Result<()> {
        if self.state != VerifiedStreamState::Pending {
            panic!("lane should be pending");
        }

        // Wait for delivery acknowledgment.
        match instrument!(
            conn.read_frame(Some(DECRYPTION_KEY_REQ_TAG)).await?,
            "tag=resume_read_da",
        ) {
            Some(UrsaFrame::DecryptionKeyRequest { .. }) => {}
            _ => unreachable!(), // Guaranteed by frame filter
        }

        self.state = VerifiedStreamState::Active;

        // Send decryption key.
        // todo: integrate crypto
        instrument!(
            conn.write_frame(UrsaFrame::DecryptionKeyResponse {
                decryption_key: [0; 33]
            })
            .await?,
            "tag=resume_write_dk",
        );

        // Handle session loop
        self.handle(conn).await
    }
}

/// UFDP Server.
pub struct UfdpServer<B: Backend> {
    backend: Arc<B>,
    pub sessions: DashMap<BlsPublicKey, [Option<VerifiedStreamHandler<B>>; MAX_LANES]>,
}

impl<B: Backend> UfdpServer<B> {
    #[inline(always)]
    pub fn new(backend: Arc<B>) -> Self {
        Self {
            backend,
            sessions: DashMap::new(),
        }
    }

    pub async fn serve<S: Stream>(&self, stream: S) -> Result<S, UrsaCodecError> {
        let mut conn = UfdpConnection::new(stream);

        // Step 1: Perform the handshake.
        let (pubkey, lane) = instrument!(self.handshake(&mut conn).await?, "tag=handshake",);
        // SAFETY: Handshake gaurantees the session will be available
        let lane = &mut self.sessions.entry(pubkey).or_default()[lane as usize];
        let session = lane.as_mut().unwrap();

        match session.state {
            VerifiedStreamState::Active => session.handle(&mut conn).await?,
            VerifiedStreamState::Pending => session.resume(&mut conn).await?,
            _ => unreachable!(),
        }

        Ok(conn.stream)
    }

    fn find_lane(&self, pubkey: BlsPublicKey) -> Option<u8> {
        let mut lanes = self.sessions.entry(pubkey).or_default();

        // Otherwise, find the first open lane
        if let Some(lane) =
            lanes.iter().enumerate().find_map(
                |(i, session)| {
                    if session.is_none() {
                        Some(i)
                    } else {
                        None
                    }
                },
            )
        {
            lanes[lane] = Some(VerifiedStreamHandler::new(self.backend.clone()));

            return Some(lane as u8);
        }

        None
    }

    /// Attempt to resume a lane if provided.
    /// Will always return none if a lane is attempted to resume, but it's not in the disconnected state.
    fn resume_lane(&self, pubkey: BlsPublicKey, lane: u8) -> Option<(u8, Option<LastLaneData>)> {
        let mut lanes = self.sessions.entry(pubkey).or_default();

        match &mut lanes[lane as usize] {
            Some(session) => {
                if session.needs_resumption() {
                    session.state = VerifiedStreamState::Pending;
                    return Some((lane, session.last_data()));
                }
            }
            None => {
                // Lane is open, and behavior is undefined
                return None;
            }
        }

        None
    }

    /// Wait and respond to a handshake request.
    #[inline(always)]
    pub async fn handshake<S: Stream>(
        &self,
        conn: &mut UfdpConnection<S>,
    ) -> Result<(BlsPublicKey, u8), UrsaCodecError> {
        match instrument!(
            conn.read_frame(Some(HANDSHAKE_REQ_TAG)).await?,
            "tag=read_handshake_req",
        ) {
            Some(UrsaFrame::HandshakeRequest { lane, pubkey, .. }) => {
                let (reserved_lane, last_data) = match lane
                    .map(|l| self.resume_lane(pubkey, l))
                    .unwrap_or_else(|| self.find_lane(pubkey).map(|l| (l, None)))
                {
                    Some(lane) => lane,
                    None => {
                        conn.termination_signal(Some(Reason::OutOfLanes)).await.ok();
                        todo!("handle out of lanes termination")
                    }
                };

                // Send res frame
                instrument!(
                    conn.write_frame(UrsaFrame::HandshakeResponse {
                        pubkey: [2; 33],
                        epoch_nonce: 1000,
                        lane: reserved_lane,
                        last: last_data,
                    })
                    .await?,
                    "tag=write_handshake_res",
                );

                Ok((pubkey, reserved_lane))
            }
            None => Err(UrsaCodecError::Unknown),
            Some(_) => unreachable!(), // Guaranteed by frame filter
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::{
        net::{TcpListener, TcpStream},
        sync::mpsc::channel,
        task,
    };

    use super::*;
    use crate::client::UfdpClient;

    const CLIENT_PUBKEY: BlsPublicKey = [0u8; 48];
    const CONTENT: &[u8] = &[0u8; 256 * 1024];
    const BLOCK_LEN: u64 = 4; // 1 MB

    #[derive(Clone)]
    struct DummyBackend {
        tree: Vec<[u8; 32]>,
    }

    impl DummyBackend {
        fn new() -> (Hash, Arc<Self>) {
            let mut tree_builder = blake3::ursa::HashTreeBuilder::new();
            (0..BLOCK_LEN).for_each(|_i| tree_builder.update(CONTENT));
            let output = tree_builder.finalize();
            (output.hash, Arc::new(Self { tree: output.tree }))
        }

        fn raw_block(block: u64) -> Option<&'static [u8]> {
            // serve 10GB
            if block < BLOCK_LEN {
                Some(CONTENT)
            } else {
                None
            }
        }
    }

    impl Backend for DummyBackend {
        fn raw_block(&self, _cid: &Hash, block: u64) -> Option<&[u8]> {
            Self::raw_block(block)
        }

        fn decryption_key(&self, _request_id: u64) -> (Secp256k1AffinePoint, u64) {
            let key = [1; 33];
            let key_id = 0;
            (key, key_id)
        }

        fn get_balance(&self, _pubkey: Secp256k1PublicKey) -> u128 {
            9001
        }

        fn save_batch(&self, _batch: BlsSignature) -> Result<(), String> {
            Ok(())
        }

        fn get_tree(&self, _cid: &Hash) -> Option<Vec<[u8; 32]>> {
            Some(self.tree.clone())
        }
    }

    #[tokio::test]
    async fn handshake() -> Result<(), UrsaCodecError> {
        let (_, backend) = DummyBackend::new();

        let server = UfdpServer::new(backend);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr()?;

        let server_task = task::spawn(async move {
            let (server_stream, _) = listener.accept().await?;
            let mut conn = UfdpConnection::new(server_stream);

            server.handshake(&mut conn).await
        });

        let client_stream = TcpStream::connect(addr).await?;
        let _ = UfdpClient::new(client_stream, CLIENT_PUBKEY, None).await?;

        let recv_key = server_task.await.unwrap()?;
        assert_eq!(CLIENT_PUBKEY, recv_key.0);

        Ok(())
    }

    #[tokio::test]
    async fn handshake_find_open_lane() -> Result<(), UrsaCodecError> {
        let (_, backend) = DummyBackend::new();

        let states = [
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            None,
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
        ];

        let server = UfdpServer::new(backend);
        server.sessions.insert(CLIENT_PUBKEY, states);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr()?;

        task::spawn(async move {
            let (server_stream, _) = listener.accept().await?;
            let mut conn = UfdpConnection::new(server_stream);

            server.handshake(&mut conn).await
        });

        let client = UfdpClient::new(TcpStream::connect(addr).await?, CLIENT_PUBKEY, None).await?;

        assert_eq!(client.lane(), 13);

        Ok(())
    }

    #[tokio::test]
    async fn handshake_fails_no_lanes_available() -> Result<(), UrsaCodecError> {
        let (_, backend) = DummyBackend::new();
        let server = UfdpServer::new(backend.clone());

        let states = [
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
            Some(VerifiedStreamHandler::new(backend.clone())),
        ];

        server.sessions.insert(CLIENT_PUBKEY, states);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr()?;

        task::spawn(async move {
            loop {
                let (mut server_stream, _) = listener.accept().await.unwrap();
                server.serve(&mut server_stream).await.unwrap();
            }
        });

        // Spawn 24 connections and only handshake, to fill up all lanes. Uses a channel to hold the
        // connection open until we finish the test and the channel is dropped.
        let mut tx_arr = vec![];
        for _ in 0..MAX_LANES {
            let (tx, mut rx) = channel::<()>(1);
            tx_arr.push(tx);
            task::spawn(async move {
                let _ =
                    UfdpClient::new(TcpStream::connect(addr).await.unwrap(), CLIENT_PUBKEY, None)
                        .await
                        .expect("first connections should succeed");
                rx.recv().await;
            });
        }

        // Send a connection over the max lanes, and expect it to fail
        let res = UfdpClient::new(TcpStream::connect(addr).await?, CLIENT_PUBKEY, None).await;
        assert!(res.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn serve() -> Result<(), UrsaCodecError> {
        let (hash, backend) = DummyBackend::new();
        let server = UfdpServer::new(backend);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr()?;

        // accept a conn and serve it
        let server_task = task::spawn(async move {
            let (server_stream, _) = listener.accept().await?;
            server.serve(server_stream).await
        });

        // create a client and request content
        let mut client =
            UfdpClient::new(TcpStream::connect(addr).await?, CLIENT_PUBKEY, None).await?;
        let bytes_read = client.request(hash).await?;
        assert_eq!(BLOCK_LEN as usize * CONTENT.len(), bytes_read);
        client.finish();

        server_task.await.unwrap().map(|_| ())
    }

    #[tokio::test]
    async fn resume() -> Result<(), UrsaCodecError> {
        let (hash, backend) = DummyBackend::new();
        let server = UfdpServer::new(backend.clone());

        // setup disconnected session to be resumed on lane 0
        let mut session = VerifiedStreamHandler::new(backend);
        session.state = VerifiedStreamState::Disconnected;
        server.sessions.entry(CLIENT_PUBKEY).or_default()[0] = Some(session);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr()?;

        task::spawn(async move {
            let (server_stream, _) = listener.accept().await?;
            server.serve(server_stream).await
        });

        let client_stream = TcpStream::connect(addr).await?;
        let mut client = UfdpClient::new(client_stream, CLIENT_PUBKEY, Some(0)).await?;

        // Resume and send a request afterwards
        client.resume([0u8; 96]).await?;
        let len = client.request(hash).await?;
        assert_eq!(BLOCK_LEN as usize * CONTENT.len(), len);

        client.finish();
        Ok(())
    }
}
