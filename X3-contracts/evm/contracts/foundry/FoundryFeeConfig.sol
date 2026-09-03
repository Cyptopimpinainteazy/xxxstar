// SPDX-License-Identifier: MIT
pragma solidity 0.8.24;

import "@openzeppelin/contracts/access/Ownable.sol";

/// @title FoundryFeeConfig — Fee Configuration Manager for X3 Foundry
/// @notice Manages fee configuration for each dApp with minimum platform fee enforcement
/// @dev Stores RevenueConfig struct with platform_fee_bps, creator_fee_bps, etc.
contract FoundryFeeConfig is Ownable {
    // ── Types ────────────────────────────────────────────────────────────────

    /// @notice Revenue configuration for a dApp
    struct RevenueConfig {
        uint256 platformFeeBps;     // Platform fee in basis points (e.g., 200 = 2%)
        uint256 creatorFeeBps;      // Creator fee in basis points
        uint256 referralFeeBps;     // Referral fee in basis points
        uint256 royaltyFeeBps;      // Royalty fee in basis points
        uint256 subscriptionFee;    // Fixed subscription fee (in wei)
        bool isActive;              // Whether this config is active
    }

    // ── Constants ────────────────────────────────────────────────────────────

    /// @notice Maximum basis points (100%)
    uint256 public constant MAX_BPS = 10000;

    /// @notice Default platform minimum fee in basis points (2%)
    uint256 public constant DEFAULT_MIN_PLATFORM_FEE_BPS = 200;

    // ── State ────────────────────────────────────────────────────────────────

    /// @notice dApp address => RevenueConfig
    mapping(address => RevenueConfig) private _feeConfigs;

    /// @notice Minimum platform fee in basis points (enforced globally)
    uint256 private _minPlatformFeeBps;

    /// @notice Whether a dApp has a fee config set
    mapping(address => bool) private _hasConfig;

    // ── Events ───────────────────────────────────────────────────────────────

    /// @notice Emitted when a fee config is set for a dApp
    event FeeConfigSet(
        address indexed dapp,
        uint256 platformFeeBps,
        uint256 creatorFeeBps,
        uint256 referralFeeBps,
        uint256 royaltyFeeBps,
        uint256 subscriptionFee,
        uint256 timestamp
    );

    /// @notice Emitted when the minimum platform fee is updated
    event MinPlatformFeeUpdated(uint256 oldFeeBps, uint256 newFeeBps, uint256 timestamp);

    // ── Errors ───────────────────────────────────────────────────────────────

    error ZeroAddress();
    error InvalidBps(uint256 bps);
    error PlatformFeeTooLow(uint256 provided, uint256 minimum);
    error TotalFeeExceedsMax(uint256 total, uint256 maxBps);
    error ConfigNotSet(address dapp);

    // ── Constructor ──────────────────────────────────────────────────────────

    constructor() {
        _transferOwnership(msg.sender);
        _minPlatformFeeBps = DEFAULT_MIN_PLATFORM_FEE_BPS;
    }

    // ── Admin Functions ──────────────────────────────────────────────────────

    /// @notice Set or update the fee configuration for a dApp
    /// @param dapp The dApp contract address
    /// @param config The RevenueConfig to set
    function setFeeConfig(address dapp, RevenueConfig calldata config) external onlyOwner {
        if (dapp == address(0)) revert ZeroAddress();

        _validateFeeConfig(config);

        _feeConfigs[dapp] = config;
        _hasConfig[dapp] = true;

        emit FeeConfigSet(
            dapp,
            config.platformFeeBps,
            config.creatorFeeBps,
            config.referralFeeBps,
            config.royaltyFeeBps,
            config.subscriptionFee,
            block.timestamp
        );
    }

    /// @notice Set the global minimum platform fee (in basis points)
    /// @param newFeeBps The new minimum platform fee in basis points
    function setPlatformMinFee(uint256 newFeeBps) external onlyOwner {
        if (newFeeBps > MAX_BPS) revert InvalidBps(newFeeBps);

        uint256 oldFeeBps = _minPlatformFeeBps;
        _minPlatformFeeBps = newFeeBps;

        emit MinPlatformFeeUpdated(oldFeeBps, newFeeBps, block.timestamp);
    }

    // ── Public Functions ─────────────────────────────────────────────────────

    /// @notice Validate a RevenueConfig against business rules
    /// @param config The RevenueConfig to validate
    /// @return True if valid, reverts otherwise
    function validateFeeConfig(RevenueConfig calldata config) external view returns (bool) {
        _validateFeeConfig(config);
        return true;
    }

    // ── Internal ─────────────────────────────────────────────────────────────

    /// @notice Internal validation logic for RevenueConfig
    function _validateFeeConfig(RevenueConfig memory config) internal view {
        if (config.platformFeeBps > MAX_BPS) revert InvalidBps(config.platformFeeBps);
        if (config.creatorFeeBps > MAX_BPS) revert InvalidBps(config.creatorFeeBps);
        if (config.referralFeeBps > MAX_BPS) revert InvalidBps(config.referralFeeBps);
        if (config.royaltyFeeBps > MAX_BPS) revert InvalidBps(config.royaltyFeeBps);

        // Enforce minimum platform fee
        if (config.platformFeeBps < _minPlatformFeeBps) {
            revert PlatformFeeTooLow(config.platformFeeBps, _minPlatformFeeBps);
        }

        // Ensure total fees don't exceed 100%
        uint256 total = config.platformFeeBps + config.creatorFeeBps
            + config.referralFeeBps + config.royaltyFeeBps;
        if (total > MAX_BPS) revert TotalFeeExceedsMax(total, MAX_BPS);
    }

    // ── View Functions ───────────────────────────────────────────────────────

    /// @notice Get the fee configuration for a dApp
    /// @param dapp The dApp contract address
    /// @return RevenueConfig struct
    function getFeeConfig(address dapp) external view returns (RevenueConfig memory) {
        if (!_hasConfig[dapp]) revert ConfigNotSet(dapp);
        return _feeConfigs[dapp];
    }

    /// @notice Get the global minimum platform fee in basis points
    /// @return The minimum platform fee in bps
    function getPlatformMinFee() external view returns (uint256) {
        return _minPlatformFeeBps;
    }

    /// @notice Check if a dApp has a fee config set
    /// @param dapp The dApp contract address
    /// @return True if config exists
    function hasFeeConfig(address dapp) external view returns (bool) {
        return _hasConfig[dapp];
    }

    /// @notice Get the effective fee breakdown for a given amount
    /// @param dapp The dApp contract address
    /// @param amount The transaction amount
    /// @return platformFee The platform fee amount
    /// @return creatorFee The creator fee amount
    /// @return referralFee The referral fee amount
    /// @return royaltyFee The royalty fee amount
    function calculateFees(
        address dapp,
        uint256 amount
    ) external view returns (
        uint256 platformFee,
        uint256 creatorFee,
        uint256 referralFee,
        uint256 royaltyFee
    ) {
        if (!_hasConfig[dapp]) revert ConfigNotSet(dapp);
        RevenueConfig storage config = _feeConfigs[dapp];

        platformFee = (amount * config.platformFeeBps) / MAX_BPS;
        creatorFee = (amount * config.creatorFeeBps) / MAX_BPS;
        referralFee = (amount * config.referralFeeBps) / MAX_BPS;
        royaltyFee = (amount * config.royaltyFeeBps) / MAX_BPS;
    }
}
