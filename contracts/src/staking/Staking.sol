// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.15;

import "./libs/Stakes.sol";
import "../management/Controlled.sol";
import "../token/FleekToken.sol";
import "../utils/TokenUtils.sol";

/**
 * @title Staking contract
 * @dev The Staking contract allows nodes to take to be elieble to be part of the network
 */
contract Staking is Controlled {
    using Stakes for Stakes.Node;

/* STORAGE */

    FleekToken internal _fleekToken;

    /// Minimum amount of tokens an node needs to stake
    uint256 public minimumNodeStake;

    /// Time in blocks to before node can unstake
    uint32 public lockTime; // in blocks

    /// Indexer stakes : indexer => Stake
    mapping(address => Stakes.Node) public stakes;

    /// List of addresses allowed to slash stakes
    mapping(address => bool) public slashers;

    //PLACEHOLDER unknown if needed
    /// Percentage of fees burned as protocol fee
    /// Parts per million. (Allows for 4 decimal points, 999,999 = 99.9999%)
    uint32 public protocolPercentage;

    // 100% in parts per million
    uint32 private constant MAX_PPM = 1000000;

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

    /**
     * @dev Emitted when `caller` set `slasher` address as `allowed` to slash stakes.
     */
    event SlasherUpdate(address indexed caller, address indexed slasher, bool allowed);

    /**
    * @dev Emitted when a contract parameter has been updated
    */
    event ParameterUpdated(string param);

    /**
     * @dev Check if the caller is the slasher.
     */
    modifier onlySlasher() {
        require(slashers[msg.sender] == true, "!slasher");
        _;
    }

     /**
     * @dev Initialize this contract.
     */
    function initialize(
        address _controller,
        address token,
        uint256 _minimumNodeStake,
        uint32 _lockTime,
        uint32 _protocolPercentage
    ) external {
        Controlled._init(_controller);

        _fleekToken = FleekToken(token);

        // Settings
        _setMinimumNodeStake(_minimumNodeStake);
        _setLockTime(_lockTime);

        _setProtocolPercentage(_protocolPercentage);

    }

    /**
     * @dev Set the minimum Node stake required to.
     * @param _minimumNodeStake Minimum Node stake
     */
    function setMinimumNodeStake(uint256 _minimumNodeStake) external onlyController {
        _setMinimumNodeStake(_minimumNodeStake);
    }

    /**
     * @dev Internal: Set the minimum Node stake required.
     * @param _minimumNodeStake Minimum Node stake
     */
    function _setMinimumNodeStake(uint256 _minimumNodeStake) private {
        require(_minimumNodeStake > 0, "minimumNodeStake must be > 0");
        minimumNodeStake = _minimumNodeStake;
        emit ParameterUpdated("minimumNodeStake");
    }

    /**
     * @dev Set the lock period for unstaking.
     * @param _lockTime in blocks to wait for token withdrawals after unstaking
     */
    function setLockTime(uint32 _lockTime) external onlyController {
        _setLockTime(_lockTime);
    }

    /**
     * @dev Internal: Set the lock time for unstaking.
     * @param _lockTime Period in blocks to wait for token withdrawals after unstaking
     */
    function _setLockTime(uint32 _lockTime) private {
        require(_lockTime > 0, "lockTime cannot be 0");
        lockTime = _lockTime;
        emit ParameterUpdated("lockTime");
    }

    /**
     * @dev Set a protocol percentage to burn when collecting query fees.
     * @param _percentage Percentage of query fees to burn as protocol fee
     */
    function setProtocolPercentage(uint32 _percentage) external onlyController {
        _setProtocolPercentage(_percentage);
    }

    /**
     * @dev Internal: Set a protocol percentage to burn when collecting query fees.
     * @param _percentage Percentage of query fees to burn as protocol fee
     */
    function _setProtocolPercentage(uint32 _percentage) private {
        // Must be within 0% to 100% (inclusive)
        require(_percentage <= MAX_PPM, ">percentage");
        protocolPercentage = _percentage;
        emit ParameterUpdated("protocolPercentage");
    }

    /**
     * @dev Set or unset an address as allowed slasher.
     * @param _slasher Address of the party allowed to slash Nodes
     * @param _allowed True if slasher is allowed
     */
    function setSlasher(address _slasher, bool _allowed) external onlyController {
        require(_slasher != address(0), "Slasher can not be 0 address");
        slashers[_slasher] = _allowed;
        emit SlasherUpdate(msg.sender, _slasher, _allowed);
    }

    /**
     * @dev Getter that returns if an Node has any stake.
     * @param _node Address of the node
     * @return True if Node has staked tokens
     */
    function hasStake(address _node) external view returns (bool) {
        return stakes[_node].tokensStaked > 0;
    }

    /**
     * @dev Get the total amount of tokens staked by the node.
     * @param _node Address of the node
     * @return Amount of tokens staked by the node
     */
    function getNodeStakedTokens(address _node) external view returns (uint256) {
        return stakes[_node].tokensStaked;
    }

    /**
     * @dev Deposit tokens on the node stake.
     * @param _tokens Amount of tokens to stake
     */
    function stake(uint256 _tokens) external {
          stakeTo(msg.sender, _tokens);
    }

    /**
     * @dev Deposit tokens on the node stake.
     * @param _node Address of the node
     * @param _tokens Amount of tokens to stake
     */
    function stakeTo(address _node, uint256 _tokens) public {
        require(_tokens > 0, "_tokens cannot be 0");

        // Ensure minimum stake
        require(
            stakes[_node].tokensStaked + _tokens >= minimumNodeStake,
            "Your stake does not meet the minimum"
      );

        // Transfer tokens to stake from caller to this contract
        TokenUtils.pullTokens(_fleekToken, msg.sender, _tokens);

        // Stake the transferred tokens
      _stake(_node, _tokens);
    }

    /**
     * @dev Stake tokens on the node.
     * This function does not check minimum indexer stake requirement to allow
     * to be called by functions that increase the stake when collecting rewards
     * without reverting
     * @param _node Address of staking party
     * @param _tokens Amount of tokens to stake
     */
    function _stake(address _node, uint256 _tokens) private {
        // Deposit tokens into the indexer stake
        stakes[_node].deposit(_tokens);

        emit StakeDeposited(_node, _tokens);
    }

}