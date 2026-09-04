// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/access/AccessControl.sol";

/// @title X3KernelBridge — Bridge interface between X3 runtime and EVM contracts
/// @notice Deployed on X3 EVM side, called by the X3 runtime pallet
/// @dev This is the contract that the X3 kernel calls to trigger EVM-side operations
contract X3KernelBridge is AccessControl {
    bytes32 public constant KERNEL_ROLE = keccak256("KERNEL_ROLE");

    /// @notice Registered token adapters: assetId => X3VmERC20 address
    mapping(bytes32 => address) public tokenAdapters;

    /// @notice Registered external gateways: chainId => gateway address
    mapping(uint256 => address) public externalGateways;

    /// @notice Emitted when a token adapter is registered
    event TokenAdapterRegistered(bytes32 indexed assetId, address indexed adapter);

    /// @notice Emitted when an external gateway is registered
    event ExternalGatewayRegistered(uint256 indexed chainId, address indexed gateway);

    /// @notice Emitted when a cross-VM transfer is completed (destination credited)
    event CrossVmTransferCompleted(
        bytes32 indexed messageId,
        bytes32 indexed assetId,
        address indexed recipient,
        uint256 amount
    );

    /// @notice Emitted when an external deposit is completed
    event ExternalDepositCompleted(
        bytes32 indexed messageId,
        bytes32 indexed assetId,
        address indexed recipient,
        uint256 amount,
        uint256 sourceChainId
    );

    constructor() {
        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
        _grantRole(KERNEL_ROLE, msg.sender);
    }

    /// @notice Register a token adapter for a canonical asset
    function registerTokenAdapter(bytes32 assetId, address adapter) external onlyRole(KERNEL_ROLE) {
        require(adapter != address(0), "ZERO_ADAPTER");
        require(tokenAdapters[assetId] == address(0), "ALREADY_REGISTERED");
        tokenAdapters[assetId] = adapter;
        emit TokenAdapterRegistered(assetId, adapter);
    }

    /// @notice Register an external gateway for a chain
    function registerExternalGateway(uint256 chainId, address gateway) external onlyRole(KERNEL_ROLE) {
        require(gateway != address(0), "ZERO_GATEWAY");
        externalGateways[chainId] = gateway;
        emit ExternalGatewayRegistered(chainId, gateway);
    }

    /// @notice Mint tokens to a user on EVM (called by kernel for cross-VM or cross-chain credit)
    function creditUser(
        bytes32 messageId,
        bytes32 assetId,
        address recipient,
        uint256 amount
    ) external onlyRole(KERNEL_ROLE) returns (bool) {
        address adapter = tokenAdapters[assetId];
        require(adapter != address(0), "NO_ADAPTER");
        require(recipient != address(0), "ZERO_RECIPIENT");
        require(amount > 0, "ZERO_AMOUNT");

        // Call the adapter to mint
        (bool success, ) = adapter.call(
            abi.encodeWithSignature("kernelMint(address,uint256)", recipient, amount)
        );
        require(success, "MINT_FAILED");

        emit CrossVmTransferCompleted(messageId, assetId, recipient, amount);
        return true;
    }

    /// @notice Burn tokens from a user on EVM (called by kernel for cross-VM or cross-chain debit)
    function debitUser(
        bytes32 messageId,
        bytes32 assetId,
        address user,
        uint256 amount
    ) external onlyRole(KERNEL_ROLE) returns (bool) {
        address adapter = tokenAdapters[assetId];
        require(adapter != address(0), "NO_ADAPTER");
        require(user != address(0), "ZERO_USER");
        require(amount > 0, "ZERO_AMOUNT");

        // Call the adapter to burn
        (bool success, ) = adapter.call(
            abi.encodeWithSignature("kernelBurn(address,uint256)", user, amount)
        );
        require(success, "BURN_FAILED");

        return true;
    }

    /// @notice Get the adapter address for a given asset
    function getAdapter(bytes32 assetId) external view returns (address) {
        return tokenAdapters[assetId];
    }

    /// @notice Get the gateway address for a given chain
    function getGateway(uint256 chainId) external view returns (address) {
        return externalGateways[chainId];
    }
}