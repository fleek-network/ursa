// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

/**
 * @title Reputation score contract
 * @dev This contract aggregates a nodes reputation per epoch
 */
contract ReputationScores {
    ///First address in the linked list
    string internal constant SENTINEL_ADDRESS = "0x01";

    /// Epoch => NodePeerId => Scores
    mapping(uint256 => mapping(string => NodeScores)) repScores;

    /// Epoch => The number of nodes who reported scores this epoch.
    mapping(uint256 => uint256) reportingNodes;

    struct NodeScores {
        Measurement[] measurements;
        string next;
    }

    struct Measurement {
        string peerId;
        uint64 bandwidth;
        uint32 latency;
        uint128 uptime;
    }

    struct EpochScores {
        string peerId;
        Measurement[] measurements;
    }

    function submitScores(uint256 _epoch, EpochScores memory _scores) external {
        require(
            bytes(repScores[_epoch][_scores.peerId].next).length == 0,
            "This node already submited scores for this epoch"
        );

        string memory next = repScores[_epoch][SENTINEL_ADDRESS].next;
        NodeScores storage scores = repScores[_epoch][_scores.peerId];
        // Set the new scores to the next node in the list
        scores.next = next;
        for (uint256 i; i < _scores.measurements.length;) {
            scores.measurements.push(_scores.measurements[i]);
            unchecked {
                i += 1;
            }
        }

        reportingNodes[_epoch] += 1;

        // Set the sentinal address next to this node since its now the first in the list.
        repScores[_epoch][SENTINEL_ADDRESS].next = _scores.peerId;
    }

    function getScores(uint256 _epoch) public view returns (EpochScores[] memory) {
        string memory nextNode = repScores[_epoch][SENTINEL_ADDRESS].next;
        EpochScores[] memory epochScores = new EpochScores[](reportingNodes[_epoch]);
        for (uint256 i;;) {
            if (bytes(nextNode).length <= 0) {
                break;
            }
            epochScores[i] = EpochScores(nextNode, repScores[_epoch][nextNode].measurements);
            nextNode = repScores[_epoch][nextNode].next;
            unchecked {
                i += 1;
            }
        }
        return (epochScores);
    }
}
