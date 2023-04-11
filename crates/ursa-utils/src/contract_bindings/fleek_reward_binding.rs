pub use fleek_reward::*;
#[allow(clippy::too_many_arguments, non_camel_case_types)]
pub mod fleek_reward {
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
    #[doc = "FleekReward was auto-generated with ethers-rs Abigen. More information at: https://github.com/gakonst/ethers-rs"]
    use std::sync::Arc;
    # [rustfmt :: skip] const __ABI : & str = "[{\"inputs\":[{\"internalType\":\"uint256\",\"name\":\"x\",\"type\":\"uint256\",\"components\":[]},{\"internalType\":\"uint256\",\"name\":\"y\",\"type\":\"uint256\",\"components\":[]}],\"type\":\"error\",\"name\":\"PRBMath_MulDiv18_Overflow\",\"outputs\":[]},{\"inputs\":[{\"internalType\":\"uint256\",\"name\":\"x\",\"type\":\"uint256\",\"components\":[]},{\"internalType\":\"uint256\",\"name\":\"y\",\"type\":\"uint256\",\"components\":[]},{\"internalType\":\"uint256\",\"name\":\"denominator\",\"type\":\"uint256\",\"components\":[]}],\"type\":\"error\",\"name\":\"PRBMath_MulDiv_Overflow\",\"outputs\":[]},{\"inputs\":[{\"internalType\":\"int256\",\"name\":\"x\",\"type\":\"int256\",\"components\":[]}],\"type\":\"error\",\"name\":\"PRBMath_SD59x18_Convert_Overflow\",\"outputs\":[]},{\"inputs\":[{\"internalType\":\"int256\",\"name\":\"x\",\"type\":\"int256\",\"components\":[]}],\"type\":\"error\",\"name\":\"PRBMath_SD59x18_Convert_Underflow\",\"outputs\":[]},{\"inputs\":[],\"type\":\"error\",\"name\":\"PRBMath_SD59x18_Div_InputTooSmall\",\"outputs\":[]},{\"inputs\":[{\"internalType\":\"SD59x18\",\"name\":\"x\",\"type\":\"int256\",\"components\":[]},{\"internalType\":\"SD59x18\",\"name\":\"y\",\"type\":\"int256\",\"components\":[]}],\"type\":\"error\",\"name\":\"PRBMath_SD59x18_Div_Overflow\",\"outputs\":[]},{\"inputs\":[{\"internalType\":\"SD59x18\",\"name\":\"x\",\"type\":\"int256\",\"components\":[]}],\"type\":\"error\",\"name\":\"PRBMath_SD59x18_IntoUint256_Underflow\",\"outputs\":[]},{\"inputs\":[],\"type\":\"error\",\"name\":\"PRBMath_SD59x18_Mul_InputTooSmall\",\"outputs\":[]},{\"inputs\":[{\"internalType\":\"SD59x18\",\"name\":\"x\",\"type\":\"int256\",\"components\":[]},{\"internalType\":\"SD59x18\",\"name\":\"y\",\"type\":\"int256\",\"components\":[]}],\"type\":\"error\",\"name\":\"PRBMath_SD59x18_Mul_Overflow\",\"outputs\":[]},{\"inputs\":[{\"internalType\":\"address\",\"name\":\"account\",\"type\":\"address\",\"components\":[],\"indexed\":true},{\"internalType\":\"uint256\",\"name\":\"amount\",\"type\":\"uint256\",\"components\":[],\"indexed\":false}],\"type\":\"event\",\"name\":\"RewardMinted\",\"outputs\":[],\"anonymous\":false},{\"inputs\":[{\"internalType\":\"uint256\",\"name\":\"epoch\",\"type\":\"uint256\",\"components\":[]}],\"stateMutability\":\"nonpayable\",\"type\":\"function\",\"name\":\"distributeRewards\",\"outputs\":[]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"epochManager\",\"outputs\":[{\"internalType\":\"contract EpochManager\",\"name\":\"\",\"type\":\"address\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"inflationInLastEpoch\",\"outputs\":[{\"internalType\":\"SD59x18\",\"name\":\"\",\"type\":\"int256\",\"components\":[]}]},{\"inputs\":[{\"internalType\":\"address\",\"name\":\"_fleekToken\",\"type\":\"address\",\"components\":[]},{\"internalType\":\"address\",\"name\":\"_epochManager\",\"type\":\"address\",\"components\":[]},{\"internalType\":\"address\",\"name\":\"_rewardsAggregator\",\"type\":\"address\",\"components\":[]},{\"internalType\":\"address\",\"name\":\"_nodeRegistry\",\"type\":\"address\",\"components\":[]}],\"stateMutability\":\"nonpayable\",\"type\":\"function\",\"name\":\"initialize\",\"outputs\":[]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"nodeRegistry\",\"outputs\":[{\"internalType\":\"contract NodeRegistry\",\"name\":\"\",\"type\":\"address\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"owner\",\"outputs\":[{\"internalType\":\"address\",\"name\":\"\",\"type\":\"address\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"rewardsAggregator\",\"outputs\":[{\"internalType\":\"contract RewardsAggregator\",\"name\":\"\",\"type\":\"address\",\"components\":[]}]},{\"inputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]}],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"rewardsDistribution\",\"outputs\":[{\"internalType\":\"bool\",\"name\":\"\",\"type\":\"bool\",\"components\":[]}]}]" ;
    #[doc = r" The parsed JSON-ABI of the contract."]
    pub static FLEEKREWARD_ABI: ethers::contract::Lazy<ethers::core::abi::Abi> =
        ethers::contract::Lazy::new(|| {
            ethers::core::utils::__serde_json::from_str(__ABI).expect("invalid abi")
        });
    #[doc = r" Bytecode of the #name contract"]
    pub static FLEEKREWARD_BYTECODE: ethers::contract::Lazy<ethers::core::types::Bytes> =
        ethers::contract::Lazy::new(|| {
            "0x608060405234801561001057600080fd5b50611107806100206000396000f3fe608060405234801561001057600080fd5b50600436106100885760003560e01c8063c723129d1161005b578063c723129d1461011d578063d9b5c4a514610134578063e2d2bfe314610147578063f8c8765e1461015a57600080fd5b806316f633681461008d57806351571900146100c557806359974e38146100f05780638da5cb5b14610105575b600080fd5b6100b061009b366004610cbe565b60066020526000908152604090205460ff1681565b60405190151581526020015b60405180910390f35b6001546100d8906001600160a01b031681565b6040516001600160a01b0390911681526020016100bc565b6101036100fe366004610cbe565b61016d565b005b6005546100d89061010090046001600160a01b031681565b61012660045481565b6040519081526020016100bc565b6003546100d8906001600160a01b031681565b6002546100d8906001600160a01b031681565b610103610168366004610cef565b6106c5565b60055461010090046001600160a01b031633146101db5760405162461bcd60e51b815260206004820152602160248201527f4f6e6c79206f776e65722063616e2063616c6c20746869732066756e6374696f6044820152603760f91b60648201526084015b60405180910390fd5b6002546040805163900cf0cf60e01b8152905183926001600160a01b03169163900cf0cf9160048083019260209291908290030181865afa158015610224573d6000803e3d6000fd5b505050506040513d601f19601f820116820180604052508101906102489190610d4b565b036102a95760405162461bcd60e51b815260206004820152602b60248201527f63616e6e6f742064697374726962757465207265776172647320666f7220637560448201526a0e4e4cadce840cae0dec6d60ab1b60648201526084016101d2565b60008181526006602052604090205460ff161561031b5760405162461bcd60e51b815260206004820152602a60248201527f7265776172647320616c726561647920646973747269627574656420666f72206044820152690e8d0d2e640cae0dec6d60b31b60648201526084016101d2565b6001546040516389b1798b60e01b815260048101839052600091610395916001600160a01b03909116906389b1798b906024015b602060405180830381865afa15801561036c573d6000803e3d6000fd5b505050506040513d601f19601f820116820180604052508101906103909190610d4b565b610799565b6001546040516303ae8bf160e11b8152600481018590529192506000916103ce916001600160a01b03169063075d17e29060240161034f565b905060006103dc838361081e565b905060006103f282670a688906bd8b0000610953565b60015460405163372c1aff60e01b8152600481018890529192506000916001600160a01b039091169063372c1aff90602401600060405180830381865afa158015610441573d6000803e3d6000fd5b505050506040513d6000823e601f3d908101601f191682016040526104699190810190610e3b565b905060005b81518110156106a45760015482516000916001600160a01b03169063d34b0daf908590859081106104a1576104a1610ef1565b60200260200101518a6040518363ffffffff1660e01b81526004016104c7929190610f33565b602060405180830381865afa1580156104e4573d6000803e3d6000fd5b505050506040513d601f19601f820116820180604052508101906105089190610d4b565b9050600061051f8861051984610799565b90610a20565b9050600061052d8287610953565b60035486519192506000916001600160a01b0390911690630d80bf649088908890811061055c5761055c610ef1565b60200260200101516040518263ffffffff1660e01b81526004016105809190610f55565b600060405180830381865afa15801561059d573d6000803e3d6000fd5b505050506040513d6000823e601f3d908101601f191682016040526105c59190810190610f68565b5050600054929350506001600160a01b0390911690506340c10f19826105ea85610acd565b6040516001600160e01b031960e085901b1681526001600160a01b0390921660048301526024820152604401600060405180830381600087803b15801561063057600080fd5b505af1158015610644573d6000803e3d6000fd5b50505050806001600160a01b03167fe8ea3d4dd0a2eaaf3f7532ad391255544f8a4bcf78f850bbff61d5bac9f7755261067c84610acd565b60405190815260200160405180910390a250505050808061069c9061103f565b91505061046e565b5050506000938452505060066020525060409020805460ff19166001179055565b60055460ff16156107245760405162461bcd60e51b8152602060048201526024808201527f5265776172647320636f6e747261637420616c726561647920696e697469616c6044820152631a5e995960e21b60648201526084016101d2565b60058054600080546001600160a01b03199081166001600160a01b0398891617909155600380548216948816949094179093556002805484169587169590951790945560018054909216929094169190911781556008546004556001600160a81b0319909116336101000260ff191617179055565b60006107b1670de0b6b3a7640000600160ff1b61106e565b8212156107d4576040516399474eeb60e01b8152600481018390526024016101d2565b6107ed670de0b6b3a76400006001600160ff1b0361106e565b82131561081057604051639d58109160e01b8152600481018390526024016101d2565b50670de0b6b3a76400000290565b60008061082b8484610afa565b905060006108398285610a20565b905060006108706108618361085b600a54600954610a2090919063ffffffff16565b90610953565b670de0b6b3a764000090610afa565b9050600061088661088383600754610b10565b90565b9050600061089f6004548361095390919063ffffffff16565b905060006108b261088383600854610b27565b600481815560008054604080516318160ddd60e01b8152905194955091936001600160a01b03909116926318160ddd928082019260209290918290030181865afa158015610904573d6000803e3d6000fd5b505050506040513d601f19601f820116820180604052508101906109289190610d4b565b905061094561093861016d610799565b6105198461085b85610799565b9a9950505050505050505050565b60008282600160ff1b82148061096c5750600160ff1b81145b1561098a5760405163a6070c2560e01b815260040160405180910390fd5b6000806000841261099b57836109a0565b836000035b9150600083126109b057826109b5565b826000035b905060006109c38383610b37565b90506001600160ff1b038111156109f75760405163120b5b4360e01b815260048101899052602481018890526044016101d2565b60001985851813610a1381610a0f5782600003610883565b8290565b9998505050505050505050565b60008282600160ff1b821480610a395750600160ff1b81145b15610a57576040516309fe2b4560e41b815260040160405180910390fd5b60008060008412610a685783610a6d565b836000035b915060008312610a7d5782610a82565b826000035b90506000610a9983670de0b6b3a764000084610beb565b90506001600160ff1b038111156109f75760405163d49c26b360e01b815260048101899052602481018890526044016101d2565b60008181811215610af457604051632463f3d560e01b8152600481018490526024016101d2565b92915050565b6000610b0961088383856110aa565b9392505050565b600081831215610b205781610b09565b5090919050565b600081831315610b205781610b09565b60008080600019848609848602925082811083820303915050670de0b6b3a76400008110610b8257604051635173648d60e01b815260048101869052602481018590526044016101d2565b6000670de0b6b3a7640000858709905081600003610bae575050670de0b6b3a764000090049050610af4565b620400008184030492109003600160ee1b02177faccb18165bd6fe31ae1cf318dc5b51eee0e1ba569b88cd74c1773b91fac1066902905092915050565b6000808060001985870985870292508281108382030391505080600003610c2557838281610c1b57610c1b611058565b0492505050610b09565b838110610c5657604051630c740aef60e31b81526004810187905260248101869052604481018590526064016101d2565b600084868809600260036001881981018916988990049182028318808302840302808302840302808302840302808302840302808302840302918202909203026000889003889004909101858311909403939093029303949094049190911702949350505050565b600060208284031215610cd057600080fd5b5035919050565b6001600160a01b0381168114610cec57600080fd5b50565b60008060008060808587031215610d0557600080fd5b8435610d1081610cd7565b93506020850135610d2081610cd7565b92506040850135610d3081610cd7565b91506060850135610d4081610cd7565b939692955090935050565b600060208284031215610d5d57600080fd5b5051919050565b634e487b7160e01b600052604160045260246000fd5b604051601f8201601f1916810167ffffffffffffffff81118282101715610da357610da3610d64565b604052919050565b60005b83811015610dc6578181015183820152602001610dae565b50506000910152565b600082601f830112610de057600080fd5b815167ffffffffffffffff811115610dfa57610dfa610d64565b610e0d601f8201601f1916602001610d7a565b818152846020838601011115610e2257600080fd5b610e33826020830160208701610dab565b949350505050565b60006020808385031215610e4e57600080fd5b825167ffffffffffffffff80821115610e6657600080fd5b818501915085601f830112610e7a57600080fd5b815181811115610e8c57610e8c610d64565b8060051b610e9b858201610d7a565b9182528381018501918581019089841115610eb557600080fd5b86860192505b83831015610a1357825185811115610ed35760008081fd5b610ee18b89838a0101610dcf565b8352509186019190860190610ebb565b634e487b7160e01b600052603260045260246000fd5b60008151808452610f1f816020860160208601610dab565b601f01601f19169290920160200192915050565b604081526000610f466040830185610f07565b90508260208301529392505050565b602081526000610b096020830184610f07565b600080600080600060a08688031215610f8057600080fd5b8551610f8b81610cd7565b602087015190955067ffffffffffffffff80821115610fa957600080fd5b610fb589838a01610dcf565b95506040880151915080821115610fcb57600080fd5b610fd789838a01610dcf565b94506060880151915080821115610fed57600080fd5b610ff989838a01610dcf565b9350608088015191508082111561100f57600080fd5b5061101c88828901610dcf565b9150509295509295909350565b634e487b7160e01b600052601160045260246000fd5b60006001820161105157611051611029565b5060010190565b634e487b7160e01b600052601260045260246000fd5b60008261108b57634e487b7160e01b600052601260045260246000fd5b600160ff1b8214600019841416156110a5576110a5611029565b500590565b81810360008312801583831316838312821617156110ca576110ca611029565b509291505056fea264697066735822122014e7aa082356c43542adc68efc39ab62188b58eaaea2aaf2efb22149e86d399f64736f6c63430008130033" . parse () . expect ("invalid bytecode")
        });
    pub struct FleekReward<M>(ethers::contract::Contract<M>);
    impl<M> Clone for FleekReward<M> {
        fn clone(&self) -> Self {
            FleekReward(self.0.clone())
        }
    }
    impl<M> std::ops::Deref for FleekReward<M> {
        type Target = ethers::contract::Contract<M>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<M> std::fmt::Debug for FleekReward<M> {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.debug_tuple(stringify!(FleekReward))
                .field(&self.address())
                .finish()
        }
    }
    impl<M: ethers::providers::Middleware> FleekReward<M> {
        #[doc = r" Creates a new contract instance with the specified `ethers`"]
        #[doc = r" client at the given `Address`. The contract derefs to a `ethers::Contract`"]
        #[doc = r" object"]
        pub fn new<T: Into<ethers::core::types::Address>>(
            address: T,
            client: ::std::sync::Arc<M>,
        ) -> Self {
            ethers::contract::Contract::new(address.into(), FLEEKREWARD_ABI.clone(), client).into()
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
                FLEEKREWARD_ABI.clone(),
                FLEEKREWARD_BYTECODE.clone().into(),
                client,
            );
            let deployer = factory.deploy(constructor_args)?;
            let deployer = ethers::contract::ContractDeployer::new(deployer);
            Ok(deployer)
        }
        #[doc = "Calls the contract's `distributeRewards` (0x59974e38) function"]
        pub fn distribute_rewards(
            &self,
            epoch: ethers::core::types::U256,
        ) -> ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([89, 151, 78, 56], epoch)
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `epochManager` (0xe2d2bfe3) function"]
        pub fn epoch_manager(
            &self,
        ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::Address> {
            self.0
                .method_hash([226, 210, 191, 227], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `inflationInLastEpoch` (0xc723129d) function"]
        pub fn inflation_in_last_epoch(&self) -> ethers::contract::builders::ContractCall<M, I256> {
            self.0
                .method_hash([199, 35, 18, 157], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `initialize` (0xf8c8765e) function"]
        pub fn initialize(
            &self,
            fleek_token: ethers::core::types::Address,
            epoch_manager: ethers::core::types::Address,
            rewards_aggregator: ethers::core::types::Address,
            node_registry: ethers::core::types::Address,
        ) -> ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash(
                    [248, 200, 118, 94],
                    (
                        fleek_token,
                        epoch_manager,
                        rewards_aggregator,
                        node_registry,
                    ),
                )
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
        #[doc = "Calls the contract's `owner` (0x8da5cb5b) function"]
        pub fn owner(
            &self,
        ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::Address> {
            self.0
                .method_hash([141, 165, 203, 91], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `rewardsAggregator` (0x51571900) function"]
        pub fn rewards_aggregator(
            &self,
        ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::Address> {
            self.0
                .method_hash([81, 87, 25, 0], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `rewardsDistribution` (0x16f63368) function"]
        pub fn rewards_distribution(
            &self,
            p0: ethers::core::types::U256,
        ) -> ethers::contract::builders::ContractCall<M, bool> {
            self.0
                .method_hash([22, 246, 51, 104], p0)
                .expect("method not found (this should never happen)")
        }
        #[doc = "Gets the contract's `RewardMinted` event"]
        pub fn reward_minted_filter(
            &self,
        ) -> ethers::contract::builders::Event<M, RewardMintedFilter> {
            self.0.event()
        }
        #[doc = r" Returns an [`Event`](#ethers_contract::builders::Event) builder for all events of this contract"]
        pub fn events(&self) -> ethers::contract::builders::Event<M, RewardMintedFilter> {
            self.0.event_with_filter(Default::default())
        }
    }
    impl<M: ethers::providers::Middleware> From<ethers::contract::Contract<M>> for FleekReward<M> {
        fn from(contract: ethers::contract::Contract<M>) -> Self {
            Self(contract)
        }
    }
    #[doc = "Custom Error type `PRBMath_MulDiv18_Overflow` with signature `PRBMath_MulDiv18_Overflow(uint256,uint256)` and selector `[81, 115, 100, 141]`"]
    #[derive(
        Clone,
        Debug,
        Default,
        Eq,
        PartialEq,
        ethers :: contract :: EthError,
        ethers :: contract :: EthDisplay,
    )]
    #[etherror(
        name = "PRBMath_MulDiv18_Overflow",
        abi = "PRBMath_MulDiv18_Overflow(uint256,uint256)"
    )]
    pub struct PRBMath_MulDiv18_Overflow {
        pub x: ethers::core::types::U256,
        pub y: ethers::core::types::U256,
    }
    #[doc = "Custom Error type `PRBMath_MulDiv_Overflow` with signature `PRBMath_MulDiv_Overflow(uint256,uint256,uint256)` and selector `[99, 160, 87, 120]`"]
    #[derive(
        Clone,
        Debug,
        Default,
        Eq,
        PartialEq,
        ethers :: contract :: EthError,
        ethers :: contract :: EthDisplay,
    )]
    #[etherror(
        name = "PRBMath_MulDiv_Overflow",
        abi = "PRBMath_MulDiv_Overflow(uint256,uint256,uint256)"
    )]
    pub struct PRBMath_MulDiv_Overflow {
        pub x: ethers::core::types::U256,
        pub y: ethers::core::types::U256,
        pub denominator: ethers::core::types::U256,
    }
    #[doc = "Custom Error type `PRBMath_SD59x18_Convert_Overflow` with signature `PRBMath_SD59x18_Convert_Overflow(int256)` and selector `[157, 88, 16, 145]`"]
    #[derive(
        Clone,
        Debug,
        Default,
        Eq,
        PartialEq,
        ethers :: contract :: EthError,
        ethers :: contract :: EthDisplay,
    )]
    #[etherror(
        name = "PRBMath_SD59x18_Convert_Overflow",
        abi = "PRBMath_SD59x18_Convert_Overflow(int256)"
    )]
    pub struct PRBMath_SD59x18_Convert_Overflow {
        pub x: I256,
    }
    #[doc = "Custom Error type `PRBMath_SD59x18_Convert_Underflow` with signature `PRBMath_SD59x18_Convert_Underflow(int256)` and selector `[153, 71, 78, 235]`"]
    #[derive(
        Clone,
        Debug,
        Default,
        Eq,
        PartialEq,
        ethers :: contract :: EthError,
        ethers :: contract :: EthDisplay,
    )]
    #[etherror(
        name = "PRBMath_SD59x18_Convert_Underflow",
        abi = "PRBMath_SD59x18_Convert_Underflow(int256)"
    )]
    pub struct PRBMath_SD59x18_Convert_Underflow {
        pub x: I256,
    }
    #[doc = "Custom Error type `PRBMath_SD59x18_Div_InputTooSmall` with signature `PRBMath_SD59x18_Div_InputTooSmall()` and selector `[159, 226, 180, 80]`"]
    #[derive(
        Clone,
        Debug,
        Default,
        Eq,
        PartialEq,
        ethers :: contract :: EthError,
        ethers :: contract :: EthDisplay,
    )]
    #[etherror(
        name = "PRBMath_SD59x18_Div_InputTooSmall",
        abi = "PRBMath_SD59x18_Div_InputTooSmall()"
    )]
    pub struct PRBMath_SD59x18_Div_InputTooSmall;
    #[doc = "Custom Error type `PRBMath_SD59x18_Div_Overflow` with signature `PRBMath_SD59x18_Div_Overflow(int256,int256)` and selector `[212, 156, 38, 179]`"]
    #[derive(
        Clone,
        Debug,
        Default,
        Eq,
        PartialEq,
        ethers :: contract :: EthError,
        ethers :: contract :: EthDisplay,
    )]
    #[etherror(
        name = "PRBMath_SD59x18_Div_Overflow",
        abi = "PRBMath_SD59x18_Div_Overflow(int256,int256)"
    )]
    pub struct PRBMath_SD59x18_Div_Overflow {
        pub x: I256,
        pub y: I256,
    }
    #[doc = "Custom Error type `PRBMath_SD59x18_IntoUint256_Underflow` with signature `PRBMath_SD59x18_IntoUint256_Underflow(int256)` and selector `[36, 99, 243, 213]`"]
    #[derive(
        Clone,
        Debug,
        Default,
        Eq,
        PartialEq,
        ethers :: contract :: EthError,
        ethers :: contract :: EthDisplay,
    )]
    #[etherror(
        name = "PRBMath_SD59x18_IntoUint256_Underflow",
        abi = "PRBMath_SD59x18_IntoUint256_Underflow(int256)"
    )]
    pub struct PRBMath_SD59x18_IntoUint256_Underflow {
        pub x: I256,
    }
    #[doc = "Custom Error type `PRBMath_SD59x18_Mul_InputTooSmall` with signature `PRBMath_SD59x18_Mul_InputTooSmall()` and selector `[166, 7, 12, 37]`"]
    #[derive(
        Clone,
        Debug,
        Default,
        Eq,
        PartialEq,
        ethers :: contract :: EthError,
        ethers :: contract :: EthDisplay,
    )]
    #[etherror(
        name = "PRBMath_SD59x18_Mul_InputTooSmall",
        abi = "PRBMath_SD59x18_Mul_InputTooSmall()"
    )]
    pub struct PRBMath_SD59x18_Mul_InputTooSmall;
    #[doc = "Custom Error type `PRBMath_SD59x18_Mul_Overflow` with signature `PRBMath_SD59x18_Mul_Overflow(int256,int256)` and selector `[18, 11, 91, 67]`"]
    #[derive(
        Clone,
        Debug,
        Default,
        Eq,
        PartialEq,
        ethers :: contract :: EthError,
        ethers :: contract :: EthDisplay,
    )]
    #[etherror(
        name = "PRBMath_SD59x18_Mul_Overflow",
        abi = "PRBMath_SD59x18_Mul_Overflow(int256,int256)"
    )]
    pub struct PRBMath_SD59x18_Mul_Overflow {
        pub x: I256,
        pub y: I256,
    }
    #[derive(Debug, Clone, PartialEq, Eq, ethers :: contract :: EthAbiType)]
    pub enum FleekRewardErrors {
        PRBMath_MulDiv18_Overflow(PRBMath_MulDiv18_Overflow),
        PRBMath_MulDiv_Overflow(PRBMath_MulDiv_Overflow),
        PRBMath_SD59x18_Convert_Overflow(PRBMath_SD59x18_Convert_Overflow),
        PRBMath_SD59x18_Convert_Underflow(PRBMath_SD59x18_Convert_Underflow),
        PRBMath_SD59x18_Div_InputTooSmall(PRBMath_SD59x18_Div_InputTooSmall),
        PRBMath_SD59x18_Div_Overflow(PRBMath_SD59x18_Div_Overflow),
        PRBMath_SD59x18_IntoUint256_Underflow(PRBMath_SD59x18_IntoUint256_Underflow),
        PRBMath_SD59x18_Mul_InputTooSmall(PRBMath_SD59x18_Mul_InputTooSmall),
        PRBMath_SD59x18_Mul_Overflow(PRBMath_SD59x18_Mul_Overflow),
    }
    impl ethers::core::abi::AbiDecode for FleekRewardErrors {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::std::result::Result<Self, ethers::core::abi::AbiError> {
            if let Ok(decoded) =
                <PRBMath_MulDiv18_Overflow as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(FleekRewardErrors::PRBMath_MulDiv18_Overflow(decoded));
            }
            if let Ok(decoded) =
                <PRBMath_MulDiv_Overflow as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(FleekRewardErrors::PRBMath_MulDiv_Overflow(decoded));
            }
            if let Ok(decoded) =
                <PRBMath_SD59x18_Convert_Overflow as ethers::core::abi::AbiDecode>::decode(
                    data.as_ref(),
                )
            {
                return Ok(FleekRewardErrors::PRBMath_SD59x18_Convert_Overflow(decoded));
            }
            if let Ok(decoded) =
                <PRBMath_SD59x18_Convert_Underflow as ethers::core::abi::AbiDecode>::decode(
                    data.as_ref(),
                )
            {
                return Ok(FleekRewardErrors::PRBMath_SD59x18_Convert_Underflow(
                    decoded,
                ));
            }
            if let Ok(decoded) =
                <PRBMath_SD59x18_Div_InputTooSmall as ethers::core::abi::AbiDecode>::decode(
                    data.as_ref(),
                )
            {
                return Ok(FleekRewardErrors::PRBMath_SD59x18_Div_InputTooSmall(
                    decoded,
                ));
            }
            if let Ok(decoded) =
                <PRBMath_SD59x18_Div_Overflow as ethers::core::abi::AbiDecode>::decode(
                    data.as_ref(),
                )
            {
                return Ok(FleekRewardErrors::PRBMath_SD59x18_Div_Overflow(decoded));
            }
            if let Ok(decoded) =
                <PRBMath_SD59x18_IntoUint256_Underflow as ethers::core::abi::AbiDecode>::decode(
                    data.as_ref(),
                )
            {
                return Ok(FleekRewardErrors::PRBMath_SD59x18_IntoUint256_Underflow(
                    decoded,
                ));
            }
            if let Ok(decoded) =
                <PRBMath_SD59x18_Mul_InputTooSmall as ethers::core::abi::AbiDecode>::decode(
                    data.as_ref(),
                )
            {
                return Ok(FleekRewardErrors::PRBMath_SD59x18_Mul_InputTooSmall(
                    decoded,
                ));
            }
            if let Ok(decoded) =
                <PRBMath_SD59x18_Mul_Overflow as ethers::core::abi::AbiDecode>::decode(
                    data.as_ref(),
                )
            {
                return Ok(FleekRewardErrors::PRBMath_SD59x18_Mul_Overflow(decoded));
            }
            Err(ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ethers::core::abi::AbiEncode for FleekRewardErrors {
        fn encode(self) -> Vec<u8> {
            match self {
                FleekRewardErrors::PRBMath_MulDiv18_Overflow(element) => element.encode(),
                FleekRewardErrors::PRBMath_MulDiv_Overflow(element) => element.encode(),
                FleekRewardErrors::PRBMath_SD59x18_Convert_Overflow(element) => element.encode(),
                FleekRewardErrors::PRBMath_SD59x18_Convert_Underflow(element) => element.encode(),
                FleekRewardErrors::PRBMath_SD59x18_Div_InputTooSmall(element) => element.encode(),
                FleekRewardErrors::PRBMath_SD59x18_Div_Overflow(element) => element.encode(),
                FleekRewardErrors::PRBMath_SD59x18_IntoUint256_Underflow(element) => {
                    element.encode()
                }
                FleekRewardErrors::PRBMath_SD59x18_Mul_InputTooSmall(element) => element.encode(),
                FleekRewardErrors::PRBMath_SD59x18_Mul_Overflow(element) => element.encode(),
            }
        }
    }
    impl ::std::fmt::Display for FleekRewardErrors {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match self {
                FleekRewardErrors::PRBMath_MulDiv18_Overflow(element) => element.fmt(f),
                FleekRewardErrors::PRBMath_MulDiv_Overflow(element) => element.fmt(f),
                FleekRewardErrors::PRBMath_SD59x18_Convert_Overflow(element) => element.fmt(f),
                FleekRewardErrors::PRBMath_SD59x18_Convert_Underflow(element) => element.fmt(f),
                FleekRewardErrors::PRBMath_SD59x18_Div_InputTooSmall(element) => element.fmt(f),
                FleekRewardErrors::PRBMath_SD59x18_Div_Overflow(element) => element.fmt(f),
                FleekRewardErrors::PRBMath_SD59x18_IntoUint256_Underflow(element) => element.fmt(f),
                FleekRewardErrors::PRBMath_SD59x18_Mul_InputTooSmall(element) => element.fmt(f),
                FleekRewardErrors::PRBMath_SD59x18_Mul_Overflow(element) => element.fmt(f),
            }
        }
    }
    impl ::std::convert::From<PRBMath_MulDiv18_Overflow> for FleekRewardErrors {
        fn from(var: PRBMath_MulDiv18_Overflow) -> Self {
            FleekRewardErrors::PRBMath_MulDiv18_Overflow(var)
        }
    }
    impl ::std::convert::From<PRBMath_MulDiv_Overflow> for FleekRewardErrors {
        fn from(var: PRBMath_MulDiv_Overflow) -> Self {
            FleekRewardErrors::PRBMath_MulDiv_Overflow(var)
        }
    }
    impl ::std::convert::From<PRBMath_SD59x18_Convert_Overflow> for FleekRewardErrors {
        fn from(var: PRBMath_SD59x18_Convert_Overflow) -> Self {
            FleekRewardErrors::PRBMath_SD59x18_Convert_Overflow(var)
        }
    }
    impl ::std::convert::From<PRBMath_SD59x18_Convert_Underflow> for FleekRewardErrors {
        fn from(var: PRBMath_SD59x18_Convert_Underflow) -> Self {
            FleekRewardErrors::PRBMath_SD59x18_Convert_Underflow(var)
        }
    }
    impl ::std::convert::From<PRBMath_SD59x18_Div_InputTooSmall> for FleekRewardErrors {
        fn from(var: PRBMath_SD59x18_Div_InputTooSmall) -> Self {
            FleekRewardErrors::PRBMath_SD59x18_Div_InputTooSmall(var)
        }
    }
    impl ::std::convert::From<PRBMath_SD59x18_Div_Overflow> for FleekRewardErrors {
        fn from(var: PRBMath_SD59x18_Div_Overflow) -> Self {
            FleekRewardErrors::PRBMath_SD59x18_Div_Overflow(var)
        }
    }
    impl ::std::convert::From<PRBMath_SD59x18_IntoUint256_Underflow> for FleekRewardErrors {
        fn from(var: PRBMath_SD59x18_IntoUint256_Underflow) -> Self {
            FleekRewardErrors::PRBMath_SD59x18_IntoUint256_Underflow(var)
        }
    }
    impl ::std::convert::From<PRBMath_SD59x18_Mul_InputTooSmall> for FleekRewardErrors {
        fn from(var: PRBMath_SD59x18_Mul_InputTooSmall) -> Self {
            FleekRewardErrors::PRBMath_SD59x18_Mul_InputTooSmall(var)
        }
    }
    impl ::std::convert::From<PRBMath_SD59x18_Mul_Overflow> for FleekRewardErrors {
        fn from(var: PRBMath_SD59x18_Mul_Overflow) -> Self {
            FleekRewardErrors::PRBMath_SD59x18_Mul_Overflow(var)
        }
    }
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthEvent,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethevent(name = "RewardMinted", abi = "RewardMinted(address,uint256)")]
    pub struct RewardMintedFilter {
        #[ethevent(indexed)]
        pub account: ethers::core::types::Address,
        pub amount: ethers::core::types::U256,
    }
    #[doc = "Container type for all input parameters for the `distributeRewards` function with signature `distributeRewards(uint256)` and selector `[89, 151, 78, 56]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "distributeRewards", abi = "distributeRewards(uint256)")]
    pub struct DistributeRewardsCall {
        pub epoch: ethers::core::types::U256,
    }
    #[doc = "Container type for all input parameters for the `epochManager` function with signature `epochManager()` and selector `[226, 210, 191, 227]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "epochManager", abi = "epochManager()")]
    pub struct EpochManagerCall;
    #[doc = "Container type for all input parameters for the `inflationInLastEpoch` function with signature `inflationInLastEpoch()` and selector `[199, 35, 18, 157]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "inflationInLastEpoch", abi = "inflationInLastEpoch()")]
    pub struct InflationInLastEpochCall;
    #[doc = "Container type for all input parameters for the `initialize` function with signature `initialize(address,address,address,address)` and selector `[248, 200, 118, 94]`"]
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
        abi = "initialize(address,address,address,address)"
    )]
    pub struct InitializeCall {
        pub fleek_token: ethers::core::types::Address,
        pub epoch_manager: ethers::core::types::Address,
        pub rewards_aggregator: ethers::core::types::Address,
        pub node_registry: ethers::core::types::Address,
    }
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
    #[doc = "Container type for all input parameters for the `owner` function with signature `owner()` and selector `[141, 165, 203, 91]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "owner", abi = "owner()")]
    pub struct OwnerCall;
    #[doc = "Container type for all input parameters for the `rewardsAggregator` function with signature `rewardsAggregator()` and selector `[81, 87, 25, 0]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "rewardsAggregator", abi = "rewardsAggregator()")]
    pub struct RewardsAggregatorCall;
    #[doc = "Container type for all input parameters for the `rewardsDistribution` function with signature `rewardsDistribution(uint256)` and selector `[22, 246, 51, 104]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "rewardsDistribution", abi = "rewardsDistribution(uint256)")]
    pub struct RewardsDistributionCall(pub ethers::core::types::U256);
    #[derive(Debug, Clone, PartialEq, Eq, ethers :: contract :: EthAbiType)]
    pub enum FleekRewardCalls {
        DistributeRewards(DistributeRewardsCall),
        EpochManager(EpochManagerCall),
        InflationInLastEpoch(InflationInLastEpochCall),
        Initialize(InitializeCall),
        NodeRegistry(NodeRegistryCall),
        Owner(OwnerCall),
        RewardsAggregator(RewardsAggregatorCall),
        RewardsDistribution(RewardsDistributionCall),
    }
    impl ethers::core::abi::AbiDecode for FleekRewardCalls {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::std::result::Result<Self, ethers::core::abi::AbiError> {
            if let Ok(decoded) =
                <DistributeRewardsCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(FleekRewardCalls::DistributeRewards(decoded));
            }
            if let Ok(decoded) =
                <EpochManagerCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(FleekRewardCalls::EpochManager(decoded));
            }
            if let Ok(decoded) =
                <InflationInLastEpochCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(FleekRewardCalls::InflationInLastEpoch(decoded));
            }
            if let Ok(decoded) =
                <InitializeCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(FleekRewardCalls::Initialize(decoded));
            }
            if let Ok(decoded) =
                <NodeRegistryCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(FleekRewardCalls::NodeRegistry(decoded));
            }
            if let Ok(decoded) = <OwnerCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(FleekRewardCalls::Owner(decoded));
            }
            if let Ok(decoded) =
                <RewardsAggregatorCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(FleekRewardCalls::RewardsAggregator(decoded));
            }
            if let Ok(decoded) =
                <RewardsDistributionCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(FleekRewardCalls::RewardsDistribution(decoded));
            }
            Err(ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ethers::core::abi::AbiEncode for FleekRewardCalls {
        fn encode(self) -> Vec<u8> {
            match self {
                FleekRewardCalls::DistributeRewards(element) => element.encode(),
                FleekRewardCalls::EpochManager(element) => element.encode(),
                FleekRewardCalls::InflationInLastEpoch(element) => element.encode(),
                FleekRewardCalls::Initialize(element) => element.encode(),
                FleekRewardCalls::NodeRegistry(element) => element.encode(),
                FleekRewardCalls::Owner(element) => element.encode(),
                FleekRewardCalls::RewardsAggregator(element) => element.encode(),
                FleekRewardCalls::RewardsDistribution(element) => element.encode(),
            }
        }
    }
    impl ::std::fmt::Display for FleekRewardCalls {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match self {
                FleekRewardCalls::DistributeRewards(element) => element.fmt(f),
                FleekRewardCalls::EpochManager(element) => element.fmt(f),
                FleekRewardCalls::InflationInLastEpoch(element) => element.fmt(f),
                FleekRewardCalls::Initialize(element) => element.fmt(f),
                FleekRewardCalls::NodeRegistry(element) => element.fmt(f),
                FleekRewardCalls::Owner(element) => element.fmt(f),
                FleekRewardCalls::RewardsAggregator(element) => element.fmt(f),
                FleekRewardCalls::RewardsDistribution(element) => element.fmt(f),
            }
        }
    }
    impl ::std::convert::From<DistributeRewardsCall> for FleekRewardCalls {
        fn from(var: DistributeRewardsCall) -> Self {
            FleekRewardCalls::DistributeRewards(var)
        }
    }
    impl ::std::convert::From<EpochManagerCall> for FleekRewardCalls {
        fn from(var: EpochManagerCall) -> Self {
            FleekRewardCalls::EpochManager(var)
        }
    }
    impl ::std::convert::From<InflationInLastEpochCall> for FleekRewardCalls {
        fn from(var: InflationInLastEpochCall) -> Self {
            FleekRewardCalls::InflationInLastEpoch(var)
        }
    }
    impl ::std::convert::From<InitializeCall> for FleekRewardCalls {
        fn from(var: InitializeCall) -> Self {
            FleekRewardCalls::Initialize(var)
        }
    }
    impl ::std::convert::From<NodeRegistryCall> for FleekRewardCalls {
        fn from(var: NodeRegistryCall) -> Self {
            FleekRewardCalls::NodeRegistry(var)
        }
    }
    impl ::std::convert::From<OwnerCall> for FleekRewardCalls {
        fn from(var: OwnerCall) -> Self {
            FleekRewardCalls::Owner(var)
        }
    }
    impl ::std::convert::From<RewardsAggregatorCall> for FleekRewardCalls {
        fn from(var: RewardsAggregatorCall) -> Self {
            FleekRewardCalls::RewardsAggregator(var)
        }
    }
    impl ::std::convert::From<RewardsDistributionCall> for FleekRewardCalls {
        fn from(var: RewardsDistributionCall) -> Self {
            FleekRewardCalls::RewardsDistribution(var)
        }
    }
    #[doc = "Container type for all return fields from the `epochManager` function with signature `epochManager()` and selector `[226, 210, 191, 227]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct EpochManagerReturn(pub ethers::core::types::Address);
    #[doc = "Container type for all return fields from the `inflationInLastEpoch` function with signature `inflationInLastEpoch()` and selector `[199, 35, 18, 157]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct InflationInLastEpochReturn(pub I256);
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
    #[doc = "Container type for all return fields from the `owner` function with signature `owner()` and selector `[141, 165, 203, 91]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct OwnerReturn(pub ethers::core::types::Address);
    #[doc = "Container type for all return fields from the `rewardsAggregator` function with signature `rewardsAggregator()` and selector `[81, 87, 25, 0]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct RewardsAggregatorReturn(pub ethers::core::types::Address);
    #[doc = "Container type for all return fields from the `rewardsDistribution` function with signature `rewardsDistribution(uint256)` and selector `[22, 246, 51, 104]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct RewardsDistributionReturn(pub bool);
}
