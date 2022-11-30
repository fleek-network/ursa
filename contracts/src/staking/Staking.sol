// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.15;

/**
 * @title Staking contract
 * @dev The Staking contract allows nodes to take to be elieble to be part of the network
 */
contract Staking {
/* STORAGE */

    /// Minimum amount of tokens an node needs to stake
    uint256 public minimumNodeStake;

    /// Time in blocks to before node can unstake
    uint32 public lockTime; // in blocks



/* EVENTS */

    /**
        * @dev Emitted when a 'node' stakes fleek token 'amount'.
    */
    event StakeDeposited(address indexed node, uint256 amount);

    /**
     * @dev Emitted when 'node' withdraws staked fleek 'amount'.
     */
    event StakeWithdrawn(address indexed node, uint256 amount);

    /**
     * @dev Emitted when 'node' was slashed for a total of fleek token 'amount'
     * also emits the 'reward' of fleek tokens give to the 'beneficiary' that made the claim
     */
    event StakeSlashed(
        address indexed node,
        uint256 amount,
        uint256 reward,
        address beneficiary
    );

}