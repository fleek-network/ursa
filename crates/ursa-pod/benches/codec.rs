#![feature(core_intrinsics)]

use std::time::Duration;

use arrayref::array_ref;
use benchmarks_utils::*;
use criterion::{measurement::Measurement, *};
use futures::executor::block_on;
use tokio::sync::Mutex;
use ursa_pod::{
    blake3::Hash,
    connection::{Reason, UfdpConnection, UrsaFrame},
};

mod transport {
    use bytes::BytesMut;
    use tokio::io::{AsyncRead, AsyncWrite};

    /// Direct transport for benchmarking frames.
    /// - `in_buf`: Holds a frame to read, every time.
    /// - `out_buf`: Holds the last frame written.
    #[derive(Default, Clone)]
    pub struct DirectTransport {
        pub in_buf: BytesMut,
        pub out_buf: BytesMut,
    }

    impl AsyncRead for DirectTransport {
        fn poll_read(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            // always return the same frame
            let bytes = self.in_buf.clone();
            buf.put_slice(&bytes);
            std::task::Poll::Ready(Ok(()))
        }
    }

    impl AsyncWrite for DirectTransport {
        fn poll_write(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> std::task::Poll<Result<usize, std::io::Error>> {
            // store the last thing written to out_buf
            let out_buf = &mut self.get_mut().out_buf;
            *out_buf = BytesMut::from(buf);
            std::task::Poll::Ready(Ok(out_buf.len()))
        }

        fn poll_flush(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), std::io::Error>> {
            std::task::Poll::Ready(Ok(()))
        }

        fn poll_shutdown(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), std::io::Error>> {
            std::task::Poll::Ready(Ok(()))
        }
    }
}

fn bench_frame<T: Measurement>(g: &mut BenchmarkGroup<T>, frame: UrsaFrame, title: &'static str) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    g.throughput(Throughput::Bytes(frame.size_hint() as u64));
    let transport = transport::DirectTransport::default();

    g.bench_function(format!("{title}/encode"), |b| {
        let conn = Mutex::new(UfdpConnection::new(transport.clone()));
        b.to_async(&runtime).iter(|| async {
            // executed sequentially so should be a minimal await
            let mut conn = conn.lock().await;
            conn.write_frame(frame.clone()).await
        })
    });

    // setup for decoding
    let transport = block_on(async move {
        let mut conn = UfdpConnection::new(transport);
        conn.write_frame(frame.clone()).await.unwrap();
        // out_buf contains the encoded frame, so we take that into in_buf to be decoded
        conn.stream.in_buf = conn.stream.out_buf.split();
        conn.stream
    });

    g.bench_function(format!("{title}/decode"), |b| {
        let conn = Mutex::new(UfdpConnection::new(transport.clone()));
        b.to_async(&runtime).iter(|| async {
            let mut conn = conn.lock().await;
            conn.read_frame(None).await
        })
    });
}

fn bench_codec_group(c: &mut Criterion) {
    let mut g = c.benchmark_group("Codec");
    g.sample_size(20);
    g.measurement_time(Duration::from_secs(15));

    // Handshake request
    let pubkey = random_vec(48);
    let frame = UrsaFrame::HandshakeRequest {
        version: 0,
        supported_compression_bitmap: 0,
        lane: None,
        pubkey: *array_ref!(pubkey, 0, 48),
    };
    bench_frame(&mut g, frame, "handshake_request");

    // Handshake response
    let pubkey = random_vec(33);
    let frame = UrsaFrame::HandshakeResponse {
        pubkey: *array_ref![pubkey, 0, 33],
        epoch_nonce: 65535,
        lane: 0,
        last: None,
    };
    bench_frame(&mut g, frame, "handshake_response");

    // Content request
    let cid = random_vec(32);
    let frame = UrsaFrame::ContentRequest {
        hash: Hash::from(*array_ref!(cid, 0, 32)),
    };
    bench_frame(&mut g, frame, "content_request");

    // Content range request
    let data = random_vec(32);
    let frame = UrsaFrame::ContentRangeRequest {
        hash: Hash::from(*array_ref!(data, 0, 32)),
        chunk_start: 0,
        chunks: 1,
    };
    bench_frame(&mut g, frame, "content_range_request");

    // Content response (frame only)
    let signature = random_vec(64);
    let frame = UrsaFrame::ContentResponse {
        compression: 0,
        proof_len: 0,
        // 0 len block is an error!
        block_len: 1,
        signature: *array_ref!(signature, 0, 64),
    };
    bench_frame(&mut g, frame, "content_response");

    // Decryption key request
    let signature = random_vec(96);
    let frame = UrsaFrame::DecryptionKeyRequest {
        delivery_acknowledgment: *array_ref!(signature, 0, 96),
    };
    bench_frame(&mut g, frame, "decryption_key_request");

    // Decryption key response
    let data = random_vec(33);
    let frame = UrsaFrame::DecryptionKeyResponse {
        decryption_key: *array_ref!(data, 0, 33),
    };
    bench_frame(&mut g, frame, "decryption_key_response");

    // Update epoch Signal
    let frame = UrsaFrame::UpdateEpochSignal(65535);
    bench_frame(&mut g, frame, "update_epoch_signal");

    // End of request signal
    let frame = UrsaFrame::EndOfRequestSignal;
    bench_frame(&mut g, frame, "end_of_request_signal");

    // Termination signal
    let frame = UrsaFrame::TerminationSignal(Reason::Unknown);
    bench_frame(&mut g, frame, "termination_signal");

    g.finish();
}

criterion_group!(benches, bench_codec_group);
criterion_main!(benches);
