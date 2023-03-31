#![feature(core_intrinsics)]

use benchmarks_utils::*;
use criterion::*;
use rand_core::OsRng;
use ursa_pod::keys::SecretKey;
use ursa_pod::primitives::{encrypt_block, RequestInfo};

fn bench_encrypt(c: &mut Criterion) {
    const SIZE: usize = 256 * KB;

    let mut g = c.benchmark_group("Encrypt");
    g.sample_size(20);
    g.throughput(Throughput::Bytes(SIZE as u64));

    let data = random_vec(SIZE);
    let s_key = SecretKey::random(OsRng);
    let req = RequestInfo::rand(OsRng);

    g.bench_function("encrypt", |b| {
        let mut result = mk_vec(SIZE);
        b.iter(|| {
            encrypt_block(&s_key, &req, &data, &mut result);
            black_box(&result);
        })
    });

    g.finish();
}

criterion_group!(benches, bench_encrypt);
criterion_main!(benches);
