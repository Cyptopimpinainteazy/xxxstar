// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

/**
 * @title AtomicSwapAdapter
 * @author Botchain Team
 * @notice HTLC-based atomic swap contract for Bitcoin ↔ ERC20 token swaps
 * @dev Implements Hash Time Locked Contracts (HTLC) pattern with SHA-256
 *      for Bitcoin interoperability
 *
 * CRITICAL: Uses sha256() NOT keccak256() to match Bitcoin's OP_SHA256
 *
 * Atomic Swap Flow (Alice has BTC, Bob has BOT):
 * 1. Alice generates secret P, computes H = SHA256(P)
 * 2. Alice creates Bitcoin HTLC locked to H with timelock T1 (e.g., 24 hours)
 * 3. Bob sees Alice's BTC HTLC, creates EVM HTLC with same H, shorter timelock T2 (e.g., 12 hours)
 * 4. Alice claims BOT by revealing P on-chain (P now public)
 * 5. Bob (or relay) extracts P from EVM event, claims BTC using P
 *
 * Timelock Safety:
 * - EVM timelock (T2) MUST be shorter than BTC timelock (T1)
 * - This ensures: if Alice doesn't claim BOT before T2, Bob refunds
 * - And Alice still has time to refund BTC before T1
 *
 * Security:
 * - SHA-256 hashlock ensures same preimage works on both chains
 * - Timelocks ensure either both complete or both refund
 * - Preimage emitted in event for watchtower/relay extraction
 */
contract AtomicSwapAdapter is ReentrancyGuard, Ownable {
    using SafeERC20 for IERC20;

    // ============ Structs ============

    struct Swap {
        address initiator; // Address that locked tokens (can refund after timelock)
        address participant; // Address that can claim with preimage
        IERC20 token; // ERC20 token being swapped
        uint256 amount; // Amount locked
        bytes32 hashlock; // SHA256(preimage) - MUST match Bitcoin HTLC
        uint256 timelock; // Unix timestamp after which initiator can refund
        bool claimed; // Has been claimed with valid preimage
        bool refunded; // Has been refunded after timelock
    }

    // ============ State Variables ============

    /// @notice Mapping from swap ID to Swap data
    mapping(bytes32 => Swap) public swaps;

    /// @notice Mapping to track used swap IDs (prevent replay)
    mapping(bytes32 => bool) public usedSwapIds;

    /// @notice Minimum timelock duration (1 hour) - must be shorter than BTC side
    uint256 public constant MIN_TIMELOCK = 1 hours;

    /// @notice Maximum timelock duration (7 days)
    uint256 public constant MAX_TIMELOCK = 7 days;

    /// @notice Nonce for swap ID generation
    uint256 private _swapNonce;

    // ============ Events ============

    /// @notice Emitted when tokens are locked - watchtowers monitor this
    event TokensLocked(
        bytes32 indexed swapId,
        address indexed initiator,
        address indexed participant,
        address token,
        uint256 amount,
        bytes32 hashlock,
        uint256 timelock
    );

    /// @notice Emitted when tokens are claimed - PREIMAGE IS PUBLIC after this
    /// @dev Watchtowers/relays MUST monitor this to extract preimage for BTC claim
    event TokensClaimed(
        bytes32 indexed swapId,
        bytes preimage, // Raw preimage bytes for relay extraction
        address claimedBy // Who submitted the claim tx
    );

    /// @notice Emitted when tokens are refunded
    event TokensRefunded(bytes32 indexed swapId, address refundedTo);

    // ============ Errors ============

    error SwapAlreadyExists();
    error SwapNotFound();
    error SwapAlreadyClaimed();
    error SwapAlreadyRefunded();
    error SwapExpired();
    error SwapNotExpired();
    error InvalidPreimage();
    error InvalidParticipant();
    error InvalidToken();
    error InvalidAmount();
    error InvalidHashlock();
    error TimelockTooShort();
    error TimelockTooLong();
    error NotInitiator();

    // ============ Constructor ============

    constructor() Ownable(msg.sender) {}

    // ============ Core Functions ============

    /**
     * @notice Lock tokens for atomic swap with SHA-256 hashlock
     * @param participant Address that can claim with correct preimage
     * @param token ERC20 token to lock
     * @param amount Amount to lock
     * @param hashlock SHA256(preimage) - compute off-chain with sha256()
     * @param timelock Unix timestamp after which initiator can refund
     * @return swapId Unique identifier for this swap
     *
     * @dev IMPORTANT: hashlock must be SHA256(preimage) to match Bitcoin OP_SHA256
     * @dev Timelock should be SHORTER than corresponding BTC HTLC timelock
     */
    function lockTokens(
        address participant,
        IERC20 token,
        uint256 amount,
        bytes32 hashlock,
        uint256 timelock
    ) external nonReentrant returns (bytes32 swapId) {
        // Input validation
        if (participant == address(0)) revert InvalidParticipant();
        if (address(token) == address(0)) revert InvalidToken();
        if (amount == 0) revert InvalidAmount();
        if (hashlock == bytes32(0)) revert InvalidHashlock();

        // Timelock validation
        if (timelock < block.timestamp + MIN_TIMELOCK)
            revert TimelockTooShort();
        if (timelock > block.timestamp + MAX_TIMELOCK) revert TimelockTooLong();

        // Generate deterministic swap ID (prevents replay attacks)
        unchecked {
            _swapNonce++;
        }
        swapId = keccak256(
            abi.encodePacked(
                msg.sender,
                participant,
                address(token),
                amount,
                hashlock,
                timelock,
                _swapNonce,
                block.chainid
            )
        );

        // Ensure swap ID hasn't been used
        if (usedSwapIds[swapId]) revert SwapAlreadyExists();
        usedSwapIds[swapId] = true;

        // Transfer tokens to contract (checks-effects-interactions: state before transfer)
        swaps[swapId] = Swap({
            initiator: msg.sender,
            participant: participant,
            token: token,
            amount: amount,
            hashlock: hashlock,
            timelock: timelock,
            claimed: false,
            refunded: false
        });

        // External call last
        token.safeTransferFrom(msg.sender, address(this), amount);

        emit TokensLocked(
            swapId,
            msg.sender,
            participant,
            address(token),
            amount,
            hashlock,
            timelock
        );
    }

    /**
     * @notice Claim tokens by revealing the preimage (SHA-256)
     * @param swapId ID of the swap
     * @param preimage Raw bytes that hash to the hashlock via SHA-256
     *
     * @dev CRITICAL: Emits preimage in event - this is PUBLIC after claim
     * @dev Watchtowers should monitor TokensClaimed events to relay preimage to BTC side
     * @dev Anyone can submit claim tx, but tokens always go to participant
     */
    function claim(
        bytes32 swapId,
        bytes calldata preimage
    ) external nonReentrant {
        Swap storage swap = swaps[swapId];

        // Validation
        if (swap.initiator == address(0)) revert SwapNotFound();
        if (swap.claimed) revert SwapAlreadyClaimed();
        if (swap.refunded) revert SwapAlreadyRefunded();
        if (block.timestamp >= swap.timelock) revert SwapExpired();

        // CRITICAL: Use SHA-256 to match Bitcoin's OP_SHA256
        bytes32 computedHash = sha256(preimage);
        if (computedHash != swap.hashlock) revert InvalidPreimage();

        // Update state BEFORE external call (checks-effects-interactions)
        swap.claimed = true;

        // Transfer tokens to participant
        swap.token.safeTransfer(swap.participant, swap.amount);

        // Emit preimage for watchtower/relay extraction
        emit TokensClaimed(swapId, preimage, msg.sender);
    }

    /**
     * @notice Refund tokens after timelock expires
     * @param swapId ID of the swap
     *
     * @dev Only initiator can refund
     * @dev Only callable after timelock has passed
     */
    function refund(bytes32 swapId) external nonReentrant {
        Swap storage swap = swaps[swapId];

        // Validation
        if (swap.initiator == address(0)) revert SwapNotFound();
        if (swap.claimed) revert SwapAlreadyClaimed();
        if (swap.refunded) revert SwapAlreadyRefunded();
        if (block.timestamp < swap.timelock) revert SwapNotExpired();
        if (msg.sender != swap.initiator) revert NotInitiator();

        // Update state BEFORE external call
        swap.refunded = true;

        // Return tokens to initiator
        swap.token.safeTransfer(swap.initiator, swap.amount);

        emit TokensRefunded(swapId, swap.initiator);
    }

    // ============ View Functions ============

    /**
     * @notice Get full swap details
     */
    function getSwap(bytes32 swapId) external view returns (Swap memory) {
        return swaps[swapId];
    }

    /**
     * @notice Check if swap is active (neither claimed nor refunded)
     */
    function isSwapActive(bytes32 swapId) external view returns (bool) {
        Swap storage swap = swaps[swapId];
        return swap.initiator != address(0) && !swap.claimed && !swap.refunded;
    }

    /**
     * @notice Check if swap can currently be claimed
     */
    function canClaim(bytes32 swapId) external view returns (bool) {
        Swap storage swap = swaps[swapId];
        return
            swap.initiator != address(0) &&
            !swap.claimed &&
            !swap.refunded &&
            block.timestamp < swap.timelock;
    }

    /**
     * @notice Check if swap can currently be refunded
     */
    function canRefund(bytes32 swapId) external view returns (bool) {
        Swap storage swap = swaps[swapId];
        return
            swap.initiator != address(0) &&
            !swap.claimed &&
            !swap.refunded &&
            block.timestamp >= swap.timelock;
    }

    /**
     * @notice Get time remaining until timelock expires
     * @return seconds remaining, or 0 if expired
     */
    function timeUntilRefund(bytes32 swapId) external view returns (uint256) {
        Swap storage swap = swaps[swapId];
        if (swap.timelock <= block.timestamp) return 0;
        return swap.timelock - block.timestamp;
    }

    // ============ Helper Functions ============

    /**
     * @notice Compute SHA-256 hashlock from preimage (for off-chain use)
     * @param preimage Raw bytes to hash
     * @return hashlock SHA256(preimage)
     *
     * @dev Use this to generate hashlock that matches Bitcoin OP_SHA256
     */
    function computeHashlock(
        bytes calldata preimage
    ) external pure returns (bytes32) {
        return sha256(preimage);
    }

    /**
     * @notice Compute hashlock from bytes32 preimage
     * @param preimage 32-byte preimage
     * @return hashlock SHA256(preimage)
     */
    function computeHashlockFromBytes32(
        bytes32 preimage
    ) external pure returns (bytes32) {
        return sha256(abi.encodePacked(preimage));
    }

    /**
     * @notice Verify a preimage matches a hashlock
     * @param preimage Raw bytes preimage
     * @param hashlock Expected SHA256 hash
     * @return valid True if sha256(preimage) == hashlock
     */
    function verifyPreimage(
        bytes calldata preimage,
        bytes32 hashlock
    ) external pure returns (bool) {
        return sha256(preimage) == hashlock;
    }

    /**
     * @notice Get current swap nonce (for debugging)
     */
    function getSwapNonce() external view returns (uint256) {
        return _swapNonce;
    }
}
