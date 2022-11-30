// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.15;

/**
 * @title Fleek Network Controller contract
 * @dev All contracts that need to be controlled should extend this contract has a 2 step transfer to provide security against transfering to a bad address
 */
abstract contract Controlled {
/* EVENTS */

    event NewPendingController(address indexed controller, address indexed newController);
    event NewController(address indexed oldController, address indexed newController);

/* STATE */

    address public controller;
    address public pendingController;

/**
 * @dev only controller can call this function
 */
    modifier onlyController() {
        require (msg.sender == controller, "Only the contract controller can call this function");
        _;
 }


/**
* @dev Initialize the controller
* @param _initController address of the initial controller
 */
function _init(address _initController) internal {
    controller = _initController;
}

/**
* @notice Start the transfer controller process. The new controller must call 
* AcceptTransfer to complete the process
* @param _newController address of the new controller
 */
 function transferController(address _newController) external onlyController {
    require (_newController != address(0), "Governor cant be null");

    pendingController = _newController;

    emit NewPendingController(controller, _newController);
 }

 /**
* @notice This function completes the controller transfer. Must be called by the pendingControlelr
 */
 function acceptTransfer() external {
    require (msg.sender == pendingController, "Caller must be the pending Controller");
    
    address oldController = controller;

    controller = pendingController;

    emit NewController(oldController, controller);
 }

}

