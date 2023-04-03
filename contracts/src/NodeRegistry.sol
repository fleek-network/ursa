// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

contract NodeRegistry {
    /**
     * STATE *
     */
    /// Node publicKey => Node struct
    mapping(string => Node) public whitelist;

    string public linkedListHead;
    uint256 public whitelistCount;
    bool private initialized;

    struct Node {
        address owner;
        string primaryAddress;
        string networkKey;
        Worker[] workers;
        string previous;
        string next;
    }

    struct NodeInfo {
        address owner;
        string primaryPublicKey;
        string primaryAddress;
        string networkKey;
        Worker[] workers;
    }

    struct Worker {
        string workerAddress;
        string workerPublicKey;
        string workerMempool;
    }

    function initialize(NodeInfo[] memory _genesis_committee) external {
        require(!initialized, "contract already initialized");
        for (uint256 i; i < _genesis_committee.length;) {
            _registerNode(_genesis_committee[i]);
            unchecked {
                i += 1;
            }
        }
        initialized = true;
    }

    function registerNode(NodeInfo memory _node) external {
        require(whitelist[_node.primaryPublicKey].owner == address(0), "This node is already on whitelist");
        _registerNode(_node);
    }

    function _registerNode(NodeInfo memory _node) private {
        whitelist[linkedListHead].previous = _node.primaryAddress;
        string memory next = linkedListHead;
        Node storage node = whitelist[_node.primaryPublicKey];
        node.owner = _node.owner;
        node.primaryAddress = _node.primaryAddress;
        node.networkKey = _node.networkKey;
        node.next = next;

        for (uint256 i; i < _node.workers.length;) {
            node.workers.push(_node.workers[i]);
            unchecked {
                i += 1;
            }
        }

        whitelistCount += 1;
        linkedListHead = _node.primaryPublicKey;
    }

    function removeNode(string calldata _nodeAddress) external {
        _removeNode(_nodeAddress);
    }

    function _removeNode(string calldata _nodePublicKey) private {
        whitelist[whitelist[_nodePublicKey].previous] = whitelist[whitelist[_nodePublicKey].next];
        whitelist[whitelist[_nodePublicKey].next] = whitelist[whitelist[_nodePublicKey].previous];
        whitelist[_nodePublicKey].owner = address(0);
        whitelistCount -= 1;
    }

    function getWhitelist() public view returns (string[] memory _whitelist) {
        string memory next = linkedListHead;
        _whitelist = new string[](whitelistCount);
        for (uint256 i;;) {
            _whitelist[i] = next;

            if (bytes(whitelist[next].next).length == 0) {
                break;
            }
            next = whitelist[next].next;
            unchecked {
                i += 1;
            }
        }

        return (_whitelist);
    }

    function getNodeInfo(string calldata nodePublicKey) public view returns (Node memory) {
        //Todo(dalton): transform to nodeinfo to not return unneeded linked list fields
        return whitelist[nodePublicKey];
    }
}
