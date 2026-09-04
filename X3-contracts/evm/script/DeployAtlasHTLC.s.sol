// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Script.sol";
import "forge-std/console.sol";
import "../contracts/AtlasHTLC.sol";

/// @title DeployAtlasHTLC — Foundry deploy script for AtlasHTLC
/// @notice Deploys the AtlasHTLC contract to an EVM chain.
///
/// The contract has no constructor arguments, making deployment straightforward.
///
/// Usage:
///   # Deploy to Sepolia (default, chainId=11155111)
///   CHAIN_ID=11155111 DEPLOYER_PRIVATE_KEY=... \
///     forge script script/DeployAtlasHTLC.s.sol:DeployAtlasHTLC \
///     --rpc-url "$SEPOLIA_RPC_URL" \
///     --broadcast \
///     --verify \
///     --etherscan-api-key "$ETHERSCAN_API_KEY" \
///     -vvvv
///
///   # Deploy to localhost (anvil)
///   CHAIN_ID=31337 DEPLOYER_PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 \
///     forge script script/DeployAtlasHTLC.s.sol:DeployAtlasHTLC \
///     --rpc-url http://localhost:8545 \
///     --broadcast \
///     -vvvv
///
///   # Deploy to Holesky
///   CHAIN_ID=17000 DEPLOYER_PRIVATE_KEY=... \
///     forge script script/DeployAtlasHTLC.s.sol:DeployAtlasHTLC \
///     --rpc-url "$HOLESKY_RPC_URL" \
///     --broadcast \
///     --verify \
///     --etherscan-api-key "$ETHERSCAN_API_KEY" \
///     -vvvv
///
///   # Simulate only (no broadcast)
///   forge script script/DeployAtlasHTLC.s.sol:DeployAtlasHTLC \
///     --rpc-url "$RPC_URL" \
///     -vvvv
///
/// Environment variables:
///   DEPLOYER_PRIVATE_KEY  — Required. Private key of the deployer account.
///   CHAIN_ID              — Optional. EVM chain ID (default: 11155111 for Sepolia).
///                           Used for logging only; actual chain is determined by --rpc-url.
contract DeployAtlasHTLC is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address deployer = vm.addr(deployerPrivateKey);

        // CHAIN_ID is for documentation/logging; the actual chain is set via --rpc-url.
        uint256 chainId = vm.envOr("CHAIN_ID", uint256(11155111)); // Sepolia default

        console.log("Deploying AtlasHTLC from:", vm.toString(deployer));
        console.log("Configured Chain ID:", chainId);
        console.log("Actual Chain ID:", block.chainid);

        vm.startBroadcast(deployerPrivateKey);

        AtlasHTLC htlc = new AtlasHTLC();

        vm.stopBroadcast();

        console.log("=== AtlasHTLC Deployed ===");
        console.log(string(abi.encodePacked("AtlasHTLC: ", vm.toString(address(htlc)))));
        console.log(string(abi.encodePacked("Chain ID: ", vm.toString(chainId))));
    }
}
