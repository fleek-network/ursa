use anyhow::{bail, Result};
use ethers::{
    abi::{
        token::{LenientTokenizer, Tokenizer},
        Function, ParamType,
    },
    types::U256,
};
use std::str::FromStr;

pub fn encode_params(func: &Function, args: &[impl AsRef<str>]) -> Result<Vec<u8>> {
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
                _ => {}
            }
        }
        tokens.push(token?);
    }

    match func.encode_input(&tokens) {
        Ok(res) => Ok(res),
        Err(e) => bail!("Error encoding the inputs: {e:?}"),
    }
}
