// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import {SD59x18} from "prb/math/SD59x18.sol";
import "../rewards/RewardsManager.sol";

/**
 * @title Parameters Contracts
 * @dev This contract exposes all the parameters used in the economic model
 * this contract is the only way to update the parameters
 */

contract Parameters {
    RewardsManager private rewards;
    bool private initialized;

    function initialize(address _rewards) external {
        require(!initialized, "Parameters contract already initialized");
        rewards = RewardsManager(_rewards);
        initialized = true;
    }

    function inflationRate() external view returns (SD59x18) {
        return rewards.maxInflation();
    }

    function setInflationRate(SD59x18 _inflationRate) public {
        rewards.setInflationRate(_inflationRate);
    }

    function minInflationFactor() public view returns (SD59x18) {
        return rewards.minInflationFactor();
    }

    function setMinInflationFactor(SD59x18 _minInflationFactor) public {
        rewards.setMinInflationFactor(_minInflationFactor);
    }

    function price() public view returns (SD59x18) {
        return rewards.price();
    }

    function setPrice(SD59x18 _price) public {
        rewards.setPrice(_price);
    }

    function cost() public view returns (SD59x18) {
        return rewards.cost();
    }

    function setCost(SD59x18 _cost) public {
        rewards.setCost(_cost);
    }
}