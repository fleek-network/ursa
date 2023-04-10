// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import "../registry/NodeRegistry.sol";
import "../epoch/EpochManager.sol";

contract RewardsAggregator {
    NodeRegistry public nodeRegistry;
    EpochManager public epochManager;

    // Todo: this will change with an added multiplier if we decide to change the epoch time
    uint16 public daysForAveragePotential;
    string[] public publicKeys;
    bool private initialized;

    ///  epoch => Node publicKey => Data served
    mapping(uint256 => mapping(string => uint256)) public DataServedInBytes;

    function initialize(address _epochManager, address _nodeRegistry) external {
        require(!initialized, "contract already initialized");
        // get whitelist nodes from registry and add epoch 0 into the metrics maaping
        epochManager = EpochManager(_epochManager);
        nodeRegistry = NodeRegistry(_nodeRegistry);

        (bool success, bytes memory result) = address(_nodeRegistry).call(abi.encodeWithSignature("getWhitelist()"));
        require(success, "Failed to call function");
        publicKeys = abi.decode(result, (string[]));
        initialized = true;
    }

    /**
     * @dev get publicKeys array that store whitelisted node
     */
    function getPublicKeys() public view returns (string[] memory) {
        return publicKeys;
    }

    /**
     * @dev record data served for given a node with given public key
     * @param epoch epoch for which the metrics are stored
     * @param publicKey public key of the node
     * @param dataServed data served from the pod transaction
     */
    function recordDataServed(uint256 epoch, string calldata publicKey, uint256 dataServed) external {
        DataServedInBytes[epoch][publicKey] += dataServed;
    }

    /**
     * @dev get data served for given node and given epoch
     * @param publicKey public key of the node
     * @param epoch epoch for which data served to get
     */
    function getDataServedByNode(string memory publicKey, uint256 epoch) public view returns (uint256) {
        return DataServedInBytes[epoch][publicKey];
    }

    /**
     * @dev get average data served per day over daysForAveragePotential epochs
     */
    function getAvgUsageNEpochs() public view returns (uint256) {
        uint256 _endEpoch = epochManager.epoch();
        uint256 _startEpoch = _endEpoch - daysForAveragePotential - 1;
        uint256 _sum = 0;
        for (uint256 i = _startEpoch; i < _endEpoch; i++) {
            _sum += getDataForEpoch(i);
        }
        return _sum / daysForAveragePotential;
    }

    /**
     * @dev get data served by all nodes in current epoch
     */
    function getDataServedCurrentEpoch() public view returns (uint256) {
        uint256 currentEpoch = epochManager.epoch();
        return getDataForEpoch(currentEpoch);
    }

    /**
     * @dev get data served by all nodes in any epoch
     * @param epoch epoch number for which served data is required
     */
    function getDataForEpoch(uint256 epoch) public view returns (uint256) {
        uint256 sum = 0;
        for (uint256 i = 0; i < publicKeys.length; i++) {
            sum += DataServedInBytes[epoch][publicKeys[i]];
        }
        return sum;
    }
}
