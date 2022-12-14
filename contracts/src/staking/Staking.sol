// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.15;

import "./libs/Stakes.sol";
import "../management/Controlled.sol";
import "../token/FleekToken.sol";
import "../registry/NodeRegistry.sol";
import "../utils/TokenUtils.sol";
import "../utils/MathUtils.sol";

/**
 * @title Staking contract
 * @dev The Staking contract allows nodes to take to be eligable to be part of the network
 */
contract Staking is Controlled {
    using Stakes for Stakes.Node;

    /* STORAGE */

    FleekToken internal _fleekToken;
    NodeRegistry internal _nodeRegistry;

    /// Minimum amount of tokens an node needs to stake
    uint256 public minimumNodeStake;

    /// Time in blocks to before a staked node can be whitelisted
    uint256 public nodeElegibiliyPeriod; // in blocks

    /// Time in blocks to before node can withdrawl tokens
    uint32 public lockTime; // in blocks

    /// node stakes : node public key => Stake
    mapping(address => Stakes.Node) public stakes;

    ///TODO: this mapping may need to be flipped with stakes mapping so we can lookup stakes directly from node address
    /// Node address to owner address
    mapping(uint256 => address) public nodeAddressToOwner;

    /// List of addresses allowed to slash stakes
    mapping(address => bool) public slashers;

    // Destination of accrued rewards : beneficiary => rewards destination
    // if unset, rewards will be restaked
    mapping(address => address) public rewardsDestination;

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
     * @dev Emitted when `node` unstaked and locked `tokens` amount `until` block.
     */
    event StakeLocked(address indexed node, uint256 tokens, uint256 until);

    /**
     * @dev Emitted when 'node' withdraws staked fleek 'amount'.
     */
    event StakeWithdrawn(address indexed node, uint256 amount);

    /**
     * @dev Emitted when 'node' was slashed for a total of fleek token 'amount'
     * also emits the 'reward' of fleek tokens give to the 'beneficiary' that made the claim
     */
    event StakeSlashed(address indexed node, uint256 amount, uint256 reward, address beneficiary);

    /**
     * @dev Emitted when `caller` set `slasher` address as `allowed` to slash stakes.
     */
    event SlasherUpdate(address indexed caller, address indexed slasher, bool allowed);

    /**
     * @dev Emitted when `node` was whitelisted.
     */
    event NodeWhitelisted(address indexed node);

    /**
     * @dev Emitted when `node` was removed from the whitlist.
     */
    event NodeWhitelistRemoval(address indexed node);

    /**
     * @dev Emitted when a contract parameter has been updated
     */
    event ParameterUpdated(string param);

    /**
     * @dev Emitted when `node` set an address to receive rewards.
     */
    event SetRewardsDestination(address indexed node, address indexed destination);

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
        uint32 _elegibilityTime,
        uint32 _lockTime,
        uint32 _protocolPercentage
    ) external {
        Controlled._init(_controller);

        _fleekToken = FleekToken(token);

        // Settings
        _setMinimumNodeStake(_minimumNodeStake);
        _setLockTime(_lockTime);
        _setNodeElegibilityPeriod(_elegibilityTime);
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
     * @dev  Set the time in blocks that a node must wait before being eligible to be whitelisted.
     * @param _elegibilityTime Time in blocks to wait before a node can be whitelisted
     */
    function setNodeElegibilityPeriod(uint32 _elegibilityTime) external onlyController {
        _setNodeElegibilityPeriod(_elegibilityTime);
    }

    /**
     * @dev Set the time in blocks that a node must wait before being eligible to be whitelisted.
     * @param _elegibilityTime Time in blocks to wait before a node can be whitelisted
     */
    function _setNodeElegibilityPeriod(uint32 _elegibilityTime) private {
        require(_elegibilityTime > 0, "elegibilityTime must be > 0");
        nodeElegibiliyPeriod = _elegibilityTime;
        emit ParameterUpdated("nodeElegibiliyPeriod");
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
    * @dev Set the node registry address
    * @param _nodeRegistryAddress The address of the NodeRegistry contract
    */
     function setNodeRegistryContract(address _nodeRegistryAddress) external onlyController {
        _setNodeRegistryContract(_nodeRegistryAddress);
     }

    /**
    * @dev Set the node registry address
    * @param _nodeRegistryAddress The address of the NodeRegistry contract
    */
     function _setNodeRegistryContract(address _nodeRegistryAddress) private {
        _nodeRegistry = _nodeRegistryAddress;
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
     * @dev Get the block number when the node is eligible to be whitelisted.
     * @param _node Address of the node
     * @return Block number when the node is eligible to be whitelisted. 0 if the node is not currently ever going to be eligible.
     */
    function getEligibleBlock(address _node) external view returns (uint256) {
        return stakes[_node].eligableAt;
    }

    /**
     * @dev Get the amount a node has locked for withdrawal.
     * @param _node Address of the node
     * @return Amount of tokens locked for withdrawal
     */
    function getLockedTokens(address _node) external view returns (uint256) {
        return stakes[_node].tokensLocked;
    }

    /**
     * @dev Get the block number when the node is eligible to be whitelisted.
     * @param _node Address of the node
     * @return Block number when the node is eligible to be whitelisted. 0 if the node is not currently ever going to be eligible.
     */
    function getLockedUntil(address _node) external view returns (uint256) {
        return stakes[_node].tokensLockedUntil;
    }

    /**
     * @dev Deposit tokens on the node stake.
     * @param _tokens Amount of tokens to stake
     * @param _nodeAddress Public Address of your node
     */
    function stake(uint256 _tokens, uint256 _nodeAddress) external {
        require(_tokens > 0, "_tokens cannot be 0");
        // Ensure minimum stake
        require(stakes[msg.sender].tokensStaked + _tokens >= minimumNodeStake, "Your stake does not meet the minimum");

        if (nodeAddressToOwner[_nodeAddress] == address(0)) {
            nodeAddressToOwner[_nodeAddress] = msg.sender;
            stakes[msg.sender].nodeAddress = _nodeAddress;
        }
        require(nodeAddressToOwner[_nodeAddress] == msg.sender, "You are not the owner of this node address");

        // Transfer tokens to stake from caller to this contract
        TokenUtils.pullTokens(_fleekToken, msg.sender, _tokens);

        // Stake the transferred tokens
        _stake(msg.sender, _tokens);
    }

    /**
     * @dev Deposit tokens on the node stake.
     * @param _node Public address of the node
     * @param _tokens Amount of tokens to stake
     */
    function stakeTo(uint256 _node, uint256 _tokens) public {
        require(_tokens > 0, "_tokens cannot be 0");

        address nodeOwner = nodeAddressToOwner[_node];
        require(nodeOwner != address(0), "Node does not exist");

        // Ensure minimum stake
        require(stakes[nodeOwner].tokensStaked + _tokens >= minimumNodeStake, "Your stake does not meet the minimum");

        // Transfer tokens to stake from caller to this contract
        TokenUtils.pullTokens(_fleekToken, msg.sender, _tokens);

        // Stake the transferred tokens
        _stake(nodeOwner, _tokens);
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

        // Set elegibility if not already set
        if (stakes[_node].eligableAt == 0) {
            stakes[_node].setElegibleBlock(nodeElegibiliyPeriod);
        }

        emit StakeDeposited(_node, _tokens);
    }

    /**
     * @dev Unstake tokens from the indexer stake, lock them until thawing period expires.
     * NOTE: The function accepts an amount greater than the currently staked tokens.
     * If that happens, it will try to unstake the max amount of tokens it can.
     * The reason for this behaviour is to avoid time conditions while the transaction
     * is in flight.
     * @param _tokens Amount of tokens to unstake
     */
    function unstake(uint256 _tokens) external {
        address node = msg.sender;
        Stakes.Node storage nodeStake = stakes[node];

        require(nodeStake.tokensStaked > 0, "node has nothing staked");

        // Tokens to lock is capped to the available tokens
        uint256 tokensToLock = MathUtils.min(nodeStake.tokensStaked, _tokens);
        require(tokensToLock > 0, "Nothing to unstake");

        // Ensure minimum stake
        uint256 newStake = nodeStake.tokensStaked - tokensToLock;

        if (newStake < minimumNodeStake) {
            _removeFromWhitelist(node);
        }

        // Before locking more tokens, withdraw any unlocked ones if possible
        uint256 tokensToWithdraw = nodeStake.tokensWithdrawable();
        if (tokensToWithdraw > 0) {
            _withdraw(node);
        }

        // Update the indexer stake locking tokens
        nodeStake.lockTokens(tokensToLock, lockTime);

        emit StakeLocked(node, nodeStake.tokensLocked, nodeStake.tokensLockedUntil);
    }

    /**
     * @dev Withdraw node tokens once the lock time has passed.
     */
    function withdraw() external {
        _withdraw(msg.sender);
    }

    /**
     * @dev Withdraw node tokens once the lock time has passed.
     * @param _node Address of node to withdraw funds from
     */
    function _withdraw(address _node) private {
        // Get tokens available for withdraw and update balance
        uint256 tokensToWithdraw = stakes[_node].withdrawTokens();
        require(tokensToWithdraw > 0, "No tokens eligable for withdraw");

        // Return tokens to the indexer
        TokenUtils.pushTokens(_fleekToken, _node, tokensToWithdraw);

        emit StakeWithdrawn(_node, tokensToWithdraw);
    }

    function whitelistNode() external {
        require(stakes[msg.sender].eligableAt >= block.number, "Node is not elegible");
        require(_nodeRegistry != address(0), "Node Registry contract not set");

        ///TODO: Add URL to stakes struct
        _nodeRegistry.register(stakes[msg.sender].nodeAddress, msg.sender, "placeholder");

        ///TODO: Should we emit this here? Already emitted on Registry contract with more info
        //Maybe its good to emit here because this just specifically says what address whitelisted the node
        emit NodeWhitelisted(msg.sender);
    }

    /**
     * @dev Set the destination where to send rewards.
     * @param _destination Rewards destination address. If set to zero, rewards will be restaked
     */
    function setRewardsDestination(address _destination) external {
        rewardsDestination[msg.sender] = _destination;
        emit SetRewardsDestination(msg.sender, _destination);
    }

    /**
     * @dev Slash the node stake. Can only be called by the slasher role
     * @param _node Address of node to slash
     * @param _tokens Amount of tokens to slash from the node stake
     * @param _reward Amount of reward tokens to send to a beneficiary
     * @param _beneficiary Address of a beneficiary to receive a reward for the slashing
     */
    function slash(address _node, uint256 _tokens, uint256 _reward, address _beneficiary) external onlySlasher {
        Stakes.Node storage nodeStake = stakes[_node];

        // Only able to slash a non-zero number of tokens
        require(_tokens > 0, "Cant slash 0 tokens");

        // Cannot reward more than you are slashing
        require(_tokens >= _reward, "Cannot reward more than you are slashing");

        // TODO: Add tokens available to stakes library instead of tokensStaked+tokensLocked
        // Cannot slash stake of an indexer without any or enough stake
        require(nodeStake.tokensStaked + nodeStake.tokensLocked > 0, "Node has no tokens to slash");

        _tokens = MathUtils.min(_tokens, nodeStake.tokensStaked + nodeStake.tokensLocked);
        //The reward can not be more than the slashed tokens
        _reward = MathUtils.min(_reward, _tokens);
        // Validate beneficiary of slashed tokens
        // Should it be able to to be zero?
        require(_beneficiary != address(0), "beneficiary cannot be zero address");

        // Slashing more tokens than freely available
        // Unlock locked tokens to avoid the node to withdraw them
        if (_tokens > nodeStake.tokensStaked && nodeStake.tokensLocked > 0) {
            uint256 tokensToUnlock = _tokens - nodeStake.tokensStaked;
            nodeStake.unlockTokens(tokensToUnlock);
            nodeStake.release(_tokens - tokensToUnlock);
        } else {
            // Remove tokens to slash from the stake
            nodeStake.release(_tokens);
        }

        // Make sure the node has enough stake to remain in the whitelist
        if (nodeStake.tokensStaked < minimumNodeStake) {
            _removeFromWhitelist(_node);
        }

        // -- Interactions --

        // Set apart the reward for the beneficiary and burn remaining slashed stake
        TokenUtils.burnTokens(_fleekToken, _tokens - _reward);

        // Give the beneficiary a reward for slashing
        TokenUtils.pushTokens(_fleekToken, _beneficiary, _reward);

        emit StakeSlashed(_node, _tokens, _reward, _beneficiary);
    }

    /**
     * @dev Remove a node from the whitelist
     * @param _node Address of node to remove from the whitelist
     */
    function _removeFromWhitelist(address _node) private {
        require(_nodeRegistry != address(0), "Node Registry contract not set");

        stakes[_node].removeElegibility();

        _nodeRegistry.removeNode(stakes[_node].nodeAddress);

        emit NodeWhitelistRemoval(_node);
    }
}
