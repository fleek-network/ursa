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
#[doc = "Epoch was auto-generated with ethers-rs Abigen. More information at: https://github.com/gakonst/ethers-rs"]
use std::sync::Arc;
# [rustfmt :: skip] const __ABI : & str = "[{\"inputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]},{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]}],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"committee\",\"outputs\":[{\"internalType\":\"string\",\"name\":\"\",\"type\":\"string\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"currentCommitteeSize\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"currentEpochEndStampMs\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"epoch\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"epochDurationMs\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"getCurrentCommittee\",\"outputs\":[{\"internalType\":\"string[]\",\"name\":\"\",\"type\":\"string[]\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"getCurrentEpochInfo\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"_epoch\",\"type\":\"uint256\",\"components\":[]},{\"internalType\":\"uint256\",\"name\":\"_currentEpochEndMs\",\"type\":\"uint256\",\"components\":[]},{\"internalType\":\"struct EpochManager.CommitteeMember[]\",\"name\":\"_committeeMembers\",\"type\":\"tuple[]\",\"components\":[{\"internalType\":\"string\",\"name\":\"publicKey\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"primaryAddress\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"networkKey\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"struct NodeRegistry.Worker[]\",\"name\":\"workers\",\"type\":\"tuple[]\",\"components\":[{\"internalType\":\"string\",\"name\":\"workerAddress\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"workerPublicKey\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"workerMempool\",\"type\":\"string\",\"components\":[]}]}]}]},{\"inputs\":[{\"internalType\":\"address\",\"name\":\"_nodeRegistry\",\"type\":\"address\",\"components\":[]},{\"internalType\":\"uint256\",\"name\":\"_firstEpochStart\",\"type\":\"uint256\",\"components\":[]},{\"internalType\":\"uint256\",\"name\":\"_epochDuration\",\"type\":\"uint256\",\"components\":[]},{\"internalType\":\"uint256\",\"name\":\"_maxCommitteeSize\",\"type\":\"uint256\",\"components\":[]}],\"stateMutability\":\"nonpayable\",\"type\":\"function\",\"name\":\"initialize\",\"outputs\":[]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"maxCommitteeSize\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"nodeRegistry\",\"outputs\":[{\"internalType\":\"contract NodeRegistry\",\"name\":\"\",\"type\":\"address\",\"components\":[]}]},{\"inputs\":[{\"internalType\":\"string\",\"name\":\"committeeMember\",\"type\":\"string\",\"components\":[]}],\"stateMutability\":\"nonpayable\",\"type\":\"function\",\"name\":\"signalEpochChange\",\"outputs\":[{\"internalType\":\"bool\",\"name\":\"\",\"type\":\"bool\",\"components\":[]}]}]" ;
#[doc = r" The parsed JSON-ABI of the contract."]
pub static EPOCH_ABI: ethers::contract::Lazy<ethers::core::abi::Abi> =
    ethers::contract::Lazy::new(|| {
        ethers::core::utils::__serde_json::from_str(__ABI).expect("invalid abi")
    });
#[doc = r" Bytecode of the #name contract"]
pub static EPOCH_BYTECODE: ethers::contract::Lazy<ethers::core::types::Bytes> =
    ethers::contract::Lazy::new(|| {
        "0x608060405234801561001057600080fd5b506115a5806100206000396000f3fe608060405234801561001057600080fd5b50600436106100a95760003560e01c80638cab1fea116100715780638cab1fea1461013b578063900cf0cf14610144578063ad4429f41461014d578063babc394f14610156578063d9b5c4a51461016d578063e0b840531461019857600080fd5b80631a4fc0e9146100ae578063484f657b146100d75780634ec81af1146100ec57806352bc7a82146101015780636b319e3114610124575b600080fd5b6100c16100bc366004610b22565b6101a1565b6040516100ce9190610b94565b60405180910390f35b6100df61025a565b6040516100ce9190610bae565b6100ff6100fa366004610c25565b610345565b005b61011461010f366004610d1b565b61042e565b60405190151581526020016100ce565b61012d60045481565b6040519081526020016100ce565b61012d60015481565b61012d60005481565b61012d60025481565b61015e61055b565b6040516100ce93929190610d9b565b600554610180906001600160a01b031681565b6040516001600160a01b0390911681526020016100ce565b61012d60035481565b600760205281600052604060002081815481106101bd57600080fd5b906000526020600020016000915091505080546101d990610edf565b80601f016020809104026020016040519081016040528092919081815260200182805461020590610edf565b80156102525780601f1061022757610100808354040283529160200191610252565b820191906000526020600020905b81548152906001019060200180831161023557829003601f168201915b505050505081565b60606007600080548152602001908152602001600020805480602002602001604051908101604052809291908181526020016000905b8282101561033c5783829060005260206000200180546102af90610edf565b80601f01602080910402602001604051908101604052809291908181526020018280546102db90610edf565b80156103285780601f106102fd57610100808354040283529160200191610328565b820191906000526020600020905b81548152906001019060200180831161030b57829003601f168201915b505050505081526020019060010190610290565b50505050905090565b600554600160a01b900460ff16156103a35760405162461bcd60e51b815260206004820152601c60248201527f636f6e747261637420616c726561647920696e697469616c697a656400000000604482015260640160405180910390fd5b600580546001600160a01b0319166001600160a01b038616179055600381905560048290556103d28284610f29565b6001556103dd6107ed565b60008054815260076020908152604090912082516104019391929190910190610a5d565b50506000805481526007602052604090205460025550506005805460ff60a01b1916600160a01b17905550565b6000805b826040516020016104439190610f42565b60408051601f1981840301815291815281516020928301206000805481526007909352912080548390811061047a5761047a610f5e565b906000526020600020016040516020016104949190610f74565b60405160208183030381529060405280519060200120036104c957600680549060006104bf83610fea565b91905055506104e4565b808060010191505060025481036104df57600080fd5b610432565b50600080600360025460026104f99190611003565b6105039190611030565b111561050d575060015b806003600254600261051f9190611003565b6105299190611030565b6105339190610f29565b6006541061054c576105436109f4565b50600192915050565b50600092915050565b50919050565b600080606060025467ffffffffffffffff81111561057b5761057b610c60565b6040519080825280602002602001820160405280156105d757816020015b6105c46040518060800160405280606081526020016060815260200160608152602001606081525090565b8152602001906001900390816105995790505b50905060005b6002548110156107dd57600554600080548152600760205260408120805491926001600160a01b03169163813775a391908590811061061e5761061e610f5e565b906000526020600020016040518263ffffffff1660e01b81526004016106449190611044565b600060405180830381865afa158015610661573d6000803e3d6000fd5b505050506040513d6000823e601f3d908101601f191682016040526106899190810190611261565b600080548152600760205260409020805491925090839081106106ae576106ae610f5e565b9060005260206000200180546106c390610edf565b80601f01602080910402602001604051908101604052809291908181526020018280546106ef90610edf565b801561073c5780601f106107115761010080835404028352916020019161073c565b820191906000526020600020905b81548152906001019060200180831161071f57829003601f168201915b505050505083838151811061075357610753610f5e565b602002602001015160000181905250806020015183838151811061077957610779610f5e565b602002602001015160200181905250806040015183838151811061079f5761079f610f5e565b60200260200101516040018190525080606001518383815181106107c5576107c5610f5e565b602090810291909101015160600152506001016105dd565b5060005460015492509250909192565b60606000600560009054906101000a90046001600160a01b03166001600160a01b031663f2624b5d6040518163ffffffff1660e01b8152600401602060405180830381865afa158015610844573d6000803e3d6000fd5b505050506040513d601f19601f82011682018060405250810190610868919061136f565b90506000600560009054906101000a90046001600160a01b03166001600160a01b031663d01f63f56040518163ffffffff1660e01b8152600401600060405180830381865afa1580156108bf573d6000803e3d6000fd5b505050506040513d6000823e601f3d908101601f191682016040526108e79190810190611388565b90506003548210156108f95792915050565b60005b6000838261090b600143611439565b40604051602001610926929190918252602082015260400190565b6040516020818303038152906040528051906020012060001c610949919061144c565b9050600083828151811061095f5761095f610f5e565b60200260200101515111156109e85782818151811061098057610980610f5e565b602002602001015185838151811061099a5761099a610f5e565b6020026020010181905250604051806020016040528060008152508382815181106109c7576109c7610f5e565b602002602001018190525060035482106109e157506109ee565b6001820191505b506108fc565b50505090565b600080549080610a0383610fea565b9190505550610a106107ed565b6000805481526007602090815260409091208251610a349391929190910190610a5d565b50600454600154610a459190610f29565b60015560008054815260076020526040902054600255565b828054828255906000526020600020908101928215610aa3579160200282015b82811115610aa35782518290610a9390826114af565b5091602001919060010190610a7d565b50610aaf929150610ab3565b5090565b80821115610aaf576000610ac78282610ad0565b50600101610ab3565b508054610adc90610edf565b6000825580601f10610aec575050565b601f016020900490600052602060002090810190610b0a9190610b0d565b50565b5b80821115610aaf5760008155600101610b0e565b60008060408385031215610b3557600080fd5b50508035926020909101359150565b60005b83811015610b5f578181015183820152602001610b47565b50506000910152565b60008151808452610b80816020860160208601610b44565b601f01601f19169290920160200192915050565b602081526000610ba76020830184610b68565b9392505050565b6000602080830181845280855180835260408601915060408160051b870101925083870160005b82811015610c0357603f19888603018452610bf1858351610b68565b94509285019290850190600101610bd5565b5092979650505050505050565b6001600160a01b0381168114610b0a57600080fd5b60008060008060808587031215610c3b57600080fd5b8435610c4681610c10565b966020860135965060408601359560600135945092505050565b634e487b7160e01b600052604160045260246000fd5b6040516060810167ffffffffffffffff81118282101715610c9957610c99610c60565b60405290565b60405160c0810167ffffffffffffffff81118282101715610c9957610c99610c60565b604051601f8201601f1916810167ffffffffffffffff81118282101715610ceb57610ceb610c60565b604052919050565b600067ffffffffffffffff821115610d0d57610d0d610c60565b50601f01601f191660200190565b600060208284031215610d2d57600080fd5b813567ffffffffffffffff811115610d4457600080fd5b8201601f81018413610d5557600080fd5b8035610d68610d6382610cf3565b610cc2565b818152856020838501011115610d7d57600080fd5b81602084016020830137600091810160200191909152949350505050565b600060608083018684526020868186015282604086015281865180845260808701915060808160051b880101935082880160005b82811015610ecf57607f198987030184528151805160808852610df56080890182610b68565b905086820151888203888a0152610e0c8282610b68565b915050604082015188820360408a0152610e268282610b68565b928a01518984038a8c015280518085529089019392508883019150600581901b8301890160005b82811015610eb857848203601f19018452855180518e8452610e718f850182610b68565b90508c8201518482038e860152610e888282610b68565b915050604082015191508381036040850152610ea48183610b68565b978d0197958d019593505050600101610e4d565b509950505094860194505090840190600101610dcf565b50939a9950505050505050505050565b600181811c90821680610ef357607f821691505b60208210810361055557634e487b7160e01b600052602260045260246000fd5b634e487b7160e01b600052601160045260246000fd5b80820180821115610f3c57610f3c610f13565b92915050565b60008251610f54818460208701610b44565b9190910192915050565b634e487b7160e01b600052603260045260246000fd5b6000808354610f8281610edf565b60018281168015610f9a5760018114610faf57610fde565b60ff1984168752821515830287019450610fde565b8760005260208060002060005b85811015610fd55781548a820152908401908201610fbc565b50505082870194505b50929695505050505050565b600060018201610ffc57610ffc610f13565b5060010190565b8082028115828204841417610f3c57610f3c610f13565b634e487b7160e01b600052601260045260246000fd5b60008261103f5761103f61101a565b500490565b600060208083526000845461105881610edf565b808487015260406001808416600081146110795760018114611093576110c1565b60ff1985168984015283151560051b8901830195506110c1565b896000528660002060005b858110156110b95781548b820186015290830190880161109e565b8a0184019650505b509398975050505050505050565b80516110da81610c10565b919050565b600082601f8301126110f057600080fd5b81516110fe610d6382610cf3565b81815284602083860101111561111357600080fd5b611124826020830160208701610b44565b949350505050565b600067ffffffffffffffff82111561114657611146610c60565b5060051b60200190565b600082601f83011261116157600080fd5b81516020611171610d638361112c565b82815260059290921b8401810191818101908684111561119057600080fd5b8286015b8481101561125657805167ffffffffffffffff808211156111b55760008081fd5b908801906060828b03601f19018113156111cf5760008081fd5b6111d7610c76565b87840151838111156111e95760008081fd5b6111f78d8a838801016110df565b8252506040808501518481111561120e5760008081fd5b61121c8e8b838901016110df565b838b0152509184015191838311156112345760008081fd5b6112428d8a858801016110df565b908201528652505050918301918301611194565b509695505050505050565b60006020828403121561127357600080fd5b815167ffffffffffffffff8082111561128b57600080fd5b9083019060c0828603121561129f57600080fd5b6112a7610c9f565b6112b0836110cf565b81526020830151828111156112c457600080fd5b6112d0878286016110df565b6020830152506040830151828111156112e857600080fd5b6112f4878286016110df565b60408301525060608301518281111561130c57600080fd5b61131887828601611150565b60608301525060808301518281111561133057600080fd5b61133c878286016110df565b60808301525060a08301518281111561135457600080fd5b611360878286016110df565b60a08301525095945050505050565b60006020828403121561138157600080fd5b5051919050565b6000602080838503121561139b57600080fd5b825167ffffffffffffffff808211156113b357600080fd5b818501915085601f8301126113c757600080fd5b81516113d5610d638261112c565b81815260059190911b830184019084810190888311156113f457600080fd5b8585015b8381101561142c578051858111156114105760008081fd5b61141e8b89838a01016110df565b8452509186019186016113f8565b5098975050505050505050565b81810381811115610f3c57610f3c610f13565b60008261145b5761145b61101a565b500690565b601f8211156114aa57600081815260208120601f850160051c810160208610156114875750805b601f850160051c820191505b818110156114a657828155600101611493565b5050505b505050565b815167ffffffffffffffff8111156114c9576114c9610c60565b6114dd816114d78454610edf565b84611460565b602080601f83116001811461151257600084156114fa5750858301515b600019600386901b1c1916600185901b1785556114a6565b600085815260208120601f198616915b8281101561154157888601518255948401946001909101908401611522565b508582101561155f5787850151600019600388901b60f8161c191681555b5050505050600190811b0190555056fea26469706673582212205abadf5d54a98d4d16d105f12ec3c7d5aac0157466b7befe5be16c3a25ab9e3564736f6c63430008110033" . parse () . expect ("invalid bytecode")
    });
pub struct Epoch<M>(ethers::contract::Contract<M>);
impl<M> Clone for Epoch<M> {
    fn clone(&self) -> Self {
        Epoch(self.0.clone())
    }
}
impl<M> std::ops::Deref for Epoch<M> {
    type Target = ethers::contract::Contract<M>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<M> std::fmt::Debug for Epoch<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_tuple(stringify!(Epoch))
            .field(&self.address())
            .finish()
    }
}
impl<M: ethers::providers::Middleware> Epoch<M> {
    #[doc = r" Creates a new contract instance with the specified `ethers`"]
    #[doc = r" client at the given `Address`. The contract derefs to a `ethers::Contract`"]
    #[doc = r" object"]
    pub fn new<T: Into<ethers::core::types::Address>>(
        address: T,
        client: ::std::sync::Arc<M>,
    ) -> Self {
        ethers::contract::Contract::new(address.into(), EPOCH_ABI.clone(), client).into()
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
            EPOCH_ABI.clone(),
            EPOCH_BYTECODE.clone(),
            client,
        );
        let deployer = factory.deploy(constructor_args)?;
        let deployer = ethers::contract::ContractDeployer::new(deployer);
        Ok(deployer)
    }
    #[doc = "Calls the contract's `committee` (0x1a4fc0e9) function"]
    pub fn committee(
        &self,
        p0: ethers::core::types::U256,
        p1: ethers::core::types::U256,
    ) -> ethers::contract::builders::ContractCall<M, String> {
        self.0
            .method_hash([26, 79, 192, 233], (p0, p1))
            .expect("method not found (this should never happen)")
    }
    #[doc = "Calls the contract's `currentCommitteeSize` (0xad4429f4) function"]
    pub fn current_committee_size(
        &self,
    ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::U256> {
        self.0
            .method_hash([173, 68, 41, 244], ())
            .expect("method not found (this should never happen)")
    }
    #[doc = "Calls the contract's `currentEpochEndStampMs` (0x8cab1fea) function"]
    pub fn current_epoch_end_stamp_ms(
        &self,
    ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::U256> {
        self.0
            .method_hash([140, 171, 31, 234], ())
            .expect("method not found (this should never happen)")
    }
    #[doc = "Calls the contract's `epoch` (0x900cf0cf) function"]
    pub fn epoch(&self) -> ethers::contract::builders::ContractCall<M, ethers::core::types::U256> {
        self.0
            .method_hash([144, 12, 240, 207], ())
            .expect("method not found (this should never happen)")
    }
    #[doc = "Calls the contract's `epochDurationMs` (0x6b319e31) function"]
    pub fn epoch_duration_ms(
        &self,
    ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::U256> {
        self.0
            .method_hash([107, 49, 158, 49], ())
            .expect("method not found (this should never happen)")
    }
    #[doc = "Calls the contract's `getCurrentCommittee` (0x484f657b) function"]
    pub fn get_current_committee(
        &self,
    ) -> ethers::contract::builders::ContractCall<M, ::std::vec::Vec<String>> {
        self.0
            .method_hash([72, 79, 101, 123], ())
            .expect("method not found (this should never happen)")
    }
    #[doc = "Calls the contract's `getCurrentEpochInfo` (0xbabc394f) function"]
    pub fn get_current_epoch_info(
        &self,
    ) -> ethers::contract::builders::ContractCall<
        M,
        (
            ethers::core::types::U256,
            ethers::core::types::U256,
            ::std::vec::Vec<CommitteeMember>,
        ),
    > {
        self.0
            .method_hash([186, 188, 57, 79], ())
            .expect("method not found (this should never happen)")
    }
    #[doc = "Calls the contract's `initialize` (0x4ec81af1) function"]
    pub fn initialize(
        &self,
        node_registry: ethers::core::types::Address,
        first_epoch_start: ethers::core::types::U256,
        epoch_duration: ethers::core::types::U256,
        max_committee_size: ethers::core::types::U256,
    ) -> ethers::contract::builders::ContractCall<M, ()> {
        self.0
            .method_hash(
                [78, 200, 26, 241],
                (
                    node_registry,
                    first_epoch_start,
                    epoch_duration,
                    max_committee_size,
                ),
            )
            .expect("method not found (this should never happen)")
    }
    #[doc = "Calls the contract's `maxCommitteeSize` (0xe0b84053) function"]
    pub fn max_committee_size(
        &self,
    ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::U256> {
        self.0
            .method_hash([224, 184, 64, 83], ())
            .expect("method not found (this should never happen)")
    }
    #[doc = "Calls the contract's `nodeRegistry` (0xd9b5c4a5) function"]
    pub fn node_registry(
        &self,
    ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::Address> {
        self.0
            .method_hash([217, 181, 196, 165], ())
            .expect("method not found (this should never happen)")
    }
    #[doc = "Calls the contract's `signalEpochChange` (0x52bc7a82) function"]
    pub fn signal_epoch_change(
        &self,
        committee_member: String,
    ) -> ethers::contract::builders::ContractCall<M, bool> {
        self.0
            .method_hash([82, 188, 122, 130], committee_member)
            .expect("method not found (this should never happen)")
    }
}
impl<M: ethers::providers::Middleware> From<ethers::contract::Contract<M>> for Epoch<M> {
    fn from(contract: ethers::contract::Contract<M>) -> Self {
        Self(contract)
    }
}
#[doc = "Container type for all input parameters for the `committee` function with signature `committee(uint256,uint256)` and selector `[26, 79, 192, 233]`"]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    ethers :: contract :: EthCall,
    ethers :: contract :: EthDisplay,
    Default,
)]
#[ethcall(name = "committee", abi = "committee(uint256,uint256)")]
pub struct CommitteeCall(pub ethers::core::types::U256, pub ethers::core::types::U256);
#[doc = "Container type for all input parameters for the `currentCommitteeSize` function with signature `currentCommitteeSize()` and selector `[173, 68, 41, 244]`"]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    ethers :: contract :: EthCall,
    ethers :: contract :: EthDisplay,
    Default,
)]
#[ethcall(name = "currentCommitteeSize", abi = "currentCommitteeSize()")]
pub struct CurrentCommitteeSizeCall;
#[doc = "Container type for all input parameters for the `currentEpochEndStampMs` function with signature `currentEpochEndStampMs()` and selector `[140, 171, 31, 234]`"]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    ethers :: contract :: EthCall,
    ethers :: contract :: EthDisplay,
    Default,
)]
#[ethcall(name = "currentEpochEndStampMs", abi = "currentEpochEndStampMs()")]
pub struct CurrentEpochEndStampMsCall;
#[doc = "Container type for all input parameters for the `epoch` function with signature `epoch()` and selector `[144, 12, 240, 207]`"]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    ethers :: contract :: EthCall,
    ethers :: contract :: EthDisplay,
    Default,
)]
#[ethcall(name = "epoch", abi = "epoch()")]
pub struct EpochCall;
#[doc = "Container type for all input parameters for the `epochDurationMs` function with signature `epochDurationMs()` and selector `[107, 49, 158, 49]`"]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    ethers :: contract :: EthCall,
    ethers :: contract :: EthDisplay,
    Default,
)]
#[ethcall(name = "epochDurationMs", abi = "epochDurationMs()")]
pub struct EpochDurationMsCall;
#[doc = "Container type for all input parameters for the `getCurrentCommittee` function with signature `getCurrentCommittee()` and selector `[72, 79, 101, 123]`"]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    ethers :: contract :: EthCall,
    ethers :: contract :: EthDisplay,
    Default,
)]
#[ethcall(name = "getCurrentCommittee", abi = "getCurrentCommittee()")]
pub struct GetCurrentCommitteeCall;
#[doc = "Container type for all input parameters for the `getCurrentEpochInfo` function with signature `getCurrentEpochInfo()` and selector `[186, 188, 57, 79]`"]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    ethers :: contract :: EthCall,
    ethers :: contract :: EthDisplay,
    Default,
)]
#[ethcall(name = "getCurrentEpochInfo", abi = "getCurrentEpochInfo()")]
pub struct GetCurrentEpochInfoCall;
#[doc = "Container type for all input parameters for the `initialize` function with signature `initialize(address,uint256,uint256,uint256)` and selector `[78, 200, 26, 241]`"]
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
    name = "initialize",
    abi = "initialize(address,uint256,uint256,uint256)"
)]
pub struct InitializeCall {
    pub node_registry: ethers::core::types::Address,
    pub first_epoch_start: ethers::core::types::U256,
    pub epoch_duration: ethers::core::types::U256,
    pub max_committee_size: ethers::core::types::U256,
}
#[doc = "Container type for all input parameters for the `maxCommitteeSize` function with signature `maxCommitteeSize()` and selector `[224, 184, 64, 83]`"]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    ethers :: contract :: EthCall,
    ethers :: contract :: EthDisplay,
    Default,
)]
#[ethcall(name = "maxCommitteeSize", abi = "maxCommitteeSize()")]
pub struct MaxCommitteeSizeCall;
#[doc = "Container type for all input parameters for the `nodeRegistry` function with signature `nodeRegistry()` and selector `[217, 181, 196, 165]`"]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    ethers :: contract :: EthCall,
    ethers :: contract :: EthDisplay,
    Default,
)]
#[ethcall(name = "nodeRegistry", abi = "nodeRegistry()")]
pub struct NodeRegistryCall;
#[doc = "Container type for all input parameters for the `signalEpochChange` function with signature `signalEpochChange(string)` and selector `[82, 188, 122, 130]`"]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    ethers :: contract :: EthCall,
    ethers :: contract :: EthDisplay,
    Default,
)]
#[ethcall(name = "signalEpochChange", abi = "signalEpochChange(string)")]
pub struct SignalEpochChangeCall {
    pub committee_member: String,
}
#[derive(Debug, Clone, PartialEq, Eq, ethers :: contract :: EthAbiType)]
pub enum EpochCalls {
    Committee(CommitteeCall),
    CurrentCommitteeSize(CurrentCommitteeSizeCall),
    CurrentEpochEndStampMs(CurrentEpochEndStampMsCall),
    Epoch(EpochCall),
    EpochDurationMs(EpochDurationMsCall),
    GetCurrentCommittee(GetCurrentCommitteeCall),
    GetCurrentEpochInfo(GetCurrentEpochInfoCall),
    Initialize(InitializeCall),
    MaxCommitteeSize(MaxCommitteeSizeCall),
    NodeRegistry(NodeRegistryCall),
    SignalEpochChange(SignalEpochChangeCall),
}
impl ethers::core::abi::AbiDecode for EpochCalls {
    fn decode(data: impl AsRef<[u8]>) -> ::std::result::Result<Self, ethers::core::abi::AbiError> {
        if let Ok(decoded) = <CommitteeCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
        {
            return Ok(EpochCalls::Committee(decoded));
        }
        if let Ok(decoded) =
            <CurrentCommitteeSizeCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
        {
            return Ok(EpochCalls::CurrentCommitteeSize(decoded));
        }
        if let Ok(decoded) =
            <CurrentEpochEndStampMsCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
        {
            return Ok(EpochCalls::CurrentEpochEndStampMs(decoded));
        }
        if let Ok(decoded) = <EpochCall as ethers::core::abi::AbiDecode>::decode(data.as_ref()) {
            return Ok(EpochCalls::Epoch(decoded));
        }
        if let Ok(decoded) =
            <EpochDurationMsCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
        {
            return Ok(EpochCalls::EpochDurationMs(decoded));
        }
        if let Ok(decoded) =
            <GetCurrentCommitteeCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
        {
            return Ok(EpochCalls::GetCurrentCommittee(decoded));
        }
        if let Ok(decoded) =
            <GetCurrentEpochInfoCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
        {
            return Ok(EpochCalls::GetCurrentEpochInfo(decoded));
        }
        if let Ok(decoded) = <InitializeCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
        {
            return Ok(EpochCalls::Initialize(decoded));
        }
        if let Ok(decoded) =
            <MaxCommitteeSizeCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
        {
            return Ok(EpochCalls::MaxCommitteeSize(decoded));
        }
        if let Ok(decoded) =
            <NodeRegistryCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
        {
            return Ok(EpochCalls::NodeRegistry(decoded));
        }
        if let Ok(decoded) =
            <SignalEpochChangeCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
        {
            return Ok(EpochCalls::SignalEpochChange(decoded));
        }
        Err(ethers::core::abi::Error::InvalidData.into())
    }
}
impl ethers::core::abi::AbiEncode for EpochCalls {
    fn encode(self) -> Vec<u8> {
        match self {
            EpochCalls::Committee(element) => element.encode(),
            EpochCalls::CurrentCommitteeSize(element) => element.encode(),
            EpochCalls::CurrentEpochEndStampMs(element) => element.encode(),
            EpochCalls::Epoch(element) => element.encode(),
            EpochCalls::EpochDurationMs(element) => element.encode(),
            EpochCalls::GetCurrentCommittee(element) => element.encode(),
            EpochCalls::GetCurrentEpochInfo(element) => element.encode(),
            EpochCalls::Initialize(element) => element.encode(),
            EpochCalls::MaxCommitteeSize(element) => element.encode(),
            EpochCalls::NodeRegistry(element) => element.encode(),
            EpochCalls::SignalEpochChange(element) => element.encode(),
        }
    }
}
impl ::std::fmt::Display for EpochCalls {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match self {
            EpochCalls::Committee(element) => element.fmt(f),
            EpochCalls::CurrentCommitteeSize(element) => element.fmt(f),
            EpochCalls::CurrentEpochEndStampMs(element) => element.fmt(f),
            EpochCalls::Epoch(element) => element.fmt(f),
            EpochCalls::EpochDurationMs(element) => element.fmt(f),
            EpochCalls::GetCurrentCommittee(element) => element.fmt(f),
            EpochCalls::GetCurrentEpochInfo(element) => element.fmt(f),
            EpochCalls::Initialize(element) => element.fmt(f),
            EpochCalls::MaxCommitteeSize(element) => element.fmt(f),
            EpochCalls::NodeRegistry(element) => element.fmt(f),
            EpochCalls::SignalEpochChange(element) => element.fmt(f),
        }
    }
}
impl ::std::convert::From<CommitteeCall> for EpochCalls {
    fn from(var: CommitteeCall) -> Self {
        EpochCalls::Committee(var)
    }
}
impl ::std::convert::From<CurrentCommitteeSizeCall> for EpochCalls {
    fn from(var: CurrentCommitteeSizeCall) -> Self {
        EpochCalls::CurrentCommitteeSize(var)
    }
}
impl ::std::convert::From<CurrentEpochEndStampMsCall> for EpochCalls {
    fn from(var: CurrentEpochEndStampMsCall) -> Self {
        EpochCalls::CurrentEpochEndStampMs(var)
    }
}
impl ::std::convert::From<EpochCall> for EpochCalls {
    fn from(var: EpochCall) -> Self {
        EpochCalls::Epoch(var)
    }
}
impl ::std::convert::From<EpochDurationMsCall> for EpochCalls {
    fn from(var: EpochDurationMsCall) -> Self {
        EpochCalls::EpochDurationMs(var)
    }
}
impl ::std::convert::From<GetCurrentCommitteeCall> for EpochCalls {
    fn from(var: GetCurrentCommitteeCall) -> Self {
        EpochCalls::GetCurrentCommittee(var)
    }
}
impl ::std::convert::From<GetCurrentEpochInfoCall> for EpochCalls {
    fn from(var: GetCurrentEpochInfoCall) -> Self {
        EpochCalls::GetCurrentEpochInfo(var)
    }
}
impl ::std::convert::From<InitializeCall> for EpochCalls {
    fn from(var: InitializeCall) -> Self {
        EpochCalls::Initialize(var)
    }
}
impl ::std::convert::From<MaxCommitteeSizeCall> for EpochCalls {
    fn from(var: MaxCommitteeSizeCall) -> Self {
        EpochCalls::MaxCommitteeSize(var)
    }
}
impl ::std::convert::From<NodeRegistryCall> for EpochCalls {
    fn from(var: NodeRegistryCall) -> Self {
        EpochCalls::NodeRegistry(var)
    }
}
impl ::std::convert::From<SignalEpochChangeCall> for EpochCalls {
    fn from(var: SignalEpochChangeCall) -> Self {
        EpochCalls::SignalEpochChange(var)
    }
}
#[doc = "Container type for all return fields from the `committee` function with signature `committee(uint256,uint256)` and selector `[26, 79, 192, 233]`"]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    ethers :: contract :: EthAbiType,
    ethers :: contract :: EthAbiCodec,
    Default,
)]
pub struct CommitteeReturn(pub String);
#[doc = "Container type for all return fields from the `currentCommitteeSize` function with signature `currentCommitteeSize()` and selector `[173, 68, 41, 244]`"]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    ethers :: contract :: EthAbiType,
    ethers :: contract :: EthAbiCodec,
    Default,
)]
pub struct CurrentCommitteeSizeReturn(pub ethers::core::types::U256);
#[doc = "Container type for all return fields from the `currentEpochEndStampMs` function with signature `currentEpochEndStampMs()` and selector `[140, 171, 31, 234]`"]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    ethers :: contract :: EthAbiType,
    ethers :: contract :: EthAbiCodec,
    Default,
)]
pub struct CurrentEpochEndStampMsReturn(pub ethers::core::types::U256);
#[doc = "Container type for all return fields from the `epoch` function with signature `epoch()` and selector `[144, 12, 240, 207]`"]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    ethers :: contract :: EthAbiType,
    ethers :: contract :: EthAbiCodec,
    Default,
)]
pub struct EpochReturn(pub ethers::core::types::U256);
#[doc = "Container type for all return fields from the `epochDurationMs` function with signature `epochDurationMs()` and selector `[107, 49, 158, 49]`"]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    ethers :: contract :: EthAbiType,
    ethers :: contract :: EthAbiCodec,
    Default,
)]
pub struct EpochDurationMsReturn(pub ethers::core::types::U256);
#[doc = "Container type for all return fields from the `getCurrentCommittee` function with signature `getCurrentCommittee()` and selector `[72, 79, 101, 123]`"]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    ethers :: contract :: EthAbiType,
    ethers :: contract :: EthAbiCodec,
    Default,
)]
pub struct GetCurrentCommitteeReturn(pub ::std::vec::Vec<String>);
#[doc = "Container type for all return fields from the `getCurrentEpochInfo` function with signature `getCurrentEpochInfo()` and selector `[186, 188, 57, 79]`"]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    ethers :: contract :: EthAbiType,
    ethers :: contract :: EthAbiCodec,
    Default,
)]
pub struct GetCurrentEpochInfoReturn {
    pub epoch: ethers::core::types::U256,
    pub current_epoch_end_ms: ethers::core::types::U256,
    pub committee_members: ::std::vec::Vec<CommitteeMember>,
}
#[doc = "Container type for all return fields from the `maxCommitteeSize` function with signature `maxCommitteeSize()` and selector `[224, 184, 64, 83]`"]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    ethers :: contract :: EthAbiType,
    ethers :: contract :: EthAbiCodec,
    Default,
)]
pub struct MaxCommitteeSizeReturn(pub ethers::core::types::U256);
#[doc = "Container type for all return fields from the `nodeRegistry` function with signature `nodeRegistry()` and selector `[217, 181, 196, 165]`"]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    ethers :: contract :: EthAbiType,
    ethers :: contract :: EthAbiCodec,
    Default,
)]
pub struct NodeRegistryReturn(pub ethers::core::types::Address);
#[doc = "Container type for all return fields from the `signalEpochChange` function with signature `signalEpochChange(string)` and selector `[82, 188, 122, 130]`"]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    ethers :: contract :: EthAbiType,
    ethers :: contract :: EthAbiCodec,
    Default,
)]
pub struct SignalEpochChangeReturn(pub bool);
#[doc = "`CommitteeMember(string,string,string,(string,string,string)[])`"]
#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    ethers :: contract :: EthAbiType,
    ethers :: contract :: EthAbiCodec,
)]
pub struct CommitteeMember {
    pub public_key: String,
    pub primary_address: String,
    pub network_key: String,
    pub workers: ::std::vec::Vec<Worker>,
}
#[doc = "`Worker(string,string,string)`"]
#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    ethers :: contract :: EthAbiType,
    ethers :: contract :: EthAbiCodec,
)]
pub struct Worker {
    pub worker_address: String,
    pub worker_public_key: String,
    pub worker_mempool: String,
}
