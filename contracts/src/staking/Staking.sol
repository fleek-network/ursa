// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import "../management/Controlled.sol";
import "../token/FleekToken.sol";
import "../registry/NodeRegistry.sol";
import "../epoch/EpochManager.sol";
import "./libs/Stakes.sol";
import "../utils/TokenUtils.sol";

/**
 * @title Staking contract
 * @dev The Staking contract allows nodes to stake to be eligable to participate in the network
 */
contract Staking is Controlled {
    using Stakes for Stakes.Node;
    /* STORAGE */

    FleekToken internal _fleekToken;
    NodeRegistry internal _nodeRegistry;
    EpochManager internal _epochManager;

    /// Minimum amount of tokens an node needs to stake.
    uint256 public minimumNodeStake;

    /// Time in epochs that a node must stake to be whitelisted.
    uint256 public nodeElegibiliyPeriod; // in epochs

    /// Time in epochs before a node can withdrawl tokens
    uint256 public lockTime; // in epochs

    /// node stakes : node BLS public key => Stake
    mapping(string => Stakes.Node) public stakes;

    /// Ethereum public key to owned node BLS key
    //todo(dalton): Can people own more than one node? Maybe make this an array of public keys
    mapping(address => string) public ownerToNode;

    /// List of addresses allowed to slash stakes
    mapping(address => bool) public slashers;

    /* EVENTS */

    /**
     * @dev Emitted when a contract parameter has been updated
     */
    event ParameterUpdated(string param);

    /**
     * @dev Emitted when a 'node' stakes fleek token 'amount'.
     */
    event StakeDeposited(string node, uint256 amount);

    /**
     * @dev Emitted when `caller` set `slasher` address as `allowed` to slash stakes.
     */
    event SlasherUpdate(address indexed caller, address indexed slasher, bool allowed);

    /**
     * @dev Emitted when `node` unstaked and locked `tokens` amount `until` block.
     */
    event StakeLocked(string node, uint256 tokens, uint256 until);

    /**
     * @dev Emitted when 'node' withdraws staked fleek 'amount'.
     */
    event StakeWithdrawn(string node, uint256 amount);

    /**
     * @dev Emitted when `node` was whitelisted.
     */
    event NodeWhitelisted(string node);

    /**
     * @dev Emitted when `node` was removed from the whitlist.
     */
    event NodeWhitelistRemoval(string node);

    /* PUBLIC FUNCTIONS */

    /**
     * @dev Initialize this contract.
     */
    function initialize(
        address _controller,
        address _token,
        address _registry,
        address _epoch,
        uint256 _minimumNodeStake,
        uint32 _elegibilityTime,
        uint32 _lockTime
    ) external {
        Controlled._init(_controller);

        _fleekToken = FleekToken(_token);
        _nodeRegistry = NodeRegistry(_registry);
        _epochManager = EpochManager(_epoch);

        // Settings
        _setMinimumNodeStake(_minimumNodeStake);
        _setLockTime(_lockTime);
        _setNodeElegibilityPeriod(_elegibilityTime);
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
     * @dev  Set the time in epochs that a node must wait before being eligible to be whitelisted.
     * @param _elegibilityTime Time in epochs to wait before a node can be whitelisted
     */
    function setNodeElegibilityPeriod(uint256 _elegibilityTime) external onlyController {
        _setNodeElegibilityPeriod(_elegibilityTime);
    }

    /**
     * @dev Set the time in epochs that a node must wait before being eligible to be whitelisted.
     * @param _elegibilityTime Time in epochs to wait before a node can be whitelisted
     */
    function _setNodeElegibilityPeriod(uint256 _elegibilityTime) private {
        require(_elegibilityTime > 0, "elegibilityTime must be > 0");
        nodeElegibiliyPeriod = _elegibilityTime;
        emit ParameterUpdated("nodeElegibiliyPeriod");
    }

    /**
     * @dev Set the lock period for unstaking.
     * @param _lockTime in epochs to wait for token withdrawals after unstaking
     */
    function setLockTime(uint256 _lockTime) external onlyController {
        _setLockTime(_lockTime);
    }

    /**
     * @dev Internal: Set the lock time for unstaking.
     * @param _lockTime Period in epochs to wait for token withdrawals after unstaking
     */
    function _setLockTime(uint256 _lockTime) private {
        require(_lockTime > 0, "lockTime cannot be 0");
        lockTime = _lockTime;
        emit ParameterUpdated("lockTime");
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
    function hasStake(string calldata _node) external view returns (bool) {
        return stakes[_node].tokensStaked > 0;
    }

    /**
     * @dev Get the total amount of tokens staked by the node.
     * @param _node Address of the node
     * @return Amount of tokens staked by the node
     */
    function getNodeStakedTokens(string calldata _node) external view returns (uint256) {
        return stakes[_node].tokensStaked;
    }

    /**
     * @dev Get the epoch number when the node is eligible to be whitelisted.
     * @param _node Address of the node
     * @return epoch number when the node is eligible to be whitelisted. 0 if the node is not currently ever going to be eligible.
     */
    function getEligibleEpoch(string calldata _node) external view returns (uint256) {
        //TODO(dalton): Epoch
        return stakes[_node].eligableAt;
    }

    /**
     * @dev Get the amount a node has locked for withdrawal.
     * @param _node Address of the node
     * @return Amount of tokens locked for withdrawal
     */
    function getLockedTokens(string calldata _node) external view returns (uint256) {
        return stakes[_node].tokensLocked;
    }

    /**
     * @dev Get the epoch number when the node is eligible to be whitelisted.
     * @param _node Address of the node
     * @return Epoch number when the node is eligible to be whitelisted. 0 if the node is not currently ever going to be eligible.
     */
    function getLockedUntil(string calldata _node) external view returns (uint256) {
        //TODO(dalton): Epoch
        return stakes[_node].tokensLockedUntil;
    }

    /**
     * @dev Deposit tokens on the node stake.
     * @param _tokens Amount of tokens to stake
     * @param _nodePublicKey BLS Public Key of your node
     */
    function stake(uint256 _tokens, string calldata _nodePublicKey) external {
        require(_tokens > 0, "_tokens cannot be 0");
        // Ensure minimum stake
        require(
            stakes[_nodePublicKey].tokensStaked + _tokens >= minimumNodeStake, "Your stake does not meet the minimum"
        );

        // Todo(dalton): Proof of possesion for BLS publicKey
        if (stakes[_nodePublicKey].owner == address(0)) {
            stakes[_nodePublicKey].owner == msg.sender;
        }

        // Transfer tokens to stake from caller to this contract
        TokenUtils.pullTokens(_fleekToken, msg.sender, _tokens);

        // Stake the transferred tokens
        _stake(_nodePublicKey, _tokens);
    }

    /**
     * @dev Deposit tokens on the node stake.
     * @param _node BLS Public key of the node
     * @param _tokens Amount of tokens to stake
     */
    function stakeTo(string calldata _node, uint256 _tokens) public {
        require(_tokens > 0, "_tokens cannot be 0");

        // Ensure minimum stake
        require(stakes[_node].tokensStaked + _tokens >= minimumNodeStake, "Your stake does not meet the minimum");

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
     * @param _node BLS public key of the node being staked too
     * @param _tokens Amount of tokens to stake
     */
    function _stake(string calldata _node, uint256 _tokens) private {
        // Deposit tokens into the indexer stake
        stakes[_node].deposit(_tokens);

        // Set elegibility if not already set
        if (stakes[_node].eligableAt == 0) {
            uint256 currentEpoch = _epochManager.epoch();
            stakes[_node].setElegibleEpoch(nodeElegibiliyPeriod, currentEpoch);
        }

        emit StakeDeposited(_node, _tokens);
    }

    /**
     * @dev Unstake tokens from the nodes stake, lock them until thawing period expires.
     * NOTE: The function accepts an amount greater than the currently staked tokens.
     * If that happens, it will try to unstake the max amount of tokens it can.
     * The reason for this behaviour is to avoid time conditions while the transaction
     * is in flight.
     * @param _tokens Amount of tokens to unstake
     * @param _node BLS public key of the node
     */
    function unstake(uint256 _tokens, string calldata _node) external {
        Stakes.Node storage nodeStake = stakes[_node];

        // TODO(dalton): BLS verification from the nodes signature instead of owner check
        require(msg.sender == nodeStake.owner, "you are not the owner of this node");
        require(nodeStake.tokensStaked > 0, "node has nothing staked");

        // Tokens to lock is capped to the available tokens
        uint256 tokensToLock = MathUtils.min(nodeStake.tokensStaked, _tokens);
        require(tokensToLock > 0, "Nothing to unstake");

        // Ensure minimum stake
        uint256 newStake = nodeStake.tokensStaked - tokensToLock;

        if (newStake < minimumNodeStake) {
            _removeFromWhitelist(_node);
        }

        // Get the current epoch from the registry contract
        uint256 currentEpoch = _epochManager.epoch();

        // Before locking more tokens, withdraw any unlocked ones if possible
        uint256 tokensToWithdraw = nodeStake.tokensWithdrawable(currentEpoch);
        if (tokensToWithdraw > 0) {
            _withdraw(_node);
        }

        // Update the node stake locking tokens
        nodeStake.lockTokens(tokensToLock, lockTime, currentEpoch);

        emit StakeLocked(_node, nodeStake.tokensLocked, nodeStake.tokensLockedUntil);
    }

    /**
     * @dev Withdraw node tokens once the lock time has passed.
     * @param _node BLS public key of the node
     */
    function withdraw(string calldata _node) external {
        _withdraw(_node);
    }

    /**
     * @dev Withdraw node tokens once the lock time has passed.
     * @param _node Address of node to withdraw funds from
     */
    function _withdraw(string calldata _node) private {
        // Get current epoch from epoch contract.
        uint256 currentEpoch = _epochManager.epoch();
        // Get tokens available for withdraw and update balance
        uint256 tokensToWithdraw = stakes[_node].withdrawTokens(currentEpoch);
        require(tokensToWithdraw > 0, "No tokens eligable for withdraw");

        // Return tokens to the indexer
        TokenUtils.pushTokens(_fleekToken, stakes[_node].owner, tokensToWithdraw);

        emit StakeWithdrawn(_node, tokensToWithdraw);
    }

    function whitelistNode(string calldata _node) external {
        uint256 currentEpoch = _epochManager.epoch();

        require(stakes[_node].eligableAt <= currentEpoch, "Node is not elegible");

        ///TODO: register the node
        // _nodeRegistry.registerNode(("placeholder"));

        ///TODO: Should we emit this here? Already emitted on Registry contract with more info
        //Maybe its good to emit here because this just specifically says what address whitelisted the node
        emit NodeWhitelisted(_node);
    }

    /**
     * @dev Remove a node from the whitelist
     * @param _node BLS Address of node to remove from the whitelist
     */
    function _removeFromWhitelist(string calldata _node) private {
        require(address(_nodeRegistry) != address(0), "Node Registry contract not set");

        stakes[_node].removeElegibility();

        // TODO(dalton): node registry interaction
        // _nodeRegistry.removeNode(stakes[_node].nodeAddress);

        emit NodeWhitelistRemoval(_node);
    }
}
