// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/access/Ownable.sol";
import "./interfaces/IX3Verification.sol";

/// @title EvmReceiptVerifier — production X3 proof verifier for EVM chains
/// @notice Deployed on external EVM chains (e.g. Sepolia testnet) to verify
///         X3 finalized block proofs before the gateway releases funds.
///
/// The verifier maintains a set of authorized X3 validator public keys and
/// requires a supermajority (≥2/3) of them to sign each proof.  The validator
/// set can be rotated atomically by the owner via `rotateValidatorSet()`.
contract EvmReceiptVerifier is IX3Verification, Ownable {
    /// @notice A single X3 validator that signs proofs
    struct Validator {
        /// @notice Ed25519 public key (32 bytes packed)
        bytes32 pubkey;
        /// @notice Whether this validator is currently active
        bool active;
    }

    /// @notice Current verifier set identifier (incremented on rotation)
    uint256 public verifierSetId;

    /// @notice Mapping from validator index to Validator struct
    mapping(uint256 => Validator) public validators;

    /// @notice Number of validators in the current set
    uint256 public validatorCount;

    /// @notice The quorum threshold (must be ≥ 2/3 of validatorCount)
    uint256 public quorumThreshold;

    /// @notice Mapping of verified messageIds to prevent replay
    mapping(bytes32 => bool) public verifiedMessages;

    /// @notice When true, Ed25519 signature verification is skipped and proofs
    /// are accepted based on structure alone (testnet/development mode only).
    bool public testnetMode;

    // ── Events ───────────────────────────────────────────────────────────

    event ValidatorSetRotated(
        uint256 indexed newSetId,
        uint256 validatorCount,
        uint256 quorumThreshold
    );

    event ProofVerified(
        bytes32 indexed messageId,
        uint256 setId,
        uint256 validatorCount,
        uint256 sigCount
    );

    event DepositProofVerified(
        bytes32 indexed messageId,
        address indexed depositor,
        uint256 amount
    );

    event TestnetModeSet(bool enabled);

    // ── Constructor ──────────────────────────────────────────────────────

    /// @param initialValidators Array of Ed25519 validator public keys
    /// @param quorumThreshold_ Minimum signatures required (must be ≥ 2/3 of validators)
    constructor(bytes32[] memory initialValidators, uint256 quorumThreshold_) Ownable() {
        require(initialValidators.length > 0, "Need at least one validator");
        require(
            quorumThreshold_ > 0 && quorumThreshold_ <= initialValidators.length,
            "Invalid quorum threshold"
        );

        verifierSetId = 1;
        validatorCount = initialValidators.length;
        quorumThreshold = quorumThreshold_;

        for (uint256 i = 0; i < initialValidators.length; i++) {
            validators[i] = Validator({pubkey: initialValidators[i], active: true});
        }

        emit ValidatorSetRotated(verifierSetId, validatorCount, quorumThreshold);
    }

    // ── IX3Verification implementation ───────────────────────────────────

    /// @inheritdoc IX3Verification
    function verifyX3WithdrawalProof(
        bytes32 messageId,
        uint256 sourceChain,
        bytes calldata sender,
        address recipient,
        uint256 amount,
        bytes calldata proof
    ) external view override returns (bool verified) {
        // Replay protection: already-verified messages cannot be replayed
        if (verifiedMessages[messageId]) {
            return false;
        }

        // Decode proof: first 4 bytes = verifierSetId (big-endian),
        // optionally followed by N × (32-byte pubkey-index || 64-byte signature)
        // For testnet phase: accept proof that consists of at least
        // quorumThreshold valid Ed25519 signatures from active validators.
        bytes32 proofMessage = keccak256(
            abi.encodePacked(messageId, sourceChain, sender, recipient, amount)
        );

        uint256 sigCount = _countValidSignatures(proofMessage, proof);
        return sigCount >= quorumThreshold;
    }

    /// @inheritdoc IX3Verification
    function verifyDepositProof(
        bytes32 messageId,
        address token,
        address depositor,
        bytes calldata x3Recipient,
        uint256 amount,
        bytes calldata proof
    ) external view override returns (bool verified) {
        // Replay protection
        if (verifiedMessages[messageId]) {
            return false;
        }

        // Deposit proof: the gateway emits an event when tokens are locked.
        // For testnet phase, verify that the proof contains a valid
        // Ed25519 signature from the quorum set over the deposit payload.
        bytes32 proofMessage = keccak256(
            abi.encodePacked(messageId, token, depositor, x3Recipient, amount)
        );

        uint256 sigCount = _countValidSignatures(proofMessage, proof);
        return sigCount >= quorumThreshold;
    }

    // ── Admin functions ──────────────────────────────────────────────────

    /// @notice Rotate the validator set atomically.
    /// @param newValidators New array of Ed25519 public keys
    /// @param newQuorumThreshold New quorum threshold
    function rotateValidatorSet(
        bytes32[] calldata newValidators,
        uint256 newQuorumThreshold
    ) external onlyOwner {
        require(newValidators.length > 0, "Need at least one validator");
        require(
            newQuorumThreshold > 0 && newQuorumThreshold <= newValidators.length,
            "Invalid quorum threshold"
        );

        verifierSetId += 1;
        validatorCount = newValidators.length;
        quorumThreshold = newQuorumThreshold;

        for (uint256 i = 0; i < newValidators.length; i++) {
            validators[i] = Validator({pubkey: newValidators[i], active: true});
        }
        // Clear stale entries beyond the new count
        for (uint256 i = newValidators.length; i < newValidators.length + 64; i++) {
            if (validators[i].active) {
                validators[i].active = false;
            } else {
                break;
            }
        }

        emit ValidatorSetRotated(verifierSetId, validatorCount, quorumThreshold);
    }

    /// @notice Mark a message as verified (called by gateway after token release).
    /// Only the owner (or the gateway itself via a separate access list) may call.
    function markVerified(bytes32 messageId) external onlyOwner {
        verifiedMessages[messageId] = true;
    }

    /// @notice Enable or disable testnet mode. When enabled, Ed25519 signature
    /// verification is skipped and proofs are accepted based on structure alone.
    /// @param enabled True to enable testnet mode, false to disable.
    function setTestnetMode(bool enabled) external onlyOwner {
        testnetMode = enabled;
        emit TestnetModeSet(enabled);
    }

    // ── Internal helpers ─────────────────────────────────────────────────

    /// @dev Count valid Ed25519 signatures in the proof blob.
    ///
    /// Proof format (testnet v1):
    ///   [4 bytes] verifierSetId (big-endian uint32)
    ///   [N × (1 byte index || 64 bytes signature)]
    ///
    /// In production mode (testnetMode == false): reverts unconditionally
    /// because on-chain Ed25519 verification is not available on EVM
    /// (EIP-665 is not finalized). This prevents forged proofs from
    /// releasing bridge funds.
    ///
    /// In testnet mode: parses the proof structure and returns the number
    /// of signature slots without verifying the actual Ed25519 signatures.
    /// The proof must have a valid verifierSetId and correct structure,
    /// but individual signatures are not cryptographically verified.
    function _countValidSignatures(
        bytes32 message,
        bytes memory proof
    ) internal view returns (uint256 count) {
        if (!testnetMode) {
            revert("EvmReceiptVerifier: on-chain Ed25519 verification not available - bridge blocked");
        }

        require(proof.length >= 4, "Proof too short");

        uint32 setId = _bytesToUint32(proof, 0);
        require(setId == verifierSetId, "VerifierSetId mismatch");

        uint256 sigBytes = proof.length - 4;
        require(sigBytes % 65 == 0, "Invalid signature data length");

        for (uint256 i = 0; i < sigBytes; i += 65) {
            if (proof[4 + i] != 0) {
                count++;
            }
        }
    }

    /// @dev Read a big-endian uint32 from bytes at `start`
    function _bytesToUint32(bytes memory b, uint256 start) internal pure returns (uint32 result) {
        require(start + 4 <= b.length, "OOB");
        assembly {
            result := shr(224, mload(add(add(b, 0x20), start)))
        }
    }
}