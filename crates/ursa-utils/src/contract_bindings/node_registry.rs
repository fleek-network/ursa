pub use node_registry::*;
#[allow(clippy::too_many_arguments, non_camel_case_types)]
pub mod node_registry {
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
    #[doc = "NodeRegistry was auto-generated with ethers-rs Abigen. More information at: https://github.com/gakonst/ethers-rs"]
    use std::sync::Arc;
    # [rustfmt :: skip] const __ABI : & str = "[{\"inputs\":[{\"internalType\":\"string\",\"name\":\"nodeAddress\",\"type\":\"string\",\"components\":[]}],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"getNodeInfo\",\"outputs\":[{\"internalType\":\"struct MockNodeRegistry.Node\",\"name\":\"\",\"type\":\"tuple\",\"components\":[{\"internalType\":\"address\",\"name\":\"owner\",\"type\":\"address\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"primaryAddress\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"workerAddress\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"workerPublicKey\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"workerMempool\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"networkKey\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"previous\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"next\",\"type\":\"string\",\"components\":[]}]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"getWhitelist\",\"outputs\":[{\"internalType\":\"string[]\",\"name\":\"_whitelist\",\"type\":\"string[]\",\"components\":[]}]},{\"inputs\":[{\"internalType\":\"struct MockNodeRegistry.NodeInfo[]\",\"name\":\"_genesis_committee\",\"type\":\"tuple[]\",\"components\":[{\"internalType\":\"address\",\"name\":\"owner\",\"type\":\"address\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"primaryPublicKey\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"primaryAddress\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"networkKey\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"workerAddress\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"workerPublicKey\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"workerMempool\",\"type\":\"string\",\"components\":[]}]}],\"stateMutability\":\"nonpayable\",\"type\":\"function\",\"name\":\"initialize\",\"outputs\":[]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"linkedListHead\",\"outputs\":[{\"internalType\":\"string\",\"name\":\"\",\"type\":\"string\",\"components\":[]}]},{\"inputs\":[{\"internalType\":\"struct MockNodeRegistry.NodeInfo\",\"name\":\"_node\",\"type\":\"tuple\",\"components\":[{\"internalType\":\"address\",\"name\":\"owner\",\"type\":\"address\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"primaryPublicKey\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"primaryAddress\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"networkKey\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"workerAddress\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"workerPublicKey\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"workerMempool\",\"type\":\"string\",\"components\":[]}]}],\"stateMutability\":\"nonpayable\",\"type\":\"function\",\"name\":\"registerNode\",\"outputs\":[]},{\"inputs\":[{\"internalType\":\"string\",\"name\":\"_nodeAddress\",\"type\":\"string\",\"components\":[]}],\"stateMutability\":\"nonpayable\",\"type\":\"function\",\"name\":\"removeNode\",\"outputs\":[]},{\"inputs\":[{\"internalType\":\"string\",\"name\":\"\",\"type\":\"string\",\"components\":[]}],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"whitelist\",\"outputs\":[{\"internalType\":\"address\",\"name\":\"owner\",\"type\":\"address\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"primaryAddress\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"workerAddress\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"workerPublicKey\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"workerMempool\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"networkKey\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"previous\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"next\",\"type\":\"string\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"whitelistCount\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]}]}]" ;
    #[doc = r" The parsed JSON-ABI of the contract."]
    pub static NODEREGISTRY_ABI: ethers::contract::Lazy<ethers::core::abi::Abi> =
        ethers::contract::Lazy::new(|| {
            ethers::core::utils::__serde_json::from_str(__ABI).expect("invalid abi")
        });
    #[doc = r" Bytecode of the #name contract"]
    pub static NODEREGISTRY_BYTECODE: ethers::contract::Lazy<ethers::core::types::Bytes> =
        ethers::contract::Lazy::new(|| {
            "0x608060405234801561001057600080fd5b50611b82806100206000396000f3fe608060405234801561001057600080fd5b50600436106100885760003560e01c80638739b3a61161005b5780638739b3a614610105578063a2c6ddeb14610118578063d01f63f51461012d578063f2624b5d1461014257600080fd5b80630d80bf641461008d578063239e3e07146100bd5780634665cb07146100d2578063813775a3146100e5575b600080fd5b6100a061009b36600461130f565b610159565b6040516100b498979695949392919061139c565b60405180910390f35b6100d06100cb366004611578565b610567565b005b6100d06100e03660046115ad565b610607565b6100f86100f33660046115ad565b610615565b6040516100b4919061161f565b6100d0610113366004611710565b610aa9565b610120610b32565b6040516100b491906117d3565b610135610bc0565b6040516100b491906117ed565b61014b60025481565b6040519081526020016100b4565b8051602081830181018051600082529282019190930120915280546001820180546001600160a01b0390921692916101909061184f565b80601f01602080910402602001604051908101604052809291908181526020018280546101bc9061184f565b80156102095780601f106101de57610100808354040283529160200191610209565b820191906000526020600020905b8154815290600101906020018083116101ec57829003601f168201915b50505050509080600201805461021e9061184f565b80601f016020809104026020016040519081016040528092919081815260200182805461024a9061184f565b80156102975780601f1061026c57610100808354040283529160200191610297565b820191906000526020600020905b81548152906001019060200180831161027a57829003601f168201915b5050505050908060030180546102ac9061184f565b80601f01602080910402602001604051908101604052809291908181526020018280546102d89061184f565b80156103255780601f106102fa57610100808354040283529160200191610325565b820191906000526020600020905b81548152906001019060200180831161030857829003601f168201915b50505050509080600401805461033a9061184f565b80601f01602080910402602001604051908101604052809291908181526020018280546103669061184f565b80156103b35780601f10610388576101008083540402835291602001916103b3565b820191906000526020600020905b81548152906001019060200180831161039657829003601f168201915b5050505050908060050180546103c89061184f565b80601f01602080910402602001604051908101604052809291908181526020018280546103f49061184f565b80156104415780601f1061041657610100808354040283529160200191610441565b820191906000526020600020905b81548152906001019060200180831161042457829003601f168201915b5050505050908060060180546104569061184f565b80601f01602080910402602001604051908101604052809291908181526020018280546104829061184f565b80156104cf5780601f106104a4576101008083540402835291602001916104cf565b820191906000526020600020905b8154815290600101906020018083116104b257829003601f168201915b5050505050908060070180546104e49061184f565b80601f01602080910402602001604051908101604052809291908181526020018280546105109061184f565b801561055d5780601f106105325761010080835404028352916020019161055d565b820191906000526020600020905b81548152906001019060200180831161054057829003601f168201915b5050505050905088565b60006001600160a01b0316600082602001516040516105869190611889565b908152604051908190036020019020546001600160a01b0316146105fb5760405162461bcd60e51b815260206004820152602160248201527f54686973206e6f646520697320616c7265616479206f6e2077686974656c69736044820152601d60fa1b60648201526084015b60405180910390fd5b61060481610d61565b50565b6106118282610f85565b5050565b61066660405180610100016040528060006001600160a01b03168152602001606081526020016060815260200160608152602001606081526020016060815260200160608152602001606081525090565b600083836040516106789291906118a5565b908152604080519182900360209081018320610100840190925281546001600160a01b03168352600182018054918401916106b29061184f565b80601f01602080910402602001604051908101604052809291908181526020018280546106de9061184f565b801561072b5780601f106107005761010080835404028352916020019161072b565b820191906000526020600020905b81548152906001019060200180831161070e57829003601f168201915b505050505081526020016002820180546107449061184f565b80601f01602080910402602001604051908101604052809291908181526020018280546107709061184f565b80156107bd5780601f10610792576101008083540402835291602001916107bd565b820191906000526020600020905b8154815290600101906020018083116107a057829003601f168201915b505050505081526020016003820180546107d69061184f565b80601f01602080910402602001604051908101604052809291908181526020018280546108029061184f565b801561084f5780601f106108245761010080835404028352916020019161084f565b820191906000526020600020905b81548152906001019060200180831161083257829003601f168201915b505050505081526020016004820180546108689061184f565b80601f01602080910402602001604051908101604052809291908181526020018280546108949061184f565b80156108e15780601f106108b6576101008083540402835291602001916108e1565b820191906000526020600020905b8154815290600101906020018083116108c457829003601f168201915b505050505081526020016005820180546108fa9061184f565b80601f01602080910402602001604051908101604052809291908181526020018280546109269061184f565b80156109735780601f1061094857610100808354040283529160200191610973565b820191906000526020600020905b81548152906001019060200180831161095657829003601f168201915b5050505050815260200160068201805461098c9061184f565b80601f01602080910402602001604051908101604052809291908181526020018280546109b89061184f565b8015610a055780601f106109da57610100808354040283529160200191610a05565b820191906000526020600020905b8154815290600101906020018083116109e857829003601f168201915b50505050508152602001600782018054610a1e9061184f565b80601f0160208091040260200160405190810160405280929190818152602001828054610a4a9061184f565b8015610a975780601f10610a6c57610100808354040283529160200191610a97565b820191906000526020600020905b815481529060010190602001808311610a7a57829003601f168201915b50505050508152505090505b92915050565b60035460ff1615610afc5760405162461bcd60e51b815260206004820152601c60248201527f636f6e747261637420616c726561647920696e697469616c697a65640000000060448201526064016105f2565b60005b815181101561061157610b2a828281518110610b1d57610b1d6118b5565b6020026020010151610d61565b600101610aff565b60018054610b3f9061184f565b80601f0160208091040260200160405190810160405280929190818152602001828054610b6b9061184f565b8015610bb85780601f10610b8d57610100808354040283529160200191610bb8565b820191906000526020600020905b815481529060010190602001808311610b9b57829003601f168201915b505050505081565b6060600060018054610bd19061184f565b80601f0160208091040260200160405190810160405280929190818152602001828054610bfd9061184f565b8015610c4a5780601f10610c1f57610100808354040283529160200191610c4a565b820191906000526020600020905b815481529060010190602001808311610c2d57829003601f168201915b5050505050905060005b81838281518110610c6757610c676118b5565b6020026020010181905250600082604051610c829190611889565b90815260200160405180910390206007018054610c9e9061184f565b159050610d5c57600082604051610cb59190611889565b90815260200160405180910390206007018054610cd19061184f565b80601f0160208091040260200160405190810160405280929190818152602001828054610cfd9061184f565b8015610d4a5780601f10610d1f57610100808354040283529160200191610d4a565b820191906000526020600020905b815481529060010190602001808311610d2d57829003601f168201915b50505050509150600181019050610c54565b505090565b806040015160006001604051610d7791906118cb565b90815260200160405180910390206006019081610d949190611990565b50600060018054610da49061184f565b80601f0160208091040260200160405190810160405280929190818152602001828054610dd09061184f565b8015610e1d5780601f10610df257610100808354040283529160200191610e1d565b820191906000526020600020905b815481529060010190602001808311610e0057829003601f168201915b50505050509050600060405180610100016040528084600001516001600160a01b0316815260200184604001518152602001846080015181526020018460a0015181526020018460c001518152602001846060015181526020016040518060200160405280600081525081526020018381525090508060008460200151604051610ea79190611889565b90815260405160209181900382019020825181546001600160a01b0319166001600160a01b03909116178155908201516001820190610ee69082611990565b5060408201516002820190610efb9082611990565b5060608201516003820190610f109082611990565b5060808201516004820190610f259082611990565b5060a08201516005820190610f3a9082611990565b5060c08201516006820190610f4f9082611990565b5060e08201516007820190610f649082611990565b50905050600160026000828254610f7b9190611a50565b9091555050505050565b6000808383604051610f989291906118a5565b9081526020016040518091039020600701604051610fb691906118cb565b90815260200160405180910390206000808484604051610fd79291906118a5565b9081526020016040518091039020600601604051610ff591906118cb565b908152604051908190036020019020815481546001600160a01b0319166001600160a01b0390911617815560018082019061103290840182611a71565b5060028181019061104590840182611a71565b5060038181019061105890840182611a71565b5060048181019061106b90840182611a71565b5060058181019061107e90840182611a71565b5060068181019061109190840182611a71565b506007818101906110a490840182611a71565b5090505060008083836040516110bb9291906118a5565b90815260200160405180910390206006016040516110d991906118cb565b908152602001604051809103902060008084846040516110fa9291906118a5565b908152602001604051809103902060070160405161111891906118cb565b908152604051908190036020019020815481546001600160a01b0319166001600160a01b0390911617815560018082019061115590840182611a71565b5060028181019061116890840182611a71565b5060038181019061117b90840182611a71565b5060048181019061118e90840182611a71565b506005818101906111a190840182611a71565b506006818101906111b490840182611a71565b506007818101906111c790840182611a71565b5090505060008083836040516111de9291906118a5565b908152602001604051809103902060000160006101000a8154816001600160a01b0302191690836001600160a01b031602179055506001600260008282546112269190611a50565b90915550505050565b634e487b7160e01b600052604160045260246000fd5b60405160e0810167ffffffffffffffff811182821017156112685761126861122f565b60405290565b604051601f8201601f1916810167ffffffffffffffff811182821017156112975761129761122f565b604052919050565b600082601f8301126112b057600080fd5b813567ffffffffffffffff8111156112ca576112ca61122f565b6112dd601f8201601f191660200161126e565b8181528460208386010111156112f257600080fd5b816020850160208301376000918101602001919091529392505050565b60006020828403121561132157600080fd5b813567ffffffffffffffff81111561133857600080fd5b6113448482850161129f565b949350505050565b60005b8381101561136757818101518382015260200161134f565b50506000910152565b6000815180845261138881602086016020860161134c565b601f01601f19169290920160200192915050565b6001600160a01b0389168152610100602082018190526000906113c18382018b611370565b905082810360408401526113d5818a611370565b905082810360608401526113e98189611370565b905082810360808401526113fd8188611370565b905082810360a08401526114118187611370565b905082810360c08401526114258186611370565b905082810360e08401526114398185611370565b9b9a5050505050505050505050565b80356001600160a01b038116811461145f57600080fd5b919050565b600060e0828403121561147657600080fd5b61147e611245565b905061148982611448565b8152602082013567ffffffffffffffff808211156114a657600080fd5b6114b28583860161129f565b602084015260408401359150808211156114cb57600080fd5b6114d78583860161129f565b604084015260608401359150808211156114f057600080fd5b6114fc8583860161129f565b6060840152608084013591508082111561151557600080fd5b6115218583860161129f565b608084015260a084013591508082111561153a57600080fd5b6115468583860161129f565b60a084015260c084013591508082111561155f57600080fd5b5061156c8482850161129f565b60c08301525092915050565b60006020828403121561158a57600080fd5b813567ffffffffffffffff8111156115a157600080fd5b61134484828501611464565b600080602083850312156115c057600080fd5b823567ffffffffffffffff808211156115d857600080fd5b818501915085601f8301126115ec57600080fd5b8135818111156115fb57600080fd5b86602082850101111561160d57600080fd5b60209290920196919550909350505050565b602081526116396020820183516001600160a01b03169052565b60006020830151610100806040850152611657610120850183611370565b91506040850151601f19808685030160608701526116758483611370565b935060608701519150808685030160808701526116928483611370565b935060808701519150808685030160a08701526116af8483611370565b935060a08701519150808685030160c08701526116cc8483611370565b935060c08701519150808685030160e08701526116e98483611370565b935060e08701519150808685030183870152506117068382611370565b9695505050505050565b6000602080838503121561172357600080fd5b823567ffffffffffffffff8082111561173b57600080fd5b818501915085601f83011261174f57600080fd5b8135818111156117615761176161122f565b8060051b61177085820161126e565b918252838101850191858101908984111561178a57600080fd5b86860192505b838310156117c6578235858111156117a85760008081fd5b6117b68b89838a0101611464565b8352509186019190860190611790565b9998505050505050505050565b6020815260006117e66020830184611370565b9392505050565b6000602080830181845280855180835260408601915060408160051b870101925083870160005b8281101561184257603f19888603018452611830858351611370565b94509285019290850190600101611814565b5092979650505050505050565b600181811c9082168061186357607f821691505b60208210810361188357634e487b7160e01b600052602260045260246000fd5b50919050565b6000825161189b81846020870161134c565b9190910192915050565b8183823760009101908152919050565b634e487b7160e01b600052603260045260246000fd5b60008083546118d98161184f565b600182811680156118f1576001811461190657611935565b60ff1984168752821515830287019450611935565b8760005260208060002060005b8581101561192c5781548a820152908401908201611913565b50505082870194505b50929695505050505050565b601f82111561198b57600081815260208120601f850160051c810160208610156119685750805b601f850160051c820191505b8181101561198757828155600101611974565b5050505b505050565b815167ffffffffffffffff8111156119aa576119aa61122f565b6119be816119b8845461184f565b84611941565b602080601f8311600181146119f357600084156119db5750858301515b600019600386901b1c1916600185901b178555611987565b600085815260208120601f198616915b82811015611a2257888601518255948401946001909101908401611a03565b5085821015611a405787850151600019600388901b60f8161c191681555b5050505050600190811b01905550565b81810381811115610aa357634e487b7160e01b600052601160045260246000fd5b818103611a7c575050565b611a86825461184f565b67ffffffffffffffff811115611a9e57611a9e61122f565b611aac816119b8845461184f565b6000601f821160018114611ae05760008315611ac85750848201545b600019600385901b1c1916600184901b178455611b45565b600085815260209020601f19841690600086815260209020845b83811015611b1a5782860154825560019586019590910190602001611afa565b5085831015611b385781850154600019600388901b60f8161c191681555b50505060018360011b0184555b505050505056fea2646970667358221220f6574ebc58cb754662b9876f7d3f994eb9126e5369600784d1b1c3e9c7420b6564736f6c63430008110033" . parse () . expect ("invalid bytecode")
        });
    pub struct NodeRegistry<M>(ethers::contract::Contract<M>);
    impl<M> Clone for NodeRegistry<M> {
        fn clone(&self) -> Self {
            NodeRegistry(self.0.clone())
        }
    }
    impl<M> std::ops::Deref for NodeRegistry<M> {
        type Target = ethers::contract::Contract<M>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<M> std::fmt::Debug for NodeRegistry<M> {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.debug_tuple(stringify!(NodeRegistry))
                .field(&self.address())
                .finish()
        }
    }
    impl<M: ethers::providers::Middleware> NodeRegistry<M> {
        #[doc = r" Creates a new contract instance with the specified `ethers`"]
        #[doc = r" client at the given `Address`. The contract derefs to a `ethers::Contract`"]
        #[doc = r" object"]
        pub fn new<T: Into<ethers::core::types::Address>>(
            address: T,
            client: ::std::sync::Arc<M>,
        ) -> Self {
            ethers::contract::Contract::new(address.into(), NODEREGISTRY_ABI.clone(), client).into()
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
                NODEREGISTRY_ABI.clone(),
                NODEREGISTRY_BYTECODE.clone(),
                client,
            );
            let deployer = factory.deploy(constructor_args)?;
            let deployer = ethers::contract::ContractDeployer::new(deployer);
            Ok(deployer)
        }
        #[doc = "Calls the contract's `getNodeInfo` (0x813775a3) function"]
        pub fn get_node_info(
            &self,
            node_address: String,
        ) -> ethers::contract::builders::ContractCall<M, Node> {
            self.0
                .method_hash([129, 55, 117, 163], node_address)
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `getWhitelist` (0xd01f63f5) function"]
        pub fn get_whitelist(
            &self,
        ) -> ethers::contract::builders::ContractCall<M, ::std::vec::Vec<String>> {
            self.0
                .method_hash([208, 31, 99, 245], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `initialize` (0x8739b3a6) function"]
        pub fn initialize(
            &self,
            genesis_committee: ::std::vec::Vec<NodeInfo>,
        ) -> ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([135, 57, 179, 166], genesis_committee)
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `linkedListHead` (0xa2c6ddeb) function"]
        pub fn linked_list_head(&self) -> ethers::contract::builders::ContractCall<M, String> {
            self.0
                .method_hash([162, 198, 221, 235], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `registerNode` (0x239e3e07) function"]
        pub fn register_node(
            &self,
            node: NodeInfo,
        ) -> ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([35, 158, 62, 7], (node,))
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `removeNode` (0x4665cb07) function"]
        pub fn remove_node(
            &self,
            node_address: String,
        ) -> ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([70, 101, 203, 7], node_address)
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `whitelist` (0x0d80bf64) function"]
        pub fn whitelist(
            &self,
            p0: String,
        ) -> ethers::contract::builders::ContractCall<
            M,
            (
                ethers::core::types::Address,
                String,
                String,
                String,
                String,
                String,
                String,
                String,
            ),
        > {
            self.0
                .method_hash([13, 128, 191, 100], p0)
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `whitelistCount` (0xf2624b5d) function"]
        pub fn whitelist_count(
            &self,
        ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::U256> {
            self.0
                .method_hash([242, 98, 75, 93], ())
                .expect("method not found (this should never happen)")
        }
    }
    impl<M: ethers::providers::Middleware> From<ethers::contract::Contract<M>> for NodeRegistry<M> {
        fn from(contract: ethers::contract::Contract<M>) -> Self {
            Self(contract)
        }
    }
    #[doc = "Container type for all input parameters for the `getNodeInfo` function with signature `getNodeInfo(string)` and selector `[129, 55, 117, 163]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "getNodeInfo", abi = "getNodeInfo(string)")]
    pub struct GetNodeInfoCall {
        pub node_address: String,
    }
    #[doc = "Container type for all input parameters for the `getWhitelist` function with signature `getWhitelist()` and selector `[208, 31, 99, 245]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "getWhitelist", abi = "getWhitelist()")]
    pub struct GetWhitelistCall;
    #[doc = "Container type for all input parameters for the `initialize` function with signature `initialize((address,string,string,string,string,string,string)[])` and selector `[135, 57, 179, 166]`"]
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
        abi = "initialize((address,string,string,string,string,string,string)[])"
    )]
    pub struct InitializeCall {
        pub genesis_committee: ::std::vec::Vec<NodeInfo>,
    }
    #[doc = "Container type for all input parameters for the `linkedListHead` function with signature `linkedListHead()` and selector `[162, 198, 221, 235]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "linkedListHead", abi = "linkedListHead()")]
    pub struct LinkedListHeadCall;
    #[doc = "Container type for all input parameters for the `registerNode` function with signature `registerNode((address,string,string,string,string,string,string))` and selector `[35, 158, 62, 7]`"]
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
        name = "registerNode",
        abi = "registerNode((address,string,string,string,string,string,string))"
    )]
    pub struct RegisterNodeCall {
        pub node: NodeInfo,
    }
    #[doc = "Container type for all input parameters for the `removeNode` function with signature `removeNode(string)` and selector `[70, 101, 203, 7]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "removeNode", abi = "removeNode(string)")]
    pub struct RemoveNodeCall {
        pub node_address: String,
    }
    #[doc = "Container type for all input parameters for the `whitelist` function with signature `whitelist(string)` and selector `[13, 128, 191, 100]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "whitelist", abi = "whitelist(string)")]
    pub struct WhitelistCall(pub String);
    #[doc = "Container type for all input parameters for the `whitelistCount` function with signature `whitelistCount()` and selector `[242, 98, 75, 93]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "whitelistCount", abi = "whitelistCount()")]
    pub struct WhitelistCountCall;
    #[derive(Debug, Clone, PartialEq, Eq, ethers :: contract :: EthAbiType)]
    pub enum NodeRegistryCalls {
        GetNodeInfo(GetNodeInfoCall),
        GetWhitelist(GetWhitelistCall),
        Initialize(InitializeCall),
        LinkedListHead(LinkedListHeadCall),
        RegisterNode(RegisterNodeCall),
        RemoveNode(RemoveNodeCall),
        Whitelist(WhitelistCall),
        WhitelistCount(WhitelistCountCall),
    }
    impl ethers::core::abi::AbiDecode for NodeRegistryCalls {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::std::result::Result<Self, ethers::core::abi::AbiError> {
            if let Ok(decoded) =
                <GetNodeInfoCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(NodeRegistryCalls::GetNodeInfo(decoded));
            }
            if let Ok(decoded) =
                <GetWhitelistCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(NodeRegistryCalls::GetWhitelist(decoded));
            }
            if let Ok(decoded) =
                <InitializeCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(NodeRegistryCalls::Initialize(decoded));
            }
            if let Ok(decoded) =
                <LinkedListHeadCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(NodeRegistryCalls::LinkedListHead(decoded));
            }
            if let Ok(decoded) =
                <RegisterNodeCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(NodeRegistryCalls::RegisterNode(decoded));
            }
            if let Ok(decoded) =
                <RemoveNodeCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(NodeRegistryCalls::RemoveNode(decoded));
            }
            if let Ok(decoded) =
                <WhitelistCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(NodeRegistryCalls::Whitelist(decoded));
            }
            if let Ok(decoded) =
                <WhitelistCountCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(NodeRegistryCalls::WhitelistCount(decoded));
            }
            Err(ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ethers::core::abi::AbiEncode for NodeRegistryCalls {
        fn encode(self) -> Vec<u8> {
            match self {
                NodeRegistryCalls::GetNodeInfo(element) => element.encode(),
                NodeRegistryCalls::GetWhitelist(element) => element.encode(),
                NodeRegistryCalls::Initialize(element) => element.encode(),
                NodeRegistryCalls::LinkedListHead(element) => element.encode(),
                NodeRegistryCalls::RegisterNode(element) => element.encode(),
                NodeRegistryCalls::RemoveNode(element) => element.encode(),
                NodeRegistryCalls::Whitelist(element) => element.encode(),
                NodeRegistryCalls::WhitelistCount(element) => element.encode(),
            }
        }
    }
    impl ::std::fmt::Display for NodeRegistryCalls {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match self {
                NodeRegistryCalls::GetNodeInfo(element) => element.fmt(f),
                NodeRegistryCalls::GetWhitelist(element) => element.fmt(f),
                NodeRegistryCalls::Initialize(element) => element.fmt(f),
                NodeRegistryCalls::LinkedListHead(element) => element.fmt(f),
                NodeRegistryCalls::RegisterNode(element) => element.fmt(f),
                NodeRegistryCalls::RemoveNode(element) => element.fmt(f),
                NodeRegistryCalls::Whitelist(element) => element.fmt(f),
                NodeRegistryCalls::WhitelistCount(element) => element.fmt(f),
            }
        }
    }
    impl ::std::convert::From<GetNodeInfoCall> for NodeRegistryCalls {
        fn from(var: GetNodeInfoCall) -> Self {
            NodeRegistryCalls::GetNodeInfo(var)
        }
    }
    impl ::std::convert::From<GetWhitelistCall> for NodeRegistryCalls {
        fn from(var: GetWhitelistCall) -> Self {
            NodeRegistryCalls::GetWhitelist(var)
        }
    }
    impl ::std::convert::From<InitializeCall> for NodeRegistryCalls {
        fn from(var: InitializeCall) -> Self {
            NodeRegistryCalls::Initialize(var)
        }
    }
    impl ::std::convert::From<LinkedListHeadCall> for NodeRegistryCalls {
        fn from(var: LinkedListHeadCall) -> Self {
            NodeRegistryCalls::LinkedListHead(var)
        }
    }
    impl ::std::convert::From<RegisterNodeCall> for NodeRegistryCalls {
        fn from(var: RegisterNodeCall) -> Self {
            NodeRegistryCalls::RegisterNode(var)
        }
    }
    impl ::std::convert::From<RemoveNodeCall> for NodeRegistryCalls {
        fn from(var: RemoveNodeCall) -> Self {
            NodeRegistryCalls::RemoveNode(var)
        }
    }
    impl ::std::convert::From<WhitelistCall> for NodeRegistryCalls {
        fn from(var: WhitelistCall) -> Self {
            NodeRegistryCalls::Whitelist(var)
        }
    }
    impl ::std::convert::From<WhitelistCountCall> for NodeRegistryCalls {
        fn from(var: WhitelistCountCall) -> Self {
            NodeRegistryCalls::WhitelistCount(var)
        }
    }
    #[doc = "Container type for all return fields from the `getNodeInfo` function with signature `getNodeInfo(string)` and selector `[129, 55, 117, 163]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct GetNodeInfoReturn(pub Node);
    #[doc = "Container type for all return fields from the `getWhitelist` function with signature `getWhitelist()` and selector `[208, 31, 99, 245]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct GetWhitelistReturn {
        pub whitelist: ::std::vec::Vec<String>,
    }
    #[doc = "Container type for all return fields from the `linkedListHead` function with signature `linkedListHead()` and selector `[162, 198, 221, 235]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct LinkedListHeadReturn(pub String);
    #[doc = "Container type for all return fields from the `whitelist` function with signature `whitelist(string)` and selector `[13, 128, 191, 100]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct WhitelistReturn {
        pub owner: ethers::core::types::Address,
        pub primary_address: String,
        pub worker_address: String,
        pub worker_public_key: String,
        pub worker_mempool: String,
        pub network_key: String,
        pub previous: String,
        pub next: String,
    }
    #[doc = "Container type for all return fields from the `whitelistCount` function with signature `whitelistCount()` and selector `[242, 98, 75, 93]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct WhitelistCountReturn(pub ethers::core::types::U256);
    #[doc = "`Node(address,string,string,string,string,string,string,string)`"]
    #[derive(
        Clone,
        Debug,
        Default,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
    )]
    pub struct Node {
        pub owner: ethers::core::types::Address,
        pub primary_address: String,
        pub worker_address: String,
        pub worker_public_key: String,
        pub worker_mempool: String,
        pub network_key: String,
        pub previous: String,
        pub next: String,
    }
    #[doc = "`NodeInfo(address,string,string,string,string,string,string)`"]
    #[derive(
        Clone,
        Debug,
        Default,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
    )]
    pub struct NodeInfo {
        pub owner: ethers::core::types::Address,
        pub primary_public_key: String,
        pub primary_address: String,
        pub network_key: String,
        pub worker_address: String,
        pub worker_public_key: String,
        pub worker_mempool: String,
    }
}
