// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Script.sol";
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
        console.log("X3ExternalGateway deployed at:", address(gateway));
        console.log("  Chain ID:", chainId);
        console.log("  X3 Chain ID:", x3ChainId);
        console.log("  Verifier:", verifier);
        console.log("  Min X3 Confirmations:", minConfirmations);
        console.log("");
        console.log("Verify with:");
        console.log("  forge verify-contract \\");
        console.log("    --watch \\");
        console.log("    --chain", chainId);
        console.log("   ", address(gateway), "contracts/X3ExternalGateway.sol:X3ExternalGateway \\");
        console.log("    --verifier etherscan \\");
        console.log("    --etherscan-api-key \"$ETHERSCAN_API_KEY\" \\");
        console.log("    --constructor-args $(cast abi-encode \"constructor(address,uint256,uint256,uint256)\"", verifier, chainId, x3ChainId, minConfirmations, ")");
    }
}