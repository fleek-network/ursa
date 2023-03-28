// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.15;

import "solmate/tokens/ERC20.sol";

contract FleekToken is ERC20 {
    /* STATE */

    mapping(address => bool) private _minters;

    /* EVENTS */

    event MinterAdded(address indexed account);
    event MinterRemoved(address indexed account);

    constructor(uint256 _initialSupply) ERC20("Fleek", "FLK", 18) {
        _mint(msg.sender, _initialSupply);

        _addMinter(msg.sender);
    }

    /**
     * @dev Add a new minter.
     * @param _account Address of the minter
     */
    function addMinter(address _account) external {
        _addMinter(_account);
    }

    /**
     * @dev Remove a minter.
     * @param _account Address of the minter
     */
    function removeMinter(address _account) external {
        _removeMinter(_account);
    }

    /**
     * @dev Mint new tokens.
     * @param _to Address to send the newly minted tokens
     * @param _amount Amount of tokens to mint
     */
    function mint(address _to, uint256 _amount) external {
        _mint(_to, _amount);
    }

    /**
     * @dev Return if the `_account` is a minter or not.
     * @param _account Address to check
     * @return True if the `_account` is minter
     */
    function isMinter(address _account) public view returns (bool) {
        return _minters[_account];
    }

    /**
     * @dev Add a new minter.
     * @param _account Address of the minter
     */
    function _addMinter(address _account) private {
        _minters[_account] = true;
        emit MinterAdded(_account);
    }

    /**
     * @dev Remove a minter.
     * @param _account Address of the minter
     */
    function _removeMinter(address _account) private {
        _minters[_account] = false;
        emit MinterRemoved(_account);
    }

    /**
     * @dev Destroys `amount` tokens from the caller.
     *
     * See {ERC20-_burn}.
     */
    function burn(uint256 amount) external {
        _burn(msg.sender, amount);
    }
}
