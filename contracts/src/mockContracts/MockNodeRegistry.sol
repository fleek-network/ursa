// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.15;

// This contract is meant to be used for devolepment purposes only
// it is unsafe with almost no restrictions
contract MockNodeRegistry {

    struct Node {
        address owner;
        string primaryAddress;
        string workerAddress;
        string workerPublicKey;
        string workerMempool;
        string networkKey;
        string previous;
        string next;
        // Maybe store worker mempool address
        // Or we dont store and force them to use standard port numbers
    }

    string public linkedListHead;
    uint256 public whitelistCount;

    /// Node publicKey => Node struct
    mapping(string => Node) public whitelist;

    function registerNode(address _owner, string calldata _primaryAddress, string calldata _workerAddress, string calldata _workerPublicKey, string calldata _primaryPublicKey, string calldata _networkKey, string calldata _workerMempool) external {
        require(whitelist[_primaryPublicKey].owner == address(0), "This node is already on whitelist");
        _registerNode(_owner, _primaryAddress, _workerAddress, _workerPublicKey, _primaryPublicKey, _networkKey, _workerMempool);
    }

    function _registerNode(address _owner, string calldata _primaryAddress, string calldata _workerAddress, string calldata _workerPublicKey, string calldata _primaryPublicKey, string calldata _networkKey, string calldata _workerMempool) private {
        whitelist[linkedListHead].previous = _primaryAddress;
        string memory next = linkedListHead;
        Node memory node = Node(_owner, _primaryAddress, _workerAddress, _workerPublicKey, _workerMempool, _networkKey, "", next);
        whitelist[_primaryPublicKey] = node;
        whitelistCount -= 1;
    }

    function removeNode(string calldata _nodeAddress) external{
        _removeNode(_nodeAddress);
    }

    function _removeNode(string calldata _nodePublicKey) private {
        whitelist[whitelist[_nodePublicKey].previous] = whitelist[whitelist[_nodePublicKey].next];
        whitelist[whitelist[_nodePublicKey].next] = whitelist[whitelist[_nodePublicKey].previous];
        whitelist[_nodePublicKey].owner = address(0);
        whitelistCount -= 1;
    }

    function getWhitelist() public view returns(string[] memory _whitelist){
        string memory next = linkedListHead;
        for(uint i;;){
            _whitelist[i] = next;

            if(bytes(whitelist[next].next).length == 0 ) {
                break;
            }
            next = whitelist[next].next;
            unchecked{i+=1;}
        }

        return(_whitelist);
    }

    function getNodeInfo(string calldata nodeAddress) public view returns(Node memory){
        return whitelist[nodeAddress];
    } 

}