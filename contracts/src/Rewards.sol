// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import "./FLK.sol";
import "./Epoch.sol";
import "./RewardsAgg.sol";
import "./NodeRegistry.sol";
import "./utils/MathUtils.sol";

contract FleekReward {
    FleekToken private fleekToken;
    RewardsAggregator public rewardsAggregator;
    EpochManager public epochManager;
    NodeRegistry public nodeRegistry;
    ufixed8x2 public inflationInLastEpoch;

    bool private initialized;
    address public owner;

    // epoch number => bool
    mapping(uint256 => bool) public rewardsDistribution;

    // these can go in governance/controlled contracts
    // min factor by which the inflation can go down based on usage in %
    ufixed8x2 minInflationFactor;
    // max inflation for the year in %
    ufixed8x2 maxInflation;
    // price per byte in 1/18th USD 
    fixed price;
    // Cost of running node per byte in 1/18th USD
    fixed cost;

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
        inflationInLastEpoch = maxInflation;
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
        int256 _uActual = int256(rewardsAggregator.getDataServedCurrentEpoch());
        int256 _uPotential = int256(rewardsAggregator.getAvgUsageNEpochs());
        require(_uPotential != 0, "potential usage cannot be zero");
        int256 _deltaUNumerator = (_uActual - _uPotential) * 10^18;
        // fixed256x18 _deltaU = fixed256x18(_deltaUNumerator / _uPotential);
        // fixed256x18 inflationChange = (price/cost) * fixed256x18(_deltaU);
    }
}
