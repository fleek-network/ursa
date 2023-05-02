use std::{
    collections::HashMap,
    env,
    process::exit,
    str::FromStr,
    time::{Duration, Instant},
};

use futures::{future::Either, stream::FuturesUnordered, StreamExt};
use tokio::net::TcpStream;
use ursa_pod::{blake3::Hash, client::UfdpClient, connection::UrsaCodecError};

const PUB_KEY: [u8; 48] = [2u8; 48];
const SIZES: [(&str, u64); 17] = [
    // MB
    (
        "488de202f73bd976de4e7048f4e1f39a776d86d582b7348ff53bf432b987fca8",
        1024 * 1024,
    ),
    (
        "8ac83f8ce09d064b023ab3c15880b02f2686cd1817fd25915b8153316ee059f8",
        2 * 1024 * 1024,
    ),
    (
        "04e52cd2da6a0e1f338b0078369130d96585c1de65057da5dd1283b12fb853e1",
        4 * 1024 * 1024,
    ),
    (
        "27ddc0a1d824fa6befe0596ddc0136fc4a1dc060d526800851f87498768b755c",
        8 * 1024 * 1024,
    ),
    (
        "b4834959bc889fed1abf3c45d5da0e384134386a4b2786cc5dbb9fe8fa853bbb",
        16 * 1024 * 1024,
    ),
    (
        "2dbe083a48e7430772c9803d474523a53b7c8aad5c7daaba0a55b6f892a98074",
        32 * 1024 * 1024,
    ),
    (
        "ea7b156fc9a810c181984f9e2da433feeeb2bf88ffa4d1f0dc1a92154b5bdc8b",
        64 * 1024 * 1024,
    ),
    (
        "e66d34ca5a36dfa692903a66d66fd1d9c87bd553c32fbf7d3960542f4cda3257",
        128 * 1024 * 1024,
    ),
    (
        "9216a60cba88b32b18349b83c57c22d2e3b514a9720916952e214e5fc065c538",
        256 * 1024 * 1024,
    ),
    (
        "34f2f34bcc048af98242e010b4a661348276a784d9f9f99fcff70bf94fe8b9ba",
        512 * 1024 * 1024,
    ),
    // GB
    (
        "94b4ec39d8d42ebda685fbb5429e8ab0086e65245e750142c1eea36a26abc24d",
        1024 * 1024 * 1024,
    ),
    (
        "cbd71ef31685ea2c6ce0c146ef1d160b4d458f29cea2a61536a8a65f195fdb82",
        2 * 1024 * 1024 * 1024,
    ),
    (
        "7dde7c9fed144013fedbe2b0bbf2d82f004b60b589485851cdec29b27be408d7",
        4 * 1024 * 1024 * 1024,
    ),
    (
        "875283713208b0d6be59b2c6862b0a3cfdd8ebe5366b815e34dfffd98554ef26",
        8 * 1024 * 1024 * 1024,
    ),
    (
        "7953bbc0a374e96ea2caa5328ebd96f3545b41c882f509e92ae850d000f3cde3",
        16 * 1024 * 1024 * 1024,
    ),
    (
        "92705fed65135cde7f3c9901c801e062f8483bdd6dd88f167858cbf47644eab1",
        32 * 1024 * 1024 * 1024,
    ),
    (
        "20e6c9ca2d7ed61951ea515e5b82bc478fbe6b7edb051272360c3b7494a3dd27",
        64 * 1024 * 1024 * 1024,
    ),
];

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), UrsaCodecError> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        help(&args[0]);
        exit(1)
    }
    let address = &args[1];
    let seconds = u64::from_str(&args[2]).expect("parse number of seconds to run new requests for");
    let duration = Duration::from_secs(seconds);
    let workers = u64::from_str(&args[3]).expect("parse number of concurrent requests");
    let file_size = u64::from_str(&args[4]).expect("parse file size");

    let hashes: HashMap<u64, Hash> = SIZES
        .iter()
        .map(|(h, s)| (*s, Hash::from_hex(h).unwrap()))
        .collect();

    let cid = *hashes
        .get(&file_size)
        .expect("Hash not found for content size");

    println!("Warming up for ~3 seconds with {workers} workers");
    let (elapsed, total) = run(cid, address, workers, Either::Right(Duration::from_secs(3))).await;
    let avg = elapsed.as_secs_f64() / total as f64;
    let estimated = (duration.as_secs_f64() / avg) as u64;

    println!("Running {estimated} requests with {workers} workers for a target {duration:?}\n");
    let (elapsed, total) = run(cid, address, workers, Either::Left(estimated)).await;
    println!("\u{001b}[1mTotal\u{001b}[0m: {elapsed:?}");
    let secs = elapsed.as_secs_f64();
    let avg = Duration::from_secs_f64(secs / total as f64);
    println!("\u{001b}[1mAverage\u{001b}[0m: {avg:?}");
    let thrpt = (file_size * total) as f64 / (1024.0 * 1024.0) / secs;
    println!("\u{001b}[1mThroughput\u{001b}[0m: {thrpt} MiB/sec");

    Ok(())
}

async fn run(cid: Hash, addr: &str, workers: u64, param: Either<u64, Duration>) -> (Duration, u64) {
    let mut futures = FuturesUnordered::new();
    let mut total = workers;

    for _ in 0..workers {
        futures.push(request(cid, addr))
    }

    let instant = Instant::now();
    match param {
        Either::Left(max_req) => {
            while let Some(_size) = futures.next().await {
                if total < max_req {
                    futures.push(request(cid, addr));
                    total += 1;
                }
            }
        }
        Either::Right(duration) => {
            while let Some(_size) = futures.next().await {
                if instant.elapsed() < duration {
                    futures.push(request(cid, addr));
                    total += 1;
                }
            }
        }
    }
    (instant.elapsed(), total)
}

async fn request(cid: Hash, addr: &str) -> usize {
    let stream = TcpStream::connect(addr).await.unwrap();
    let mut client = UfdpClient::new(stream, PUB_KEY, None).await.unwrap();
    client.request(cid).await.unwrap()
}

fn help(bin: &str) {
    println!("USAGE: {bin} <server ip> <duration> <workers> <file size>");
}
