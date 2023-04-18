use std::{
    env,
    process::exit,
    str::FromStr,
    time::{Duration, Instant},
};

use futures::{future::Either, stream::FuturesUnordered, StreamExt};
use tokio::net::TcpStream;
use ursa_pod::{client::UfdpClient, connection::UrsaCodecError, types::Blake3Cid};

const PUB_KEY: [u8; 48] = [2u8; 48];

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), UrsaCodecError> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 6 {
        help(&args[0]);
        exit(1)
    }
    let address = &args[1];
    let seconds = u64::from_str(&args[2]).expect("parse number of seconds to run new requests for");
    let duration = Duration::from_secs(seconds);
    let workers = u64::from_str(&args[3]).expect("parse number of concurrent requests");
    let block_size = u64::from_str(&args[4]).expect("parse block size");
    let file_size = u64::from_str(&args[5]).expect("parse file size");

    let mut cid = [0u8; 32];
    cid[0..8].copy_from_slice(&block_size.to_be_bytes());
    cid[8..16].copy_from_slice(&file_size.to_be_bytes());
    let cid = Blake3Cid(cid);

    println!("warming up for ~3 seconds with {workers} workers");
    let (elapsed, total) = run(cid, address, workers, Either::Right(Duration::from_secs(3))).await;
    let avg = elapsed.as_secs_f64() / total as f64;
    let estimated = (duration.as_secs_f64() / avg) as u64;

    println!("Running {estimated} requests with {workers} workers for a target {duration:?}");
    let (elapsed, total) = run(cid, address, workers, Either::Left(estimated)).await;
    println!("\u{001b}[1mTotal\u{001b}[0m: {elapsed:?}");
    let secs = elapsed.as_secs_f64();
    let avg = Duration::from_secs_f64(secs / total as f64);
    println!("\u{001b}[1mAverage\u{001b}[0m: {avg:?}");
    let thrpt = (file_size * total) as f64 / (1024.0 * 1024.0) / secs;
    println!("\u{001b}[1mThroughput\u{001b}[0m: {thrpt} MiB/sec");

    Ok(())
}

async fn run(
    cid: Blake3Cid,
    addr: &str,
    workers: u64,
    param: Either<u64, Duration>,
) -> (Duration, u64) {
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

async fn request(cid: Blake3Cid, addr: &str) -> usize {
    let stream = TcpStream::connect(addr).await.unwrap();
    let mut client = UfdpClient::new(stream, PUB_KEY, None).await.unwrap();
    client.request(cid).await.unwrap()
}

fn help(bin: &str) {
    println!("USAGE: {bin} <server ip> <duration> <concurrent requests> <file size> <block size>");
}
