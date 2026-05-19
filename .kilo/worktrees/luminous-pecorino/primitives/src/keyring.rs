//! Keyring verification primitives and traits.

use sp_runtime::DispatchResult;

/// A trait for verifying keyring attestations on-chain.
///
/// This trait provides an abstraction over the keyring verification process,
/// allowing the runtime to dispatch keyring verification to the appropriate
/// pallet implementation.
pub trait KeyringVerifier<AccountId, BlockNumber> {
    /// Verify that a keyring has been properly attested by a quorum of registered
    /// attestors. Returns `true` if the keyring is verified, `false` otherwise.
    fn verify_keyring(keyring_id: &[u8; 32]) -> bool;

    /// Check whether a specific account is registered as a keyring attestor.
    fn is_attestor(account: &AccountId) -> bool;

    /// Get the number of active attestors currently registered.
    fn active_attestor_count() -> u32;
}

/// A trait for registering and managing keyring attestors.
pub trait KeyringAttestorRegistry<AccountId, Balance, BlockNumber> {
    /// Register a new attestor with the given stake.
    fn register_attestor(account: &AccountId, stake: Balance) -> DispatchResult;

    /// Deregister an attestor, returning their stake.
    fn deregister_attestor(account: &AccountId) -> DispatchResult;

    /// Check if an account is a registered attestor.
    fn is_registered(account: &AccountId) -> bool;

    /// Get the total number of registered attestors.
    fn total_attestors() -> u64;
}

/// A trait for recovering a public key from a compact ECDSA signature.
///
/// This trait provides a standard interface for on-chain signature verification
/// using the secp256k1 ECDSA recoverable signature scheme (Compact format: 64-byte
/// signature + 27/28 recovery byte).
///
/// The `verify_from_compact` method takes:
/// - `signature`: A 65-byte recoverable ECDSA signature (compact format: r||s||v)
/// - `message`: The 32-byte keccak256 hash of the signed message
/// - `expected`: The expected 33-byte compressed public key
///
/// Returns `true` if the recovered public key matches the expected key.
pub trait VerifyFromCompact<Signature, Message, Public> {
    /// Verify a compact ECDSA signature against an expected public key.
    ///
    /// # Arguments
    /// * `signature` - A 65-byte recoverable ECDSA signature in compact format (r||s||v)
    /// * `message` - The 32-byte keccak256 hash of the original signed message
    /// * `expected` - The expected 33-byte compressed secp256k1 public key
    ///
    /// # Returns
    /// `true` if signature is valid and recovers to the expected public key, `false` otherwise.
    fn verify_from_compact(signature: &Signature, message: &Message, expected: &Public) -> bool;
}