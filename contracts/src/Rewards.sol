// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import "./FLK.sol";
import "./Epoch.sol";
import "./RewardsAgg.sol";
import "./NodeRegistry.sol";
import "./utils/MathUtils.sol";
import {SD59x18, sd, intoInt256, intoUint256, UNIT, convert} from "prb/math/SD59x18.sol";

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

        SD59x18 _uActual = convert(int256(rewardsAggregator.getDataForEpoch(epoch)));
        SD59x18 _uPotential = convert(int256(rewardsAggregator.getAvgUsageNEpochs()));

        SD59x18 _total_mint = _getMintRate(_uActual, _uPotential);
        string[] memory publicKeys = rewardsAggregator.getPublicKeys();

        for (uint256 i = 0; i < publicKeys.length; i++) {
            uint256 dataServedByNode = rewardsAggregator.getDataServedByNode(publicKeys[i], epoch);
            SD59x18 servedPercentage = convert(int256(dataServedByNode)).div(_uActual);
            SD59x18 rewardsAmount = servedPercentage.mul(_total_mint);
            (address _to,,,,) = nodeRegistry.whitelist(publicKeys[i]);
            fleekToken.mint(_to, intoUint256(rewardsAmount));
        }
        rewardsDistribution[epoch] = true;
    }

    /**
     * @dev calculate the minting based actual usage and potential usage
     * @param _uActual actual usage in the epoch for which the minting is calculated
     * @param _uPotential potential usage in the epoch for which the minting is calculated
     */
    function _getMintRate(SD59x18 _uActual, SD59x18 _uPotential) private returns (SD59x18 totalMint) {
        // Equation 2 from the paper
        // delta U = (_uActual - _uPotential)/uPotential
        SD59x18 _deltaUNumerator = _uActual.sub(_uPotential);
        SD59x18 _deltaU = _deltaUNumerator.div(_uPotential);

        // Equation 3 from the paper
        SD59x18 potentialFactor = UNIT.sub(((price).div(cost)).mul(_deltaU));
        SD59x18 firstMax = sd(MathUtils.signedMax(intoInt256(potentialFactor), intoInt256(minInflationFactor)));
        SD59x18 dynamicInflation = firstMax.mul(inflationInLastEpoch);
        SD59x18 currentInflation = sd(MathUtils.signedMin(intoInt256(dynamicInflation), intoInt256(maxInflation)));
        inflationInLastEpoch = currentInflation;

        // Equation 4 from the paper
        uint256 totalSupply = fleekToken.totalSupply();
        totalMint = ((convert(int256(totalSupply))).mul(currentInflation)).div(convert(int256(DAYS_IN_YEAR)));
    }
}
