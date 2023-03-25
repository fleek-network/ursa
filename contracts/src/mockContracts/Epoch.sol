// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.15;

import "./MockNodeRegistry.sol";

// This contract is meant to be used for devolepment purposes only
// it is unsafe with almost no restrictions
contract MockEpoch {
    /**  STATE **/
    uint256 public epoch;
    uint256 public maxCommitteeSize = 512;
    uint256 public currentCommitteeSize;
    MockNodeRegistry public nodeRegistry;
    bool private initialized;
    uint256 private readyToChange;

    /// epoch => committee public keys
    mapping(uint256 => string[]) public committee;

    function initialize(address _nodeRegistry) external {
        require(!initialized, "contract already initialized");
        nodeRegistry = MockNodeRegistry(_nodeRegistry);
    }

    function getCurrentCommittee() public view returns(string[] memory){
        return committee[epoch];
    }

    function signalEpochChange(string memory committeeMember) external returns(bool){
        for(uint i;;) {
            if(keccak256(abi.encodePacked(committee[epoch][i])) == keccak256(abi.encodePacked(committeeMember))){
                readyToChange++;
                break;
            }
            unchecked{i++;}
            if(i == currentCommitteeSize){
                revert();
            }
        }
        uint roundUp;
        if(currentCommitteeSize * 2 / 3 > 0){
            roundUp = 1;
        }

        if(readyToChange >= (currentCommitteeSize * 2 / 3) + roundUp){
            _changeEpoch();
            return(true);
        } else{
            return(false);
        }

    }
        function _changeEpoch() private {
        epoch++;

        committee[epoch] = _chooseNewCommittee();
        currentCommitteeSize = committee[epoch].length;
    }

    function _chooseNewCommittee() private view returns(string[] memory _committee){
        //TODO: actual randomnes
        uint256 nodeCount = nodeRegistry.whitelistCount();
        string[] memory allNodes = nodeRegistry.getWhitelist();
        if(nodeCount < maxCommitteeSize){
            return allNodes;
        }
       
        for(uint i;;){
            uint randomNumber = uint(keccak256(abi.encodePacked(i, blockhash(block.number - 1)))) % nodeCount;
            if(bytes(allNodes[randomNumber]).length > 0){
                _committee[i] = allNodes[randomNumber];
                //set chosen node to 0 so it doesnt get chosen again
                allNodes[randomNumber] = "";
                
                if(i >= maxCommitteeSize){
                    break;
                }
                unchecked{i += 1;}
            }
        }
        return _committee;
    }
}