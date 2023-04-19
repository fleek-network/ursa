#![feature(core_intrinsics)]

use benchmarks_utils::*;
use criterion::*;
use rand_core::OsRng;
use ursa_pod::crypto::ed25519::{self, Ed25519Engine};
use ursa_pod::crypto::key::SecretKey;
use ursa_pod::crypto::request::RequestInfo;

fn sizes() -> Vec<usize> {
    let mut sizes = Vec::new();
    sizes.extend_from_slice(&[1, 16, 32, 48, 63, 64, 512, 1024]);
    sizes.extend((8..=256).step_by(8).map(|i| i * 1024));
    sizes
}

fn bench_blake3(c: &mut Criterion) {
    let mut g = c.benchmark_group("Blake3");
    g.sample_size(30);

    for size in sizes() {
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
                    let hash = blake3::Hasher::new().update_rayon(&input).finalize();
                    black_box(hash);
                })
            },
        );
    }
}

fn bench_blake3_rayon_noise(c: &mut Criterion) {
    let noises = [
        -128, 0, 128, 256, 512, 1024, 1536, 2048, 4095, 8192, 10240, 12288, 13312,
    ];

    for base in [128 * 1024, 256 * 1024] {
        let mut g = c.benchmark_group(format!("Blake3RayonNoise[base={}KiB]", base / 1024));
        g.sample_size(20);

        for noise in noises {
            let size = (base + noise) as usize;
            g.throughput(Throughput::Bytes(size as u64));

            g.bench_with_input(BenchmarkId::new("blake3::hash", noise), &size, |b, size| {
                let input = random_vec(*size);
                b.iter(|| {
                    let hash = blake3::hash(&input);
                    black_box(hash);
                })
            });

            g.bench_with_input(
                BenchmarkId::new("blake3::update", noise),
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
                BenchmarkId::new("blake3::update_rayon", noise),
                &size,
                |b, size| {
                    let input = random_vec(*size);
                    b.iter(|| {
                        let hash = blake3::Hasher::new().update_rayon(&input).finalize();
                        black_box(hash);
                    })
                },
            );
        }

        g.finish();
    }
}

fn bench_primitives(c: &mut Criterion) {
    let mut g = c.benchmark_group("Primitives");
    g.sample_size(50);

    g.bench_function("hash_request_info", |b| {
        let req = RequestInfo::rand(OsRng);
        b.iter(|| {
            black_box(req.hash());
        })
    });

    g.bench_function("hash_to_curve", |b| {
        b.iter(|| {
            // black_box(hash_to_curve(&[0; 32]));
        })
    });

    g.bench_function("generate_symmetric_key", |b| {
        let sk = ed25519::libsodium_impl::Ed25519SecretKey::generate().unwrap();
        let hash = RequestInfo::rand(OsRng).hash();
        b.iter(|| {
            let ret = ed25519::libsodium_impl::Ed25519::generate_symmetric_key(&sk, &hash).unwrap();
            black_box(ret);
        })
    });

    g.bench_function("sign_ciphertext", |b| {
        // let sk = SecretKey::random(OsRng);
        b.iter(|| {
            // let ret = sign_ciphertext(&sk, &[0; 32], &[0; 32]);
            // black_box(ret);
        })
    });

    g.finish();

    let mut g = c.benchmark_group("Primitive Stream");
    g.sample_size(20);

    for size in sizes() {
        g.throughput(Throughput::Bytes(size as u64));

        g.bench_with_input(
            BenchmarkId::new("hash_ciphertext", size),
            &size,
            |b, size| {
                let input = random_vec(*size);
                b.iter(|| {
                    // let hash = hash_ciphertext(&input);
                    // black_box(hash);
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
                    // apply_aes_128_ctr(Mode::Encrypt, [0; 32], &input, &mut output);
                    black_box(&output);
                })
            },
        );
    }
}

fn bench_routines(c: &mut Criterion) {
    let mut g = c.benchmark_group("Routines");
    g.sample_size(20);

    for size in sizes() {
        g.throughput(Throughput::Bytes(size as u64));

        g.bench_with_input(BenchmarkId::new("encrypt_block", size), &size, |b, size| {
            let mut output = mk_vec(*size + 64);
            let input = random_vec(*size);
            // let sk = SecretKey::random(OsRng);
            // let req = RequestInfo::rand(OsRng);
            b.iter(|| {
                // encrypt_block(&sk, &req, &input, &mut output);
                black_box(&output);
            })
        });
    }
}

criterion_group!(
    benches,
    bench_blake3,
    bench_blake3_rayon_noise,
    bench_primitives,
    bench_routines,
);
criterion_main!(benches);
