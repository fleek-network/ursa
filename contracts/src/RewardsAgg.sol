// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import "./NodeRegistry.sol";
import "./Epoch.sol";

contract RewardsAggregator {
    /// Node publicKey => epoch => Metrics struct
    mapping(string => mapping(uint256 => Metrics)) public metrics;

    NodeRegistry public nodeRegistry;
    EpochManager public epochManager;
    uint256 public daysForPotential;
    address public owner;

    bool private initialized;

    struct Metrics {
        uint256 DataInBytesServed;
        uint256 performanceScore;
    }

    function initialize(address _epochManager) external {
        require(!initialized, "contract already initialized");
        // get whitelist nodes from registry and add epoch 0 into the metrics maaping
        epochManager = EpochManager(_epochManager);
        initialized = true;
    }

    // maybe we don't need addNewEpoch and addNewNode and recordMetrics can handle that
    function addNewEpoch(uint256 epoch) external {
        // add a new epoch for all whitelist nodes existing in the registry
    }

    function addNewNode(string memory pubKey) external {
        // listen for new registration of the node and add node for the current epoch
    }

    /**
     * @dev record metrics for given a node with given public key
     * @param publicKey public key of the node
     * @param metric metric struct
     */
    function recordMetrics(string memory publicKey, Metrics memory metric) external {
        // must check if the node is whitelisted
        // handle the case where new epoch or node is to be added
    }

    /**
     * @dev get data served for given node and given epoch
     * @param publicKey public key of the node
     * @param epoch epoch for which data served to get
     */
    function getDataServedForNode(string memory publicKey, uint256 epoch) public view returns (uint256) {
        return metrics[publicKey][epoch].DataInBytesServed;
    }

    /**
     * @dev get average data served per day over daysForPotential epochs
     */
    function getAvgUsageNEpochs() public view returns (uint256) {
        uint256 _endEpoch = epochManager.epoch();
        uint256 _startEpoch = _endEpoch - (daysForPotential - 1);
        uint256 agg = 0;
        for (uint256 i = _startEpoch; i < _endEpoch; i++) {
            agg += _getDataForEpoch(i);
        }
        return agg;
    }

    /**
     * @dev get data served by all nodes in current epoch
     */
    function getDataServedCurrentEpoch() public view returns (uint256) {
        uint256 currentEpoch = epochManager.epoch();
        return _getDataForEpoch(currentEpoch);
    }

    /**
     * @dev get data served by all nodes in any epoch
     * @param epoch epoch number for which served data is required
     */
    function _getDataForEpoch(uint256 epoch) private view returns (uint256) {
        // go through all the whitelist node and aggregate data for an epoch
    }
}
