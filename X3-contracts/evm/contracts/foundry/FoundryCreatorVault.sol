// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

/// @title FoundryCreatorVault — Creator Revenue Vault for X3 Foundry
/// @notice Vault for creator revenue using pull-over-push pattern
/// @dev Uses ReentrancyGuard for safety and Ownable for admin functions
contract FoundryCreatorVault is Ownable, ReentrancyGuard {
    using SafeERC20 for IERC20;

    // ── Types ────────────────────────────────────────────────────────────────

    /// @notice Represents a creator's balance snapshot
    struct CreatorBalance {
        uint256 nativeBalance;      // Balance in native currency (wei)
        uint256 totalDeposited;     // Total native deposited all-time
        uint256 totalWithdrawn;     // Total native withdrawn all-time
        uint256 lastClaimTime;      // Timestamp of last withdrawal
    }

    /// @notice Represents a creator's ERC20 token balance
    struct TokenBalance {
        uint256 balance;
        uint256 totalDeposited;
        uint256 totalWithdrawn;
    }

    // ── State ────────────────────────────────────────────────────────────────

    /// @notice Creator address => CreatorBalance (native)
    mapping(address => CreatorBalance) private _nativeBalances;

    /// @notice Creator address => token address => TokenBalance
    mapping(address => mapping(address => TokenBalance)) private _tokenBalances;

    /// @notice Total native currency distributed all-time
    uint256 public totalNativeDistributed;

    /// @notice Total native currency withdrawn all-time
    uint256 public totalNativeWithdrawn;

    /// @notice Set of creators who have balances
    address[] private _creators;

    /// @notice Whether a creator is tracked
    mapping(address => bool) private _isCreator;

    // ── Events ───────────────────────────────────────────────────────────────

    /// @notice Emitted when native currency is deposited to a creator's vault
    event NativeDeposited(address indexed creator, uint256 amount, uint256 timestamp);

    /// @notice Emitted when ERC20 tokens are deposited to a creator's vault
    event TokenDeposited(address indexed creator, address indexed token, uint256 amount, uint256 timestamp);

    /// @notice Emitted when a creator withdraws native currency
    event NativeWithdrawn(address indexed creator, uint256 amount, uint256 timestamp);

    /// @notice Emitted when a creator withdraws ERC20 tokens
    event TokenWithdrawn(address indexed creator, address indexed token, uint256 amount, uint256 timestamp);

    // ── Errors ───────────────────────────────────────────────────────────────

    error ZeroAddress();
    error ZeroAmount();
    error InsufficientBalance(uint256 requested, uint256 available);
    error TransferFailed();

    // ── Constructor ──────────────────────────────────────────────────────────

    constructor() {
        _transferOwnership(msg.sender);
    }

    // ── Deposit Functions ────────────────────────────────────────────────────

    /// @notice Deposit native currency into a creator's vault
    /// @param creator The creator address to credit
    function deposit(address creator) external payable nonReentrant {
        if (creator == address(0)) revert ZeroAddress();
        if (msg.value == 0) revert ZeroAmount();

        CreatorBalance storage bal = _nativeBalances[creator];
        bal.nativeBalance += msg.value;
        bal.totalDeposited += msg.value;
        totalNativeDistributed += msg.value;

        if (!_isCreator[creator]) {
            _isCreator[creator] = true;
            _creators.push(creator);
        }

        emit NativeDeposited(creator, msg.value, block.timestamp);
    }

    /// @notice Deposit ERC20 tokens into a creator's vault
    /// @param creator The creator address to credit
    /// @param token The ERC20 token address
    /// @param amount The amount of tokens
    function depositToken(address creator, IERC20 token, uint256 amount) external nonReentrant {
        if (creator == address(0)) revert ZeroAddress();
        if (address(token) == address(0)) revert ZeroAddress();
        if (amount == 0) revert ZeroAmount();

        token.safeTransferFrom(msg.sender, address(this), amount);

        TokenBalance storage bal = _tokenBalances[creator][address(token)];
        bal.balance += amount;
        bal.totalDeposited += amount;

        if (!_isCreator[creator]) {
            _isCreator[creator] = true;
            _creators.push(creator);
        }

        emit TokenDeposited(creator, address(token), amount, block.timestamp);
    }

    // ── Withdrawal Functions (Pull-over-Push) ────────────────────────────────

    /// @notice Claim available native currency balance (pull-over-push)
    /// @param amount The amount to withdraw
    function claimRevenue(uint256 amount) external nonReentrant {
        if (amount == 0) revert ZeroAmount();

        CreatorBalance storage bal = _nativeBalances[msg.sender];
        if (bal.nativeBalance < amount) revert InsufficientBalance(amount, bal.nativeBalance);

        bal.nativeBalance -= amount;
        bal.totalWithdrawn += amount;
        bal.lastClaimTime = block.timestamp;
        totalNativeWithdrawn += amount;

        (bool success,) = payable(msg.sender).call{value: amount}("");
        if (!success) revert TransferFailed();

        emit NativeWithdrawn(msg.sender, amount, block.timestamp);
    }

    /// @notice Claim available ERC20 token balance (pull-over-push)
    /// @param token The ERC20 token address
    /// @param amount The amount to withdraw
    function claimTokenRevenue(address token, uint256 amount) external nonReentrant {
        if (token == address(0)) revert ZeroAddress();
        if (amount == 0) revert ZeroAmount();

        TokenBalance storage bal = _tokenBalances[msg.sender][token];
        if (bal.balance < amount) revert InsufficientBalance(amount, bal.balance);

        bal.balance -= amount;
        bal.totalWithdrawn += amount;

        IERC20(token).safeTransfer(msg.sender, amount);

        emit TokenWithdrawn(msg.sender, token, amount, block.timestamp);
    }

    /// @notice Withdraw all available native currency balance
    function claimAllRevenue() external nonReentrant {
        CreatorBalance storage bal = _nativeBalances[msg.sender];
        uint256 amount = bal.nativeBalance;
        if (amount == 0) revert InsufficientBalance(0, 0);

        bal.nativeBalance = 0;
        bal.totalWithdrawn += amount;
        bal.lastClaimTime = block.timestamp;
        totalNativeWithdrawn += amount;

        (bool success,) = payable(msg.sender).call{value: amount}("");
        if (!success) revert TransferFailed();

        emit NativeWithdrawn(msg.sender, amount, block.timestamp);
    }

    /// @notice Withdraw all available ERC20 token balance for a specific token
    /// @param token The ERC20 token address
    function claimAllTokenRevenue(address token) external nonReentrant {
        if (token == address(0)) revert ZeroAddress();

        TokenBalance storage bal = _tokenBalances[msg.sender][token];
        uint256 amount = bal.balance;
        if (amount == 0) revert InsufficientBalance(0, 0);

        bal.balance = 0;
        bal.totalWithdrawn += amount;

        IERC20(token).safeTransfer(msg.sender, amount);

        emit TokenWithdrawn(msg.sender, token, amount, block.timestamp);
    }

    // ── View Functions ───────────────────────────────────────────────────────

    /// @notice Get native currency balance for a creator
    /// @param creator The creator address
    /// @return balance The available native balance
    function getBalance(address creator) external view returns (uint256) {
        return _nativeBalances[creator].nativeBalance;
    }

    /// @notice Get ERC20 token balance for a creator
    /// @param creator The creator address
    /// @param token The ERC20 token address
    /// @return balance The available token balance
    function getTokenBalance(address creator, address token) external view returns (uint256) {
        return _tokenBalances[creator][token].balance;
    }

    /// @notice Get full native balance info for a creator
    /// @param creator The creator address
    /// @return CreatorBalance struct
    function getCreatorNativeInfo(address creator) external view returns (CreatorBalance memory) {
        return _nativeBalances[creator];
    }

    /// @notice Get full token balance info for a creator
    /// @param creator The creator address
    /// @param token The ERC20 token address
    /// @return TokenBalance struct
    function getCreatorTokenInfo(address creator, address token) external view returns (TokenBalance memory) {
        return _tokenBalances[creator][token];
    }

    /// @notice Get total native currency distributed all-time
    /// @return Total distributed
    function getTotalDistributed() external view returns (uint256) {
        return totalNativeDistributed;
    }

    /// @notice Get total native currency withdrawn all-time
    /// @return Total withdrawn
    function getTotalWithdrawn() external view returns (uint256) {
        return totalNativeWithdrawn;
    }

    /// @notice Get the number of tracked creators
    /// @return count The creator count
    function getCreatorCount() external view returns (uint256) {
        return _creators.length;
    }

    /// @notice Get creator at index
    /// @param index The index
    /// @return creator The creator address
    function getCreatorAt(uint256 index) external view returns (address) {
        return _creators[index];
    }
}
