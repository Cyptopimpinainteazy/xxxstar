// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "./interfaces/IX3Verification.sol";

/// @title X3ExternalGateway — External chain lock/release gateway for X3
/// @notice Deployed once per external EVM chain (Ethereum, Base, Arbitrum, etc.)
/// @dev Users deposit ERC20 tokens here → relayers submit proofs to X3 → X3 mints wrapped
///      X3 burns wrapped → relayers submit proofs here → tokens released
contract X3ExternalGateway is Ownable {
    using SafeERC20 for IERC20;

    /// @notice Proof verifier contract (handles X3 finalized proof verification)
    IX3Verification public verifier;

    /// @notice Supported tokens that can be deposited into X3
    mapping(address => bool) public supportedTokens;

    /// @notice Per-token daily deposit limits (in token decimals)
    mapping(address => uint256) public dailyDepositLimits;

    /// @notice Per-token daily withdrawal limits
    mapping(address => uint256) public dailyWithdrawalLimits;

    /// @notice Rolling window accumulators (resets daily)
    /// token => dayTimestamp => totalDeposited
    mapping(address => mapping(uint256 => uint256)) public dailyDeposited;

    /// @notice token => dayTimestamp => totalWithdrawn
    mapping(address => mapping(uint256 => uint256)) public dailyWithdrawn;

    /// @notice Replay protection — used message IDs
    mapping(bytes32 => bool) public usedMessages;

    /// @notice Total value locked per token
    mapping(address => uint256) public totalLocked;

    /// @notice Emergency pause
    bool public paused;

    /// @notice Chain ID of this gateway (Ethereum=1, Base=8453, Arbitrum=42161, etc.)
    uint256 public immutable chainId;

    /// @notice X3 destination chain ID
    uint256 public immutable x3ChainId;

    /// @notice Minimum confirmations required for X3 finality proof
    uint256 public minX3Confirmations;

    // ── Events ──────────────────────────────────────────────────────────────

    /// @notice Emitted when a user deposits tokens to bridge to X3
    event DepositLocked(
        bytes32 indexed messageId,
        address indexed token,
        address indexed depositor,
        bytes x3Recipient,
        uint256 amount,
        uint256 nonce,
        uint256 chainId
    );

    /// @notice Emitted when tokens are released to a recipient after X3 proof
    event WithdrawalReleased(
        bytes32 indexed messageId,
        address indexed token,
        address indexed recipient,
        uint256 amount
    );

    /// @notice Emitted when gateway is paused or unpaused
    event Paused(bool isPaused);

    /// @notice Emitted when a token is added or removed from supported list
    event SupportedTokenUpdated(address indexed token, bool supported, uint256 dailyDepositCap, uint256 dailyWithdrawalCap);

    /// @notice Emitted when verifier contract is updated
    event VerifierUpdated(address indexed newVerifier);

    // ── Constructor ─────────────────────────────────────────────────────────

    constructor(
        address _verifier,
        uint256 _chainId,
        uint256 _x3ChainId,
        uint256 _minX3Confirmations
    ) {
        require(_verifier != address(0), "ZERO_VERIFIER");
        verifier = IX3Verification(_verifier);
        chainId = _chainId;
        x3ChainId = _x3ChainId;
        minX3Confirmations = _minX3Confirmations;
    }

    // ── Modifiers ───────────────────────────────────────────────────────────

    modifier whenNotPaused() {
        require(!paused, "GATEWAY_PAUSED");
        _;
    }

    modifier onlySupportedToken(address token) {
        require(supportedTokens[token], "TOKEN_NOT_SUPPORTED");
        _;
    }

    // ── Admin Functions ─────────────────────────────────────────────────────

    /// @notice Set or update a supported token with daily limits
    function setSupportedToken(
        address token,
        bool supported,
        uint256 dailyDepositCap,
        uint256 dailyWithdrawalCap
    ) external onlyOwner {
        supportedTokens[token] = supported;
        dailyDepositLimits[token] = dailyDepositCap;
        dailyWithdrawalLimits[token] = dailyWithdrawalCap;
        emit SupportedTokenUpdated(token, supported, dailyDepositCap, dailyWithdrawalCap);
    }

    /// @notice Update the verifier contract address
    function setVerifier(address _verifier) external onlyOwner {
        require(_verifier != address(0), "ZERO_VERIFIER");
        verifier = IX3Verification(_verifier);
        emit VerifierUpdated(_verifier);
    }

    /// @notice Emergency pause/unpause
    function setPaused(bool _paused) external onlyOwner {
        paused = _paused;
        emit Paused(_paused);
    }

    /// @notice Set minimum X3 confirmations for proof acceptance
    function setMinX3Confirmations(uint256 _min) external onlyOwner {
        minX3Confirmations = _min;
    }

    // ── Core Functions ──────────────────────────────────────────────────────

    /// @notice Deposit ERC20 tokens into the gateway to mint on X3
    /// @param token The ERC20 token address
    /// @param x3Recipient The recipient address on X3 (SCALE-encoded)
    /// @param amount The amount of tokens to deposit
    /// @param nonce User-provided nonce for replay protection
    function depositToX3(
        address token,
        bytes calldata x3Recipient,
        uint256 amount,
        uint256 nonce
    ) external whenNotPaused onlySupportedToken(token) {
        require(amount > 0, "ZERO_AMOUNT");
        require(x3Recipient.length > 0 && x3Recipient.length <= 64, "INVALID_RECIPIENT");

        bytes32 messageId = keccak256(
            abi.encodePacked(
                "X3_DEPOSIT_V1",
                chainId,
                token,
                msg.sender,
                x3Recipient,
                amount,
                nonce
            )
        );

        require(!usedMessages[messageId], "REPLAY");
        usedMessages[messageId] = true;

        // Enforce daily deposit limit
        uint256 dayKey = block.timestamp / 86400;
        uint256 dayDeposited = dailyDeposited[token][dayKey];
        uint256 newDayDeposited = dayDeposited + amount;
        require(newDayDeposited <= dailyDepositLimits[token], "DAILY_DEPOSIT_LIMIT");
        dailyDeposited[token][dayKey] = newDayDeposited;

        // Transfer tokens from user to this contract
        IERC20(token).safeTransferFrom(msg.sender, address(this), amount);
        totalLocked[token] += amount;

        emit DepositLocked(
            messageId,
            token,
            msg.sender,
            x3Recipient,
            amount,
            nonce,
            chainId
        );
    }

    /// @notice Release tokens to recipient after verified X3 withdrawal proof
    /// @param messageId The X3 withdrawal message ID
    /// @param token The ERC20 token address to release
    /// @param recipient The recipient address on this chain
    /// @param amount The amount to release
    /// @param sender The sender on X3 side (SCALE-encoded)
    /// @param proof The X3 finalized proof data
    function releaseFromX3(
        bytes32 messageId,
        address token,
        address recipient,
        uint256 amount,
        bytes calldata sender,
        bytes calldata proof
    ) external whenNotPaused onlySupportedToken(token) {
        require(!usedMessages[messageId], "REPLAY");
        require(recipient != address(0), "ZERO_RECIPIENT");
        require(amount > 0, "ZERO_AMOUNT");
        require(totalLocked[token] >= amount, "INSUFFICIENT_LIQUIDITY");

        // Verify X3 withdrawal proof
        bool verified = verifier.verifyX3WithdrawalProof(
            messageId,
            x3ChainId,
            sender,
            recipient,
            amount,
            proof
        );
        require(verified, "INVALID_X3_PROOF");

        usedMessages[messageId] = true;

        // Enforce daily withdrawal limit
        uint256 dayKey = block.timestamp / 86400;
        uint256 dayWithdrawn = dailyWithdrawn[token][dayKey];
        uint256 newDayWithdrawn = dayWithdrawn + amount;
        require(newDayWithdrawn <= dailyWithdrawalLimits[token], "DAILY_WITHDRAWAL_LIMIT");
        dailyWithdrawn[token][dayKey] = newDayWithdrawn;

        // Release tokens
        totalLocked[token] -= amount;
        IERC20(token).safeTransfer(recipient, amount);

        emit WithdrawalReleased(messageId, token, recipient, amount);
    }

    /// @notice Get daily remaining capacity for a token
    function getRemainingDailyDeposit(address token) external view returns (uint256) {
        uint256 dayKey = block.timestamp / 86400;
        uint256 deposited = dailyDeposited[token][dayKey];
        uint256 limit = dailyDepositLimits[token];
        if (deposited >= limit) return 0;
        return limit - deposited;
    }

    /// @notice Get daily remaining withdrawal capacity for a token
    function getRemainingDailyWithdrawal(address token) external view returns (uint256) {
        uint256 dayKey = block.timestamp / 86400;
        uint256 withdrawn = dailyWithdrawn[token][dayKey];
        uint256 limit = dailyWithdrawalLimits[token];
        if (withdrawn >= limit) return 0;
        return limit - withdrawn;
    }
}