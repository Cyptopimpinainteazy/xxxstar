// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Script.sol";
import "../contracts/EvmReceiptVerifier.sol";

/// @title DeployX3Verifier — Foundry deploy script for EvmReceiptVerifier
/// @notice Deploys the X3 proof verifier to an EVM chain. Should be run
///         BEFORE DeployX3Gateway so the gateway can reference it.
///
/// Usage:
///   # Deploy to Sepolia with 3 testnet validators and 2/3 quorum
///   forge script script/DeployX3Verifier.s.sol:DeployX3Verifier \
///     --rpc-url "$SEPOLIA_RPC_URL" \
///     --broadcast \
///     --verify \
///     --etherscan-api-key "$ETHERSCAN_API_KEY" \
///     -vvvv
///
///   # Simulate only (no broadcast)
///   forge script script/DeployX3Verifier.s.sol:DeployX3Verifier \
///     --rpc-url "$SEPOLIA_RPC_URL" \
///     -vvvv
///
/// Environment variables (injected via --sig or env):
///   VALIDATOR_PUBKEYS        — Comma-separated hex validator Ed25519 pubkeys (32 bytes each)
///   QUORUM_THRESHOLD         — Minimum signatures required (default: ceil(N * 2/3))
contract DeployX3Verifier is Script {
    function run() external returns (address verifierAddr) {
        // Read validator pubkeys from env (comma-separated 64-char hex)
        string memory pubkeysRaw = vm.envOr("VALIDATOR_PUBKEYS", string(""));
        uint256 quorum = vm.envOr("QUORUM_THRESHOLD", uint256(0));

        // Parse pubkeys into bytes32[] array
        bytes memory pubkeysBytes = bytes(pubkeysRaw);
        require(pubkeysBytes.length > 0, "VALIDATOR_PUBKEYS must be set");

        // Count commas to determine array length
        uint256 count = 1;
        for (uint256 i = 0; i < pubkeysBytes.length; i++) {
            if (pubkeysBytes[i] == ",") {
                count++;
            }
        }

        bytes32[] memory pubkeys = new bytes32[](count);
        uint256 idx = 0;
        uint256 start = 0;
        for (uint256 i = 0; i <= pubkeysBytes.length; i++) {
            if (i == pubkeysBytes.length || pubkeysBytes[i] == ",") {
                bytes memory hexStr = new bytes(i - start);
                for (uint256 j = start; j < i; j++) {
                    hexStr[j - start] = pubkeysBytes[j];
                }
                pubkeys[idx] = _hexToBytes32(string(hexStr));
                idx++;
                start = i + 1; // skip comma
            }
        }

        // Default quorum to 2/3 if not specified
        if (quorum == 0) {
            quorum = (count * 2) / 3 + 1; // ceil(2/3 * count)
        }
        require(quorum > 0 && quorum <= count, "Invalid quorum threshold");

        vm.startBroadcast();

        EvmReceiptVerifier verifier = new EvmReceiptVerifier(pubkeys, quorum);

        vm.stopBroadcast();

        verifierAddr = address(verifier);

        // Log for CI / verification scripts to capture
        console.log("EvmReceiptVerifier deployed at:", address(verifier));
        console.log("  Validator count:", count);
        console.log("  Quorum threshold:", quorum);
        console.log("  Verifier set ID:", verifier.verifierSetId());
        console.log("");
        console.log("Verify with:");
        console.log("  forge verify-contract \\");
        console.log("    --watch \\");
        console.log("    --chain", vm.envOr("CHAIN_ID", uint256(11155111)));
        console.log("   ", address(verifier), "contracts/EvmReceiptVerifier.sol:EvmReceiptVerifier \\");
        console.log("    --verifier etherscan \\");
        console.log("    --etherscan-api-key \"$ETHERSCAN_API_KEY\" \\");
        console.log("    --constructor-args $(cast abi-encode \"constructor(bytes32[],uint256)\" \\");
        console.log("        \"[$VALIDATOR_PUBKEYS]\"", quorum, ")");
    }

    /// @dev Convert a 64-char (or 66 with 0x) hex string to bytes32
    function _hexToBytes32(string memory hexStr) internal pure returns (bytes32 result) {
        bytes memory b = bytes(hexStr);
        uint256 offset = 0;
        if (b.length >= 2 && b[0] == "0" && (b[1] == "x" || b[1] == "X")) {
            offset = 2;
        }
        require(b.length - offset == 64, "Invalid pubkey hex length");

        for (uint256 i = 0; i < 32; i++) {
            uint8 high = _hexCharToUint8(b[offset + i * 2]);
            uint8 low = _hexCharToUint8(b[offset + i * 2 + 1]);
            result |= bytes32(uint256(high) << 4 | uint256(low)) >> (i * 8);
        }
    }

    function _hexCharToUint8(bytes1 c) internal pure returns (uint8) {
        if (c >= "0" && c <= "9") return uint8(c) - uint8(bytes1("0"));
        if (c >= "a" && c <= "f") return uint8(c) - uint8(bytes1("a")) + 10;
        if (c >= "A" && c <= "F") return uint8(c) - uint8(bytes1("A")) + 10;
        revert("Invalid hex character");
    }
}