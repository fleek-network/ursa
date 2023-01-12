use crate::node::Node;
use cid::multihash::Code;
use ipld_traversal::blockstore::Blockstore;
use libipld::{cbor::DagCborCodec, ipld, Block, DefaultParams, Ipld};
use testground::client::Client;
use tokio::sync::oneshot;
use ursa_network::NetworkCommand;

fn get_block(content: &[u8]) -> Block<DefaultParams> {
    create_block(ipld!(content))
}

fn create_block(ipld: Ipld) -> Block<DefaultParams> {
    Block::encode(DagCborCodec, Code::Blake3_256, &ipld).unwrap()
}

pub async fn test_cache_request(_client: &mut Client, _node: Node) -> Result<(), String> {
    // let block = get_block(&b"hello world"[..]);
    // node.store.insert(&block).unwrap();
    // assert!(node.store.has(block.cid()).unwrap());
    //
    // let (sender, _) = oneshot::channel();
    // let _request = NetworkCommand::Put {
    //     cid: *block.cid(),
    //     sender,
    // };

    Ok(())
}
