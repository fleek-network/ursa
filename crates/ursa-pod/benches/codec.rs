#![feature(core_intrinsics)]

use std::time::Duration;

use arrayref::array_ref;
use benchmarks_utils::*;
use bytes::BytesMut;
use criterion::{measurement::Measurement, *};
use tokio_util::codec::{Decoder, Encoder};

use ursa_pod::codec::{Reason, UrsaCodec, UrsaFrame};

fn bench_frame<T: Measurement>(g: &mut BenchmarkGroup<T>, frame: UrsaFrame, title: &'static str) {
    g.throughput(Throughput::Bytes(frame.size_hint() as u64));
    let mut codec = UrsaCodec::default();

    g.bench_function(format!("{title}/encode"), |b| {
        let mut result = BytesMut::new();
        b.iter(|| {
            codec.encode(frame.clone(), &mut result).unwrap();
            black_box(&result);
        })
    });

    let mut bytes = BytesMut::new();
    codec.encode(frame, &mut bytes).unwrap();
    g.bench_function(format!("{title}/decode"), |b| {
        b.iter(|| {
            let res = codec.decode(&mut bytes.clone()).unwrap();
            black_box(res);
        })
    });
}

fn bench_encode_group(c: &mut Criterion) {
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
        hash: *array_ref!(cid, 0, 32),
    };
    bench_frame(&mut g, frame, "content_request");

    // Content range request
    let data = random_vec(32);
    let frame = UrsaFrame::ContentRangeRequest {
        hash: *array_ref!(data, 0, 32),
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

criterion_group!(benches, bench_encode_group);
criterion_main!(benches);