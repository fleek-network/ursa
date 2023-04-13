pub use rewards_aggregator::*;
#[allow(clippy::too_many_arguments, non_camel_case_types)]
pub mod rewards_aggregator {
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
    #[doc = "RewardsAggregator was auto-generated with ethers-rs Abigen. More information at: https://github.com/gakonst/ethers-rs"]
    use std::sync::Arc;
    # [rustfmt :: skip] const __ABI : & str = "[{\"inputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"\",\"type\":\"string\",\"components\":[]}],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"DataServedInBytes\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"daysForAveragePotential\",\"outputs\":[{\"internalType\":\"uint16\",\"name\":\"\",\"type\":\"uint16\",\"components\":[]}]},{\"inputs\":[{\"internalType\":\"uint256\",\"name\":\"_epoch\",\"type\":\"uint256\",\"components\":[]}],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"getAvgUsageNEpochs\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]}]},{\"inputs\":[{\"internalType\":\"uint256\",\"name\":\"epoch\",\"type\":\"uint256\",\"components\":[]}],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"getDataForEpoch\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]}]},{\"inputs\":[{\"internalType\":\"string\",\"name\":\"publicKey\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"uint256\",\"name\":\"epoch\",\"type\":\"uint256\",\"components\":[]}],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"getDataServedByNode\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]}]},{\"inputs\":[{\"internalType\":\"uint256\",\"name\":\"_epoch\",\"type\":\"uint256\",\"components\":[]}],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"getPublicKeys\",\"outputs\":[{\"internalType\":\"string[]\",\"name\":\"\",\"type\":\"string[]\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"nonpayable\",\"type\":\"function\",\"name\":\"initialize\",\"outputs\":[]},{\"inputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"\",\"type\":\"string\",\"components\":[]}],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"publicKeyAdded\",\"outputs\":[{\"internalType\":\"bool\",\"name\":\"\",\"type\":\"bool\",\"components\":[]}]},{\"inputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]},{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]}],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"publicKeys\",\"outputs\":[{\"internalType\":\"string\",\"name\":\"\",\"type\":\"string\",\"components\":[]}]},{\"inputs\":[{\"internalType\":\"uint256\",\"name\":\"epoch\",\"type\":\"uint256\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"publicKey\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"uint256\",\"name\":\"dataServed\",\"type\":\"uint256\",\"components\":[]}],\"stateMutability\":\"nonpayable\",\"type\":\"function\",\"name\":\"recordDataServed\",\"outputs\":[]}]" ;
    #[doc = r" The parsed JSON-ABI of the contract."]
    pub static REWARDSAGGREGATOR_ABI: ethers::contract::Lazy<ethers::core::abi::Abi> =
        ethers::contract::Lazy::new(|| {
            ethers::core::utils::__serde_json::from_str(__ABI).expect("invalid abi")
        });
    #[doc = r" Bytecode of the #name contract"]
    pub static REWARDSAGGREGATOR_BYTECODE: ethers::contract::Lazy<ethers::core::types::Bytes> =
        ethers::contract::Lazy::new(|| {
            "0x608060405234801561001057600080fd5b50610ba8806100206000396000f3fe608060405234801561001057600080fd5b506004361061009e5760003560e01c80637a3b0aff116100665780637a3b0aff1461016d5780638129fc1c1461018e57806389b1798b146101965780639d4bc36c146101a9578063d34b0daf146101e557600080fd5b8063075d17e2146100a357806325e7913e146100c9578063372c1aff1461011857806366f0f022146101385780636b6528d714610158575b600080fd5b6100b66100b1366004610641565b6101f8565b6040519081526020015b60405180910390f35b6101086100d73660046106fd565b6003602090815260009283526040909220815180830184018051928152908401929093019190912091525460ff1681565b60405190151581526020016100c0565b61012b610126366004610641565b610274565b6040516100c09190610794565b61014b6101463660046107f6565b610360565b6040516100c09190610818565b61016b610166366004610832565b610419565b005b60005461017b9061ffff1681565b60405161ffff90911681526020016100c0565b61016b61050b565b6100b66101a4366004610641565b61057b565b6100b66101b73660046106fd565b6001602090815260009283526040909220815180830184018051928152908401929093019190912091525481565b6100b66101f33660046108b4565b61060d565b60008054819061ffff1683111561021f5760005461021a9061ffff168461090f565b610222565b60005b90506000815b84811015610257576102398161057b565b6102439083610922565b91508061024f81610935565b915050610228565b50610262828561090f565b61026c908261094e565b949350505050565b606060026000838152602001908152602001600020805480602002602001604051908101604052809291908181526020016000905b828210156103555783829060005260206000200180546102c890610970565b80601f01602080910402602001604051908101604052809291908181526020018280546102f490610970565b80156103415780601f1061031657610100808354040283529160200191610341565b820191906000526020600020905b81548152906001019060200180831161032457829003601f168201915b5050505050815260200190600101906102a9565b505050509050919050565b6002602052816000526040600020818154811061037c57600080fd5b9060005260206000200160009150915050805461039890610970565b80601f01602080910402602001604051908101604052809291908181526020018280546103c490610970565b80156104115780601f106103e657610100808354040283529160200191610411565b820191906000526020600020905b8154815290600101906020018083116103f457829003601f168201915b505050505081565b60008481526003602052604090819020905161043890859085906109aa565b9081526040519081900360200190205460ff166104bf57600084815260026020908152604082208054600181018255908352912001610478838583610a09565b50600160036000868152602001908152602001600020848460405161049e9291906109aa565b908152604051908190036020019020805491151560ff199092169190911790555b806001600086815260200190815260200160002084846040516104e39291906109aa565b908152602001604051809103902060008282546105009190610922565b909155505050505050565b60005462010000900460ff16156105685760405162461bcd60e51b815260206004820152601c60248201527f636f6e747261637420616c726561647920696e697469616c697a656400000000604482015260640160405180910390fd5b6000805462ff0000191662010000179055565b600080805b60008481526002602052604090205481101561060657600084815260016020908152604080832060029092529091208054839081106105c1576105c1610aca565b906000526020600020016040516105d89190610ae0565b908152602001604051809103902054826105f29190610922565b9150806105fe81610935565b915050610580565b5092915050565b6000818152600160205260408082209051610629908590610b56565b90815260200160405180910390205490505b92915050565b60006020828403121561065357600080fd5b5035919050565b634e487b7160e01b600052604160045260246000fd5b600082601f83011261068157600080fd5b813567ffffffffffffffff8082111561069c5761069c61065a565b604051601f8301601f19908116603f011681019082821181831017156106c4576106c461065a565b816040528381528660208588010111156106dd57600080fd5b836020870160208301376000602085830101528094505050505092915050565b6000806040838503121561071057600080fd5b82359150602083013567ffffffffffffffff81111561072e57600080fd5b61073a85828601610670565b9150509250929050565b60005b8381101561075f578181015183820152602001610747565b50506000910152565b60008151808452610780816020860160208601610744565b601f01601f19169290920160200192915050565b6000602080830181845280855180835260408601915060408160051b870101925083870160005b828110156107e957603f198886030184526107d7858351610768565b945092850192908501906001016107bb565b5092979650505050505050565b6000806040838503121561080957600080fd5b50508035926020909101359150565b60208152600061082b6020830184610768565b9392505050565b6000806000806060858703121561084857600080fd5b84359350602085013567ffffffffffffffff8082111561086757600080fd5b818701915087601f83011261087b57600080fd5b81358181111561088a57600080fd5b88602082850101111561089c57600080fd5b95986020929092019750949560400135945092505050565b600080604083850312156108c757600080fd5b823567ffffffffffffffff8111156108de57600080fd5b6108ea85828601610670565b95602094909401359450505050565b634e487b7160e01b600052601160045260246000fd5b8181038181111561063b5761063b6108f9565b8082018082111561063b5761063b6108f9565b600060018201610947576109476108f9565b5060010190565b60008261096b57634e487b7160e01b600052601260045260246000fd5b500490565b600181811c9082168061098457607f821691505b6020821081036109a457634e487b7160e01b600052602260045260246000fd5b50919050565b8183823760009101908152919050565b601f821115610a0457600081815260208120601f850160051c810160208610156109e15750805b601f850160051c820191505b81811015610a00578281556001016109ed565b5050505b505050565b67ffffffffffffffff831115610a2157610a2161065a565b610a3583610a2f8354610970565b836109ba565b6000601f841160018114610a695760008515610a515750838201355b600019600387901b1c1916600186901b178355610ac3565b600083815260209020601f19861690835b82811015610a9a5786850135825560209485019460019092019101610a7a565b5086821015610ab75760001960f88860031b161c19848701351681555b505060018560011b0183555b5050505050565b634e487b7160e01b600052603260045260246000fd5b6000808354610aee81610970565b60018281168015610b065760018114610b1b57610b4a565b60ff1984168752821515830287019450610b4a565b8760005260208060002060005b85811015610b415781548a820152908401908201610b28565b50505082870194505b50929695505050505050565b60008251610b68818460208701610744565b919091019291505056fea2646970667358221220c7bc4e4f3e9cfce1428ce0890d25517e87733bd7dbed48da09dbe9b4f2cdd32f64736f6c63430008130033" . parse () . expect ("invalid bytecode")
        });
    pub struct RewardsAggregator<M>(ethers::contract::Contract<M>);
    impl<M> Clone for RewardsAggregator<M> {
        fn clone(&self) -> Self {
            RewardsAggregator(self.0.clone())
        }
    }
    impl<M> std::ops::Deref for RewardsAggregator<M> {
        type Target = ethers::contract::Contract<M>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<M> std::fmt::Debug for RewardsAggregator<M> {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.debug_tuple(stringify!(RewardsAggregator))
                .field(&self.address())
                .finish()
        }
    }
    impl<M: ethers::providers::Middleware> RewardsAggregator<M> {
        #[doc = r" Creates a new contract instance with the specified `ethers`"]
        #[doc = r" client at the given `Address`. The contract derefs to a `ethers::Contract`"]
        #[doc = r" object"]
        pub fn new<T: Into<ethers::core::types::Address>>(
            address: T,
            client: ::std::sync::Arc<M>,
        ) -> Self {
            ethers::contract::Contract::new(address.into(), REWARDSAGGREGATOR_ABI.clone(), client)
                .into()
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
                REWARDSAGGREGATOR_ABI.clone(),
                REWARDSAGGREGATOR_BYTECODE.clone().into(),
                client,
            );
            let deployer = factory.deploy(constructor_args)?;
            let deployer = ethers::contract::ContractDeployer::new(deployer);
            Ok(deployer)
        }
        #[doc = "Calls the contract's `DataServedInBytes` (0x9d4bc36c) function"]
        pub fn data_served_in_bytes(
            &self,
            p0: ethers::core::types::U256,
            p1: String,
        ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::U256> {
            self.0
                .method_hash([157, 75, 195, 108], (p0, p1))
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `daysForAveragePotential` (0x7a3b0aff) function"]
        pub fn days_for_average_potential(
            &self,
        ) -> ethers::contract::builders::ContractCall<M, u16> {
            self.0
                .method_hash([122, 59, 10, 255], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `getAvgUsageNEpochs` (0x075d17e2) function"]
        pub fn get_avg_usage_n_epochs(
            &self,
            epoch: ethers::core::types::U256,
        ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::U256> {
            self.0
                .method_hash([7, 93, 23, 226], epoch)
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `getDataForEpoch` (0x89b1798b) function"]
        pub fn get_data_for_epoch(
            &self,
            epoch: ethers::core::types::U256,
        ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::U256> {
            self.0
                .method_hash([137, 177, 121, 139], epoch)
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `getDataServedByNode` (0xd34b0daf) function"]
        pub fn get_data_served_by_node(
            &self,
            public_key: String,
            epoch: ethers::core::types::U256,
        ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::U256> {
            self.0
                .method_hash([211, 75, 13, 175], (public_key, epoch))
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `getPublicKeys` (0x372c1aff) function"]
        pub fn get_public_keys(
            &self,
            epoch: ethers::core::types::U256,
        ) -> ethers::contract::builders::ContractCall<M, ::std::vec::Vec<String>> {
            self.0
                .method_hash([55, 44, 26, 255], epoch)
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `initialize` (0x8129fc1c) function"]
        pub fn initialize(&self) -> ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([129, 41, 252, 28], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `publicKeyAdded` (0x25e7913e) function"]
        pub fn public_key_added(
            &self,
            p0: ethers::core::types::U256,
            p1: String,
        ) -> ethers::contract::builders::ContractCall<M, bool> {
            self.0
                .method_hash([37, 231, 145, 62], (p0, p1))
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `publicKeys` (0x66f0f022) function"]
        pub fn public_keys(
            &self,
            p0: ethers::core::types::U256,
            p1: ethers::core::types::U256,
        ) -> ethers::contract::builders::ContractCall<M, String> {
            self.0
                .method_hash([102, 240, 240, 34], (p0, p1))
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `recordDataServed` (0x6b6528d7) function"]
        pub fn record_data_served(
            &self,
            epoch: ethers::core::types::U256,
            public_key: String,
            data_served: ethers::core::types::U256,
        ) -> ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([107, 101, 40, 215], (epoch, public_key, data_served))
                .expect("method not found (this should never happen)")
        }
    }
    impl<M: ethers::providers::Middleware> From<ethers::contract::Contract<M>>
        for RewardsAggregator<M>
    {
        fn from(contract: ethers::contract::Contract<M>) -> Self {
            Self(contract)
        }
    }
    #[doc = "Container type for all input parameters for the `DataServedInBytes` function with signature `DataServedInBytes(uint256,string)` and selector `[157, 75, 195, 108]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "DataServedInBytes", abi = "DataServedInBytes(uint256,string)")]
    pub struct DataServedInBytesCall(pub ethers::core::types::U256, pub String);
    #[doc = "Container type for all input parameters for the `daysForAveragePotential` function with signature `daysForAveragePotential()` and selector `[122, 59, 10, 255]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "daysForAveragePotential", abi = "daysForAveragePotential()")]
    pub struct DaysForAveragePotentialCall;
    #[doc = "Container type for all input parameters for the `getAvgUsageNEpochs` function with signature `getAvgUsageNEpochs(uint256)` and selector `[7, 93, 23, 226]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "getAvgUsageNEpochs", abi = "getAvgUsageNEpochs(uint256)")]
    pub struct GetAvgUsageNEpochsCall {
        pub epoch: ethers::core::types::U256,
    }
    #[doc = "Container type for all input parameters for the `getDataForEpoch` function with signature `getDataForEpoch(uint256)` and selector `[137, 177, 121, 139]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "getDataForEpoch", abi = "getDataForEpoch(uint256)")]
    pub struct GetDataForEpochCall {
        pub epoch: ethers::core::types::U256,
    }
    #[doc = "Container type for all input parameters for the `getDataServedByNode` function with signature `getDataServedByNode(string,uint256)` and selector `[211, 75, 13, 175]`"]
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
        name = "getDataServedByNode",
        abi = "getDataServedByNode(string,uint256)"
    )]
    pub struct GetDataServedByNodeCall {
        pub public_key: String,
        pub epoch: ethers::core::types::U256,
    }
    #[doc = "Container type for all input parameters for the `getPublicKeys` function with signature `getPublicKeys(uint256)` and selector `[55, 44, 26, 255]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "getPublicKeys", abi = "getPublicKeys(uint256)")]
    pub struct GetPublicKeysCall {
        pub epoch: ethers::core::types::U256,
    }
    #[doc = "Container type for all input parameters for the `initialize` function with signature `initialize()` and selector `[129, 41, 252, 28]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "initialize", abi = "initialize()")]
    pub struct InitializeCall;
    #[doc = "Container type for all input parameters for the `publicKeyAdded` function with signature `publicKeyAdded(uint256,string)` and selector `[37, 231, 145, 62]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "publicKeyAdded", abi = "publicKeyAdded(uint256,string)")]
    pub struct PublicKeyAddedCall(pub ethers::core::types::U256, pub String);
    #[doc = "Container type for all input parameters for the `publicKeys` function with signature `publicKeys(uint256,uint256)` and selector `[102, 240, 240, 34]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "publicKeys", abi = "publicKeys(uint256,uint256)")]
    pub struct PublicKeysCall(pub ethers::core::types::U256, pub ethers::core::types::U256);
    #[doc = "Container type for all input parameters for the `recordDataServed` function with signature `recordDataServed(uint256,string,uint256)` and selector `[107, 101, 40, 215]`"]
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
        name = "recordDataServed",
        abi = "recordDataServed(uint256,string,uint256)"
    )]
    pub struct RecordDataServedCall {
        pub epoch: ethers::core::types::U256,
        pub public_key: String,
        pub data_served: ethers::core::types::U256,
    }
    #[derive(Debug, Clone, PartialEq, Eq, ethers :: contract :: EthAbiType)]
    pub enum RewardsAggregatorCalls {
        DataServedInBytes(DataServedInBytesCall),
        DaysForAveragePotential(DaysForAveragePotentialCall),
        GetAvgUsageNEpochs(GetAvgUsageNEpochsCall),
        GetDataForEpoch(GetDataForEpochCall),
        GetDataServedByNode(GetDataServedByNodeCall),
        GetPublicKeys(GetPublicKeysCall),
        Initialize(InitializeCall),
        PublicKeyAdded(PublicKeyAddedCall),
        PublicKeys(PublicKeysCall),
        RecordDataServed(RecordDataServedCall),
    }
    impl ethers::core::abi::AbiDecode for RewardsAggregatorCalls {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::std::result::Result<Self, ethers::core::abi::AbiError> {
            if let Ok(decoded) =
                <DataServedInBytesCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(RewardsAggregatorCalls::DataServedInBytes(decoded));
            }
            if let Ok(decoded) =
                <DaysForAveragePotentialCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(RewardsAggregatorCalls::DaysForAveragePotential(decoded));
            }
            if let Ok(decoded) =
                <GetAvgUsageNEpochsCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(RewardsAggregatorCalls::GetAvgUsageNEpochs(decoded));
            }
            if let Ok(decoded) =
                <GetDataForEpochCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(RewardsAggregatorCalls::GetDataForEpoch(decoded));
            }
            if let Ok(decoded) =
                <GetDataServedByNodeCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(RewardsAggregatorCalls::GetDataServedByNode(decoded));
            }
            if let Ok(decoded) =
                <GetPublicKeysCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(RewardsAggregatorCalls::GetPublicKeys(decoded));
            }
            if let Ok(decoded) =
                <InitializeCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(RewardsAggregatorCalls::Initialize(decoded));
            }
            if let Ok(decoded) =
                <PublicKeyAddedCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(RewardsAggregatorCalls::PublicKeyAdded(decoded));
            }
            if let Ok(decoded) =
                <PublicKeysCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(RewardsAggregatorCalls::PublicKeys(decoded));
            }
            if let Ok(decoded) =
                <RecordDataServedCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(RewardsAggregatorCalls::RecordDataServed(decoded));
            }
            Err(ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ethers::core::abi::AbiEncode for RewardsAggregatorCalls {
        fn encode(self) -> Vec<u8> {
            match self {
                RewardsAggregatorCalls::DataServedInBytes(element) => element.encode(),
                RewardsAggregatorCalls::DaysForAveragePotential(element) => element.encode(),
                RewardsAggregatorCalls::GetAvgUsageNEpochs(element) => element.encode(),
                RewardsAggregatorCalls::GetDataForEpoch(element) => element.encode(),
                RewardsAggregatorCalls::GetDataServedByNode(element) => element.encode(),
                RewardsAggregatorCalls::GetPublicKeys(element) => element.encode(),
                RewardsAggregatorCalls::Initialize(element) => element.encode(),
                RewardsAggregatorCalls::PublicKeyAdded(element) => element.encode(),
                RewardsAggregatorCalls::PublicKeys(element) => element.encode(),
                RewardsAggregatorCalls::RecordDataServed(element) => element.encode(),
            }
        }
    }
    impl ::std::fmt::Display for RewardsAggregatorCalls {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match self {
                RewardsAggregatorCalls::DataServedInBytes(element) => element.fmt(f),
                RewardsAggregatorCalls::DaysForAveragePotential(element) => element.fmt(f),
                RewardsAggregatorCalls::GetAvgUsageNEpochs(element) => element.fmt(f),
                RewardsAggregatorCalls::GetDataForEpoch(element) => element.fmt(f),
                RewardsAggregatorCalls::GetDataServedByNode(element) => element.fmt(f),
                RewardsAggregatorCalls::GetPublicKeys(element) => element.fmt(f),
                RewardsAggregatorCalls::Initialize(element) => element.fmt(f),
                RewardsAggregatorCalls::PublicKeyAdded(element) => element.fmt(f),
                RewardsAggregatorCalls::PublicKeys(element) => element.fmt(f),
                RewardsAggregatorCalls::RecordDataServed(element) => element.fmt(f),
            }
        }
    }
    impl ::std::convert::From<DataServedInBytesCall> for RewardsAggregatorCalls {
        fn from(var: DataServedInBytesCall) -> Self {
            RewardsAggregatorCalls::DataServedInBytes(var)
        }
    }
    impl ::std::convert::From<DaysForAveragePotentialCall> for RewardsAggregatorCalls {
        fn from(var: DaysForAveragePotentialCall) -> Self {
            RewardsAggregatorCalls::DaysForAveragePotential(var)
        }
    }
    impl ::std::convert::From<GetAvgUsageNEpochsCall> for RewardsAggregatorCalls {
        fn from(var: GetAvgUsageNEpochsCall) -> Self {
            RewardsAggregatorCalls::GetAvgUsageNEpochs(var)
        }
    }
    impl ::std::convert::From<GetDataForEpochCall> for RewardsAggregatorCalls {
        fn from(var: GetDataForEpochCall) -> Self {
            RewardsAggregatorCalls::GetDataForEpoch(var)
        }
    }
    impl ::std::convert::From<GetDataServedByNodeCall> for RewardsAggregatorCalls {
        fn from(var: GetDataServedByNodeCall) -> Self {
            RewardsAggregatorCalls::GetDataServedByNode(var)
        }
    }
    impl ::std::convert::From<GetPublicKeysCall> for RewardsAggregatorCalls {
        fn from(var: GetPublicKeysCall) -> Self {
            RewardsAggregatorCalls::GetPublicKeys(var)
        }
    }
    impl ::std::convert::From<InitializeCall> for RewardsAggregatorCalls {
        fn from(var: InitializeCall) -> Self {
            RewardsAggregatorCalls::Initialize(var)
        }
    }
    impl ::std::convert::From<PublicKeyAddedCall> for RewardsAggregatorCalls {
        fn from(var: PublicKeyAddedCall) -> Self {
            RewardsAggregatorCalls::PublicKeyAdded(var)
        }
    }
    impl ::std::convert::From<PublicKeysCall> for RewardsAggregatorCalls {
        fn from(var: PublicKeysCall) -> Self {
            RewardsAggregatorCalls::PublicKeys(var)
        }
    }
    impl ::std::convert::From<RecordDataServedCall> for RewardsAggregatorCalls {
        fn from(var: RecordDataServedCall) -> Self {
            RewardsAggregatorCalls::RecordDataServed(var)
        }
    }
    #[doc = "Container type for all return fields from the `DataServedInBytes` function with signature `DataServedInBytes(uint256,string)` and selector `[157, 75, 195, 108]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct DataServedInBytesReturn(pub ethers::core::types::U256);
    #[doc = "Container type for all return fields from the `daysForAveragePotential` function with signature `daysForAveragePotential()` and selector `[122, 59, 10, 255]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct DaysForAveragePotentialReturn(pub u16);
    #[doc = "Container type for all return fields from the `getAvgUsageNEpochs` function with signature `getAvgUsageNEpochs(uint256)` and selector `[7, 93, 23, 226]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct GetAvgUsageNEpochsReturn(pub ethers::core::types::U256);
    #[doc = "Container type for all return fields from the `getDataForEpoch` function with signature `getDataForEpoch(uint256)` and selector `[137, 177, 121, 139]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct GetDataForEpochReturn(pub ethers::core::types::U256);
    #[doc = "Container type for all return fields from the `getDataServedByNode` function with signature `getDataServedByNode(string,uint256)` and selector `[211, 75, 13, 175]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct GetDataServedByNodeReturn(pub ethers::core::types::U256);
    #[doc = "Container type for all return fields from the `getPublicKeys` function with signature `getPublicKeys(uint256)` and selector `[55, 44, 26, 255]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct GetPublicKeysReturn(pub ::std::vec::Vec<String>);
    #[doc = "Container type for all return fields from the `publicKeyAdded` function with signature `publicKeyAdded(uint256,string)` and selector `[37, 231, 145, 62]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct PublicKeyAddedReturn(pub bool);
    #[doc = "Container type for all return fields from the `publicKeys` function with signature `publicKeys(uint256,uint256)` and selector `[102, 240, 240, 34]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct PublicKeysReturn(pub String);
}
