// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.15;

import "../../utils/MathUtils.sol";

/**
 * @title A collection of data structures and functions to manage the Node Stake state.
 *        Used for low-level state changes, require() conditions should be evaluated
 *        at the caller function scope.
 */
library Stakes {
    using Stakes for Stakes.Node;

    struct Node {
        uint256 tokensStaked; // Tokens staked that are not locked.
        uint256 tokensLocked; // Tokens locked for withdrawal subject to lockPeriod.
        uint256 tokensLockedUntil; // The epoch that locked tokens can be withdrawn.
        uint256 eligableAt; // The epoch this node is eligable to be whitelisted.
        // uint256 nodeAddress; // The address associated with this node.
        address owner; // The ethereum address that owns this node.
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
     * @param _period Period in epochs that need to pass before withdrawal
     */
    function lockTokens(Stakes.Node storage stake, uint256 _tokens, uint256 _period, uint256 _currentEpoch) internal {
        release(stake, _tokens);
        // Take into account period averaging for multiple unstake requests
        uint256 lockingPeriod = _period;

        if (stake.tokensLocked > 0) {
            lockingPeriod = MathUtils.weightedAverage(
                MathUtils.diffOrZero(stake.tokensLockedUntil, _currentEpoch), // Remaining thawing period
                stake.tokensLocked, // Weighted by remaining unstaked tokens
                _period, // Thawing period
                _tokens // Weighted by new tokens to unstake
            );
        }

        // Update balances
        stake.tokensLocked = stake.tokensLocked + _tokens;

        stake.tokensLockedUntil = _currentEpoch + lockingPeriod;
    }

    /**
     * @dev Unlock tokens.
     * @param stake Stake data
     * @param _tokens Amount of tokens to unlock
     */
    function unlockTokens(Stakes.Node storage stake, uint256 _tokens) internal {
        stake.tokensLocked = stake.tokensLocked - _tokens;
        if (stake.tokensLocked == 0) {
            stake.tokensLockedUntil = 0;
        }
    }

    /**
     * @dev Take all tokens out from the locked stake for withdrawal.
     * @param stake Stake data
     * @return Amount of tokens being withdrawn
     */
    function withdrawTokens(Stakes.Node storage stake, uint256 _currentEpoch) internal returns (uint256) {
        // Calculate tokens that can be released
        uint256 tokensToWithdraw = stake.tokensWithdrawable(_currentEpoch);

        if (tokensToWithdraw > 0) {
            // Reset locked tokens
            stake.unlockTokens(tokensToWithdraw);
        }

        return tokensToWithdraw;
    }

    /**
     * @dev Set the epoch this node is elegible to be whitelisted
     * @param stake Stake data
     * @param _elegibilityPeriod The period set on staking.sol that a node must stake for
     */
    function setElegibleEpoch(Stakes.Node storage stake, uint256 _elegibilityPeriod, uint256 _currentEpoch) internal {
        stake.eligableAt = _currentEpoch + _elegibilityPeriod;
    }

    /**
     * @dev Remove this nodes eligibility time
     * @param stake Stake data
     */
    function removeElegibility(Stakes.Node storage stake) internal {
        stake.eligableAt = 0;
    }

    /**
     * @dev Tokens available for withdrawal after lock period.
     * @param stake Stake data
     * @return Token amount
     */
    function tokensWithdrawable(Stakes.Node memory stake, uint256 _currentEpoch) internal pure returns (uint256) {
        // No tokens to withdraw before locking period
        if (stake.tokensLockedUntil == 0 || _currentEpoch < stake.tokensLockedUntil) {
            return 0;
        }
        return stake.tokensLocked;
    }
}
