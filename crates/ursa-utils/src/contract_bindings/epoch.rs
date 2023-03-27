pub use epoch::*;
#[allow(clippy::too_many_arguments, non_camel_case_types)]
pub mod epoch {
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
    # [rustfmt :: skip] const __ABI : & str = "[{\"inputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]},{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]}],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"committee\",\"outputs\":[{\"internalType\":\"string\",\"name\":\"\",\"type\":\"string\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"currentCommitteeSize\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"currentEpochEndStampMs\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"epoch\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"epochDurationMs\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"getCurrentCommittee\",\"outputs\":[{\"internalType\":\"string[]\",\"name\":\"\",\"type\":\"string[]\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"getCurrentEpochInfo\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"_epoch\",\"type\":\"uint256\",\"components\":[]},{\"internalType\":\"uint256\",\"name\":\"_currentEpochEndMs\",\"type\":\"uint256\",\"components\":[]},{\"internalType\":\"struct MockEpoch.CommitteeMember[]\",\"name\":\"_committeeMembers\",\"type\":\"tuple[]\",\"components\":[{\"internalType\":\"string\",\"name\":\"publicKey\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"primaryAddress\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"workerAddress\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"workerMempool\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"workerPublicKey\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"networkKey\",\"type\":\"string\",\"components\":[]}]}]},{\"inputs\":[{\"internalType\":\"address\",\"name\":\"_nodeRegistry\",\"type\":\"address\",\"components\":[]},{\"internalType\":\"uint256\",\"name\":\"_firstEpochStart\",\"type\":\"uint256\",\"components\":[]},{\"internalType\":\"uint256\",\"name\":\"_epochDuration\",\"type\":\"uint256\",\"components\":[]},{\"internalType\":\"uint256\",\"name\":\"_maxCommitteeSize\",\"type\":\"uint256\",\"components\":[]}],\"stateMutability\":\"nonpayable\",\"type\":\"function\",\"name\":\"initialize\",\"outputs\":[]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"maxCommitteeSize\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"nodeRegistry\",\"outputs\":[{\"internalType\":\"contract MockNodeRegistry\",\"name\":\"\",\"type\":\"address\",\"components\":[]}]},{\"inputs\":[{\"internalType\":\"string\",\"name\":\"committeeMember\",\"type\":\"string\",\"components\":[]}],\"stateMutability\":\"nonpayable\",\"type\":\"function\",\"name\":\"signalEpochChange\",\"outputs\":[{\"internalType\":\"bool\",\"name\":\"\",\"type\":\"bool\",\"components\":[]}]}]" ;
    #[doc = r" The parsed JSON-ABI of the contract."]
    pub static EPOCH_ABI: ethers::contract::Lazy<ethers::core::abi::Abi> =
        ethers::contract::Lazy::new(|| {
            ethers::core::utils::__serde_json::from_str(__ABI).expect("invalid abi")
        });
    #[doc = r" Bytecode of the #name contract"]
    pub static EPOCH_BYTECODE: ethers::contract::Lazy<ethers::core::types::Bytes> =
        ethers::contract::Lazy::new(|| {
            "0x608060405234801561001057600080fd5b50611435806100206000396000f3fe608060405234801561001057600080fd5b50600436106100a95760003560e01c80638cab1fea116100715780638cab1fea1461013b578063900cf0cf14610144578063ad4429f41461014d578063babc394f14610156578063d9b5c4a51461016d578063e0b840531461019857600080fd5b80631a4fc0e9146100ae578063484f657b146100d75780634ec81af1146100ec57806352bc7a82146101015780636b319e3114610124575b600080fd5b6100c16100bc366004610af4565b6101a1565b6040516100ce9190610b66565b60405180910390f35b6100df61025a565b6040516100ce9190610b80565b6100ff6100fa366004610bf7565b610345565b005b61011461010f366004610ccb565b61042e565b60405190151581526020016100ce565b61012d60045481565b6040519081526020016100ce565b61012d60015481565b61012d60005481565b61012d60025481565b61015e61055b565b6040516100ce93929190610d4b565b600554610180906001600160a01b031681565b6040516001600160a01b0390911681526020016100ce565b61012d60035481565b600760205281600052604060002081815481106101bd57600080fd5b906000526020600020016000915091505080546101d990610e49565b80601f016020809104026020016040519081016040528092919081815260200182805461020590610e49565b80156102525780601f1061022757610100808354040283529160200191610252565b820191906000526020600020905b81548152906001019060200180831161023557829003601f168201915b505050505081565b60606007600080548152602001908152602001600020805480602002602001604051908101604052809291908181526020016000905b8282101561033c5783829060005260206000200180546102af90610e49565b80601f01602080910402602001604051908101604052809291908181526020018280546102db90610e49565b80156103285780601f106102fd57610100808354040283529160200191610328565b820191906000526020600020905b81548152906001019060200180831161030b57829003601f168201915b505050505081526020019060010190610290565b50505050905090565b600554600160a01b900460ff16156103a35760405162461bcd60e51b815260206004820152601c60248201527f636f6e747261637420616c726561647920696e697469616c697a656400000000604482015260640160405180910390fd5b600580546001600160a01b0319166001600160a01b038616179055600381905560048290556103d28284610e93565b6001556103dd6107bf565b60008054815260076020908152604090912082516104019391929190910190610a2f565b50506000805481526007602052604090205460025550506005805460ff60a01b1916600160a01b17905550565b6000805b826040516020016104439190610eac565b60408051601f1981840301815291815281516020928301206000805481526007909352912080548390811061047a5761047a610ec8565b906000526020600020016040516020016104949190610ede565b60405160208183030381529060405280519060200120036104c957600680549060006104bf83610f54565b91905055506104e4565b808060010191505060025481036104df57600080fd5b610432565b50600080600360025460026104f99190610f6d565b6105039190610f9a565b111561050d575060015b806003600254600261051f9190610f6d565b6105299190610f9a565b6105339190610e93565b6006541061054c576105436109c6565b50600192915050565b50600092915050565b50919050565b600080606060005b6002548110156107af57600554600080548152600760205260408120805491926001600160a01b03169163813775a39190859081106105a4576105a4610ec8565b906000526020600020016040518263ffffffff1660e01b81526004016105ca9190610fae565b600060405180830381865afa1580156105e7573d6000803e3d6000fd5b505050506040513d6000823e601f3d908101601f1916820160405261060f9190810190611096565b6000805481526007602052604090208054919250908390811061063457610634610ec8565b90600052602060002001805461064990610e49565b80601f016020809104026020016040519081016040528092919081815260200182805461067590610e49565b80156106c25780601f10610697576101008083540402835291602001916106c2565b820191906000526020600020905b8154815290600101906020018083116106a557829003601f168201915b50505050508383815181106106d9576106d9610ec8565b60200260200101516000018190525080602001518383815181106106ff576106ff610ec8565b602002602001015160200181905250806040015183838151811061072557610725610ec8565b602002602001015160400181905250806060015183838151811061074b5761074b610ec8565b6020026020010151608001819052508060a0015183838151811061077157610771610ec8565b602002602001015160a00181905250806080015183838151811061079757610797610ec8565b60209081029190910101516060015250600101610563565b5060005460015492509250909192565b60606000600560009054906101000a90046001600160a01b03166001600160a01b031663f2624b5d6040518163ffffffff1660e01b8152600401602060405180830381865afa158015610816573d6000803e3d6000fd5b505050506040513d601f19601f8201168201806040525081019061083a91906111ed565b90506000600560009054906101000a90046001600160a01b03166001600160a01b031663d01f63f56040518163ffffffff1660e01b8152600401600060405180830381865afa158015610891573d6000803e3d6000fd5b505050506040513d6000823e601f3d908101601f191682016040526108b99190810190611206565b90506003548210156108cb5792915050565b60005b600083826108dd6001436112c9565b406040516020016108f8929190918252602082015260400190565b6040516020818303038152906040528051906020012060001c61091b91906112dc565b9050600083828151811061093157610931610ec8565b60200260200101515111156109ba5782818151811061095257610952610ec8565b602002602001015185838151811061096c5761096c610ec8565b60200260200101819052506040518060200160405280600081525083828151811061099957610999610ec8565b602002602001018190525060035482106109b357506109c0565b6001820191505b506108ce565b50505090565b6000805490806109d583610f54565b91905055506109e26107bf565b6000805481526007602090815260409091208251610a069391929190910190610a2f565b50600454600154610a179190610e93565b60015560008054815260076020526040902054600255565b828054828255906000526020600020908101928215610a75579160200282015b82811115610a755782518290610a65908261133f565b5091602001919060010190610a4f565b50610a81929150610a85565b5090565b80821115610a81576000610a998282610aa2565b50600101610a85565b508054610aae90610e49565b6000825580601f10610abe575050565b601f016020900490600052602060002090810190610adc9190610adf565b50565b5b80821115610a815760008155600101610ae0565b60008060408385031215610b0757600080fd5b50508035926020909101359150565b60005b83811015610b31578181015183820152602001610b19565b50506000910152565b60008151808452610b52816020860160208601610b16565b601f01601f19169290920160200192915050565b602081526000610b796020830184610b3a565b9392505050565b6000602080830181845280855180835260408601915060408160051b870101925083870160005b82811015610bd557603f19888603018452610bc3858351610b3a565b94509285019290850190600101610ba7565b5092979650505050505050565b6001600160a01b0381168114610adc57600080fd5b60008060008060808587031215610c0d57600080fd5b8435610c1881610be2565b966020860135965060408601359560600135945092505050565b634e487b7160e01b600052604160045260246000fd5b604051610100810167ffffffffffffffff81118282101715610c6c57610c6c610c32565b60405290565b604051601f8201601f1916810167ffffffffffffffff81118282101715610c9b57610c9b610c32565b604052919050565b600067ffffffffffffffff821115610cbd57610cbd610c32565b50601f01601f191660200190565b600060208284031215610cdd57600080fd5b813567ffffffffffffffff811115610cf457600080fd5b8201601f81018413610d0557600080fd5b8035610d18610d1382610ca3565b610c72565b818152856020838501011115610d2d57600080fd5b81602084016020830137600091810160200191909152949350505050565b600060608083018684526020868186015260408381870152828751808552608094508488019150848160051b890101848a0160005b83811015610e3757607f198b8403018552815160c08151818652610da682870182610b3a565b915050888201518582038a870152610dbe8282610b3a565b9150508782015185820389870152610dd68282610b3a565b9150508a8201518582038c870152610dee8282610b3a565b915050898201518582038b870152610e068282610b3a565b91505060a08083015192508582038187015250610e238183610b3a565b968901969450505090860190600101610d80565b50909c9b505050505050505050505050565b600181811c90821680610e5d57607f821691505b60208210810361055557634e487b7160e01b600052602260045260246000fd5b634e487b7160e01b600052601160045260246000fd5b80820180821115610ea657610ea6610e7d565b92915050565b60008251610ebe818460208701610b16565b9190910192915050565b634e487b7160e01b600052603260045260246000fd5b6000808354610eec81610e49565b60018281168015610f045760018114610f1957610f48565b60ff1984168752821515830287019450610f48565b8760005260208060002060005b85811015610f3f5781548a820152908401908201610f26565b50505082870194505b50929695505050505050565b600060018201610f6657610f66610e7d565b5060010190565b8082028115828204841417610ea657610ea6610e7d565b634e487b7160e01b600052601260045260246000fd5b600082610fa957610fa9610f84565b500490565b6000602080835260008454610fc281610e49565b80848701526040600180841660008114610fe35760018114610ffd5761102b565b60ff1985168984015283151560051b89018301955061102b565b896000528660002060005b858110156110235781548b8201860152908301908801611008565b8a0184019650505b509398975050505050505050565b805161104481610be2565b919050565b600082601f83011261105a57600080fd5b8151611068610d1382610ca3565b81815284602083860101111561107d57600080fd5b61108e826020830160208701610b16565b949350505050565b6000602082840312156110a857600080fd5b815167ffffffffffffffff808211156110c057600080fd5b9083019061010082860312156110d557600080fd5b6110dd610c48565b6110e683611039565b81526020830151828111156110fa57600080fd5b61110687828601611049565b60208301525060408301518281111561111e57600080fd5b61112a87828601611049565b60408301525060608301518281111561114257600080fd5b61114e87828601611049565b60608301525060808301518281111561116657600080fd5b61117287828601611049565b60808301525060a08301518281111561118a57600080fd5b61119687828601611049565b60a08301525060c0830151828111156111ae57600080fd5b6111ba87828601611049565b60c08301525060e0830151828111156111d257600080fd5b6111de87828601611049565b60e08301525095945050505050565b6000602082840312156111ff57600080fd5b5051919050565b6000602080838503121561121957600080fd5b825167ffffffffffffffff8082111561123157600080fd5b818501915085601f83011261124557600080fd5b81518181111561125757611257610c32565b8060051b611266858201610c72565b918252838101850191858101908984111561128057600080fd5b86860192505b838310156112bc5782518581111561129e5760008081fd5b6112ac8b89838a0101611049565b8352509186019190860190611286565b9998505050505050505050565b81810381811115610ea657610ea6610e7d565b6000826112eb576112eb610f84565b500690565b601f82111561133a57600081815260208120601f850160051c810160208610156113175750805b601f850160051c820191505b8181101561133657828155600101611323565b5050505b505050565b815167ffffffffffffffff81111561135957611359610c32565b61136d816113678454610e49565b846112f0565b602080601f8311600181146113a2576000841561138a5750858301515b600019600386901b1c1916600185901b178555611336565b600085815260208120601f198616915b828110156113d1578886015182559484019460019091019084016113b2565b50858210156113ef5787850151600019600388901b60f8161c191681555b5050505050600190811b0190555056fea2646970667358221220d3ab20f458f9bc53cf0cb55abe3325d1c8d05905460f816eae85b23d9983159164736f6c63430008110033" . parse () . expect ("invalid bytecode")
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
        pub fn epoch(
            &self,
        ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::U256> {
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
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::std::result::Result<Self, ethers::core::abi::AbiError> {
            if let Ok(decoded) =
                <CommitteeCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
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
            if let Ok(decoded) = <EpochCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
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
            if let Ok(decoded) =
                <InitializeCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
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
    #[doc = "`CommitteeMember(string,string,string,string,string,string)`"]
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
        pub worker_address: String,
        pub worker_mempool: String,
        pub worker_public_key: String,
        pub network_key: String,
    }
}
