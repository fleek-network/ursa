use std::cmp::min;

use arrayref::array_ref;
use tokio::net::TcpListener;
use ursa_pod::{
    instrument,
    server::{Backend, UfdpHandler},
    types::{Blake3Cid, BlsSignature, Secp256k1PublicKey},
};

const ADDRESS: &str = "0.0.0.0:6969";
// max length a block can be
const BLOCK: &[u8] = &[0; 256 * 1024 * 1024];

#[derive(Clone, Copy)]
struct BenchmarkBackend {}

impl Backend for BenchmarkBackend {
    fn raw_block(&self, cid: &Blake3Cid, block: u64) -> Option<&[u8]> {
        // for benchmarking, we'll determine how much data to send from the hash.
        // The first 8 bytes are the block size, and the next 8 are the content size
        let block_size_bytes = array_ref!(cid.0, 0, 8);
        let block_size = min(u64::from_be_bytes(*block_size_bytes), BLOCK.len() as u64);
        let file_size_bytes = array_ref!(cid.0, 8, 8);
        let file_size = u64::from_be_bytes(*file_size_bytes);

        let block_max = (file_size + block_size - 1) / block_size;

        if block < block_max {
            // return remainder if it's the last block and the size is less than 1 block
            let take = if block + 1 == block_max && file_size != block_size {
                file_size % block_size
            } else {
                block_size
            };

            if take != 0 {
                Some(&BLOCK[0..take as usize])
            } else {
                None
            }
        } else {
            None
        }
    }

    fn decryption_key(&self, _request_id: u64) -> (ursa_pod::types::Secp256k1AffinePoint, u64) {
        let key = [1; 33];
        let key_id = 0;
        (key, key_id)
    }

    fn get_balance(&self, _pubkey: Secp256k1PublicKey) -> u128 {
        9001
    }

    fn save_batch(&self, _batch: BlsSignature) -> Result<(), String> {
        Ok(())
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let listener = TcpListener::bind(ADDRESS).await.unwrap();

    loop {
        let (stream, _) = listener.accept().await.unwrap();

        tokio::spawn(async move {
            let handler = UfdpHandler::new(stream, BenchmarkBackend {});
            instrument!(handler.serve().await.unwrap(), "");
        });
    }
}
