// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.15;

import "../../utils/MathUtils.sol";

/**
 * @title A collection of data structures and functions to manage the Indexer Stake state.
 *        Used for low-level state changes, require() conditions should be evaluated
 *        at the caller function scope.
 */
 library Stakes {
    using Stakes for Stakes.Node;

    struct Node {
        uint256 tokensStaked; // Total tokens Node has staked
        uint256 tokensLocked; // Tokens locked for withdrawal subject to lockPeriod
        uint256 tokensLockedUntil; // The block that locked tokens can be withdrawn
    }

    /**
     * @dev Deposit tokens to the Node stake.
     * @param stake Stake data
     * @param _tokens Amount of tokens to deposit
     */
    function deposit(Stakes.Node storage stake, uint256 _tokens) internal {
        stake.tokensStaked = stake.tokensStaked + _tokens;
    }

    /**
     * @dev Release tokens from the Node stake.
     * @param stake Stake data
     * @param _tokens Amount of tokens to release
     */
    function release(Stakes.Node storage stake, uint256 _tokens) internal {
        stake.tokensStaked = stake.tokensStaked - _tokens;
    }

    /**
     * @dev Lock tokens until a locking period passes.
     * @param stake Stake data
     * @param _tokens Amount of tokens to unstake
     * @param _period Period in blocks that need to pass before withdrawal
     */
    function lockTokens(
        Stakes.Node storage stake,
        uint256 _tokens,
        uint256 _period
    ) internal {
        // Take into account period averaging for multiple unstake requests
        uint256 lockingPeriod = _period;
        if (stake.tokensLocked > 0) {
            lockingPeriod = MathUtils.weightedAverage(
                MathUtils.diffOrZero(stake.tokensLockedUntil, block.number), // Remaining thawing period
                stake.tokensLocked, // Weighted by remaining unstaked tokens
                _period, // Thawing period
                _tokens // Weighted by new tokens to unstake
            );
        }

        // Update balances
        stake.tokensLocked = stake.tokensLocked + _tokens;
        stake.tokensLockedUntil = block.number + lockingPeriod;
    }

 }