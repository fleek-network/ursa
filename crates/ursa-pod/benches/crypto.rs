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

    g.bench_function("generate_encryption_key", |b| {
        let sk = SecretKey::random(OsRng);
        b.iter(|| {
            let ret = generate_encryption_key(&sk, &[0; 32]);
            black_box(ret);
        })
    });

    g.bench_function("sign_response", |b| {
        let sk = SecretKey::random(OsRng);
        b.iter(|| {
            let ret = sign_response(&sk, &[0; 32], &[0; 32]);
            black_box(ret);
        })
    });

    g.sample_size(20);

    for size in sizes {
        g.throughput(Throughput::Bytes(size as u64));

        g.bench_with_input(
            BenchmarkId::new("blake3::update_rayon", size),
            &size,
            |b, size| {
                let input = random_vec(*size);
                b.iter(|| {
                    let hash = blake3::Hasher::new().update_rayon(&input).finalize();
                    black_box(hash);
                })
            },
        );

        g.bench_with_input(
            BenchmarkId::new("blake3::update", size),
            &size,
            |b, size| {
                let input = random_vec(*size);
                b.iter(|| {
                    let hash = blake3::Hasher::new().update(&input).finalize();
                    black_box(hash);
                })
            },
        );

        g.bench_with_input(
            BenchmarkId::new("apply_cipher_in_place", size),
            &size,
            |b, size| {
                let mut result = mk_vec(*size);
                b.iter(|| {
                    apply_cipher_in_place([0; 32], 0, &mut result);
                    black_box(&result);
                })
            },
        );
    }
}

criterion_group!(benches, bench_primitives);
criterion_main!(benches);
