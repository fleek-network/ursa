
#![allow(clippy::enum_variant_names)]
#![allow(dead_code)]
#![allow(clippy::type_complexity)]
#![allow(unused_imports)]
use ethers::contract::{
    builders::{ContractCall, Event},
    Contract, Lazy,
};
use ethers::core::{
    abi::{Abi, Detokenize, InvalidOutputType, Token, Tokenizable},
    types::*,
};
use ethers::providers::Middleware;
#[doc = "RewardManager was auto-generated with ethers-rs Abigen. More information at: https://github.com/gakonst/ethers-rs"]
use std::sync::Arc;
# [rustfmt :: skip] const __ABI : & str = "[{\"inputs\":[{\"internalType\":\"uint256\",\"name\":\"_epoch\",\"type\":\"uint256\",\"components\":[]}],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"getScores\",\"outputs\":[{\"internalType\":\"struct ReputationScores.EpochScores[]\",\"name\":\"\",\"type\":\"tuple[]\",\"components\":[{\"internalType\":\"string\",\"name\":\"peerId\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"struct ReputationScores.Measurement[]\",\"name\":\"measurements\",\"type\":\"tuple[]\",\"components\":[{\"internalType\":\"string\",\"name\":\"peerId\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"uint64\",\"name\":\"bandwidth\",\"type\":\"uint64\",\"components\":[]},{\"internalType\":\"uint32\",\"name\":\"latency\",\"type\":\"uint32\",\"components\":[]},{\"internalType\":\"uint128\",\"name\":\"uptime\",\"type\":\"uint128\",\"components\":[]}]}]}]},{\"inputs\":[{\"internalType\":\"uint256\",\"name\":\"_epoch\",\"type\":\"uint256\",\"components\":[]},{\"internalType\":\"struct ReputationScores.EpochScores\",\"name\":\"_scores\",\"type\":\"tuple\",\"components\":[{\"internalType\":\"string\",\"name\":\"peerId\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"struct ReputationScores.Measurement[]\",\"name\":\"measurements\",\"type\":\"tuple[]\",\"components\":[{\"internalType\":\"string\",\"name\":\"peerId\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"uint64\",\"name\":\"bandwidth\",\"type\":\"uint64\",\"components\":[]},{\"internalType\":\"uint32\",\"name\":\"latency\",\"type\":\"uint32\",\"components\":[]},{\"internalType\":\"uint128\",\"name\":\"uptime\",\"type\":\"uint128\",\"components\":[]}]}]}],\"stateMutability\":\"nonpayable\",\"type\":\"function\",\"name\":\"submitScores\",\"outputs\":[]}]" ;
#[doc = r" The parsed JSON-ABI of the contract."]
pub static REWARDMANAGER_ABI: ethers::contract::Lazy<ethers::core::abi::Abi> =
    ethers::contract::Lazy::new(|| {
        ethers::core::utils::__serde_json::from_str(__ABI).expect("invalid abi")
    });
#[doc = r" Bytecode of the #name contract"]
pub static REWARDMANAGER_BYTECODE: ethers::contract::Lazy<ethers::core::types::Bytes> =
    ethers::contract::Lazy::new(|| {
        "0x608060405234801561001057600080fd5b50610d3e806100206000396000f3fe608060405234801561001057600080fd5b50600436106100365760003560e01c806304527d901461003b5780638b19868314610064575b600080fd5b61004e610049366004610738565b610079565b60405161005b91906107a1565b60405180910390f35b6100776100723660046109cd565b61041e565b005b6000818152602081815260408083208151808301835260048152633078303160e01b938101939093529051606093926100b191610b66565b908152602001604051809103902060010180546100cd90610b82565b80601f01602080910402602001604051908101604052809291908181526020018280546100f990610b82565b80156101465780601f1061011b57610100808354040283529160200191610146565b820191906000526020600020905b81548152906001019060200180831161012957829003601f168201915b5050506000868152600160205260408120549394509291505067ffffffffffffffff811115610177576101776108ae565b6040519080825280602002602001820160405280156101bc57816020015b60408051808201909152606080825260208201528152602001906001900390816101955790505b50905060005b825115610416576040518060400160405280848152602001600080888152602001908152602001600020856040516101fa9190610b66565b908152604080519182900360209081018320805480830285018301909352828452919060009084015b8282101561032b578382906000526020600020906002020160405180608001604052908160008201805461025690610b82565b80601f016020809104026020016040519081016040528092919081815260200182805461028290610b82565b80156102cf5780601f106102a4576101008083540402835291602001916102cf565b820191906000526020600020905b8154815290600101906020018083116102b257829003601f168201915b505050918352505060019182015467ffffffffffffffff8116602080840191909152600160401b820463ffffffff166040840152600160601b9091046001600160801b0316606090920191909152918352929092019101610223565b5050505081525082828151811061034457610344610bbc565b60200260200101819052506000808681526020019081526020016000208360405161036f9190610b66565b9081526020016040518091039020600101805461038b90610b82565b80601f01602080910402602001604051908101604052809291908181526020018280546103b790610b82565b80156104045780601f106103d957610100808354040283529160200191610404565b820191906000526020600020905b8154815290600101906020018083116103e757829003601f168201915b505050505092506001810190506101c2565b509392505050565b6000828152602081905260409081902082519151909161043d91610b66565b9081526020016040518091039020600101805461045990610b82565b1590506104c55760405162461bcd60e51b815260206004820152603060248201527f54686973206e6f646520616c7265616479207375626d697465642073636f726560448201526f0e640ccdee440e8d0d2e640cae0dec6d60831b606482015260840160405180910390fd5b6000828152602081815260408083208151808301835260048152633078303160e01b93810193909352905190916104fb91610b66565b9081526020016040518091039020600101805461051790610b82565b80601f016020809104026020016040519081016040528092919081815260200182805461054390610b82565b80156105905780601f1061056557610100808354040283529160200191610590565b820191906000526020600020905b81548152906001019060200180831161057357829003601f168201915b50505050509050600080600085815260200190815260200160002083600001516040516105bd9190610b66565b9081526040519081900360200190209050600181016105dc8382610c21565b5060005b8360200151518110156106b357816000018460200151828151811061060757610607610bbc565b602090810291909101810151825460018101845560009384529190922082516002909202019081906106399082610c21565b5060208201516001918201805460408501516060909501516001600160801b0316600160601b026fffffffffffffffffffffffffffffffff60601b1963ffffffff909616600160401b026bffffffffffffffffffffffff1990921667ffffffffffffffff90941693909317179390931617909155016105e0565b50600180600086815260200190815260200160002060008282546106d79190610ce1565b90915550508251600085815260208181526040918290208251808401845260048152633078303160e01b9281019290925291516107149190610b66565b908152602001604051809103902060010190816107319190610c21565b5050505050565b60006020828403121561074a57600080fd5b5035919050565b60005b8381101561076c578181015183820152602001610754565b50506000910152565b6000815180845261078d816020860160208601610751565b601f01601f19169290920160200192915050565b60006020808301818452808551808352604092508286019150828160051b87010184880160005b838110156108a057888303603f19018552815180518785526107ec88860182610775565b91890151858303868b01528051808452908a0192915089820190600581901b83018b0160005b8281101561088957601f1985830301845285516080815181855261083882860182610775565b91505067ffffffffffffffff8f830151168f85015263ffffffff8e830151168e85015260606001600160801b038184015116818601525080935050508c860195508c84019350600181019050610812565b50988b0198965050509288019250506001016107c8565b509098975050505050505050565b634e487b7160e01b600052604160045260246000fd5b6040805190810167ffffffffffffffff811182821017156108e7576108e76108ae565b60405290565b6040516080810167ffffffffffffffff811182821017156108e7576108e76108ae565b604051601f8201601f1916810167ffffffffffffffff81118282101715610939576109396108ae565b604052919050565b600082601f83011261095257600080fd5b813567ffffffffffffffff81111561096c5761096c6108ae565b61097f601f8201601f1916602001610910565b81815284602083860101111561099457600080fd5b816020850160208301376000918101602001919091529392505050565b80356001600160801b03811681146109c857600080fd5b919050565b600080604083850312156109e057600080fd5b8235915060208084013567ffffffffffffffff80821115610a0057600080fd5b9085019060408288031215610a1457600080fd5b610a1c6108c4565b823582811115610a2b57600080fd5b610a3789828601610941565b8252508383013582811115610a4b57600080fd5b80840193505087601f840112610a6057600080fd5b823582811115610a7257610a726108ae565b8060051b610a81868201610910565b918252848101860191868101908b841115610a9b57600080fd5b87870192505b83831015610b5057823586811115610ab857600080fd5b87016080818e03601f19011215610acf5760008081fd5b610ad76108ed565b8982013588811115610ae95760008081fd5b610af78f8c83860101610941565b82525060408201358881168114610b0e5760008081fd5b818b015260608281013563ffffffff81168114610b2b5760008081fd5b6040830152610b3c608084016109b1565b908201528352509187019190870190610aa1565b9684019690965250959890975095505050505050565b60008251610b78818460208701610751565b9190910192915050565b600181811c90821680610b9657607f821691505b602082108103610bb657634e487b7160e01b600052602260045260246000fd5b50919050565b634e487b7160e01b600052603260045260246000fd5b601f821115610c1c57600081815260208120601f850160051c81016020861015610bf95750805b601f850160051c820191505b81811015610c1857828155600101610c05565b5050505b505050565b815167ffffffffffffffff811115610c3b57610c3b6108ae565b610c4f81610c498454610b82565b84610bd2565b602080601f831160018114610c845760008415610c6c5750858301515b600019600386901b1c1916600185901b178555610c18565b600085815260208120601f198616915b82811015610cb357888601518255948401946001909101908401610c94565b5085821015610cd15787850151600019600388901b60f8161c191681555b5050505050600190811b01905550565b80820180821115610d0257634e487b7160e01b600052601160045260246000fd5b9291505056fea26469706673582212202fff56abdde8ec3df7c690c047fbc95853b0851ce3fcd204ed28b55d92b189b164736f6c63430008110033" . parse () . expect ("invalid bytecode")
    });
pub struct RewardManager<M>(ethers::contract::Contract<M>);
impl<M> Clone for RewardManager<M> {
    fn clone(&self) -> Self {
        RewardManager(self.0.clone())
    }
}
impl<M> std::ops::Deref for RewardManager<M> {
    type Target = ethers::contract::Contract<M>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<M> std::fmt::Debug for RewardManager<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_tuple(stringify!(RewardManager))
            .field(&self.address())
            .finish()
    }
}
impl<M: ethers::providers::Middleware> RewardManager<M> {
    #[doc = r" Creates a new contract instance with the specified `ethers`"]
    #[doc = r" client at the given `Address`. The contract derefs to a `ethers::Contract`"]
    #[doc = r" object"]
    pub fn new<T: Into<ethers::core::types::Address>>(
        address: T,
        client: ::std::sync::Arc<M>,
    ) -> Self {
        ethers::contract::Contract::new(address.into(), REWARDMANAGER_ABI.clone(), client).into()
    }
    #[doc = r" Constructs the general purpose `Deployer` instance based on the provided constructor arguments and sends it."]
    #[doc = r" Returns a new instance of a deployer that returns an instance of this contract after sending the transaction"]
    #[doc = r""]
    #[doc = r" Notes:"]
    #[doc = r" 1. If there are no constructor arguments, you should pass `()` as the argument."]
    #[doc = r" 1. The default poll duration is 7 seconds."]
    #[doc = r" 1. The default number of confirmations is 1 block."]
    #[doc = r""]
    #[doc = r""]
    #[doc = r" # Example"]
    #[doc = r""]
    #[doc = r" Generate contract bindings with `abigen!` and deploy a new contract instance."]
    #[doc = r""]
    #[doc = r" *Note*: this requires a `bytecode` and `abi` object in the `greeter.json` artifact."]
    #[doc = r""]
    #[doc = r" ```ignore"]
    #[doc = r" # async fn deploy<M: ethers::providers::Middleware>(client: ::std::sync::Arc<M>) {"]
    #[doc = r#"     abigen!(Greeter,"../greeter.json");"#]
    #[doc = r""]
    #[doc = r#"    let greeter_contract = Greeter::deploy(client, "Hello world!".to_string()).unwrap().send().await.unwrap();"#]
    #[doc = r"    let msg = greeter_contract.greet().call().await.unwrap();"]
    #[doc = r" # }"]
    #[doc = r" ```"]
    pub fn deploy<T: ethers::core::abi::Tokenize>(
        client: ::std::sync::Arc<M>,
        constructor_args: T,
    ) -> ::std::result::Result<
        ethers::contract::builders::ContractDeployer<M, Self>,
        ethers::contract::ContractError<M>,
    > {
        let factory = ethers::contract::ContractFactory::new(
            REWARDMANAGER_ABI.clone(),
            REWARDMANAGER_BYTECODE.clone().into(),
            client,
        );
        let deployer = factory.deploy(constructor_args)?;
        let deployer = ethers::contract::ContractDeployer::new(deployer);
        Ok(deployer)
    }
    #[doc = "Calls the contract's `getScores` (0x04527d90) function"]
    pub fn get_scores(
        &self,
        epoch: ethers::core::types::U256,
    ) -> ethers::contract::builders::ContractCall<M, ::std::vec::Vec<EpochScores>> {
        self.0
            .method_hash([4, 82, 125, 144], epoch)
            .expect("method not found (this should never happen)")
    }
    #[doc = "Calls the contract's `submitScores` (0x8b198683) function"]
    pub fn submit_scores(
        &self,
        epoch: ethers::core::types::U256,
        scores: EpochScores,
    ) -> ethers::contract::builders::ContractCall<M, ()> {
        self.0
            .method_hash([139, 25, 134, 131], (epoch, scores))
            .expect("method not found (this should never happen)")
    }
}
impl<M: ethers::providers::Middleware> From<ethers::contract::Contract<M>> for RewardManager<M> {
    fn from(contract: ethers::contract::Contract<M>) -> Self {
        Self(contract)
    }
}
#[doc = "Container type for all input parameters for the `getScores` function with signature `getScores(uint256)` and selector `[4, 82, 125, 144]`"]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    ethers :: contract :: EthCall,
    ethers :: contract :: EthDisplay,
    Default,
)]
#[ethcall(name = "getScores", abi = "getScores(uint256)")]
pub struct GetScoresCall {
    pub epoch: ethers::core::types::U256,
}
#[doc = "Container type for all input parameters for the `submitScores` function with signature `submitScores(uint256,(string,(string,uint64,uint32,uint128)[]))` and selector `[139, 25, 134, 131]`"]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    ethers :: contract :: EthCall,
    ethers :: contract :: EthDisplay,
    Default,
)]
#[ethcall(
    name = "submitScores",
    abi = "submitScores(uint256,(string,(string,uint64,uint32,uint128)[]))"
)]
pub struct SubmitScoresCall {
    pub epoch: ethers::core::types::U256,
    pub scores: EpochScores,
}
#[derive(Debug, Clone, PartialEq, Eq, ethers :: contract :: EthAbiType)]
pub enum RewardManagerCalls {
    GetScores(GetScoresCall),
    SubmitScores(SubmitScoresCall),
}
impl ethers::core::abi::AbiDecode for RewardManagerCalls {
    fn decode(data: impl AsRef<[u8]>) -> ::std::result::Result<Self, ethers::core::abi::AbiError> {
        if let Ok(decoded) = <GetScoresCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
        {
            return Ok(RewardManagerCalls::GetScores(decoded));
        }
        if let Ok(decoded) =
            <SubmitScoresCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
        {
            return Ok(RewardManagerCalls::SubmitScores(decoded));
        }
        Err(ethers::core::abi::Error::InvalidData.into())
    }
}
impl ethers::core::abi::AbiEncode for RewardManagerCalls {
    fn encode(self) -> Vec<u8> {
        match self {
            RewardManagerCalls::GetScores(element) => element.encode(),
            RewardManagerCalls::SubmitScores(element) => element.encode(),
        }
    }
}
impl ::std::fmt::Display for RewardManagerCalls {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match self {
            RewardManagerCalls::GetScores(element) => element.fmt(f),
            RewardManagerCalls::SubmitScores(element) => element.fmt(f),
        }
    }
}
impl ::std::convert::From<GetScoresCall> for RewardManagerCalls {
    fn from(var: GetScoresCall) -> Self {
        RewardManagerCalls::GetScores(var)
    }
}
impl ::std::convert::From<SubmitScoresCall> for RewardManagerCalls {
    fn from(var: SubmitScoresCall) -> Self {
        RewardManagerCalls::SubmitScores(var)
    }
}
#[doc = "Container type for all return fields from the `getScores` function with signature `getScores(uint256)` and selector `[4, 82, 125, 144]`"]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    ethers :: contract :: EthAbiType,
    ethers :: contract :: EthAbiCodec,
    Default,
)]
pub struct GetScoresReturn(pub ::std::vec::Vec<EpochScores>);
#[doc = "`EpochScores(string,(string,uint64,uint32,uint128)[])`"]
#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    ethers :: contract :: EthAbiType,
    ethers :: contract :: EthAbiCodec,
)]
pub struct EpochScores {
    pub peer_id: String,
    pub measurements: ::std::vec::Vec<Measurement>,
}
#[doc = "`Measurement(string,uint64,uint32,uint128)`"]
#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    ethers :: contract :: EthAbiType,
    ethers :: contract :: EthAbiCodec,
)]
pub struct Measurement {
    pub peer_id: String,
    pub bandwidth: u64,
    pub latency: u32,
    pub uptime: u128,
}
