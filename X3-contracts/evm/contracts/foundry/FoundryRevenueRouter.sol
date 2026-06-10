// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

/// @title FoundryRevenueRouter — Revenue Routing for X3 Foundry
/// @notice Routes revenue from dApps to the treasury split destinations
/// @dev Uses ReentrancyGuard for safety and Ownable for admin functions
contract FoundryRevenueRouter is Ownable, ReentrancyGuard {
    using SafeERC20 for IERC20;

    // ── Types ────────────────────────────────────────────────────────────────

    /// @notice Treasury split configuration
    struct TreasurySplit {
        uint256 protocolTreasuryBps;   // Basis points for protocol treasury
        uint256 gpuSwarmBps;           // Basis points for GPU Swarm
        uint256 devVaultBps;           // Basis points for Developer Vault
        uint256 maintenanceBps;        // Basis points for Maintenance
        uint256 liquidityBps;          // Basis points for Liquidity
        uint256 grantsBps;             // Basis points for Grants
    }

    // ── Constants ────────────────────────────────────────────────────────────

    /// @notice Maximum basis points (100%)
    uint256 public constant MAX_BPS = 10000;

    /// @notice Default protocol treasury share (40%)
    uint256 public constant DEFAULT_PROTOCOL_TREASURY_BPS = 4000;
    /// @notice Default GPU swarm share (20%)
    uint256 public constant DEFAULT_GPU_SWARM_BPS = 2000;
    /// @notice Default dev vault share (15%)
    uint256 public constant DEFAULT_DEV_VAULT_BPS = 1500;
    /// @notice Default maintenance share (10%)
    uint256 public constant DEFAULT_MAINTENANCE_BPS = 1000;
    /// @notice Default liquidity share (10%)
    uint256 public constant DEFAULT_LIQUIDITY_BPS = 1000;
    /// @notice Default grants share (5%)
    uint256 public constant DEFAULT_GRANTS_BPS = 500;

    // ── State ────────────────────────────────────────────────────────────────

    /// @notice Current treasury split configuration
    TreasurySplit private _treasurySplit;

    /// @notice Protocol treasury address
    address public protocolTreasury;

    /// @notice GPU Swarm address
    address public gpuSwarm;

    /// @notice Developer Vault address
    address public devVault;

    /// @notice Maintenance address
    address public maintenance;

    /// @notice Liquidity address
    address public liquidity;

    /// @notice Grants address
    address public grants;

    /// @notice Revenue accumulated per dApp (in wei / token units)
    mapping(address => uint256) public revenueByApp;

    /// @notice Revenue accumulated per creator
    mapping(address => uint256) public revenueByCreator;

    /// @notice Total revenue routed all-time
    uint256 public totalRevenue;

    /// @notice dApp => creator mapping
    mapping(address => address) public dappCreator;

    // ── Events ───────────────────────────────────────────────────────────────

    /// @notice Emitted when revenue is routed
    event RevenueRouted(
        address indexed dapp,
        address indexed token,
        uint256 amount,
        uint256 timestamp
    );

    /// @notice Emitted when treasury split is updated
    event TreasurySplitUpdated(
        uint256 protocolTreasuryBps,
        uint256 gpuSwarmBps,
        uint256 devVaultBps,
        uint256 maintenanceBps,
        uint256 liquidityBps,
        uint256 grantsBps,
        uint256 timestamp
    );

    /// @notice Emitted when a destination address is updated
    event DestinationUpdated(string indexed name, address indexed newAddress);

    // ── Errors ───────────────────────────────────────────────────────────────

    error InvalidSplit();
    error ZeroAddress();
    error ZeroAmount();
    error NoCreatorSet();

    // ── Constructor ──────────────────────────────────────────────────────────

    constructor(
        address _protocolTreasury,
        address _gpuSwarm,
        address _devVault,
        address _maintenance,
        address _liquidity,
        address _grants
    ) {
        _transferOwnership(msg.sender);
        if (_protocolTreasury == address(0)) revert ZeroAddress();
        if (_gpuSwarm == address(0)) revert ZeroAddress();
        if (_devVault == address(0)) revert ZeroAddress();
        if (_maintenance == address(0)) revert ZeroAddress();
        if (_liquidity == address(0)) revert ZeroAddress();
        if (_grants == address(0)) revert ZeroAddress();

        protocolTreasury = _protocolTreasury;
        gpuSwarm = _gpuSwarm;
        devVault = _devVault;
        maintenance = _maintenance;
        liquidity = _liquidity;
        grants = _grants;

        _treasurySplit = TreasurySplit({
            protocolTreasuryBps: DEFAULT_PROTOCOL_TREASURY_BPS,
            gpuSwarmBps: DEFAULT_GPU_SWARM_BPS,
            devVaultBps: DEFAULT_DEV_VAULT_BPS,
            maintenanceBps: DEFAULT_MAINTENANCE_BPS,
            liquidityBps: DEFAULT_LIQUIDITY_BPS,
            grantsBps: DEFAULT_GRANTS_BPS
        });
    }

    // ── Admin Functions ──────────────────────────────────────────────────────

    /// @notice Set the treasury split percentages (in basis points)
    /// @param split The new TreasurySplit configuration
    function setTreasurySplit(TreasurySplit calldata split) external onlyOwner {
        uint256 total = split.protocolTreasuryBps + split.gpuSwarmBps + split.devVaultBps
            + split.maintenanceBps + split.liquidityBps + split.grantsBps;
        if (total != MAX_BPS) revert InvalidSplit();

        _treasurySplit = split;

        emit TreasurySplitUpdated(
            split.protocolTreasuryBps,
            split.gpuSwarmBps,
            split.devVaultBps,
            split.maintenanceBps,
            split.liquidityBps,
            split.grantsBps,
            block.timestamp
        );
    }

    /// @notice Set a destination address for a named treasury component
    function setDestination(string calldata name, address newAddress) external onlyOwner {
        if (newAddress == address(0)) revert ZeroAddress();

        if (keccak256(bytes(name)) == keccak256(bytes("protocolTreasury"))) {
            protocolTreasury = newAddress;
        } else if (keccak256(bytes(name)) == keccak256(bytes("gpuSwarm"))) {
            gpuSwarm = newAddress;
        } else if (keccak256(bytes(name)) == keccak256(bytes("devVault"))) {
            devVault = newAddress;
        } else if (keccak256(bytes(name)) == keccak256(bytes("maintenance"))) {
            maintenance = newAddress;
        } else if (keccak256(bytes(name)) == keccak256(bytes("liquidity"))) {
            liquidity = newAddress;
        } else if (keccak256(bytes(name)) == keccak256(bytes("grants"))) {
            grants = newAddress;
        } else {
            revert("INVALID_DESTINATION");
        }

        emit DestinationUpdated(name, newAddress);
    }

    /// @notice Set the creator for a dApp (for revenue tracking)
    function setDappCreator(address dapp, address creator) external onlyOwner {
        if (dapp == address(0) || creator == address(0)) revert ZeroAddress();
        dappCreator[dapp] = creator;
    }

    // ── Core Functions ───────────────────────────────────────────────────────

    /// @notice Route incoming revenue (native currency) according to the treasury split
    /// @param dapp The dApp address generating the revenue
    /// @param creator The creator address to credit
    function routeRevenue(address dapp, address creator) external payable nonReentrant {
        if (msg.value == 0) revert ZeroAmount();
        if (creator == address(0)) revert ZeroAddress();

        uint256 amount = msg.value;

        // Track revenue
        revenueByApp[dapp] += amount;
        revenueByCreator[creator] += amount;
        totalRevenue += amount;

        // Split and forward
        _splitAndForward(amount);

        emit RevenueRouted(dapp, address(0), amount, block.timestamp);
    }

    /// @notice Route incoming ERC20 revenue according to the treasury split
    /// @param dapp The dApp address generating the revenue
    /// @param creator The creator address to credit
    /// @param token The ERC20 token address
    /// @param amount The amount of tokens
    function routeRevenueToken(
        address dapp,
        address creator,
        IERC20 token,
        uint256 amount
    ) external nonReentrant {
        if (amount == 0) revert ZeroAmount();
        if (creator == address(0)) revert ZeroAddress();
        if (address(token) == address(0)) revert ZeroAddress();

        // Transfer tokens from sender
        token.safeTransferFrom(msg.sender, address(this), amount);

        // Track revenue
        revenueByApp[dapp] += amount;
        revenueByCreator[creator] += amount;
        totalRevenue += amount;

        // Split and forward
        _splitAndForwardToken(token, amount);

        emit RevenueRouted(dapp, address(token), amount, block.timestamp);
    }

    // ── Internal ─────────────────────────────────────────────────────────────

    /// @notice Split native currency and forward to destinations
    function _splitAndForward(uint256 amount) internal {
        TreasurySplit memory split = _treasurySplit;

        _sendNative(protocolTreasury, (amount * split.protocolTreasuryBps) / MAX_BPS);
        _sendNative(gpuSwarm, (amount * split.gpuSwarmBps) / MAX_BPS);
        _sendNative(devVault, (amount * split.devVaultBps) / MAX_BPS);
        _sendNative(maintenance, (amount * split.maintenanceBps) / MAX_BPS);
        _sendNative(liquidity, (amount * split.liquidityBps) / MAX_BPS);
        _sendNative(grants, (amount * split.grantsBps) / MAX_BPS);
    }

    /// @notice Split ERC20 tokens and forward to destinations
    function _splitAndForwardToken(IERC20 token, uint256 amount) internal {
        TreasurySplit memory split = _treasurySplit;

        _sendToken(token, protocolTreasury, (amount * split.protocolTreasuryBps) / MAX_BPS);
        _sendToken(token, gpuSwarm, (amount * split.gpuSwarmBps) / MAX_BPS);
        _sendToken(token, devVault, (amount * split.devVaultBps) / MAX_BPS);
        _sendToken(token, maintenance, (amount * split.maintenanceBps) / MAX_BPS);
        _sendToken(token, liquidity, (amount * split.liquidityBps) / MAX_BPS);
        _sendToken(token, grants, (amount * split.grantsBps) / MAX_BPS);
    }

    /// @notice Send native currency to an address
    function _sendNative(address to, uint256 amount) internal {
        if (amount > 0) {
            (bool success,) = payable(to).call{value: amount}("");
            require(success, "TRANSFER_FAILED");
        }
    }

    /// @notice Send ERC20 tokens to an address
    function _sendToken(IERC20 token, address to, uint256 amount) internal {
        if (amount > 0) {
            token.safeTransfer(to, amount);
        }
    }

    // ── View Functions ───────────────────────────────────────────────────────

    /// @notice Get the current treasury split configuration
    /// @return TreasurySplit struct
    function getTreasurySplit() external view returns (TreasurySplit memory) {
        return _treasurySplit;
    }

    /// @notice Get total revenue routed for a specific dApp
    /// @param dapp The dApp address
    /// @return Total revenue in wei/token units
    function getRevenueByApp(address dapp) external view returns (uint256) {
        return revenueByApp[dapp];
    }

    /// @notice Get total revenue attributed to a specific creator
    /// @param creator The creator address
    /// @return Total revenue in wei/token units
    function getRevenueByCreator(address creator) external view returns (uint256) {
        return revenueByCreator[creator];
    }

    /// @notice Get total revenue routed all-time
    /// @return Total revenue in wei/token units
    function getTotalRevenue() external view returns (uint256) {
        return totalRevenue;
    }
}
