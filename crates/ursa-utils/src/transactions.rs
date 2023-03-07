use anyhow::{bail, Result};
use ethers::{
    abi::{
        token::{LenientTokenizer, Tokenizer},
        AbiParser, Function, ParamType,
    },
    types::{Address, TransactionRequest, U256},
};
use std::str::FromStr;

pub fn build_transaction(
    address: &str,
    function: &str,
    args: &[String],
) -> Result<(Function, TransactionRequest)> {
    let to = match address.parse::<Address>() {
        Ok(addr) => addr,
        Err(_) => {
            bail!("Not a valid address");
        }
    };

    let function_abi = match AbiParser::default().parse_function(function) {
        Ok(func) => func,
        Err(e) => {
            bail!("Unable to parse function: {e:?}");
        }
    };

    let data = match encode_params(&function_abi, args) {
        Ok(params) => params,
        Err(e) => {
            bail!("unable to encode params: {e:?}");
        }
    };

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
                // TODO: Not sure what to do here. Put the no effect in for now, but that is not
                // ideal. We could attempt massage for every value type?
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
