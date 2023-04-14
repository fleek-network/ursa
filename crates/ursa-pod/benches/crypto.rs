#![feature(core_intrinsics)]

use benchmarks_utils::*;
use criterion::*;
use rand::Rng;
use rand_core::OsRng;
use ursa_pod::{crypto::*, keys::SecretKey};

fn bench_blake3(c: &mut Criterion) {
    let mut g = c.benchmark_group("Blake3");
    g.sample_size(30);

    let mut sizes = Vec::new();
    sizes.extend_from_slice(&[1, 16, 32, 48, 63, 64, 512, 1024]);
    sizes.extend((8..256).step_by(8).map(|i| i * 1024));

    for size in sizes {
        g.throughput(Throughput::Bytes(size as u64));

        g.bench_with_input(BenchmarkId::new("blake3::hash", size), &size, |b, size| {
            let input = random_vec(*size);
            b.iter(|| {
                let hash = blake3::hash(&input);
                black_box(hash);
            })
        });

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
            BenchmarkId::new("blake3::update_rayon", size),
            &size,
            |b, size| {
                let input = random_vec(*size);
                b.iter(|| {
                    let hash = blake3::Hasher::new().update(&input).finalize();
                    black_box(hash);
                })
            },
        );
    }
}

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

        g.bench_with_input(
            BenchmarkId::new("encrypt_per_session_ec", size),
            &size,
            |b, size| {
                let mut output = mk_vec(*size + 64);
                let input = random_vec(*size);
                let sk = SecretKey::random(OsRng);
                let req = RequestInfo::rand(OsRng);
                let session_secret_key_hash: [u8; 32] = OsRng.gen();

                b.iter(|| {
                    let request_info_hash = req.hash();

                    let nonce: [u8; 32] = rand::thread_rng().gen();
                    let symmetric_key = {
                        let mut buffer = arrayvec::ArrayVec::<u8, 64>::new();
                        buffer
                            .try_extend_from_slice(&session_secret_key_hash)
                            .unwrap();
                        buffer.try_extend_from_slice(&nonce).unwrap();
                        *blake3::hash(&buffer).as_bytes()
                    };

                    apply_aes_128_ctr(
                        Mode::Encrypt,
                        symmetric_key,
                        &input,
                        &mut output[0..input.len()],
                    );

                    let ciphertext_hash = hash_ciphertext(&output[..input.len()]);
                    let commitment = sign_ciphertext(&sk, &ciphertext_hash, &request_info_hash);
                    output[input.len()..].copy_from_slice(&commitment);

                    nonce
                })
            },
        );
    }
}

criterion_group!(benches, bench_blake3, bench_primitives, bench_routines);
criterion_main!(benches);
