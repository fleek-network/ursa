// SPDX-License-Identifier: MIT

pragma solidity ^0.8.15;

import "../token/FleekToken.sol";

library TokenUtils {
    /**
     * @dev Pull tokens from an address to this contract.
     * @param _fleekToken Token to transfer
     * @param _from Address sending the tokens
     * @param _amount Amount of tokens to transfer
     */
    function pullTokens(FleekToken _fleekToken, address _from, uint256 _amount) internal {
        if (_amount > 0) {
            require(_fleekToken.transferFrom(_from, address(this), _amount), "transfer failed");
        }
    }

    /**
     * @dev Push tokens from this contract to a receiving address.
     * @param _fleekToken Token to transfer
     * @param _to Address receiving the tokens
     * @param _amount Amount of tokens to transfer
     */
    function pushTokens(FleekToken _fleekToken, address _to, uint256 _amount) internal {
        if (_amount > 0) {
            require(_fleekToken.transfer(_to, _amount), "transfer failed");
        }
    }

    /**
     * @dev Burn tokens held by this contract.
     * @param _fleekToken Token to burn
     * @param _amount Amount of tokens to burn
     */
    function burnTokens(FleekToken _fleekToken, uint256 _amount) internal {
        if (_amount > 0) {
            _fleekToken.burn(_amount);
        }
    }
}
