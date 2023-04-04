// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

/**
 * @title MathUtils Library
 * @notice Useful maths
 */
library MathUtils {
    /**
     * @dev Calculates the weighted average of two values pondering each of these
     * values based on configured weights. The contribution of each value N is
     * weightN/(weightA + weightB).
     * @param valueA The amount for value A
     * @param weightA The weight to use for value A
     * @param valueB The amount for value B
     * @param weightB The weight to use for value B
     */
    function weightedAverage(uint256 valueA, uint256 weightA, uint256 valueB, uint256 weightB)
        internal
        pure
        returns (uint256)
    {
        return ((valueA * weightA) + (valueB * weightB)) / (weightA + weightB);
    }

    /**
     * @dev Returns the difference between two numbers or zero if negative.
     */
    function diffOrZero(uint256 x, uint256 y) internal pure returns (uint256) {
        return (x > y) ? x - y : 0;
    }

    /**
     * @dev Returns the minimum of two numbers.
     */
    function min(uint256 x, uint256 y) internal pure returns (uint256) {
        return x <= y ? x : y;
    }

    /**
     * @dev Returns the max of two numbers.
     */
    function max(uint256 x, uint256 y) internal pure returns (uint256) {
        return x >= y ? x : y;
    }

    /**
     * @dev Returns the max of two fixed point numbers.
     */
    function fixedPointmax(fixed x, fixed y) internal pure returns (fixed) {
        return x >= y ? x : y;
    }
}