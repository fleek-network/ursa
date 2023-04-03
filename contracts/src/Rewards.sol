// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import "./FLK.sol";
import "./Epoch.sol";
import "./RewardsAgg.sol";
import "./NodeRegistry.sol";

contract FleekReward {
    FleekToken private fleekToken;
    RewardsAggregator public rewardsAggregator;
    EpochManager public epochManager;
    NodeRegistry public nodeRegistry;

    bool private initialized;
    address public owner;

    // epoch number => bool
    mapping(uint256 => bool) public rewardsDistribution;

    // these can go in governance/controlled contracts
    // min factor by which the inflation can go down based on usage
    uint256 minInflationFactor;
    // max inflation for the year
    uint256 maxInflation;
    // price per GB in dollars
    uint256 pUSD;
    // Cost of running node per GB
    uint256 cAvg;

    event RewardMinted(address indexed account, uint256 amount);

    function initialize(address _fleekToken, address _epochManager, address _rewardsAggregator, address _nodeRegistry)
        external
    {
        require(!initialized, "Rewards contract already initialized");
        owner = msg.sender;
        fleekToken = FleekToken(_fleekToken);
        nodeRegistry = NodeRegistry(_nodeRegistry);
        epochManager = EpochManager(_epochManager);
        rewardsAggregator = RewardsAggregator(_rewardsAggregator);
        initialized = true;
    }

    modifier onlyOwner() {
        require(msg.sender == owner, "Only owner can call this function");
        _;
    }

    /**
     * @dev Distribute reward tokens to addresses.
     * @param epoch epoch for which the rewards to be distributed
     */
    function distributeRewards(uint256 epoch) public onlyOwner {
        require(epochManager.epoch() != epoch, "cannot distribute rewards for current epoch");
        require(!rewardsDistribution[epoch], "rewards already distributed for this epoch");
        // calculateRewards(epoch) calculate rewards based on rewards aggregator
        // mint tokens to distribute the rewards to the nodes identified
        rewardsDistribution[epoch] = true;
    }

    /**
     * @dev calculate rewards for all the nodes in node registry
     * @param epoch epoch for which the rewards to be calculated
     */
    function _calculateRewards(uint256 epoch) private view {
        uint256 _uActual = rewardsAggregator.getDataServedCurrentEpoch();
        uint256 _uPotential = rewardsAggregator.getAvgUsageNEpochs();

        require(_uPotential != 0, "potential usage cannot be zero");

        uint256 _deltaU = (_uActual - _uPotential) / _uPotential;

        // uint256 inflationChange = 1 - (Math.max((pUSD/cAvg) * _deltaU, minInflationFactor));
    }
}
