use std::{env, process::exit, str::FromStr};

use tokio::net::TcpStream;
use tracing::info;
use ursa_pod::{client::UfdpClient, connection::UrsaCodecError};

const PUB_KEY: [u8; 48] = [2u8; 48];

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), UrsaCodecError> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        help(&args[0]);
        exit(1)
    }
    let address = &args[1];
    let requests = u64::from_str(&args[2]).expect("parse num requests");
    let file_size = u64::from_str(&args[3]).expect("parse file size");
    let block_size = u64::from_str(&args[4]).expect("parse block size");

    let mut cid = [0u8; 32];
    cid[0..8].copy_from_slice(&block_size.to_be_bytes());
    cid[8..16].copy_from_slice(&file_size.to_be_bytes());

    let mut handles = vec![];

    for _ in 0..requests {
        let address = address.clone();
        handles.push(tokio::spawn(async move {
            let time = std::time::Instant::now();
            let stream = TcpStream::connect(address).await.unwrap();
            let mut client = UfdpClient::new(stream, PUB_KEY, None).await.unwrap();
            let size = client
                .request(ursa_pod::types::Blake3Cid(cid))
                .await
                .unwrap();
            assert_eq!(file_size as usize, size);
            let elapsed = time.elapsed().as_nanos();
            info!("request_completed,{elapsed},{block_size},{file_size}");
        }));
    }

    futures::future::join_all(handles).await;

    Ok(())
}

fn help(bin: &String) {
    println!("USAGE: {bin} <server ip> <concurrent requests> <file size> <block size>");
}
