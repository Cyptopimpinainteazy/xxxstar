// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/access/AccessControl.sol";

/// @title X3VmERC20 — Kernel-callable ERC20 adapter for X3 cross-VM and cross-chain
/// @notice This contract is the EVM-side representation of any X3-registered asset.
/// @dev Only the X3 kernel address can mint/burn. This ensures supply parity
///      with the canonical X3 supply ledger.
contract X3VmERC20 is ERC20, AccessControl {
    bytes32 public constant KERNEL_ROLE = keccak256("KERNEL_ROLE");
    bytes32 public constant BRIDGE_ROLE = keccak256("BRIDGE_ROLE");

    /// @notice The canonical X3 asset ID (32 bytes, H256 format)
    bytes32 public immutable assetId;

    /// @notice The origin domain this ERC20 represents (X3Native, X3Svm, Ethereum, etc.)
    uint8 public immutable originDomain;

    /// @notice The original chain ID (for external assets)
    uint256 public immutable originChainId;

    /// @notice The original token address on its native chain (zero for X3-native)
    address public immutable originToken;

    /// @notice Decimals for this token
    uint8 private immutable _decimals;

    // ── Events ──────────────────────────────────────────────────────────────

    /// @notice Emitted when tokens are sent to another VM via the kernel
    event CrossVmTransferInitiated(
        bytes32 indexed assetId,
        address indexed sender,
        uint8 destinationDomain,
        bytes recipient,
        uint256 amount
    );

    /// @notice Emitted when tokens are minted by the kernel
    event KernelMint(
        bytes32 indexed assetId,
        address indexed to,
        uint256 amount
    );

    /// @notice Emitted when tokens are burned by the kernel
    event KernelBurn(
        bytes32 indexed assetId,
        address indexed from,
        uint256 amount
    );

    // ── Constructor ─────────────────────────────────────────────────────────

    constructor(
        string memory _name,
        string memory _symbol,
        uint8 _tokenDecimals,
        bytes32 _assetId,
        uint8 _originDomain,
        uint256 _originChainId,
        address _originToken,
        address _kernel
    ) ERC20(_name, _symbol) {
        require(_kernel != address(0), "ZERO_KERNEL");
        assetId = _assetId;
        originDomain = _originDomain;
        originChainId = _originChainId;
        originToken = _originToken;
        _decimals = _tokenDecimals;

        _grantRole(DEFAULT_ADMIN_ROLE, _kernel);
        _grantRole(KERNEL_ROLE, _kernel);
    }

    // ── Overrides ──────────────────────────────────────────────────────────

    function decimals() public view override returns (uint8) {
        return _decimals;
    }

    // ── Kernel Functions ────────────────────────────────────────────────────

    /// @notice Mint tokens via kernel authority (cross-chain deposit or cross-VM mint)
    /// @param to The recipient address
    /// @param amount The amount to mint
    function kernelMint(address to, uint256 amount) external onlyRole(KERNEL_ROLE) {
        require(to != address(0), "ZERO_TO");
        require(amount > 0, "ZERO_AMOUNT");
        _mint(to, amount);
        emit KernelMint(assetId, to, amount);
    }

    /// @notice Burn tokens via kernel authority (cross-chain withdrawal or cross-VM burn)
    /// @param from The address to burn from
    /// @param amount The amount to burn
    function kernelBurn(address from, uint256 amount) external onlyRole(KERNEL_ROLE) {
        require(from != address(0), "ZERO_FROM");
        require(amount > 0, "ZERO_AMOUNT");
        _burn(from, amount);
        emit KernelBurn(assetId, from, amount);
    }

    /// @notice Initiate a cross-VM transfer through the kernel
    /// @param destinationDomain The target VM domain ID
    /// @param recipient The recipient on the destination VM (domain-encoded)
    /// @param amount The amount to transfer
    function sendToVm(
        uint8 destinationDomain,
        bytes calldata recipient,
        uint256 amount
    ) external {
        require(balanceOf(msg.sender) >= amount, "INSUFFICIENT_BALANCE");
        require(recipient.length > 0 && recipient.length <= 64, "INVALID_RECIPIENT");

        // Burn local representation — the cross-VM router will mint on destination
        _burn(msg.sender, amount);
        emit Transfer(msg.sender, address(0), amount);

        emit CrossVmTransferInitiated(
            assetId,
            msg.sender,
            destinationDomain,
            recipient,
            amount
        );
    }

    // ── Bridge Role Functions ──────────────────────────────────────────────

    /// @notice Mint tokens via bridge authority (for external chain deposits)
    /// @param to The recipient address
    /// @param amount The amount to mint
    function bridgeMint(address to, uint256 amount) external onlyRole(BRIDGE_ROLE) {
        require(to != address(0), "ZERO_TO");
        require(amount > 0, "ZERO_AMOUNT");
        _mint(to, amount);
        emit KernelMint(assetId, to, amount);
    }

    /// @notice Burn tokens via bridge authority (for external chain withdrawals)
    /// @param from The address to burn from
    /// @param amount The amount to burn
    function bridgeBurn(address from, uint256 amount) external onlyRole(BRIDGE_ROLE) {
        require(from != address(0), "ZERO_FROM");
        require(amount > 0, "ZERO_AMOUNT");
        _burn(from, amount);
        emit KernelBurn(assetId, from, amount);
    }

    /// @notice Grant bridge role to the external gateway contract
    function grantBridgeRole(address gateway) external onlyRole(DEFAULT_ADMIN_ROLE) {
        grantRole(BRIDGE_ROLE, gateway);
    }

    /// @notice Revoke bridge role
    function revokeBridgeRole(address gateway) external onlyRole(DEFAULT_ADMIN_ROLE) {
        revokeRole(BRIDGE_ROLE, gateway);
    }
}