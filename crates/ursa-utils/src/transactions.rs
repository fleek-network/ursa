use crate::contract_bindings::epoch_bindings::{
    CommitteeMember, EpochManagerCalls, GetCurrentEpochInfoCall, GetCurrentEpochInfoReturn,
    SignalEpochChangeCall,
};
use anyhow::{anyhow, bail, Context as _, Result};
use ethers::{
    abi::{
        token::{LenientTokenizer, Tokenizer},
        AbiDecode, AbiEncode, AbiParser, Function, ParamType,
    },
    types::{Address, Bytes, TransactionRequest, U256},
};
use fastcrypto::traits::EncodeDecodeBase64;
use narwhal_config::{Authority, Committee, WorkerCache, WorkerIndex, WorkerInfo};
use narwhal_crypto::{NetworkPublicKey, PublicKey};
use std::{collections::BTreeMap, str::FromStr};

pub const REGISTRY_ADDRESS: &str = "0xCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC";
pub const EPOCH_ADDRESS: &str = "0xAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAC";

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
