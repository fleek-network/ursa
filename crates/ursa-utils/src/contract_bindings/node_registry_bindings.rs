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
# [rustfmt :: skip] const __ABI : & str = "[{\"inputs\":[{\"internalType\":\"string\",\"name\":\"nodeAddress\",\"type\":\"string\",\"components\":[]}],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"getNodeInfo\",\"outputs\":[{\"internalType\":\"struct NodeRegistry.Node\",\"name\":\"\",\"type\":\"tuple\",\"components\":[{\"internalType\":\"address\",\"name\":\"owner\",\"type\":\"address\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"primaryAddress\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"networkKey\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"struct NodeRegistry.Worker[]\",\"name\":\"workers\",\"type\":\"tuple[]\",\"components\":[{\"internalType\":\"string\",\"name\":\"workerAddress\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"workerPublicKey\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"workerMempool\",\"type\":\"string\",\"components\":[]}]},{\"internalType\":\"string\",\"name\":\"previous\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"next\",\"type\":\"string\",\"components\":[]}]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"getWhitelist\",\"outputs\":[{\"internalType\":\"string[]\",\"name\":\"_whitelist\",\"type\":\"string[]\",\"components\":[]}]},{\"inputs\":[{\"internalType\":\"struct NodeRegistry.NodeInfo[]\",\"name\":\"_genesis_committee\",\"type\":\"tuple[]\",\"components\":[{\"internalType\":\"address\",\"name\":\"owner\",\"type\":\"address\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"primaryPublicKey\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"primaryAddress\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"networkKey\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"struct NodeRegistry.Worker[]\",\"name\":\"workers\",\"type\":\"tuple[]\",\"components\":[{\"internalType\":\"string\",\"name\":\"workerAddress\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"workerPublicKey\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"workerMempool\",\"type\":\"string\",\"components\":[]}]}]}],\"stateMutability\":\"nonpayable\",\"type\":\"function\",\"name\":\"initialize\",\"outputs\":[]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"linkedListHead\",\"outputs\":[{\"internalType\":\"string\",\"name\":\"\",\"type\":\"string\",\"components\":[]}]},{\"inputs\":[{\"internalType\":\"struct NodeRegistry.NodeInfo\",\"name\":\"_node\",\"type\":\"tuple\",\"components\":[{\"internalType\":\"address\",\"name\":\"owner\",\"type\":\"address\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"primaryPublicKey\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"primaryAddress\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"networkKey\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"struct NodeRegistry.Worker[]\",\"name\":\"workers\",\"type\":\"tuple[]\",\"components\":[{\"internalType\":\"string\",\"name\":\"workerAddress\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"workerPublicKey\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"workerMempool\",\"type\":\"string\",\"components\":[]}]}]}],\"stateMutability\":\"nonpayable\",\"type\":\"function\",\"name\":\"registerNode\",\"outputs\":[]},{\"inputs\":[{\"internalType\":\"string\",\"name\":\"_nodeAddress\",\"type\":\"string\",\"components\":[]}],\"stateMutability\":\"nonpayable\",\"type\":\"function\",\"name\":\"removeNode\",\"outputs\":[]},{\"inputs\":[{\"internalType\":\"string\",\"name\":\"\",\"type\":\"string\",\"components\":[]}],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"whitelist\",\"outputs\":[{\"internalType\":\"address\",\"name\":\"owner\",\"type\":\"address\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"primaryAddress\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"networkKey\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"previous\",\"type\":\"string\",\"components\":[]},{\"internalType\":\"string\",\"name\":\"next\",\"type\":\"string\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"whitelistCount\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]}]}]" ;
#[doc = r" The parsed JSON-ABI of the contract."]
pub static NODEREGISTRY_ABI: ethers::contract::Lazy<ethers::core::abi::Abi> =
    ethers::contract::Lazy::new(|| {
        ethers::core::utils::__serde_json::from_str(__ABI).expect("invalid abi")
    });
#[doc = r" Bytecode of the #name contract"]
pub static NODEREGISTRY_BYTECODE: ethers::contract::Lazy<ethers::core::types::Bytes> =
    ethers::contract::Lazy::new(|| {
        "0x608060405234801561001057600080fd5b50611c2a806100206000396000f3fe608060405234801561001057600080fd5b50600436106100885760003560e01c8063813775a31161005b578063813775a3146100f5578063a2c6ddeb14610115578063d01f63f51461012a578063f2624b5d1461013f57600080fd5b806304859f791461008d5780630d80bf64146100a2578063139dc12a146100cf5780634665cb07146100e2575b600080fd5b6100a061009b3660046114f1565b610156565b005b6100b56100b036600461152d565b6101f6565b6040516100c69594939291906115b1565b60405180910390f35b6100a06100dd36600461161b565b61045a565b6100a06100f03660046116cb565b6104f4565b6101086101033660046116cb565b610502565b6040516100c6919061173c565b61011d6109e5565b6040516100c69190611862565b610132610a73565b6040516100c6919061187c565b61014860025481565b6040519081526020016100c6565b60006001600160a01b03166000826020015160405161017591906118de565b908152604051908190036020019020546001600160a01b0316146101ea5760405162461bcd60e51b815260206004820152602160248201527f54686973206e6f646520697320616c7265616479206f6e2077686974656c69736044820152601d60fa1b60648201526084015b60405180910390fd5b6101f381610c64565b50565b8051602081830181018051600082529282019190930120915280546001820180546001600160a01b03909216929161022d906118fa565b80601f0160208091040260200160405190810160405280929190818152602001828054610259906118fa565b80156102a65780601f1061027b576101008083540402835291602001916102a6565b820191906000526020600020905b81548152906001019060200180831161028957829003601f168201915b5050505050908060020180546102bb906118fa565b80601f01602080910402602001604051908101604052809291908181526020018280546102e7906118fa565b80156103345780601f1061030957610100808354040283529160200191610334565b820191906000526020600020905b81548152906001019060200180831161031757829003601f168201915b505050505090806004018054610349906118fa565b80601f0160208091040260200160405190810160405280929190818152602001828054610375906118fa565b80156103c25780601f10610397576101008083540402835291602001916103c2565b820191906000526020600020905b8154815290600101906020018083116103a557829003601f168201915b5050505050908060050180546103d7906118fa565b80601f0160208091040260200160405190810160405280929190818152602001828054610403906118fa565b80156104505780601f1061042557610100808354040283529160200191610450565b820191906000526020600020905b81548152906001019060200180831161043357829003601f168201915b5050505050905085565b60035460ff16156104ad5760405162461bcd60e51b815260206004820152601c60248201527f636f6e747261637420616c726561647920696e697469616c697a65640000000060448201526064016101e1565b60005b81518110156104e3576104db8282815181106104ce576104ce611934565b6020026020010151610c64565b6001016104b0565b50506003805460ff19166001179055565b6104fe8282610e68565b5050565b6105446040518060c0016040528060006001600160a01b0316815260200160608152602001606081526020016060815260200160608152602001606081525090565b6000838360405161055692919061194a565b90815260408051918290036020908101832060c0840190925281546001600160a01b031683526001820180549184019161058f906118fa565b80601f01602080910402602001604051908101604052809291908181526020018280546105bb906118fa565b80156106085780601f106105dd57610100808354040283529160200191610608565b820191906000526020600020905b8154815290600101906020018083116105eb57829003601f168201915b50505050508152602001600282018054610621906118fa565b80601f016020809104026020016040519081016040528092919081815260200182805461064d906118fa565b801561069a5780601f1061066f5761010080835404028352916020019161069a565b820191906000526020600020905b81548152906001019060200180831161067d57829003601f168201915b5050505050815260200160038201805480602002602001604051908101604052809291908181526020016000905b828210156108b057838290600052602060002090600302016040518060600160405290816000820180546106fb906118fa565b80601f0160208091040260200160405190810160405280929190818152602001828054610727906118fa565b80156107745780601f1061074957610100808354040283529160200191610774565b820191906000526020600020905b81548152906001019060200180831161075757829003601f168201915b5050505050815260200160018201805461078d906118fa565b80601f01602080910402602001604051908101604052809291908181526020018280546107b9906118fa565b80156108065780601f106107db57610100808354040283529160200191610806565b820191906000526020600020905b8154815290600101906020018083116107e957829003601f168201915b5050505050815260200160028201805461081f906118fa565b80601f016020809104026020016040519081016040528092919081815260200182805461084b906118fa565b80156108985780601f1061086d57610100808354040283529160200191610898565b820191906000526020600020905b81548152906001019060200180831161087b57829003601f168201915b505050505081525050815260200190600101906106c8565b5050505081526020016004820180546108c8906118fa565b80601f01602080910402602001604051908101604052809291908181526020018280546108f4906118fa565b80156109415780601f1061091657610100808354040283529160200191610941565b820191906000526020600020905b81548152906001019060200180831161092457829003601f168201915b5050505050815260200160058201805461095a906118fa565b80601f0160208091040260200160405190810160405280929190818152602001828054610986906118fa565b80156109d35780601f106109a8576101008083540402835291602001916109d3565b820191906000526020600020905b8154815290600101906020018083116109b657829003601f168201915b50505050508152505090505b92915050565b600180546109f2906118fa565b80601f0160208091040260200160405190810160405280929190818152602001828054610a1e906118fa565b8015610a6b5780601f10610a4057610100808354040283529160200191610a6b565b820191906000526020600020905b815481529060010190602001808311610a4e57829003601f168201915b505050505081565b6060600060018054610a84906118fa565b80601f0160208091040260200160405190810160405280929190818152602001828054610ab0906118fa565b8015610afd5780601f10610ad257610100808354040283529160200191610afd565b820191906000526020600020905b815481529060010190602001808311610ae057829003601f168201915b505050505090506002546001600160401b03811115610b1e57610b1e6111d5565b604051908082528060200260200182016040528015610b5157816020015b6060815260200190600190039081610b3c5790505b50915060005b81838281518110610b6a57610b6a611934565b6020026020010181905250600082604051610b8591906118de565b90815260200160405180910390206005018054610ba1906118fa565b159050610c5f57600082604051610bb891906118de565b90815260200160405180910390206005018054610bd4906118fa565b80601f0160208091040260200160405190810160405280929190818152602001828054610c00906118fa565b8015610c4d5780601f10610c2257610100808354040283529160200191610c4d565b820191906000526020600020905b815481529060010190602001808311610c3057829003601f168201915b50505050509150600181019050610b57565b505090565b806040015160006001604051610c7a919061195a565b90815260200160405180910390206004019081610c979190611a1f565b50600060018054610ca7906118fa565b80601f0160208091040260200160405190810160405280929190818152602001828054610cd3906118fa565b8015610d205780601f10610cf557610100808354040283529160200191610d20565b820191906000526020600020905b815481529060010190602001808311610d0357829003601f168201915b505050505090506000808360200151604051610d3c91906118de565b9081526040805191829003602001909120845181546001600160a01b039091166001600160a01b0319909116178155908401519091506001820190610d819082611a1f565b5060608301516002820190610d969082611a1f565b5060058101610da58382611a1f565b5060005b836080015151811015610e37578160030184608001518281518110610dd057610dd0611934565b60209081029190910181015182546001810184556000938452919092208251600390920201908190610e029082611a1f565b5060208201516001820190610e179082611a1f565b5060408201516002820190610e2c9082611a1f565b505050600101610da9565b50600160026000828254610e4b9190611af4565b90915550506020830151600190610e629082611a1f565b50505050565b6000808383604051610e7b92919061194a565b9081526020016040518091039020600501604051610e99919061195a565b90815260200160405180910390206000808484604051610eba92919061194a565b9081526020016040518091039020600401604051610ed8919061195a565b908152604051908190036020019020815481546001600160a01b0319166001600160a01b03909116178155600180820190610f1590840182611b07565b50600281810190610f2890840182611b07565b5060038281018054610f3d92840191906110ca565b50600481810190610f5090840182611b07565b50600581810190610f6390840182611b07565b509050506000808383604051610f7a92919061194a565b9081526020016040518091039020600401604051610f98919061195a565b90815260200160405180910390206000808484604051610fb992919061194a565b9081526020016040518091039020600501604051610fd7919061195a565b908152604051908190036020019020815481546001600160a01b0319166001600160a01b0390911617815560018082019061101490840182611b07565b5060028181019061102790840182611b07565b506003828101805461103c92840191906110ca565b5060048181019061104f90840182611b07565b5060058181019061106290840182611b07565b50905050600080838360405161107992919061194a565b908152602001604051809103902060000160006101000a8154816001600160a01b0302191690836001600160a01b031602179055506001600260008282546110c19190611be1565b90915550505050565b8280548282559060005260206000209060030281019282156111425760005260206000209160030282015b828111156111425782828061110a8382611b07565b5060018181019061111d90840182611b07565b5060028181019061113090840182611b07565b505050916003019190600301906110f5565b5061114e929150611152565b5090565b8082111561114e576000611166828261118b565b61117460018301600061118b565b61118260028301600061118b565b50600301611152565b508054611197906118fa565b6000825580601f106111a7575050565b601f0160209004906000526020600020908101906101f391905b8082111561114e57600081556001016111c1565b634e487b7160e01b600052604160045260246000fd5b604051606081016001600160401b038111828210171561120d5761120d6111d5565b60405290565b60405160a081016001600160401b038111828210171561120d5761120d6111d5565b604051601f8201601f191681016001600160401b038111828210171561125d5761125d6111d5565b604052919050565b80356001600160a01b038116811461127c57600080fd5b919050565b600082601f83011261129257600080fd5b81356001600160401b038111156112ab576112ab6111d5565b6112be601f8201601f1916602001611235565b8181528460208386010111156112d357600080fd5b816020850160208301376000918101602001919091529392505050565b60006001600160401b03821115611309576113096111d5565b5060051b60200190565b600082601f83011261132457600080fd5b81356020611339611334836112f0565b611235565b82815260059290921b8401810191818101908684111561135857600080fd5b8286015b8481101561141d5780356001600160401b038082111561137c5760008081fd5b908801906060828b03601f19018113156113965760008081fd5b61139e6111eb565b87840135838111156113b05760008081fd5b6113be8d8a83880101611281565b825250604080850135848111156113d55760008081fd5b6113e38e8b83890101611281565b838b0152509184013591838311156113fb5760008081fd5b6114098d8a85880101611281565b90820152865250505091830191830161135c565b509695505050505050565b600060a0828403121561143a57600080fd5b611442611213565b905061144d82611265565b815260208201356001600160401b038082111561146957600080fd5b61147585838601611281565b6020840152604084013591508082111561148e57600080fd5b61149a85838601611281565b604084015260608401359150808211156114b357600080fd5b6114bf85838601611281565b606084015260808401359150808211156114d857600080fd5b506114e584828501611313565b60808301525092915050565b60006020828403121561150357600080fd5b81356001600160401b0381111561151957600080fd5b61152584828501611428565b949350505050565b60006020828403121561153f57600080fd5b81356001600160401b0381111561155557600080fd5b61152584828501611281565b60005b8381101561157c578181015183820152602001611564565b50506000910152565b6000815180845261159d816020860160208601611561565b601f01601f19169290920160200192915050565b6001600160a01b038616815260a0602082018190526000906115d590830187611585565b82810360408401526115e78187611585565b905082810360608401526115fb8186611585565b9050828103608084015261160f8185611585565b98975050505050505050565b6000602080838503121561162e57600080fd5b82356001600160401b038082111561164557600080fd5b818501915085601f83011261165957600080fd5b8135611667611334826112f0565b81815260059190911b8301840190848101908883111561168657600080fd5b8585015b838110156116be578035858111156116a25760008081fd5b6116b08b89838a0101611428565b84525091860191860161168a565b5098975050505050505050565b600080602083850312156116de57600080fd5b82356001600160401b03808211156116f557600080fd5b818501915085601f83011261170957600080fd5b81358181111561171857600080fd5b86602082850101111561172a57600080fd5b60209290920196919550909350505050565b6000602080835260018060a01b038451168184015280840151604060c08186015261176a60e0860183611585565b915080860151601f1960608188860301818901526117888584611585565b89820151898203840160808b015280518083529196508701935086860190600581901b8701880160005b828110156118195785898303018452865180518684526117d487850182611585565b90508b8201518482038d8601526117eb8282611585565b9150508982015191508381038a8501526118058183611585565b988c0198958c0195935050506001016117b2565b5060808c01519850848b82030160a08c0152611835818a611585565b98505050505060a08801519350808786030160c08801525050506118598282611585565b95945050505050565b6020815260006118756020830184611585565b9392505050565b6000602080830181845280855180835260408601915060408160051b870101925083870160005b828110156118d157603f198886030184526118bf858351611585565b945092850192908501906001016118a3565b5092979650505050505050565b600082516118f0818460208701611561565b9190910192915050565b600181811c9082168061190e57607f821691505b60208210810361192e57634e487b7160e01b600052602260045260246000fd5b50919050565b634e487b7160e01b600052603260045260246000fd5b8183823760009101908152919050565b6000808354611968816118fa565b600182811680156119805760018114611995576119c4565b60ff19841687528215158302870194506119c4565b8760005260208060002060005b858110156119bb5781548a8201529084019082016119a2565b50505082870194505b50929695505050505050565b601f821115611a1a57600081815260208120601f850160051c810160208610156119f75750805b601f850160051c820191505b81811015611a1657828155600101611a03565b5050505b505050565b81516001600160401b03811115611a3857611a386111d5565b611a4c81611a4684546118fa565b846119d0565b602080601f831160018114611a815760008415611a695750858301515b600019600386901b1c1916600185901b178555611a16565b600085815260208120601f198616915b82811015611ab057888601518255948401946001909101908401611a91565b5085821015611ace5787850151600019600388901b60f8161c191681555b5050505050600190811b01905550565b634e487b7160e01b600052601160045260246000fd5b808201808211156109df576109df611ade565b818103611b12575050565b611b1c82546118fa565b6001600160401b03811115611b3357611b336111d5565b611b4181611a4684546118fa565b6000601f821160018114611b755760008315611b5d5750848201545b600019600385901b1c1916600184901b178455611bda565b600085815260209020601f19841690600086815260209020845b83811015611baf5782860154825560019586019590910190602001611b8f565b5085831015611bcd5781850154600019600388901b60f8161c191681555b50505060018360011b0184555b5050505050565b818103818111156109df576109df611ade56fea264697066735822122056f026c8f7d27fdabac2d5fdd6f1491a1334fe939220913e6a817ff17fe4be1e64736f6c63430008110033" . parse () . expect ("invalid bytecode")
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
    #[doc = "Calls the contract's `initialize` (0x139dc12a) function"]
    pub fn initialize(
        &self,
        genesis_committee: ::std::vec::Vec<NodeInfo>,
    ) -> ethers::contract::builders::ContractCall<M, ()> {
        self.0
            .method_hash([19, 157, 193, 42], genesis_committee)
            .expect("method not found (this should never happen)")
    }
    #[doc = "Calls the contract's `linkedListHead` (0xa2c6ddeb) function"]
    pub fn linked_list_head(&self) -> ethers::contract::builders::ContractCall<M, String> {
        self.0
            .method_hash([162, 198, 221, 235], ())
            .expect("method not found (this should never happen)")
    }
    #[doc = "Calls the contract's `registerNode` (0x04859f79) function"]
    pub fn register_node(&self, node: NodeInfo) -> ethers::contract::builders::ContractCall<M, ()> {
        self.0
            .method_hash([4, 133, 159, 121], (node,))
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
        (ethers::core::types::Address, String, String, String, String),
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
#[doc = "Container type for all input parameters for the `initialize` function with signature `initialize((address,string,string,string,(string,string,string)[])[])` and selector `[19, 157, 193, 42]`"]
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
    abi = "initialize((address,string,string,string,(string,string,string)[])[])"
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
#[doc = "Container type for all input parameters for the `registerNode` function with signature `registerNode((address,string,string,string,(string,string,string)[]))` and selector `[4, 133, 159, 121]`"]
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
    abi = "registerNode((address,string,string,string,(string,string,string)[]))"
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
    fn decode(data: impl AsRef<[u8]>) -> ::std::result::Result<Self, ethers::core::abi::AbiError> {
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
        if let Ok(decoded) = <InitializeCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
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
        if let Ok(decoded) = <RemoveNodeCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
        {
            return Ok(NodeRegistryCalls::RemoveNode(decoded));
        }
        if let Ok(decoded) = <WhitelistCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
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
#[doc = "`Node(address,string,string,(string,string,string)[],string,string)`"]
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
    pub network_key: String,
    pub workers: ::std::vec::Vec<Worker>,
    pub previous: String,
    pub next: String,
}
#[doc = "`NodeInfo(address,string,string,string,(string,string,string)[])`"]
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
