// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.15;

import "solmate/tokens/ERC20.sol";
import "../management/Controlled.sol";

contract FleekToken is Controlled, ERC20 {
    /* STATE */

    mapping(address => bool) private _minters;

    /* EVENTS */

    event MinterAdded(address indexed account);
    event MinterRemoved(address indexed account);


    constructor(uint256 _initialSupply) ERC20("Fleek", "FLK", 18) {

        Controlled._init(msg.sender);
        
        _mint(msg.sender, _initialSupply);

        _addMinter(msg.sender);

    }

    /**
     * @dev Add a new minter.
     * @param _account Address of the minter
     */
    function addMinter(address _account) external onlyController {
        _addMinter(_account);
    }

    /**
     * @dev Remove a minter.
     * @param _account Address of the minter
     */
    function removeMinter(address _account) external onlyController {
        _removeMinter(_account);
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
