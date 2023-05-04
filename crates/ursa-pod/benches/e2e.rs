use std::time::Duration;

use criterion::{measurement::Measurement, *};
use futures::Future;
use tokio::sync::oneshot;
use ursa_pod::{
    blake3::Hash,
    connection::consts::MAX_BLOCK_SIZE,
    server::Backend,
    types::{BlsSignature, Secp256k1PublicKey},
};

use crate::tls_utils::TestTlsConfig;

const MAX_REQUESTS: usize = 64;
const DECRYPTION_KEY: [u8; 33] = [3u8; 33];
const CLIENT_PUB_KEY: [u8; 48] = [3u8; 48];
const CID: Blake3Cid = Blake3Cid([3u8; 32]);

const KILOBYTE_FILES: &[&[u8]] = &[
    &[0u8; 1024],
    &[0u8; 2 * 1024],
    &[0u8; 4 * 1024],
    &[0u8; 8 * 1024],
    &[0u8; 16 * 1024],
    &[0u8; 32 * 1024],
    &[0u8; 64 * 1024],
    &[0u8; 128 * 1024],
    &[0u8; 256 * 1024],
    &[0u8; 512 * 1024],
];
const MEGABYTE_FILES: &[&[u8]] = &[
    &[0u8; 1024 * 1024],
    &[0u8; 2 * 1024 * 1024],
    &[0u8; 4 * 1024 * 1024],
    &[0u8; 8 * 1024 * 1024],
    &[0u8; 16 * 1024 * 1024],
    &[0u8; 32 * 1024 * 1024],
    &[0u8; 64 * 1024 * 1024],
    &[0u8; 128 * 1024 * 1024],
    &[0u8; 256 * 1024 * 1024],
    &[0u8; 512 * 1024 * 1024],
];

fn benchmark_sizes<T: Measurement, C, S>(
    g: &mut BenchmarkGroup<T>,
    files: &[&'static [u8]],
    uses_tls: bool,
    unit: usize,
    client: impl Fn(String, usize, Hash, Option<TestTlsConfig>) -> C,
    server: impl Fn(String, &'static [u8], oneshot::Sender<(u16, Hash)>, Option<TestTlsConfig>) -> S,
) where
    C: Future,
    S: Future + Send + 'static,
    S::Output: Send + 'static,
{
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let certificate = uses_tls.then(TestTlsConfig::new);

    for file in files {
        // Spawn the server and wait for it to signal that it's ready.
        let (tx_started, rx_started) = oneshot::channel();
        let server_task = runtime.spawn(server(
            "127.0.0.1:0".into(),
            file,
            tx_started,
            certificate.clone(),
        ));
        let (port, hash) = futures::executor::block_on(rx_started).unwrap();
        let addr = format!("127.0.0.1:{port}");

        let mut num_requests = 1;
        while num_requests <= MAX_REQUESTS {
            let len = file.len() * num_requests;
            g.throughput(Throughput::Bytes(len as u64));

            // We need to allocate additional time to carry the same accuracy between the benchmarks
            let mut time = Duration::from_secs(10 + num_requests as u64);
            time += Duration::from_micros(len as u64 / 40);
            g.measurement_time(time);

            g.bench_with_input(
                BenchmarkId::new(
                    format!(
                        "{num_requests} request{}",
                        if num_requests != 1 { "s" } else { "" }
                    ),
                    file.len() / unit,
                ),
                &num_requests,
                |b, &n| {
                    b.to_async(&runtime)
                        .iter(|| client(addr.clone(), n, hash, certificate.clone()));
                },
            );

            num_requests *= 2;
        }

        server_task.abort();
    }
}

fn protocol_benchmarks(c: &mut Criterion) {
    // benchmark different file sizes
    for (range, files, unit) in [
        ("Content Size (Kilobyte)", KILOBYTE_FILES, 1024),
        ("Content Size (Megabyte)", MEGABYTE_FILES, 1024 * 1024),
    ] {
        {
            let mut g = c.benchmark_group(format!("TCP UFDP/{range}"));
            g.sample_size(20);
            benchmark_sizes(
                &mut g,
                files,
                false,
                unit,
                tcp_ufdp::client_loop,
                tcp_ufdp::server_loop,
            );
        }

        {
            let mut g = c.benchmark_group(format!("TCP/TLS UFDP/{range}"));
            g.sample_size(20);
            benchmark_sizes(
                &mut g,
                files,
                true,
                unit,
                tcp_tls_ufdp::client_loop,
                tcp_tls_ufdp::server_loop,
            );
        }

        #[cfg(feature = "bench-hyper")]
        {
            let mut g = c.benchmark_group(format!("HTTP Hyper/{range}"));
            g.sample_size(20);
            benchmark_sizes(
                &mut g,
                files,
                false,
                unit,
                http_hyper::client_loop,
                http_hyper::server_loop,
            );
        }

        #[cfg(feature = "bench-quic")]
        {
            let mut g = c.benchmark_group(format!("QUINN UFDP/{range}"));
            g.sample_size(20);
            benchmark_sizes(
                &mut g,
                files,
                true,
                unit,
                quinn_ufdp::client_loop,
                quinn_ufdp::server_loop,
            );
        }

        #[cfg(feature = "bench-websockets")]
        {
            let mut g = c.benchmark_group(format!("Websockets UFDP/{range}"));
            g.sample_size(20);
            benchmark_sizes(
                &mut g,
                files,
                true,
                unit,
                websocket_ufdp::client_loop,
                websocket_ufdp::server_loop,
            );
        }
    }
}

#[derive(Clone)]
struct DummyBackend {
    content: &'static [u8],
    tree: Vec<[u8; 32]>,
}

impl DummyBackend {
    fn new(content: &'static [u8]) -> (Self, Hash) {
        // build a 10GB blake3 tree
        let mut tree_builder = blake3::ursa::HashTreeBuilder::new();
        let mut i = 0;
        while let Some(block) = Self::raw_block(content, i) {
            tree_builder.update(block);
            i += 1
        }
        let output = tree_builder.finalize();
        let hash = output.hash;
        let tree = output.tree;
        (Self { content, tree }, hash)
    }

    fn raw_block(content: &'static [u8], block: u64) -> Option<&[u8]> {
        let s = block as usize * MAX_BLOCK_SIZE;
        if s < content.len() {
            let e = content.len().min(s + MAX_BLOCK_SIZE);
            Some(&content[s..e])
        } else {
            None
        }
    }
}

impl Backend for DummyBackend {
    fn raw_block(&self, _hash: &Hash, block: u64) -> Option<&[u8]> {
        Self::raw_block(self.content, block)
    }

    fn decryption_key(&self, _request_id: u64) -> (ursa_pod::types::Secp256k1AffinePoint, u64) {
        (DECRYPTION_KEY, 0)
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

mod tcp_ufdp {
    use futures::future::join_all;
    use tokio::{
        net::{TcpListener, TcpStream},
        task,
    };
    use ursa_pod::{blake3::Hash, client::UfdpClient, server::UfdpHandler};
    
    use super::{
      CLIENT_PUB_KEY, DummyBackend,
      tls_utils::TestTlsConfig,
     };

    /// Simple tcp server loop that replies with static content
    pub async fn server_loop(
        addr: String,
        content: &'static [u8],
        tx_started: tokio::sync::oneshot::Sender<(u16, Hash)>,
        _: Option<TestTlsConfig>,
    ) {
        let (backend, hash) = DummyBackend::new(content);

        let listener = TcpListener::bind(&addr).await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tx_started.send((port, hash)).unwrap();

        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let handler = UfdpHandler::new(stream, backend.clone(), 0);
            task::spawn(async move {
                if let Err(e) = handler.serve().await {
                    println!("server error: {e:?}");
                }
            });
        }
    }

    /// Simple client loop that sends a request and loops over the block stream, dropping the bytes
    /// immediately.

    pub async fn client_loop(
        addr: String,
        iterations: usize,
        hash: Hash,
        _: Option<TestTlsConfig>,
    ) {
        let mut tasks = vec![];
        for _ in 0..iterations {
            let stream = TcpStream::connect(&addr).await.unwrap();
            let task = task::spawn(async move {
                let mut client = UfdpClient::new(stream, CLIENT_PUB_KEY, None).await.unwrap();

                client.request(hash).await.unwrap();
            });
            tasks.push(task);
        }
        join_all(tasks).await;
    }
}

mod tcp_tls_ufdp {
    use std::sync::Arc;

    use futures::future::join_all;
    use tokio::{
        io::AsyncWriteExt,
        net::{TcpListener, TcpStream},
        task,
    };
    use tokio_rustls::{TlsAcceptor, TlsConnector};
    use ursa_pod::{blake3::Hash, client::UfdpClient, server::UfdpHandler};

    use super::{tls_utils::TestTlsConfig, DummyBackend, CLIENT_PUB_KEY};

    pub async fn server_loop(
        addr: String,
        content: &'static [u8],
        tx_started: tokio::sync::oneshot::Sender<(u16, Hash)>,
        tls_config: Option<TestTlsConfig>,
    ) {
        let (backend, hash) = DummyBackend::new(content);
        let listener = TcpListener::bind(&addr).await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tx_started.send((port, hash)).unwrap();

        let acceptor = TlsAcceptor::from(Arc::new(tls_config.unwrap().server_config()));
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let stream = acceptor.accept(stream).await.unwrap();
            let handler = UfdpHandler::new(stream, backend.clone(), 0);
            task::spawn(async move {
                if let Err(e) = handler.serve().await {
                    println!("server error: {e:?}");
                }
            });
        }
    }

    pub async fn client_loop(
        addr: String,
        iterations: usize,
        hash: Hash,
        tls_config: Option<TestTlsConfig>,
    ) {
        let domain = rustls::ServerName::try_from("localhost").unwrap();
        let mut tasks = vec![];
        let tls_config = Arc::new(tls_config.unwrap().client_config());
        for _ in 0..iterations {
            let connector = TlsConnector::from(tls_config.clone());
            let stream = TcpStream::connect(&addr).await.unwrap();
            let stream = connector.connect(domain.clone(), stream).await.unwrap();
            let task = task::spawn(async move {
                let mut client = UfdpClient::new(stream, CLIENT_PUB_KEY, None).await.unwrap();
                client.request(hash).await.unwrap();
                client.finish().shutdown().await.unwrap();
            });
            tasks.push(task);
        }
        join_all(tasks).await;
    }
}

#[cfg(feature = "bench-websockets")]
mod websocket_ufdp {
    use super::DummyBackend;
    use crate::tls_utils::TestTlsConfig;
    use bytes::Buf;
    use futures::future::join_all;
    use futures::{ready, Sink, TryStream};
    use std::io;
    use std::io::Read;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::task::{Context, Poll};
    use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
    use tokio::{net::TcpListener, task};
    use tokio_rustls::TlsAcceptor;
    use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
    use tokio_tungstenite::{Connector, WebSocketStream};
    use url::Url;
    use ursa_pod::{client::UfdpClient, server::UfdpHandler};
    use crate::{CID, CLIENT_PUB_KEY};

    pub async fn server_loop(
        addr: String,
        content: &'static [u8],
        tx_started: tokio::sync::oneshot::Sender<u16>,
        tls_config: Option<TestTlsConfig>,
    ) {
        let listener = TcpListener::bind(&addr).await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tx_started.send(port).unwrap();

        let acceptor = TlsAcceptor::from(Arc::new(tls_config.unwrap().server_config()));
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let stream = acceptor.accept(stream).await.unwrap();
            let stream = tokio_tungstenite::accept_async(stream).await.unwrap();
            let handler = UfdpHandler::new(
                WebSocketStreamWrap::new(stream),
                DummyBackend { content },
                0,
            );
            task::spawn(async move {
                if let Err(e) = handler.serve().await {
                    println!("server error: {e:?}");
                }
            });
        }
    }

    pub async fn client_loop(addr: String, iterations: usize, tls_config: Option<TestTlsConfig>) {
        let mut tasks = vec![];
        let addr = format!("wss://localhost:{}/bench", addr.strip_prefix("127.0.0.1:").unwrap());
        let tls_config = Arc::new(tls_config.unwrap().client_config());
        for _ in 0..iterations {
            let url = Url::parse(&addr).unwrap();
            let (stream, _) = tokio_tungstenite::connect_async_tls_with_config(
                url,
                Some(WebSocketConfig::default()),
                Some(Connector::Rustls(tls_config.clone())),
            )
            .await
            .unwrap();
            let task = task::spawn(async {
                let mut client =
                    UfdpClient::new(WebSocketStreamWrap::new(stream), CLIENT_PUB_KEY, None)
                        .await
                        .unwrap();

                client.request(CID).await.unwrap();
                let mut stream = client.finish();
                stream.inner.close(None).await.unwrap();
            });
            tasks.push(task);
        }
        join_all(tasks).await;
    }

    // We need this because tokio_tungstenite::WebSocketStream implement Sink and Stream.
    pub struct WebSocketStreamWrap<S> {
        inner: WebSocketStream<S>,
        current_item: Option<io::Cursor<Vec<u8>>>,
    }

    impl<S> WebSocketStreamWrap<S> {
        fn new(inner: WebSocketStream<S>) -> Self {
            Self {
                inner,
                current_item: None,
            }
        }
    }

    impl<S: AsyncWrite + AsyncRead + Unpin> AsyncWrite for WebSocketStreamWrap<S> {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            let mut inner = self.as_mut();
            ready!(Pin::new(&mut inner.inner)
                .poll_ready(cx)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?);
            let len = buf.len();
            if let Err(e) = Pin::new(&mut inner.inner).start_send(buf.into()) {
                return Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, e.to_string())));
            }
            Poll::Ready(Ok(len))
        }

        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Pin::new(&mut self.inner)
                .poll_flush(cx)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
        }

        fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Pin::new(&mut self.inner)
                .poll_close(cx)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
        }
    }

    impl<S: AsyncWrite + AsyncRead + Unpin> AsyncRead for WebSocketStreamWrap<S> {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            let item_to_copy = loop {
                if let Some(ref mut i) = self.current_item {
                    if i.position() < i.get_ref().len() as u64 {
                        break i;
                    }
                }
                self.current_item =
                    Some(match ready!(Pin::new(&mut self.inner).try_poll_next(cx)) {
                        Some(Ok(i)) => {
                            if i.is_binary() {
                                io::Cursor::new(i.into_data())
                            } else if i.is_close() {
                                return Poll::Ready(Ok(()));
                            } else {
                                panic!("Non binary frame sent");
                            }
                        }
                        Some(Err(e)) => {
                            return Poll::Ready(Err(io::Error::new(
                                io::ErrorKind::Other,
                                e.to_string(),
                            )))
                        }
                        None => return Poll::Ready(Ok(())),
                    });
            };

            let mut buff = vec![0; item_to_copy.remaining().min(buf.remaining())];
            item_to_copy.read_exact(&mut buff).unwrap();
            buf.put_slice(buff.as_slice());

            Poll::Ready(Ok(()))
        }
    }
}

#[cfg(feature = "bench-hyper")]
mod http_hyper {
    use bytes::Bytes;
    use http_body_util::{BodyExt, Empty, Full};
    use hyper::{server::conn::http1, service::service_fn, Request, Response};
    use tokio::net::{TcpListener, TcpStream};
    use ursa_pod::blake3::Hash;

    use super::tls_utils::TestTlsConfig;

    pub async fn server_loop(
        addr: String,
        content: &'static [u8],
        tx_started: tokio::sync::oneshot::Sender<(u16, Hash)>,
        _: Option<TestTlsConfig>,
    ) {
        let listener = TcpListener::bind(addr).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tx_started.send((port, Hash::from([0u8; 32]))).unwrap();

        loop {
            let (stream, _) = listener.accept().await.unwrap();

            let service = service_fn(move |_req| async move {
                Ok::<_, Error>(Response::new(Full::new(Bytes::from(content))))
            });

            if let Err(err) = http1::Builder::new()
                .serve_connection(stream, service)
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        }
    }

    pub async fn client_loop(
        addr: String,
        iterations: usize,
        _hash: Hash,
        _: Option<TestTlsConfig>,
    ) {
        for _ in 0..iterations {
            // Open a TCP connection to the remote host
            let stream = TcpStream::connect(&addr).await.unwrap();
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
                .uri(&addr)
                .header(hyper::header::HOST, "127.0.0.1")
                .body(Empty::<Bytes>::new())
                .unwrap();
            // Send it
            let mut res = sender.send_request(req).await.unwrap();
            // Stream the body, dropping each chunk immediately
            while let Some(frame) = res.frame().await {
                match frame {
                    Ok(_bytes) => {}
                    Err(e) => panic!("{e:?}"),
                }
            }
        }
    }
}

#[cfg(feature = "bench-quic")]
mod quinn_ufdp {
    use std::{
        io::Error,
        pin::Pin,
        sync::Arc,
        task::{Context, Poll},
    };

    use futures::future::join_all;
    use quinn::{ConnectionError, Endpoint, RecvStream, SendStream, ServerConfig};
    use tokio::{
        io::{AsyncRead, AsyncWrite, ReadBuf},
        task,
    };
    use ursa_pod::{blake3::Hash, client::UfdpClient, server::UfdpHandler};

    use super::{CLIENT_PUB_KEY, tls_utils::TestTlsConfig, DummyBackend};

    pub async fn server_loop(
        addr: String,
        content: &'static [u8],
        tx_started: tokio::sync::oneshot::Sender<(u16, Hash)>,
        tls_config: Option<TestTlsConfig>,
    ) {
        let (backend, hash) = DummyBackend::new(content);
        let server_config =
            ServerConfig::with_crypto(Arc::new(tls_config.unwrap().server_config()));
        let server = Endpoint::server(server_config, addr.parse().unwrap()).unwrap();
        let port = server.local_addr().unwrap().port();

        tx_started.send((port, hash)).unwrap();

        while let Some(connecting) = server.accept().await {
            let connection = connecting.await.unwrap();
            let backend = backend.clone();
            task::spawn(async move {
                loop {
                    let backend = backend.clone();
                    match connection.accept_bi().await {
                        Ok((tx, rx)) => {
                            task::spawn(async move {
                                let stream = BiStream { tx, rx };
                                let handler = UfdpHandler::new(stream, backend, 0);
                                if let Err(e) = handler.serve().await {
                                    println!("server error: {e:?}");
                                }
                            });
                        }
                        Err(ConnectionError::ApplicationClosed(_)) => {
                            // Client closed the connection.
                            break;
                        }
                        Err(e) => panic!("{e:?}"),
                    }
                }
            });
        }
    }

    pub async fn client_loop(
        addr: String,
        iterations: usize,
        hash: Hash,
        tls_config: Option<TestTlsConfig>,
    ) {
        let mut tasks = vec![];
        let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap()).unwrap();
        let client_config = quinn::ClientConfig::new(Arc::new(tls_config.unwrap().client_config()));
        endpoint.set_default_client_config(client_config);
        let stream = endpoint
            .connect(addr.parse().unwrap(), "localhost")
            .unwrap()
            .await
            .unwrap();
        for _ in 0..iterations {
            let (tx, rx) = stream.open_bi().await.unwrap();
            let stream = BiStream { tx, rx };
            let task = task::spawn(async move {
                let mut client = UfdpClient::new(stream, CLIENT_PUB_KEY, None).await.unwrap();
                client.request(hash).await.unwrap();
                let mut stream = client.finish();
                stream.tx.finish().await.unwrap();
            });
            tasks.push(task);
        }
        join_all(tasks).await;
    }

    // Bidirectional QUIC stream for sending one request.
    struct BiStream {
        tx: SendStream,
        rx: RecvStream,
    }

    impl AsyncRead for BiStream {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.rx).poll_read(cx, buf)
        }
    }

    impl AsyncWrite for BiStream {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize, Error>> {
            Pin::new(&mut self.tx).poll_write(cx, buf)
        }

        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
            Pin::new(&mut self.tx).poll_flush(cx)
        }

        fn poll_shutdown(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Result<(), Error>> {
            Pin::new(&mut self.tx).poll_shutdown(cx)
        }
    }
}

mod tls_utils {
    use rustls::{Certificate, ClientConfig, PrivateKey, ServerConfig};

    #[derive(Clone)]
    pub struct TestTlsConfig {
        pub cert: Vec<Certificate>,
        pub key: PrivateKey,
        server_config: Option<ServerConfig>,
        client_config: Option<ClientConfig>,
    }

    impl TestTlsConfig {
        pub fn new() -> Self {
            Self::with_configs(None, None)
        }

        pub fn with_configs(
            server_config: Option<ServerConfig>,
            client_config: Option<ClientConfig>,
        ) -> Self {
            let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
            let key = PrivateKey(cert.serialize_private_key_der());
            let cert = vec![Certificate(cert.serialize_der().unwrap())];
            Self {
                cert,
                key,
                server_config,
                client_config,
            }
        }

        pub fn server_config(self) -> ServerConfig {
            if let Some(config) = self.server_config {
                config
            } else {
                let mut config = ServerConfig::builder()
                    .with_safe_defaults()
                    .with_no_client_auth()
                    .with_single_cert(self.cert, self.key)
                    .unwrap();
                config.alpn_protocols = vec![b"ufdp".to_vec()];
                config
            }
        }

        pub fn client_config(self) -> ClientConfig {
            if let Some(config) = self.client_config {
                config
            } else {
                let mut roots = rustls::RootCertStore::empty();
                roots.add(self.cert.first().unwrap()).unwrap();

                let mut config = ClientConfig::builder()
                    .with_safe_defaults()
                    .with_root_certificates(roots)
                    .with_no_client_auth();
                config.alpn_protocols = vec![b"ufdp".to_vec()];
                config
            }
        }
    }
}

criterion_group!(benches, protocol_benchmarks);
criterion_main!(benches);
