// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import "../registry/NodeRegistry.sol";

contract EpochManager {
    /**
     * STATE *
     */
    uint256 public epoch;

    uint256 public currentEpochEndStampMs;

    uint256 public currentCommitteeSize;

    uint256 public maxCommitteeSize;

    uint256 public epochDurationMs;

    NodeRegistry public nodeRegistry;
    bool private initialized;
    uint256 private readyToChange;

    struct CommitteeMember {
        string publicKey;
        string primaryAddress;
        string networkKey;
        NodeRegistry.Worker[] workers;
    }

    /// epoch => committee public keys
    mapping(uint256 => string[]) public committee;

    function initialize(
        address _nodeRegistry,
        uint256 _firstEpochStart,
        uint256 _epochDuration,
        uint256 _maxCommitteeSize
    ) external {
        require(!initialized, "contract already initialized");
        nodeRegistry = NodeRegistry(_nodeRegistry);
        maxCommitteeSize = _maxCommitteeSize;
        epochDurationMs = _epochDuration;
        currentEpochEndStampMs = _firstEpochStart + _epochDuration;

        committee[epoch] = _chooseNewCommittee();
        currentCommitteeSize = committee[epoch].length;
        initialized = true;
    }

    function getCurrentEpochInfo()
        public
        view
        returns (uint256 _epoch, uint256 _currentEpochEndMs, CommitteeMember[] memory _committeeMembers)
    {
        _committeeMembers = new CommitteeMember[](currentCommitteeSize);
        for (uint256 i; i < currentCommitteeSize;) {
            NodeRegistry.Node memory node = nodeRegistry.getNodeInfo(committee[epoch][i]);
            _committeeMembers[i].publicKey = committee[epoch][i];
            _committeeMembers[i].primaryAddress = node.primaryAddress;
            _committeeMembers[i].networkKey = node.networkKey;
            _committeeMembers[i].workers = node.workers;

            unchecked {
                i += 1;
            }
        }

        return (epoch, currentEpochEndStampMs, _committeeMembers);
    }

    function getCurrentCommittee() public view returns (string[] memory) {
        return committee[epoch];
    }

    function signalEpochChange(string memory committeeMember) external returns (bool) {
        for (uint256 i;;) {
            if (keccak256(abi.encodePacked(committee[epoch][i])) == keccak256(abi.encodePacked(committeeMember))) {
                readyToChange++;
                break;
            }
            unchecked {
                i++;
            }
            if (i == currentCommitteeSize) {
                revert();
            }
        }
        uint256 roundUp;
        if (currentCommitteeSize * 2 / 3 > 0) {
            roundUp = 1;
        }

        if (readyToChange >= (currentCommitteeSize * 2 / 3) + roundUp) {
            _changeEpoch();
            return (true);
        } else {
            return (false);
        }
    }

    function _changeEpoch() private {
        epoch++;

        committee[epoch] = _chooseNewCommittee();
        currentEpochEndStampMs = currentEpochEndStampMs + epochDurationMs;
        currentCommitteeSize = committee[epoch].length;
    }

    function _chooseNewCommittee() private view returns (string[] memory _committee) {
        //TODO: actual randomnes
        uint256 nodeCount = nodeRegistry.whitelistCount();
        string[] memory allNodes = nodeRegistry.getWhitelist();
        if (nodeCount < maxCommitteeSize) {
            return allNodes;
        }

        for (uint256 i;;) {
            uint256 randomNumber = uint256(keccak256(abi.encodePacked(i, blockhash(block.number - 1)))) % nodeCount;
            if (bytes(allNodes[randomNumber]).length > 0) {
                _committee[i] = allNodes[randomNumber];
                //set chosen node to 0 so it doesnt get chosen again
                allNodes[randomNumber] = "";

                if (i >= maxCommitteeSize) {
                    break;
                }
                unchecked {
                    i += 1;
                }
            }
        }
        return _committee;
    }
}
