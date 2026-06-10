// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @title IX3Verification — proof verification interface for X3 cross-chain gateway
/// @notice Deployed on external EVM chains to verify X3 finalized proofs
interface IX3Verification {
    /// @notice Verify an X3 finalized message inclusion proof
    /// @param messageId The keccak256 message identifier
    /// @param sourceChain The X3 chain ID
    /// @param sender The sender on X3 side
    /// @param recipient The recipient on this chain
    /// @param amount The transfer amount
    /// @param proof The X3 finalized block proof (validator signatures or light client proof)
    /// @return verified True if the proof is valid
    function verifyX3WithdrawalProof(
        bytes32 messageId,
        uint256 sourceChain,
        bytes calldata sender,
        address recipient,
        uint256 amount,
        bytes calldata proof
    ) external view returns (bool verified);

    /// @notice Verify a deposit proof from this chain to X3
    /// @param messageId The deposit message identifier
    /// @param token The ERC20 token address
    /// @param depositor The account that deposited tokens
    /// @param x3Recipient The recipient on X3
    /// @param amount The deposit amount
    /// @param proof The deposit proof (receipt or event proof)
    /// @return verified True if the proof is valid
    function verifyDepositProof(
        bytes32 messageId,
        address token,
        address depositor,
        bytes calldata x3Recipient,
        uint256 amount,
        bytes calldata proof
    ) external view returns (bool verified);

    /// @notice Get the current verifier set ID (for rotation tracking)
    function verifierSetId() external view returns (uint256);
}