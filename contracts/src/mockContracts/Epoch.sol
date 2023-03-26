// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.15;

import "./MockNodeRegistry.sol";

// This contract is meant to be used for devolepment purposes only
// it is unsafe with almost no restrictions
contract MockEpoch {
    /**
     * STATE *
     */
    uint256 public epoch;

    uint256 public currentEpochEndStampMs;

    uint256 public currentCommitteeSize;

    uint256 public maxCommitteeSize;

    uint256 public epochDurationMs;

    MockNodeRegistry public nodeRegistry;
    bool private initialized;
    uint256 private readyToChange;

    struct CommitteeMember {
        string publicKey;
        string primaryAddress;
        string workerAddress;
        string workerMempool;
        string workerPublicKey;
        string networkKey;
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
        nodeRegistry = MockNodeRegistry(_nodeRegistry);
        maxCommitteeSize = _maxCommitteeSize;
        epochDurationMs = _epochDuration;
        currentEpochEndStampMs = _firstEpochStart + _epochDuration;
    }

    function getCurrentEpochInfo() public view returns (uint256, uint256, CommitteeMember[] memory) {
        CommitteeMember[] memory committeeMembers;
        for (uint256 i; i < currentCommitteeSize;) {
            MockNodeRegistry.Node memory node = nodeRegistry.getNodeInfo(committee[epoch][i]);
            committeeMembers[i].publicKey = committee[epoch][i];
            committeeMembers[i].primaryAddress = node.primaryAddress;
            committeeMembers[i].workerAddress = node.workerAddress;
            committeeMembers[i].workerPublicKey = node.workerPublicKey;
            committeeMembers[i].networkKey = node.networkKey;
            committeeMembers[i].workerMempool = node.workerMempool;

            unchecked {
                i += 1;
            }
        }

        return (epoch, currentEpochEndStampMs, committeeMembers);
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
