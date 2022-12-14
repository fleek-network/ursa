// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.15;

import "../management/Controlled.sol";

contract NodeRegistry is Controlled {
    struct Node {
        string url;
        address owner;
    }

    /**  STATE **/

    address public stakingContract;

    /// Node address => Node struct
    mapping(uint256 => Node) public whitelist;

    /* EVENTS */

    /**
     * @dev emmitted when a node is added to the whitelist
     */
    event NodeRegistered(uint256 indexed node, string indexed url);

    /**
     * @dev emmitted when a node is removed from the whitelist
     */
    event NodeRemoved(uint256 indexed node);

    /**
     * @dev Initialize this contract.
     */
    function initialize(address _controller, address _stakingContract) external {
        Controlled._init(_controller);
        stakingContract = _stakingContract;
    }

    /** MODIFIERS **/

    /**
     * @dev Check if the caller is the staking contract
     */
    modifier onlyStaking() {
        require(msg.sender == stakingContract, "Only the staking contract can call this function");
        _;
    }

    /**
     * @dev External only controller: Set address for the staking contract
     * @param _stakingContract Staking Contract address.
     */ 
    function setStakingContract(address _stakingContract) external onlyController {
        require(_stakingContract != address(0), "Staking contract cannot be address 0");
        stakingContract = _stakingContract;
    }

    function registerNode(uint256 _nodeAddress, address _owner, string calldata _url) external onlyStaking {
        _registerNode(_nodeAddress,_owner,_url);
    }

    function removeNode(uint256 _nodeAddress) external onlyStaking{
        _removeNode(_nodeAddress);
    }

    function isWhitelisted(uint256 _nodeAddress) public view returns(bool) {
        return whitelist[_nodeAddress].owner != address(0);
    }

    function _registerNode(uint256 _nodeAddress, address _owner, string calldata _url) private {
        require(bytes(_url).length > 0, "Node must specify a URL");
        require(whitelist[_nodeAddress].owner == address(0), "This node is already on whitelist");

        whitelist[_nodeAddress] = Node(_url, _owner);

        emit NodeRegistered(_nodeAddress, _url);
    }

    function _removeNode(uint256 _nodeAddress) private {
        require(whitelist[_nodeAddress].owner == address(0), "No node with that address on whitelist");

        whitelist[_nodeAddress].url = "";
        whitelist[_nodeAddress].owner = address(0);

        emit NodeRemoved(_nodeAddress);
    }
}
