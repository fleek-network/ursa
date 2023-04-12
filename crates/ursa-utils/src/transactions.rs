use anyhow::{ bail, Context as _, Result};
use ethers::{
    abi::{
        token::{LenientTokenizer, Tokenizer}, AbiParser, Function, ParamType,
    },
    types::{Address, TransactionRequest, U256},
};
use std::{ str::FromStr};

/// This will take in strings of address, human readable function abi, and args. And return ethers function abi and filled out transaction request with encoded params
/// example of human readable function abi is "myFunction(string, uint256):(uin256)" parenthesis after : are the return
pub fn build_transaction(
    address: &str,
    function: &str,
    args: &[String],
) -> Result<(Function, TransactionRequest)> {
    let to = address
        .parse::<Address>()
        .with_context(|| "Not a valid Address")?;

    let function_abi = AbiParser::default()
        .parse_function(function)
        .with_context(|| "Unable to parse function")?;

    let data = encode_params(&function_abi, args).with_context(|| "Unable to encode params")?;

    Ok((function_abi, TransactionRequest::new().to(to).data(data)))
}

pub fn encode_params(func: &Function, args: &[impl AsRef<str>]) -> Result<Vec<u8>> {
    if func.inputs.len() != args.len() {
        bail!(
            "Expected {} args, but got {} args",
            func.inputs.len(),
            args.len()
        );
    }

    let params = func
        .inputs
        .iter()
        .zip(args)
        .map(|(input, arg)| (&input.kind, arg.as_ref()))
        .collect::<Vec<_>>();

    let mut tokens = Vec::with_capacity(params.len());

    for (param, value) in params.into_iter() {
        let mut token = LenientTokenizer::tokenize(param, value);

        if token.is_err() && value.starts_with("0x") {
            match param {
                ParamType::FixedBytes(32) => {
                    if value.len() < 66 {
                        let padded_value = [value, &"0".repeat(66 - value.len())].concat();
                        token = LenientTokenizer::tokenize(param, &padded_value);
                    }
                }
                ParamType::Uint(_) => {
                    // try again if value is hex
                    if let Ok(value) = U256::from_str(value).map(|v| v.to_string()) {
                        token = LenientTokenizer::tokenize(param, &value);
                    }
                }
                _ => (),
            }
        }
        tokens.push(token?);
    }

    match func.encode_input(&tokens) {
        Ok(res) => Ok(res),
        Err(e) => bail!("Error encoding the inputs: {e:?}"),
    }
}

// pub fn get_epoch_info_params() -> TransactionRequest {
//     // safe unwrap
//     let to = EPOCH_ADDRESS.parse::<Address>().unwrap();
//     let data = EpochCalls::GetCurrentEpochInfo(GetCurrentEpochInfoCall::default()).encode();
//     TransactionRequest::new().to(to).data(data)
// }

// pub fn decode_epoch_info_return(output: Vec<u8>) -> GetCurrentEpochInfoReturn {
//     GetCurrentEpochInfoReturn::decode(Bytes::from(output)).unwrap()
// }

// pub fn decode_committee(
//     committee_members: Vec<CommitteeMember>,
//     epoch: u64,
// ) -> (Committee, WorkerCache) {
//     let epoch_info = EpochInformation {
//         authorities: committee_members
//             .iter()
//             .filter_map(|authority| {
//                 if let Ok(public_key) = PublicKey::decode_base64(&authority.public_key) {
//                     Some((public_key, authority.clone()))
//                 } else {
//                     None
//                 }
//             })
//             .collect(),
//         epoch,
//     };

//     (Committee::from(&epoch_info), WorkerCache::from(&epoch_info))
// }

// pub fn encode_signal_epoch_call(public_key: String) -> TransactionRequest {
//     let to = EPOCH_ADDRESS.parse::<Address>().unwrap();
//     let data = EpochCalls::SignalEpochChange(SignalEpochChangeCall {
//         committee_member: public_key,
//     })
//     .encode();
//     TransactionRequest::new().to(to).data(data)
// }
// pub struct EpochInformation {
//     authorities: BTreeMap<PublicKey, CommitteeMember>,
//     epoch: u64,
// }

// impl From<&EpochInformation> for Committee {
//     fn from(output: &EpochInformation) -> Self {
//         Committee {
//             epoch: output.epoch,
//             authorities: output
//                 .authorities
//                 .iter()
//                 .filter_map(|(public_key, authority)| {
//                     if let Ok(authority) = Authority::try_from(authority) {
//                         Some((public_key.clone(), authority))
//                     } else {
//                         None
//                     }
//                 })
//                 .collect(),
//         }
//     }
// }

// impl TryFrom<&CommitteeMember> for Authority {
//     type Error = anyhow::Error;
//     fn try_from(member: &CommitteeMember) -> Result<Self> {
//         let network_key = NetworkPublicKey::decode_base64(&member.network_key)
//             .map_err(|_| anyhow!("Failed parsing network Key"))?;
//         Ok(Authority {
//             stake: 1,
//             primary_address: member
//                 .primary_address
//                 .parse()
//                 .map_err(|_| anyhow!("Failed parsing primary address"))?,
//             network_key,
//         })
//     }
// }

// impl From<&EpochInformation> for WorkerCache {
//     fn from(output: &EpochInformation) -> Self {
//         let worker_cache = WorkerCache {
//             epoch: output.epoch,
//             workers: output
//                 .authorities
//                 .iter()
//                 .map(|(key, authority)| {
//                     let mut worker_index = BTreeMap::new();
//                     authority
//                         .workers
//                         .iter()
//                         .filter_map(|worker| {
//                             Some(WorkerInfo {
//                                 name: NetworkPublicKey::decode_base64(&worker.worker_public_key)
//                                     .ok()?,
//                                 transactions: worker.worker_mempool.parse().ok()?,
//                                 worker_address: worker.worker_address.parse().ok()?,
//                             })
//                         })
//                         .enumerate()
//                         .for_each(|(index, worker)| {
//                             //Todo(dalton): Safe unwrap? The idea of the index overflowing u32 seems wild
//                             worker_index.insert(index.try_into().unwrap(), worker);
//                         });
//                     (key.clone(), WorkerIndex(worker_index))
//                 })
//                 .collect(),
//         };
//         worker_cache
//     }
// }

#[cfg(test)]
mod test {
    use super::build_transaction;

    #[test]
    fn test_encode_params() {
        let address = "0xBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB".to_string();
        let function_one = "functionOne(string, uint256, bool[]):(uint256)".to_string();
        let function_two = "functionTwo(uint256[]):(string)".to_string();
        let function_three = "functionThree(address[]):(address)".to_string();

        let function_one_args = Vec::from([
            "this is a string".to_string(),
            "1".to_string(),
            "[true,false,true]".to_string(),
        ]);
        let function_two_args = Vec::from(["[1,2,3,4,5,6,7,8,9,0]".to_string()]);
        let function_three_args = Vec::from(["[0xBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB,0xAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAB,0xCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC]".to_string()]);

        let _ = build_transaction(&address, &function_one, &function_one_args).unwrap();
        let _ = build_transaction(&address, &function_two, &function_two_args).unwrap();
        let _ = build_transaction(&address, &function_three, &function_three_args).unwrap();
    }
}
