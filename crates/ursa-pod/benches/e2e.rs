use std::time::Duration;

use bytes::BytesMut;
use criterion::*;
use tokio::net::{TcpListener, TcpStream};
use tokio_stream::StreamExt;

use ursa_pod::{
    client::UfdpClient,
    codec::UrsaCodecError,
    server::{Backend, UfdpServer},
    types::{Blake3Cid, BlsSignature, Secp256k1PublicKey},
};

const SERVER_ADDRESS: &str = "127.0.0.1:8080";
const DECRYPTION_KEY: [u8; 33] = [3u8; 33];
const CLIENT_PUB_KEY: [u8; 48] = [3u8; 48];
const CID: [u8; 32] = [3u8; 32];

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

#[derive(Clone, Copy)]
struct DummyBackend {
    content: &'static [u8],
}

impl Backend for DummyBackend {
    fn raw_content(&self, _cid: Blake3Cid) -> (BytesMut, u64) {
        let content = BytesMut::from(self.content);
        (content, 0)
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
async fn server_tcp_loop(content: &'static [u8]) -> Result<(), UrsaCodecError> {
    let listener = TcpListener::bind(SERVER_ADDRESS).await?;
    let mut server = UfdpServer::new(DummyBackend { content })?;

    loop {
        let (stream, _) = listener.accept().await?;
        server.handle(stream)?;
    }
}

/* CLIENT */

/// Simple client loop that sends a request and loops over the block stream, dropping the bytes
/// immediately.
async fn client_tcp_loop(iterations: usize) -> Result<(), UrsaCodecError> {
    let stream = TcpStream::connect(SERVER_ADDRESS).await?;
    let mut client = UfdpClient::new(stream, CLIENT_PUB_KEY, None).await?;

    for _ in 0..iterations {
        let mut res = client.request(CID).await?;
        loop {
            match res.next().await {
                Some(Ok(data)) => drop(data),
                Some(Err(e)) => panic!("{e}"),
                None => break,
            }
        }
    }

    Ok(())
}

fn bench_tcp_group(c: &mut Criterion) {
    for (range, files) in [("kilobyte", KILOBYTE_FILES), ("megabyte", MEGABYTE_FILES)] {
        let mut g = c.benchmark_group(format!("TCP UFDP Session (range: {range})"));
        g.sample_size(20);
        for num_requests in 1..MAX_REQUESTS + 1 {
            for file in files {
                let len = file.len() * num_requests;
                g.throughput(Throughput::Bytes(len as u64));

                // We need to allocate additional time to have the same accuracy between the benchmarks
                let mut time = Duration::from_secs(7);
                if num_requests > 3 {
                    time += Duration::from_secs(5);
                }
                if file.len() >= 256 * 1024 * 1024 {
                    time += Duration::from_secs(5);
                }
                g.measurement_time(time);

                g.bench_with_input(
                    BenchmarkId::new(
                        format!(
                            "{num_requests} Request{}",
                            if num_requests != 1 { "s" } else { "" }
                        ),
                        file.len(),
                    ),
                    &num_requests,
                    |b, &n| {
                        let runtime = tokio::runtime::Runtime::new().unwrap();
                        runtime.spawn(server_tcp_loop(file));
                        b.to_async(runtime).iter(|| client_tcp_loop(n));
                    },
                );
            }
        }
    }
}

criterion_group!(benches, bench_tcp_group);
criterion_main!(benches);
