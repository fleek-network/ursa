// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import "./FLK.sol";
import "./Epoch.sol";
import "./RewardsAgg.sol";
import "./NodeRegistry.sol";
import "./utils/MathUtils.sol";
import {SD59x18, sd, intoInt256} from "prb/math/SD59x18.sol";

contract FleekReward {
    uint256 constant DAYS_IN_YEAR = 365;
    FleekToken private fleekToken;
    RewardsAggregator public rewardsAggregator;
    EpochManager public epochManager;
    NodeRegistry public nodeRegistry;
    SD59x18 public inflationInLastEpoch;

    bool private initialized;
    address public owner;

    // epoch number => bool
    mapping(uint256 => bool) public rewardsDistribution;

    // these can go in governance/controlled contracts
    // min factor by which the inflation can go down based on usage in %
    SD59x18 minInflationFactor;
    // max inflation for the year in %
    SD59x18 maxInflation;
    // price per byte in 1/18th USD
    SD59x18 price;
    // Cost of running node per byte in 1/18th USD
    SD59x18 cost;

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
        SD59x18 _mint_rate = _getMintRate();
        // calculateRewards(epoch) calculate rewards based on rewards aggregator
        // mint tokens to distribute the rewards to the nodes identified
        rewardsDistribution[epoch] = true;
    }

    /**
     * @dev calculate rewards for all the nodes in node registry
     */
    function _getMintRate() private view returns (SD59x18) {
        int256 _uActual = int256(rewardsAggregator.getDataServedCurrentEpoch());
        int256 _uPotential = int256(rewardsAggregator.getAvgUsageNEpochs());
        require(_uPotential != 0, "potential usage cannot be zero");

        // Equation 2 from the paper
        // delta U = (_uActual - _uPotential)/uPotential
        SD59x18 _deltaUNumerator = sd((_uActual - _uPotential) * 1e18);
        SD59x18 _deltaU = _deltaUNumerator.div(sd(_uPotential));

        // Equation 3 from the paper
        SD59x18 potentialFactor = ((price.mul(sd(1e18))).div(cost)).mul(_deltaU);
        int256 firstMax = 1e18 - (MathUtils.signedMax(intoInt256(potentialFactor), intoInt256(minInflationFactor)));
        SD59x18 dynamicInflation = sd(firstMax).mul(inflationInLastEpoch);
        SD59x18 currentInflation = sd(MathUtils.signedMin(intoInt256(dynamicInflation), intoInt256(maxInflation)));

        // Equation 4 from the paper
        uint256 totalSupply = fleekToken.totalSupply();
        return (sd(int256(totalSupply * 1e18))).mul(currentInflation).div((sd(int256(DAYS_IN_YEAR * 1e18))));
    }
}
