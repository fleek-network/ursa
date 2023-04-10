// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

/**
 * @title Fleek Reward Aggreagator
 * @dev This contract aggregates data served by each node in an epoch
 */
contract RewardsAggregator {
    uint16 public daysForAveragePotential;
    bool private initialized;

    ///  epoch => Node publicKey => Data served
    mapping(uint256 => mapping(string => uint256)) public DataServedInBytes;
    ///  epoch => Public key list
    mapping(uint256 => string []) public publicKeys;
    ///  epoch => public key => key added
    mapping(uint256 => mapping(string => bool)) public publicKeyAdded;


    function initialize() external {
        require(!initialized, "contract already initialized");

        initialized = true;
    }

    /**
     * @dev get publicKeys array that store whitelisted node
     */
    function getPublicKeys(uint256 _epoch) public view returns (string[] memory) {
        return publicKeys[_epoch];
    }

    /**
     * @dev record data served for given a node with given public key
     * @param epoch epoch for which the metrics are stored
     * @param publicKey public key of the node
     * @param dataServed data served from the pod transaction
     */
    function recordDataServed(uint256 epoch, string calldata publicKey, uint256 dataServed) external {
        if (!publicKeyAdded[epoch][publicKey]) {
            publicKeys[epoch].push(publicKey);
            publicKeyAdded[epoch][publicKey] = true;
        }
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
    function getAvgUsageNEpochs(uint256 _epoch) public view returns (uint256) {
        uint256 _startEpoch = _epoch <= daysForAveragePotential ? 0 : _epoch - daysForAveragePotential;
        uint256 _sum = 0;
        for (uint256 i = _startEpoch; i < _epoch; i++) {
            _sum += getDataForEpoch(i);
        }
        return _sum / (_epoch - _startEpoch);
    }

    /**
     * @dev get data served by all nodes in any epoch
     * @param epoch epoch number for which served data is required
     */
    function getDataForEpoch(uint256 epoch) public view returns (uint256) {
        uint256 sum = 0;
        for (uint256 i = 0; i < publicKeys[epoch].length; i++) {
            sum += DataServedInBytes[epoch][publicKeys[epoch][i]];
        }
        return sum;
    }
}
