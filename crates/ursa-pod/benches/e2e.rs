use crate::tls_utils::TestCertificate;
use criterion::{measurement::Measurement, *};
use futures::Future;
use std::time::Duration;
use tokio::sync::oneshot;
use ursa_pod::connection::consts::MAX_BLOCK_SIZE;
use ursa_pod::server::Backend;
use ursa_pod::types::{Blake3Cid, BlsSignature, Secp256k1PublicKey};

const MAX_REQUESTS: usize = 64;
const DECRYPTION_KEY: [u8; 33] = [3u8; 33];

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
    client: impl Fn(String, usize, Option<TestCertificate>) -> C,
    server: impl Fn(String, &'static [u8], oneshot::Sender<u16>, Option<TestCertificate>) -> S,
) where
    C: Future,
    S: Future + Send + 'static,
    S::Output: Send + 'static,
{
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let certificate = uses_tls.then(|| TestCertificate::new());

    for file in files {
        // Spawn the server and wait for it to signal that it's ready.
        let (tx_started, rx_started) = oneshot::channel();
        let server_task = runtime.spawn(server(
            "127.0.0.1:0".into(),
            file,
            tx_started,
            certificate.clone(),
        ));
        let port = futures::executor::block_on(rx_started).unwrap();
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
                        .iter(|| client(addr.clone(), n, certificate.clone()));
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

        #[cfg(feature = "bench-quic")]
        {
            let mut g = c.benchmark_group(format!("S2N-QUIC UFDP/{range}"));
            g.sample_size(20);
            benchmark_sizes(
                &mut g,
                files,
                true,
                unit,
                s2n_quic_ufdp::client_loop,
                s2n_quic_ufdp::server_loop,
            );
        }
    }
}

#[derive(Clone, Copy)]
struct DummyBackend {
    content: &'static [u8],
}

impl Backend for DummyBackend {
    fn raw_block(&self, _cid: &Blake3Cid, block: u64) -> Option<&[u8]> {
        let s = block as usize * MAX_BLOCK_SIZE;
        if s < self.content.len() {
            let e = self.content.len().min(s + MAX_BLOCK_SIZE);
            Some(&self.content[s..e])
        } else {
            None
        }
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
}

mod tcp_ufdp {
    use super::DummyBackend;
    use crate::tls_utils::TestCertificate;
    use futures::future::join_all;
    use tokio::{
        net::{TcpListener, TcpStream},
        task,
    };
    use ursa_pod::{client::UfdpClient, server::UfdpHandler, types::Blake3Cid};

    const CLIENT_PUB_KEY: [u8; 48] = [3u8; 48];
    const CID: Blake3Cid = Blake3Cid([3u8; 32]);

    /// Simple tcp server loop that replies with static content
    pub async fn server_loop(
        addr: String,
        content: &'static [u8],
        tx_started: tokio::sync::oneshot::Sender<u16>,
        _: Option<TestCertificate>,
    ) {
        let listener = TcpListener::bind(&addr).await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tx_started.send(port).unwrap();

        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let handler = UfdpHandler::new(stream, DummyBackend { content }, 0);
            task::spawn(async move {
                if let Err(e) = handler.serve().await {
                    println!("server error: {e:?}");
                }
            });
        }
    }

    /// Simple client loop that sends a request and loops over the block stream, dropping the bytes
    /// immediately.
    pub async fn client_loop(addr: String, iterations: usize, _: Option<TestCertificate>) {
        let mut tasks = vec![];
        for _ in 0..iterations {
            let stream = TcpStream::connect(&addr).await.unwrap();
            let task = task::spawn(async {
                let mut client = UfdpClient::new(stream, CLIENT_PUB_KEY, None).await.unwrap();

                client.request(CID).await.unwrap();
            });
            tasks.push(task);
        }
        join_all(tasks).await;
    }
}

#[cfg(feature = "bench-hyper")]
mod http_hyper {
    use std::io::Error;

    use crate::tls_utils::TestCertificate;
    use bytes::Bytes;
    use http_body_util::{BodyExt, Empty, Full};
    use hyper::{server::conn::http1, service::service_fn, Request, Response};
    use tokio::net::{TcpListener, TcpStream};

    pub async fn server_loop(
        addr: String,
        content: &'static [u8],
        tx_started: tokio::sync::oneshot::Sender<u16>,
        _: Option<TestCertificate>,
    ) {
        let listener = TcpListener::bind(addr).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tx_started.send(port).unwrap();

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

    pub async fn client_loop(addr: String, iterations: usize, _: Option<TestCertificate>) {
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
    use super::{
        tls_utils::{client_config, server_config},
        DummyBackend,
    };
    use crate::tls_utils::TestCertificate;
    use futures::future::join_all;
    use quinn::{ConnectionError, Endpoint, RecvStream, SendStream, ServerConfig};
    use std::{
        io::Error,
        pin::Pin,
        sync::Arc,
        task::{Context, Poll},
    };
    use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
    use tokio::task;
    use ursa_pod::{client::UfdpClient, server::UfdpHandler, types::Blake3Cid};

    const CLIENT_PUB_KEY: [u8; 48] = [3u8; 48];
    const CID: Blake3Cid = Blake3Cid([3u8; 32]);

    pub async fn server_loop(
        addr: String,
        content: &'static [u8],
        tx_started: tokio::sync::oneshot::Sender<u16>,
        cert: Option<TestCertificate>,
    ) {
        let server_config = ServerConfig::with_crypto(Arc::new(server_config(cert.unwrap())));
        let server = Endpoint::server(server_config, addr.parse().unwrap()).unwrap();
        let port = server.local_addr().unwrap().port();

        tx_started.send(port).unwrap();

        while let Some(connecting) = server.accept().await {
            let connection = connecting.await.unwrap();
            let content_clone = content.clone();
            task::spawn(async move {
                loop {
                    match connection.accept_bi().await {
                        Ok((tx, rx)) => {
                            task::spawn(async {
                                let stream = BiStream { tx, rx };
                                let handler = UfdpHandler::new(
                                    stream,
                                    DummyBackend {
                                        content: content_clone,
                                    },
                                    0,
                                );
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

    pub async fn client_loop(addr: String, iterations: usize, cert: Option<TestCertificate>) {
        let mut tasks = vec![];
        let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap()).unwrap();
        let client_config = quinn::ClientConfig::new(Arc::new(client_config(cert.unwrap())));
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
                client.request(CID).await.unwrap();
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

#[cfg(feature = "bench-quic")]
mod s2n_quic_ufdp {
    use super::{
        tls_utils::{client_config, server_config},
        DummyBackend,
    };
    use crate::tls_utils::TestCertificate;
    use futures::future::join_all;
    use s2n_quic::{client::Connect, provider::tls, Client, Server};
    use std::net::SocketAddr;
    use tokio::task;
    use ursa_pod::{client::UfdpClient, server::UfdpHandler, types::Blake3Cid};

    const CLIENT_PUB_KEY: [u8; 48] = [3u8; 48];
    const CID: Blake3Cid = Blake3Cid([3u8; 32]);

    pub struct TlsProvider(TestCertificate);

    impl tls::Provider for TlsProvider {
        type Server = tls::rustls::Server;
        type Client = tls::rustls::Client;
        type Error = rustls::Error;

        fn start_server(self) -> Result<Self::Server, Self::Error> {
            Ok(server_config(self.0).into())
        }

        fn start_client(self) -> Result<Self::Client, Self::Error> {
            Ok(client_config(self.0).into())
        }
    }

    pub async fn server_loop(
        addr: String,
        content: &'static [u8],
        tx_started: tokio::sync::oneshot::Sender<u16>,
        cert: Option<TestCertificate>,
    ) {
        let mut server = Server::builder()
            .with_tls(TlsProvider(cert.unwrap()))
            .unwrap()
            .with_io(addr.as_str())
            .unwrap()
            .start()
            .unwrap();

        tx_started
            .send(server.local_addr().unwrap().port())
            .unwrap();

        while let Some(mut conn) = server.accept().await {
            let content_clone = content.clone();
            task::spawn(async move {
                loop {
                    match conn.accept_bidirectional_stream().await {
                        Ok(Some(stream)) => {
                            task::spawn(async {
                                let handler = UfdpHandler::new(
                                    stream,
                                    DummyBackend {
                                        content: content_clone,
                                    },
                                    0,
                                );
                                if let Err(e) = handler.serve().await {
                                    println!("server error: {e:?}");
                                }
                            });
                        }
                        Ok(None) => break,
                        Err(s2n_quic::connection::Error::Closed { .. }) => break,
                        Err(e) => panic!("{e:?}"),
                    }
                }
            });
        }
    }

    pub async fn client_loop(addr: String, iterations: usize, cert: Option<TestCertificate>) {
        let mut tasks = vec![];
        let client = Client::builder()
            .with_tls(TlsProvider(cert.unwrap()))
            .unwrap()
            .with_io("0.0.0.0:0")
            .unwrap()
            .start()
            .unwrap();
        let addr: SocketAddr = addr.parse().unwrap();
        let mut connection = client
            .connect(Connect::new(addr).with_server_name("localhost"))
            .await
            .unwrap();
        for _ in 0..iterations {
            let stream = connection.open_bidirectional_stream().await.unwrap();
            let task = task::spawn(async move {
                let mut client = UfdpClient::new(stream, CLIENT_PUB_KEY, None).await.unwrap();
                client.request(CID).await.unwrap();
            });
            tasks.push(task);
        }
        join_all(tasks).await;
    }
}

#[cfg(feature = "bench-quic")]
mod tls_utils {
    #[derive(Clone)]
    pub struct TestCertificate {
        pub cert: Vec<rustls::Certificate>,
        pub key: rustls::PrivateKey,
    }

    impl TestCertificate {
        pub fn new() -> Self {
            let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
            let key = rustls::PrivateKey(cert.serialize_private_key_der());
            let cert = vec![rustls::Certificate(cert.serialize_der().unwrap())];
            Self { cert, key }
        }
    }

    pub fn server_config(cert: TestCertificate) -> rustls::ServerConfig {
        let mut config = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert.cert, cert.key)
            .unwrap();
        config.alpn_protocols = vec![b"ufdp".to_vec()];
        config
    }

    pub fn client_config(cert: TestCertificate) -> rustls::ClientConfig {
        let mut roots = rustls::RootCertStore::empty();
        roots.add(&cert.cert.first().unwrap()).unwrap();

        let mut config = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(roots)
            .with_no_client_auth();
        config.alpn_protocols = vec![b"ufdp".to_vec()];
        config
    }
}

criterion_group!(benches, protocol_benchmarks);
criterion_main!(benches);
