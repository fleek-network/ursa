#![feature(core_intrinsics)]

use benchmarks_utils::*;
use criterion::*;
use rand_core::OsRng;
use ursa_pod::{crypto::*, keys::SecretKey};

fn bench_primitives(c: &mut Criterion) {
    let mut g = c.benchmark_group("Primitives");
    g.sample_size(50);

    let sizes = [
        1,
        16,
        32,
        48,
        63,
        64,
        512,
        1 * 1024,
        4 * 1024,
        8 * 1024,
        1 * 16 * 1024,
        2 * 16 * 1024,
        3 * 16 * 1024,
        4 * 16 * 1024,
        5 * 16 * 1024,
        6 * 16 * 1024,
        7 * 16 * 1024,
        8 * 16 * 1024,
        9 * 16 * 1024,
        10 * 16 * 1024,
        11 * 16 * 1024,
        12 * 16 * 1024,
        13 * 16 * 1024,
        14 * 16 * 1024,
        15 * 16 * 1024,
        16 * 16 * 1024, // 256KiB
    ];

    g.bench_function("hash_request_info", |b| {
        let req = RequestInfo::rand(OsRng);
        b.iter(|| {
            black_box(req.hash());
        })
    });

    g.bench_function("hash_to_curve", |b| {
        b.iter(|| {
            black_box(hash_to_curve(&[0; 32]));
        })
    });

    g.bench_function("generate_symmetric_key", |b| {
        let sk = SecretKey::random(OsRng);
        b.iter(|| {
            let ret = generate_symmetric_key(&sk, &[0; 32]);
            black_box(ret);
        })
    });
    g.bench_function("sign_ciphertext", |b| {
        let sk = SecretKey::random(OsRng);
        b.iter(|| {
            let ret = sign_ciphertext(&sk, &[0; 32], &[0; 32]);
            black_box(ret);
        })
    });

    g.sample_size(20);

    for size in sizes {
        g.throughput(Throughput::Bytes(size as u64));

        g.bench_with_input(
            BenchmarkId::new("hash_ciphertext", size),
            &size,
            |b, size| {
                let input = random_vec(*size);
                b.iter(|| {
                    let hash = hash_ciphertext(&input);
                    black_box(hash);
                })
            },
        );

        g.bench_with_input(
            BenchmarkId::new("apply_aes_128_ctr", size),
            &size,
            |b, size| {
                let mut output = mk_vec(*size);
                let input = random_vec(*size);
                b.iter(|| {
                    apply_aes_128_ctr(Mode::Encrypt, [0; 32], &input, &mut output);
                    black_box(&output);
                })
            },
        );
    }
}

fn bench_routines(c: &mut Criterion) {
    let mut g = c.benchmark_group("Routines");
    g.sample_size(20);

    let sizes = [
        1,
        16,
        32,
        48,
        63,
        64,
        512,
        1 * 1024,
        4 * 1024,
        8 * 1024,
        1 * 16 * 1024,
        2 * 16 * 1024,
        3 * 16 * 1024,
        4 * 16 * 1024,
        8 * 16 * 1024,
        12 * 16 * 1024,
        16 * 16 * 1024, // 256KiB
    ];

    for size in sizes {
        g.throughput(Throughput::Bytes(size as u64));

        g.bench_with_input(BenchmarkId::new("encrypt_block", size), &size, |b, size| {
            let mut output = mk_vec(*size + 64);
            let input = random_vec(*size);
            let sk = SecretKey::random(OsRng);
            let req = RequestInfo::rand(OsRng);
            b.iter(|| {
                encrypt_block(&sk, &req, &input, &mut output);
                black_box(&output);
            })
        });
    }
}

criterion_group!(benches, bench_primitives, bench_routines);
criterion_main!(benches);
