// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Script.sol";
import "forge-std/console.sol";
import "../contracts/X3ExternalGateway.sol";

/// @title DeployX3Gateway — Foundry deploy script for X3ExternalGateway
/// @notice Deploys the X3ExternalGateway to an EVM chain with configurable parameters.
///
/// Usage:
///   # Deploy to Sepolia (chainId=11155111)
///   forge script script/DeployX3Gateway.s.sol:DeployX3Gateway \
///     --rpc-url "$SEPOLIA_RPC_URL" \
///     --broadcast \
///     --verify \
///     --etherscan-api-key "$ETHERSCAN_API_KEY" \
///     -vvvv
///
///   # Simulate only (no broadcast)
///   forge script script/DeployX3Gateway.s.sol:DeployX3Gateway \
///     --rpc-url "$SEPOLIA_RPC_URL" \
///     -vvvv
///
/// Environment variables (injected via --sig or env):
///   VERIFIER_ADDRESS         — Address of the IX3Verification contract
///   CHAIN_ID                 — EVM chain ID (e.g. 11155111 for Sepolia)
///   X3_CHAIN_ID              — X3 destination chain ID
///   MIN_X3_CONFIRMATIONS     — Minimum X3 finality confirmations required
contract DeployX3Gateway is Script {
    function run() external {
        // Read constructor parameters from environment with sensible defaults.
        // For testnet: override VERIFIER_ADDRESS and CHAIN_ID.
        address verifier = vm.envOr("VERIFIER_ADDRESS", address(0));
        uint256 chainId = vm.envOr("CHAIN_ID", uint256(11155111));       // Sepolia default
        uint256 x3ChainId = vm.envOr("X3_CHAIN_ID", uint256(200));       // X3 domain
        uint256 minConfirmations = vm.envOr("MIN_X3_CONFIRMATIONS", uint256(12));

        require(verifier != address(0), "VERIFIER_ADDRESS must be set (e.g. to deployed EvmReceiptVerifier)");

        vm.startBroadcast();

        X3ExternalGateway gateway = new X3ExternalGateway(
            verifier,
            chainId,
            x3ChainId,
            minConfirmations
        );

        vm.stopBroadcast();

        // Log for CI / verification scripts to capture
        console.log("=== X3ExternalGateway Deployed ===");
        console.log(string(abi.encodePacked("Gateway: ", vm.toString(address(gateway)))));
        console.log(string(abi.encodePacked("Chain ID: ", vm.toString(chainId))));
        console.log(string(abi.encodePacked("X3 Chain ID: ", vm.toString(x3ChainId))));
        console.log(string(abi.encodePacked("Verifier: ", vm.toString(verifier))));
        console.log(string(abi.encodePacked("Min Confirmations: ", vm.toString(minConfirmations))));
    }
}