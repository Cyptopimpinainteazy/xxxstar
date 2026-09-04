// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../contracts/EvmReceiptVerifier.sol";

contract EvmReceiptVerifierTest is Test {
    EvmReceiptVerifier public verifier;

    bytes32[] public validators;

    function setUp() public {
        // Three validators with distinct 32-byte pubkeys
        validators.push(bytes32(uint256(1)));
        validators.push(bytes32(uint256(2)));
        validators.push(bytes32(uint256(3)));
        verifier = new EvmReceiptVerifier(validators, 2);
    }

    // ── Testnet mode toggle ──────────────────────────────────────────────

    function testTestnetModeDefaultOff() public view {
        assertFalse(verifier.testnetMode());
    }

    function testSetTestnetMode() public {
        vm.expectEmit(true, false, false, false);
        emit EvmReceiptVerifier.TestnetModeSet(true);
        verifier.setTestnetMode(true);
        assertTrue(verifier.testnetMode());

        verifier.setTestnetMode(false);
        assertFalse(verifier.testnetMode());
    }

    function testSetTestnetModeRevertsWhenNotOwner() public {
        vm.prank(address(0xdead));
        vm.expectRevert();
        verifier.setTestnetMode(true);
    }

    // ── Production mode reverts ──────────────────────────────────────────

    function testProductionModeReverts() public {
        bytes memory proof = _makeProof(1, 2);
        vm.expectRevert(
            "EvmReceiptVerifier: on-chain Ed25519 verification not available - bridge blocked"
        );
        verifier.verifyX3WithdrawalProof(
            keccak256("msg"), 1, hex"beef", address(0xa), 100, proof
        );
    }

    // ── Testnet mode: valid proofs ───────────────────────────────────────

    function testTestnetModeValidProof() public {
        verifier.setTestnetMode(true);
        bytes memory proof = _makeProof(1, 3);
        bool ok = verifier.verifyX3WithdrawalProof(
            keccak256("msg"), 1, hex"beef", address(0xa), 100, proof
        );
        assertTrue(ok);
    }

    function testTestnetModeValidProofDeposit() public {
        verifier.setTestnetMode(true);
        bytes memory proof = _makeProof(1, 3);
        bool ok = verifier.verifyDepositProof(
            keccak256("dep"), address(0x123), address(0x456), hex"72656376", 200, proof
        );
        assertTrue(ok);
    }

    function testTestnetModeQuorumNotMet() public {
        verifier.setTestnetMode(true);
        // Only 1 signature, but threshold is 2
        bytes memory proof = _makeProof(1, 1);
        bool ok = verifier.verifyX3WithdrawalProof(
            keccak256("msg"), 1, hex"beef", address(0xa), 100, proof
        );
        assertFalse(ok);
    }

    // ── Testnet mode: invalid proofs ────────────────────────────────────

    function testTestnetModeWrongSetId() public {
        verifier.setTestnetMode(true);
        bytes memory proof = _makeProof(999, 2);
        vm.expectRevert("VerifierSetId mismatch");
        verifier.verifyX3WithdrawalProof(
            keccak256("msg"), 1, hex"beef", address(0xa), 100, proof
        );
    }

    function testTestnetModeBadFormat() public {
        verifier.setTestnetMode(true);
        bytes memory proof = new bytes(10);
        proof[0] = 0;
        proof[1] = 0;
        proof[2] = 0;
        proof[3] = bytes1(uint8(1));
        // Only 10 bytes total — not aligned to 65-byte slots after 4-byte header
        vm.expectRevert("Invalid signature data length");
        verifier.verifyX3WithdrawalProof(
            keccak256("msg"), 1, hex"beef", address(0xa), 100, proof
        );
    }

    function testTestnetModeProofTooShort() public {
        verifier.setTestnetMode(true);
        bytes memory proof = new bytes(2);
        vm.expectRevert("Proof too short");
        verifier.verifyX3WithdrawalProof(
            keccak256("msg"), 1, hex"beef", address(0xa), 100, proof
        );
    }

    function testTestnetModeEmptyAfterSetId() public {
        verifier.setTestnetMode(true);
        bytes memory proof = new bytes(4);
        proof[0] = 0;
        proof[1] = 0;
        proof[2] = 0;
        proof[3] = bytes1(uint8(1));
        // 0 signature slots, threshold is 2 => false
        bool ok = verifier.verifyX3WithdrawalProof(
            keccak256("msg"), 1, hex"beef", address(0xa), 100, proof
        );
        assertFalse(ok);
    }

    // ── Replay protection still works ───────────────────────────────────

    function testTestnetModeReplayProtection() public {
        verifier.setTestnetMode(true);
        bytes32 msgId = keccak256("unique");
        bytes memory proof = _makeProof(1, 3);

        // First verification should succeed
        bool ok = verifier.verifyX3WithdrawalProof(
            msgId, 1, hex"beef", address(0xa), 100, proof
        );
        assertTrue(ok);

        // Mark as verified (simulate gateway)
        verifier.markVerified(msgId);

        // Second verification should fail (replay)
        ok = verifier.verifyX3WithdrawalProof(
            msgId, 1, hex"beef", address(0xa), 100, proof
        );
        assertFalse(ok);
    }

    // ── Helpers ─────────────────────────────────────────────────────────

    /// Build a proof blob with the given verifierSetId and number of sig slots.
    /// Each slot has index = slot_number + 1 (non-zero) and a dummy 64-byte sig.
    function _makeProof(uint32 setId, uint256 slotCount) internal pure returns (bytes memory) {
        uint256 totalLen = 4 + slotCount * 65;
        bytes memory proof = new bytes(totalLen);
        proof[0] = bytes1(uint8(setId >> 24));
        proof[1] = bytes1(uint8(setId >> 16));
        proof[2] = bytes1(uint8(setId >> 8));
        proof[3] = bytes1(uint8(setId));
        for (uint256 i = 0; i < slotCount; i++) {
            uint256 offset = 4 + i * 65;
            proof[offset] = bytes1(uint8(i + 1)); // non-zero index
            // Rest of slot (64 bytes) left as zero — testnet mode doesn't verify sigs
        }
        return proof;
    }
}
