use criterion::{measurement::Measurement, *};
use futures::Future;
use std::time::Duration;
use tokio::sync::oneshot;

/* SERVER */

const MAX_REQUESTS: usize = 5;

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

fn benchmark<T: Measurement, C, S>(
    g: &mut BenchmarkGroup<T>,
    proto: &'static str,
    files: &[&'static [u8]],
    client: impl Fn(String, usize) -> C,
    server: impl Fn(String, &'static [u8], oneshot::Sender<u16>) -> S,
) where
    C: Future,
    S: Future + Send + 'static,
    S::Output: Send + 'static,
{
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    for file in files {
        // Spawn the server and wait for it to signal that it's ready.
        let (tx_started, rx_started) = oneshot::channel();
        let server_task = runtime.spawn(server("127.0.0.1:0".into(), file, tx_started));
        let port = futures::executor::block_on(rx_started).unwrap();
        let addr = format!("127.0.0.1:{port}");

        for num_requests in 1..=MAX_REQUESTS {
            let len = file.len() * num_requests;
            g.throughput(Throughput::Bytes(len as u64));

            // We need to allocate additional time to carry the same accuracy between the benchmarks
            let mut time = Duration::from_secs(10 + num_requests as u64);
            time += Duration::from_micros(len as u64 / 40);
            g.measurement_time(time);

            g.bench_with_input(
                BenchmarkId::new(
                    format!(
                        "{proto}/{num_requests} Request{}",
                        if num_requests != 1 { "s" } else { "" }
                    ),
                    file.len(),
                ),
                &num_requests,
                |b, &n| {
                    b.to_async(&runtime).iter(|| client(addr.clone(), n));
                },
            );
        }

        server_task.abort();
    }
}

fn protocol_benchmarks(c: &mut Criterion) {
    for (range, files) in [("Kilobyte", KILOBYTE_FILES), ("Megabyte", MEGABYTE_FILES)] {
        let mut g = c.benchmark_group(range);
        g.sample_size(20);

        benchmark(
            &mut g,
            "TCP UFDP",
            files,
            tcp_ufdp::client_loop,
            tcp_ufdp::server_loop,
        );

        #[cfg(feature = "bench-hyper")]
        benchmark(
            &mut g,
            "HTTP Hyper",
            files,
            http_hyper::client_loop,
            http_hyper::server_loop,
        );
    }
}

mod tcp_ufdp {
    use tokio::net::{TcpListener, TcpStream};
    use tokio_stream::StreamExt;
    use ursa_pod::{
        client::UfdpClient,
        codec::consts::MAX_BLOCK_SIZE,
        server::{Backend, UfdpServer},
        types::{Blake3Cid, BlsSignature, Secp256k1PublicKey},
    };

    const DECRYPTION_KEY: [u8; 33] = [3u8; 33];
    const CLIENT_PUB_KEY: [u8; 48] = [3u8; 48];
    const CID: [u8; 32] = [3u8; 32];

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

    /// Simple tcp server loop that replies with static content
    pub async fn server_loop(
        addr: String,
        content: &'static [u8],
        tx_started: tokio::sync::oneshot::Sender<u16>,
    ) {
        let listener = TcpListener::bind(&addr).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let mut server = UfdpServer::new(DummyBackend { content }).unwrap();

        tx_started.send(port).unwrap();

        loop {
            let (stream, _) = listener.accept().await.unwrap();
            server.handle(stream).unwrap();
        }
    }

    /// Simple client loop that sends a request and loops over the block stream, dropping the bytes
    /// immediately.
    pub async fn client_loop(addr: String, iterations: usize) {
        let stream = TcpStream::connect(addr).await.unwrap();
        let mut client = UfdpClient::new(stream, CLIENT_PUB_KEY, None).await.unwrap();

        for _ in 0..iterations {
            let mut res = client.request(CID).await.unwrap();
            while let Some(frame) = res.next().await {
                match frame {
                    Ok(_data) => {}
                    Err(e) => panic!("{e}"),
                }
            }
        }
    }
}

#[cfg(feature = "bench-hyper")]
mod http_hyper {
    use std::io::Error;

    use bytes::Bytes;
    use http_body_util::{BodyExt, Empty, Full};
    use hyper::{server::conn::http1, service::service_fn, Request, Response};
    use tokio::net::{TcpListener, TcpStream};

    pub async fn server_loop(
        addr: String,
        content: &'static [u8],
        tx_started: tokio::sync::oneshot::Sender<u16>,
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

    pub async fn client_loop(addr: String, iterations: usize) {
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

criterion_group!(benches, protocol_benchmarks);
criterion_main!(benches);
