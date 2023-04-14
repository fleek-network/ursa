// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import "../management/Controlled.sol";
import "../token/FleekToken.sol";
import "../registry/NodeRegistry.sol";
import "../epoch/EpochManager.sol";
import "./RewardsAggregator.sol";
import "../utils/MathUtils.sol";
import {SD59x18, sd, intoInt256, intoUint256, UNIT, convert} from "prb/math/SD59x18.sol";

/**
 * @title Fleek Reward Contract
 * @dev This contract calculates and distributes the rewards
 */

contract FleekReward is Controlled {
    FleekToken private fleekToken;
    RewardsAggregator public rewardsAggregator;
    EpochManager public epochManager;
    NodeRegistry public nodeRegistry;
    SD59x18 public inflationInLastEpoch;

    bool private initialized;

    // epoch number => bool
    mapping(uint256 => bool) public rewardsDistribution;

    uint256 constant DAYS_IN_YEAR = 365;

    // min factor by which the inflation can go down based on usage in %
    SD59x18 public minInflationFactor;
    // max inflation for the year in %
    SD59x18 public maxInflation;
    // price per byte
    SD59x18 public price;
    // Cost of running node
    SD59x18 public cost;

    event RewardMinted(address indexed account, uint256 amount);

    function initialize(address _controller, address _token, address _epoch, address _aggregator, address _registry)
        external
    {
        require(!initialized, "Rewards contract already initialized");
        Controlled._init(_controller);
        fleekToken = FleekToken(_token);
        nodeRegistry = NodeRegistry(_registry);
        epochManager = EpochManager(_epoch);
        rewardsAggregator = RewardsAggregator(_aggregator);
        inflationInLastEpoch = maxInflation;
        initialized = true;
    }

    function setInflationRate(SD59x18 _inflationRate) external {
        maxInflation = _inflationRate;
    }

    function setMinInflationFactor(SD59x18 _minInflationFactor) external {
        minInflationFactor = _minInflationFactor;
    }

    function setPrice(SD59x18 _price) external {
        price = _price;
    }

    function setCost(SD59x18 _cost) external {
        cost = _cost;
    }

    /**
     * @dev Distribute reward tokens to addresses.
     * @param epoch epoch for which the rewards to be distributed
     */
    function distributeRewards(uint256 epoch) public onlyController {
        require(epochManager.epoch() != epoch, "cannot distribute rewards for current epoch");
        require(!rewardsDistribution[epoch], "rewards already distributed for this epoch");

        SD59x18 _uActual = convert(int256(rewardsAggregator.getDataForEpoch(epoch)));
        SD59x18 _uPotential = convert(int256(rewardsAggregator.getAvgUsageNEpochs(epoch)));

        SD59x18 _totalMint = _getMintRate(_uActual, _uPotential);
        // todo: variable distribution
        // 75% goes to edge node
        SD59x18 _toEdgeNode = _totalMint.mul(sd(0.75e18));
        string[] memory publicKeys = rewardsAggregator.getPublicKeys(epoch);
        uint256 pkLen =  publicKeys.length;
        rewardsDistribution[epoch] = true;

        for (uint256 i = 0; i < pkLen;) {
            uint256 dataServedByNode = rewardsAggregator.getDataServedByNode(publicKeys[i], epoch);
            SD59x18 servedPercentage = convert(int256(dataServedByNode)).div(_uActual);
            SD59x18 rewardsAmount = servedPercentage.mul(_toEdgeNode);
            // check if the node with public key is white listed
            (address to,,,,) = nodeRegistry.whitelist(publicKeys[i]);
            fleekToken.mint(_to, intoUint256(rewardsAmount));
            emit RewardMinted(_to, intoUint256(rewardsAmount));
                        
            unchecked {
                i += 1;
            }
        }
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
