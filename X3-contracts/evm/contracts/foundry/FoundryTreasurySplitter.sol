// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

/// @title FoundryTreasurySplitter — Treasury Splitter for X3 Foundry
/// @notice Splits incoming funds according to governance-set percentages using basis points
/// @dev Uses pull-over-push pattern for fund distribution
contract FoundryTreasurySplitter is Ownable, ReentrancyGuard {
    using SafeERC20 for IERC20;

    // ── Types ────────────────────────────────────────────────────────────────

    /// @notice A single split destination
    struct SplitDestination {
        address payable destination;
        uint256 bps;        // Basis points (e.g., 4000 = 40%)
    }

    // ── Constants ────────────────────────────────────────────────────────────

    /// @notice Maximum basis points (100%)
    uint256 public constant MAX_BPS = 10000;

    /// @notice Maximum number of split destinations
    uint256 public constant MAX_DESTINATIONS = 20;

    // ── State ────────────────────────────────────────────────────────────────

    /// @notice Array of split destinations
    SplitDestination[] private _destinations;

    /// @notice Total basis points allocated (must equal MAX_BPS)
    uint256 private _totalBps;

    /// @notice Available native currency balance per destination
    mapping(address => uint256) public availableNative;

    /// @notice Available ERC20 token balance per destination per token
    mapping(address => mapping(address => uint256)) public availableTokens;

    /// @notice Total native currency split all-time
    uint256 public totalNativeSplit;

    /// @notice Total ERC20 tokens split all-time per token
    mapping(address => uint256) public totalTokenSplit;

    // ── Events ───────────────────────────────────────────────────────────────

    /// @notice Emitted when native currency is split
    event NativeSplit(uint256 amount, uint256 timestamp);

    /// @notice Emitted when ERC20 tokens are split
    event TokenSplit(address indexed token, uint256 amount, uint256 timestamp);

    /// @notice Emitted when the split configuration is updated
    event SplitUpdated(uint256 timestamp);

    /// @notice Emitted when a destination claims native funds
    event NativeClaimed(address indexed destination, uint256 amount, uint256 timestamp);

    /// @notice Emitted when a destination claims token funds
    event TokenClaimed(address indexed destination, address indexed token, uint256 amount, uint256 timestamp);

    // ── Errors ───────────────────────────────────────────────────────────────

    error ZeroAddress();
    error InvalidBps(uint256 bps);
    error BpsTotalMismatch(uint256 total, uint256 expected);
    error TooManyDestinations(uint256 count, uint256 max);
    error NoDestinations();
    error NoFundsToClaim();
    error TransferFailed();
    error DuplicateDestination(address dest);

    // ── Constructor ──────────────────────────────────────────────────────────

    constructor() {
        _transferOwnership(msg.sender);
    }

    // ── Admin Functions ──────────────────────────────────────────────────────

    /// @notice Set the split configuration (replaces existing)
    /// @param destinations Array of SplitDestination with addresses and BPS
    function setSplit(SplitDestination[] calldata destinations) external onlyOwner {
        if (destinations.length == 0) revert NoDestinations();
        if (destinations.length > MAX_DESTINATIONS) {
            revert TooManyDestinations(destinations.length, MAX_DESTINATIONS);
        }

        uint256 totalBps;
        for (uint256 i = 0; i < destinations.length; i++) {
            if (destinations[i].destination == address(0)) revert ZeroAddress();
            if (destinations[i].bps == 0) revert InvalidBps(0);
            if (destinations[i].bps > MAX_BPS) revert InvalidBps(destinations[i].bps);

            // Check for duplicates
            for (uint256 j = i + 1; j < destinations.length; j++) {
                if (destinations[i].destination == destinations[j].destination) {
                    revert DuplicateDestination(destinations[i].destination);
                }
            }

            totalBps += destinations[i].bps;
        }

        if (totalBps != MAX_BPS) revert BpsTotalMismatch(totalBps, MAX_BPS);

        // Clear existing and set new
        delete _destinations;
        for (uint256 i = 0; i < destinations.length; i++) {
            _destinations.push(destinations[i]);
        }
        _totalBps = totalBps;

        emit SplitUpdated(block.timestamp);
    }

    // ── Core Functions ───────────────────────────────────────────────────────

    /// @notice Split incoming native currency according to the configured splits
    function split() external payable nonReentrant {
        if (msg.value == 0) revert("ZERO_AMOUNT");
        if (_destinations.length == 0) revert NoDestinations();

        uint256 amount = msg.value;
        totalNativeSplit += amount;

        for (uint256 i = 0; i < _destinations.length; i++) {
            SplitDestination memory dest = _destinations[i];
            uint256 destAmount = (amount * dest.bps) / MAX_BPS;
            if (destAmount > 0) {
                availableNative[dest.destination] += destAmount;
            }
        }

        emit NativeSplit(amount, block.timestamp);
    }

    /// @notice Split incoming ERC20 tokens according to the configured splits
    /// @param token The ERC20 token address
    /// @param amount The amount of tokens to split
    function splitToken(IERC20 token, uint256 amount) external nonReentrant {
        if (address(token) == address(0)) revert ZeroAddress();
        if (amount == 0) revert("ZERO_AMOUNT");
        if (_destinations.length == 0) revert NoDestinations();

        token.safeTransferFrom(msg.sender, address(this), amount);
        totalTokenSplit[address(token)] += amount;

        for (uint256 i = 0; i < _destinations.length; i++) {
            SplitDestination memory dest = _destinations[i];
            uint256 destAmount = (amount * dest.bps) / MAX_BPS;
            if (destAmount > 0) {
                availableTokens[dest.destination][address(token)] += destAmount;
            }
        }

        emit TokenSplit(address(token), amount, block.timestamp);
    }

    // ── Claim Functions (Pull-over-Push) ─────────────────────────────────────

    /// @notice Claim available native currency for the caller
    /// @param amount The amount to claim
    function claimNative(uint256 amount) external nonReentrant {
        if (availableNative[msg.sender] < amount) revert NoFundsToClaim();
        if (amount == 0) revert("ZERO_AMOUNT");

        availableNative[msg.sender] -= amount;

        (bool success,) = payable(msg.sender).call{value: amount}("");
        if (!success) revert TransferFailed();

        emit NativeClaimed(msg.sender, amount, block.timestamp);
    }

    /// @notice Claim all available native currency for the caller
    function claimAllNative() external nonReentrant {
        uint256 amount = availableNative[msg.sender];
        if (amount == 0) revert NoFundsToClaim();

        availableNative[msg.sender] = 0;

        (bool success,) = payable(msg.sender).call{value: amount}("");
        if (!success) revert TransferFailed();

        emit NativeClaimed(msg.sender, amount, block.timestamp);
    }

    /// @notice Claim available ERC20 tokens for the caller
    /// @param token The ERC20 token address
    /// @param amount The amount to claim
    function claimToken(address token, uint256 amount) external nonReentrant {
        if (token == address(0)) revert ZeroAddress();
        if (availableTokens[msg.sender][token] < amount) revert NoFundsToClaim();
        if (amount == 0) revert("ZERO_AMOUNT");

        availableTokens[msg.sender][token] -= amount;

        IERC20(token).safeTransfer(msg.sender, amount);

        emit TokenClaimed(msg.sender, token, amount, block.timestamp);
    }

    /// @notice Claim all available ERC20 tokens for the caller
    /// @param token The ERC20 token address
    function claimAllToken(address token) external nonReentrant {
        if (token == address(0)) revert ZeroAddress();
        uint256 amount = availableTokens[msg.sender][token];
        if (amount == 0) revert NoFundsToClaim();

        availableTokens[msg.sender][token] = 0;

        IERC20(token).safeTransfer(msg.sender, amount);

        emit TokenClaimed(msg.sender, token, amount, block.timestamp);
    }

    // ── View Functions ───────────────────────────────────────────────────────

    /// @notice Get the current split configuration
    /// @return destinations Array of SplitDestination
    function getSplit() external view returns (SplitDestination[] memory destinations) {
        destinations = _destinations;
    }

    /// @notice Get the total basis points allocated
    /// @return totalBps The total BPS
    function getTotalBps() external view returns (uint256) {
        return _totalBps;
    }

    /// @notice Get the number of split destinations
    /// @return count The destination count
    function getDestinationCount() external view returns (uint256) {
        return _destinations.length;
    }

    /// @notice Get available native funds for a destination
    /// @param destination The destination address
    /// @return amount Available native currency
    function getAvailableFunds(address destination) external view returns (uint256) {
        return availableNative[destination];
    }

    /// @notice Get available token funds for a destination
    /// @param destination The destination address
    /// @param token The ERC20 token address
    /// @return amount Available tokens
    function getAvailableTokenFunds(address destination, address token) external view returns (uint256) {
        return availableTokens[destination][token];
    }

    /// @notice Get a destination at a specific index
    /// @param index The index
    /// @return SplitDestination struct
    function getDestinationAt(uint256 index) external view returns (SplitDestination memory) {
        return _destinations[index];
    }
}
