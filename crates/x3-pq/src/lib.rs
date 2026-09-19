//! X3 Post-Quantum Cryptography Integration
//!
//! Post-quantum cryptographic primitives for X3 validator and account security.
//! Provides quantum-resistant algorithms for key generation, signing, and verification.
//!
//! **Status: research simulation.** Key generation, signing and verification are
//! placeholder implementations (zeroed keys, `blake2_256(message)` padded to the
//! nominal signature size). Verification therefore **fails closed**: it returns
//! [`PQError::VerificationUnavailable`] rather than reporting a "valid" signature,
//! because the check it used to perform ignored the public key entirely — any
//! caller could forge `blake2_256(message) || padding` and be accepted. Do not
//! treat this crate as a cryptographic root of trust; see issue #285.

pub mod types;

use sp_core::crypto::Pair as PairCrypto;
use sp_std::vec::Vec;

/// Post-quantum cryptography manager
pub struct PQManager {
    /// Current key pair
    keypair: PQKeyPair,
    /// Signature scheme (Dilithium, Falcon, etc.)
    scheme: PQScheme,
    /// Key rotation schedule
    rotation_schedule: KeyRotationSchedule,
}

impl PQManager {
    /// Create new PQ manager with specified scheme
    pub fn new(scheme: PQScheme) -> Result<Self, PQError> {
        let keypair = Self::generate_keypair(scheme)?;
        let rotation_schedule = KeyRotationSchedule::default();

        Ok(Self {
            keypair,
            scheme,
            rotation_schedule,
        })
    }

    /// Generate new post-quantum keypair
    pub fn generate_keypair(scheme: PQScheme) -> Result<PQKeyPair, PQError> {
        match scheme {
            PQScheme::Dilithium3 => Self::generate_dilithium_keypair(),
            PQScheme::Falcon512 => Self::generate_falcon_keypair(),
            PQScheme::Sphincs256 => Self::generate_sphincs_keypair(),
        }
    }

    /// Sign message with post-quantum algorithm
    pub fn sign(&self, message: &[u8]) -> Result<PQSignature, PQError> {
        match self.scheme {
            PQScheme::Dilithium3 => self.sign_dilithium(message),
            PQScheme::Falcon512 => self.sign_falcon(message),
            PQScheme::Sphincs256 => self.sign_sphincs(message),
        }
    }

    /// Verify post-quantum signature
    pub fn verify(
        &self,
        message: &[u8],
        signature: &PQSignature,
        public_key: &PQPublicKey,
    ) -> Result<bool, PQError> {
        match self.scheme {
            PQScheme::Dilithium3 => self.verify_dilithium(message, signature, public_key),
            PQScheme::Falcon512 => self.verify_falcon(message, signature, public_key),
            PQScheme::Sphincs256 => self.verify_sphincs(message, signature, public_key),
        }
    }

    /// Rotate to new keypair, if `current_block` is due.
    pub fn rotate_keys(&mut self, current_block: u64) -> Result<(), PQError> {
        if !self.rotation_schedule.should_rotate(current_block)? {
            return Ok(());
        }

        let new_keypair = Self::generate_keypair(self.scheme)?;
        self.keypair = new_keypair;
        self.rotation_schedule.record_rotation(current_block)?;

        Ok(())
    }

    /// Check if keys need rotation
    pub fn needs_rotation(&self, current_block: u64) -> Result<bool, PQError> {
        self.rotation_schedule.should_rotate(current_block)
    }

    /// Get current public key
    pub fn public_key(&self) -> &PQPublicKey {
        &self.keypair.public_key
    }

    // Dilithium implementation
    fn generate_dilithium_keypair() -> Result<PQKeyPair, PQError> {
        // Dilithium-3 key generation
        // In practice, this would use the pqclean_dilithium crate
        let public_key = PQPublicKey(vec![0u8; 1952]); // Dilithium-3 public key size
        let private_key = PQPrivateKey(vec![0u8; 4016]); // Dilithium-3 private key size

        Ok(PQKeyPair {
            public_key,
            private_key,
        })
    }

    fn sign_dilithium(&self, message: &[u8]) -> Result<PQSignature, PQError> {
        // Dilithium signing
        // Hash message first
        let message_hash = sp_io::hashing::blake2_256(message);

        // Generate signature (simplified)
        let mut signature = vec![0u8; 3293]; // Dilithium-3 signature size

        // In practice: use pqclean_dilithium::sign
        // For now, simulate with deterministic signature based on message
        signature[0..32].copy_from_slice(&message_hash);

        Ok(PQSignature(signature))
    }

    fn verify_dilithium(
        &self,
        message: &[u8],
        signature: &PQSignature,
        public_key: &PQPublicKey,
    ) -> Result<bool, PQError> {
        // The placeholder that used to live here compared
        // `blake2_256(message)` with the first 32 bytes of the signature and never
        // touched `public_key`, so any caller could "sign" without a private key.
        // Refusing is the only honest answer until a real Dilithium verifier lands.
        let _ = (message, signature, public_key);
        Err(PQError::VerificationUnavailable)
    }

    // Falcon implementation
    fn generate_falcon_keypair() -> Result<PQKeyPair, PQError> {
        // Falcon-512 key generation
        let public_key = PQPublicKey(vec![0u8; 897]); // Falcon-512 public key size
        let private_key = PQPrivateKey(vec![0u8; 1281]); // Falcon-512 private key size

        Ok(PQKeyPair {
            public_key,
            private_key,
        })
    }

    fn sign_falcon(&self, message: &[u8]) -> Result<PQSignature, PQError> {
        // Falcon signing
        let message_hash = sp_io::hashing::blake2_256(message);
        let mut signature = vec![0u8; 666]; // Falcon-512 signature size
        signature[0..32].copy_from_slice(&message_hash);

        Ok(PQSignature(signature))
    }

    fn verify_falcon(
        &self,
        message: &[u8],
        signature: &PQSignature,
        public_key: &PQPublicKey,
    ) -> Result<bool, PQError> {
        // Same placeholder shape as Dilithium: signature bytes compared against a
        // message hash, public key ignored. Fails closed (issue #285).
        let _ = (message, signature, public_key);
        Err(PQError::VerificationUnavailable)
    }

    // Sphincs+ implementation
    fn generate_sphincs_keypair() -> Result<PQKeyPair, PQError> {
        // Sphincs+-256 key generation
        let public_key = PQPublicKey(vec![0u8; 64]); // Sphincs+ public key size
        let private_key = PQPrivateKey(vec![0u8; 128]); // Sphincs+ private key size

        Ok(PQKeyPair {
            public_key,
            private_key,
        })
    }

    fn sign_sphincs(&self, message: &[u8]) -> Result<PQSignature, PQError> {
        // Sphincs+ signing
        let message_hash = sp_io::hashing::blake2_256(message);
        let mut signature = vec![0u8; 29792]; // Sphincs+-256 signature size
        signature[0..32].copy_from_slice(&message_hash);

        Ok(PQSignature(signature))
    }

    fn verify_sphincs(
        &self,
        message: &[u8],
        signature: &PQSignature,
        public_key: &PQPublicKey,
    ) -> Result<bool, PQError> {
        // Same placeholder shape as Dilithium: signature bytes compared against a
        // message hash, public key ignored. Fails closed (issue #285).
        let _ = (message, signature, public_key);
        Err(PQError::VerificationUnavailable)
    }
}

/// Hybrid classical + post-quantum signature scheme
pub struct HybridSigner {
    /// Classical signature (for backwards compatibility)
    classical_key: sp_core::sr25519::Pair,
    /// Post-quantum signature
    pq_manager: PQManager,
}

impl HybridSigner {
    /// Create hybrid signer
    pub fn new(pq_scheme: PQScheme) -> Result<Self, PQError> {
        let classical_key = sp_core::sr25519::Pair::from_string("//Alice", None)
            .map_err(|_| PQError::KeyGenerationFailed)?;
        let pq_manager = PQManager::new(pq_scheme)?;

        Ok(Self {
            classical_key,
            pq_manager,
        })
    }

    /// Sign with both classical and PQ algorithms
    pub fn sign_hybrid(&self, message: &[u8]) -> Result<HybridSignature, PQError> {
        let classical_sig =
            <sp_core::sr25519::Pair as PairCrypto>::sign(&self.classical_key, message);
        let pq_sig = self.pq_manager.sign(message)?;

        Ok(HybridSignature {
            classical: classical_sig,
            post_quantum: pq_sig,
        })
    }

    /// Verify hybrid signature
    pub fn verify_hybrid(
        &self,
        message: &[u8],
        signature: &HybridSignature,
        classical_pk: &sp_core::sr25519::Public,
        pq_pk: &PQPublicKey,
    ) -> Result<bool, PQError> {
        // Verify classical signature
        let classical_valid = <sp_core::sr25519::Pair as PairCrypto>::verify(
            &signature.classical,
            message,
            classical_pk,
        );

        // Verify PQ signature
        let pq_valid = self
            .pq_manager
            .verify(message, &signature.post_quantum, pq_pk)?;

        Ok(classical_valid && pq_valid)
    }
}

/// Key rotation schedule for PQ keys
pub struct KeyRotationSchedule {
    /// Last rotation timestamp
    last_rotation: u64,
    /// Rotation interval (in blocks)
    rotation_interval: u64,
}

impl KeyRotationSchedule {
    /// Check whether rotation is due at `current_block`.
    ///
    /// The block number is a parameter, the way `node/src/authority.rs` already
    /// does it for session keys. This used to read `b"current_block"` out of
    /// `sp_io` storage: a key no runtime writes, which panicked with
    /// `get_version_1 called outside of an Externalities-provided environment`
    /// anywhere but inside a runtime, and whose subtraction panicked on
    /// underflow when a schedule was created ahead of the chain.
    pub fn should_rotate(&self, current_block: u64) -> Result<bool, PQError> {
        Ok(current_block.saturating_sub(self.last_rotation) >= self.rotation_interval)
    }

    /// Record that the keys were rotated at `current_block`.
    ///
    /// The schedule is in-memory state. The old version also wrote
    /// `b"pq_last_rotation"` straight into `sp_io` storage, outside any pallet's
    /// storage prefix; persistence belongs to whoever owns the key material.
    pub fn record_rotation(&mut self, current_block: u64) -> Result<(), PQError> {
        self.last_rotation = current_block;
        Ok(())
    }
}

impl Default for KeyRotationSchedule {
    /// A schedule that rotates every 100,000 blocks.
    fn default() -> Self {
        Self {
            last_rotation: 0,
            rotation_interval: 100_000,
        }
    }
}

/// Integration with validator identity
pub struct PQValidatorIdentity {
    /// Validator ID
    validator_id: u64,
    /// PQ key manager
    pq_manager: PQManager,
    /// Hybrid signer
    hybrid_signer: HybridSigner,
}

impl PQValidatorIdentity {
    /// The validator this identity belongs to.
    pub fn validator_id(&self) -> u64 {
        self.validator_id
    }

    /// Create new validator identity with PQ keys
    pub fn new(validator_id: u64, pq_scheme: PQScheme) -> Result<Self, PQError> {
        let pq_manager = PQManager::new(pq_scheme)?;
        let hybrid_signer = HybridSigner::new(pq_scheme)?;

        Ok(Self {
            validator_id,
            pq_manager,
            hybrid_signer,
        })
    }

    /// Sign validator message with hybrid signature
    pub fn sign_validator_message(&self, message: &[u8]) -> Result<HybridSignature, PQError> {
        self.hybrid_signer.sign_hybrid(message)
    }

    /// Rotate validator keys if needed
    pub fn rotate_keys_if_needed(&mut self, current_block: u64) -> Result<bool, PQError> {
        if self.pq_manager.needs_rotation(current_block)? {
            self.pq_manager.rotate_keys(current_block)?;
            // Update hybrid signer with new keys
            self.hybrid_signer = HybridSigner::new(self.pq_manager.scheme)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// Account-level PQ integration
pub struct PQAccountManager {
    /// Account address
    account: sp_core::H160,
    /// PQ key manager
    pq_manager: Option<PQManager>,
    /// Hybrid mode enabled
    hybrid_enabled: bool,
}

impl PQAccountManager {
    /// The account these PQ keys protect.
    pub fn account(&self) -> sp_core::H160 {
        self.account
    }

    /// Enable PQ protection for account
    pub fn enable_pq(&mut self, scheme: PQScheme) -> Result<(), PQError> {
        self.pq_manager = Some(PQManager::new(scheme)?);
        self.hybrid_enabled = true;
        Ok(())
    }

    /// Sign transaction with PQ if enabled
    pub fn sign_transaction(&self, tx: &[u8]) -> Result<Option<PQSignature>, PQError> {
        if let Some(ref manager) = self.pq_manager {
            manager.sign(tx).map(Some)
        } else {
            Ok(None)
        }
    }

    /// Verify PQ signature if present
    pub fn verify_transaction_signature(
        &self,
        tx: &[u8],
        signature: &PQSignature,
        public_key: &PQPublicKey,
    ) -> Result<bool, PQError> {
        if let Some(ref manager) = self.pq_manager {
            manager.verify(tx, signature, public_key)
        } else {
            Ok(false)
        }
    }
}

// Type definitions
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PQKeyPair {
    pub public_key: PQPublicKey,
    pub private_key: PQPrivateKey,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PQPublicKey(pub Vec<u8>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PQPrivateKey(pub Vec<u8>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PQSignature(pub Vec<u8>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HybridSignature {
    pub classical: sp_core::sr25519::Signature,
    pub post_quantum: PQSignature,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PQScheme {
    /// CRYSTALS-Dilithium3 (NIST Round 3 finalist)
    Dilithium3,
    /// Falcon-512 (NIST Round 3 finalist)
    Falcon512,
    /// Sphincs+-256 (NIST Round 3 finalist)
    Sphincs256,
}

#[derive(Debug, thiserror::Error)]
pub enum PQError {
    #[error("Key generation failed")]
    KeyGenerationFailed,
    #[error("Signing failed")]
    SigningFailed,
    #[error("Verification failed")]
    VerificationFailed,
    /// No real post-quantum verifier exists in this crate yet, so every
    /// verification request is refused rather than answered by a placeholder.
    #[error("post-quantum verification is not implemented; refusing to verify")]
    VerificationUnavailable,
    #[error("Invalid key")]
    InvalidKey,
    #[error("Rotation failed")]
    RotationFailed,
    #[error("Storage error")]
    StorageError,
    #[error("Codec error")]
    CodecError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pq_manager_creation() {
        let manager = PQManager::new(PQScheme::Dilithium3).unwrap();
        assert_eq!(manager.scheme, PQScheme::Dilithium3);
    }

    #[test]
    fn test_dilithium_keypair_generation() {
        let keypair = PQManager::generate_keypair(PQScheme::Dilithium3).unwrap();
        assert_eq!(keypair.public_key.0.len(), 1952); // Dilithium-3 public key size
        assert_eq!(keypair.private_key.0.len(), 4016); // Dilithium-3 private key size
    }

    #[test]
    fn test_dilithium_sign_verify() {
        let manager = PQManager::new(PQScheme::Dilithium3).unwrap();
        let message = b"Hello, quantum world!";

        let signature = manager.sign(message).unwrap();
        let public_key = manager.public_key();

        // Signing is a placeholder (`blake2_256(message)` padded to the nominal
        // signature size), so nothing here can check a signature. Verification
        // refuses instead of answering `true` for a check it never performed.
        // See issue #285.
        assert!(matches!(
            manager.verify(message, &signature, public_key),
            Err(PQError::VerificationUnavailable)
        ));
    }

    #[test]
    fn test_hybrid_signer_creation() {
        let hybrid = HybridSigner::new(PQScheme::Dilithium3).unwrap();
        assert_eq!(hybrid.pq_manager.scheme, PQScheme::Dilithium3);
    }

    #[test]
    fn test_key_rotation_schedule() {
        let schedule = KeyRotationSchedule::default();
        assert_eq!(schedule.rotation_interval, 100_000);
        assert_eq!(schedule.last_rotation, 0);
    }

    #[test]
    fn test_pq_validator_identity() {
        let identity = PQValidatorIdentity::new(1, PQScheme::Falcon512).unwrap();
        assert_eq!(identity.validator_id, 1);
        assert_eq!(identity.pq_manager.scheme, PQScheme::Falcon512);
    }

    #[test]
    fn test_pq_account_manager() {
        let mut manager = PQAccountManager {
            account: sp_core::H160::default(),
            pq_manager: None,
            hybrid_enabled: false,
        };

        manager.enable_pq(PQScheme::Sphincs256).unwrap();
        assert!(manager.pq_manager.is_some());
        assert!(manager.hybrid_enabled);
    }

    #[test]
    fn test_pq_scheme_enum() {
        assert_eq!(PQScheme::Dilithium3 as u8, 0);
        assert_eq!(PQScheme::Falcon512 as u8, 1);
        assert_eq!(PQScheme::Sphincs256 as u8, 2);
    }

    #[test]
    fn test_falcon_keypair_generation() {
        let keypair = PQManager::generate_keypair(PQScheme::Falcon512).unwrap();
        assert_eq!(keypair.public_key.0.len(), 897); // Falcon-512 public key size
        assert_eq!(keypair.private_key.0.len(), 1281); // Falcon-512 private key size
    }

    #[test]
    fn test_falcon_sign_verify() {
        let manager = PQManager::new(PQScheme::Falcon512).unwrap();
        let message = b"Falcon test message";

        let signature = manager.sign(message).unwrap();
        let public_key = manager.public_key();

        assert!(matches!(
            manager.verify(message, &signature, public_key),
            Err(PQError::VerificationUnavailable)
        ));
    }

    #[test]
    fn test_sphincs_keypair_generation() {
        let keypair = PQManager::generate_keypair(PQScheme::Sphincs256).unwrap();
        assert_eq!(keypair.public_key.0.len(), 64); // Sphincs+ public key size
        assert_eq!(keypair.private_key.0.len(), 128); // Sphincs+ private key size
    }

    #[test]
    fn test_sphincs_sign_verify() {
        let manager = PQManager::new(PQScheme::Sphincs256).unwrap();
        let message = b"Sphincs test message";

        let signature = manager.sign(message).unwrap();
        let public_key = manager.public_key();

        assert!(matches!(
            manager.verify(message, &signature, public_key),
            Err(PQError::VerificationUnavailable)
        ));
    }

    #[test]
    fn test_signature_verification_fails_wrong_message() {
        let manager = PQManager::new(PQScheme::Dilithium3).unwrap();
        let message1 = b"Message 1";
        let message2 = b"Message 2";

        let signature = manager.sign(message1).unwrap();
        let public_key = manager.public_key();

        // A different message is refused — and so is the message that was
        // actually signed. The refusal is unconditional, which is the only
        // honest answer while the verifier is a placeholder: a verifier that
        // returned `true` for the second call would be accepting
        // `blake2_256(message) || padding` as a signature.
        assert!(matches!(
            manager.verify(message2, &signature, public_key),
            Err(PQError::VerificationUnavailable)
        ));
        assert!(matches!(
            manager.verify(message1, &signature, public_key),
            Err(PQError::VerificationUnavailable)
        ));
    }

    #[test]
    fn test_signature_verification_fails_wrong_key() {
        let manager1 = PQManager::new(PQScheme::Dilithium3).unwrap();
        let manager2 = PQManager::new(PQScheme::Dilithium3).unwrap();
        let message = b"Test message";

        let signature = manager1.sign(message).unwrap();
        let wrong_public_key = manager2.public_key();

        assert!(matches!(
            manager1.verify(message, &signature, wrong_public_key),
            Err(PQError::VerificationUnavailable)
        ));
        assert!(matches!(
            manager1.verify(message, &signature, manager1.public_key()),
            Err(PQError::VerificationUnavailable)
        ));
    }

    #[test]
    fn test_hybrid_signature_creation() {
        let hybrid = HybridSigner::new(PQScheme::Dilithium3).unwrap();
        let message = b"Hybrid test";

        let hybrid_sig = hybrid.sign_hybrid(message).unwrap();

        // Check that both signatures are present
        assert_eq!(hybrid_sig.classical.0.len(), 64); // sr25519 signature size
        assert_eq!(hybrid_sig.post_quantum.0.len(), 3293); // Dilithium-3 signature size
    }

    #[test]
    fn test_hybrid_signature_verification() {
        let hybrid = HybridSigner::new(PQScheme::Dilithium3).unwrap();
        let message = b"Hybrid verification test";

        let hybrid_sig = hybrid.sign_hybrid(message).unwrap();

        let classical_pk = hybrid.classical_key.public();
        let pq_pk = hybrid.pq_manager.public_key();

        // The classical half would verify — sr25519 is real — but the
        // post-quantum half cannot, and a hybrid signature is only valid when
        // both halves are. The call refuses rather than reporting the
        // classical half's answer as the hybrid one.
        assert!(matches!(
            hybrid.verify_hybrid(message, &hybrid_sig, &classical_pk, pq_pk),
            Err(PQError::VerificationUnavailable)
        ));
    }

    #[test]
    fn test_hybrid_verification_fails_wrong_classical_key() {
        let hybrid1 = HybridSigner::new(PQScheme::Dilithium3).unwrap();
        let hybrid2 = HybridSigner::new(PQScheme::Dilithium3).unwrap();
        let message = b"Test";

        let hybrid_sig = hybrid1.sign_hybrid(message).unwrap();

        let wrong_classical_pk = hybrid2.classical_key.public();
        let pq_pk = hybrid1.pq_manager.public_key();

        assert!(matches!(
            hybrid1.verify_hybrid(message, &hybrid_sig, &wrong_classical_pk, pq_pk),
            Err(PQError::VerificationUnavailable)
        ));
    }

    #[test]
    fn test_hybrid_verification_fails_wrong_pq_key() {
        let hybrid1 = HybridSigner::new(PQScheme::Dilithium3).unwrap();
        let hybrid2 = HybridSigner::new(PQScheme::Dilithium3).unwrap();
        let message = b"Test";

        let hybrid_sig = hybrid1.sign_hybrid(message).unwrap();

        let classical_pk = hybrid1.classical_key.public();
        let wrong_pq_pk = hybrid2.pq_manager.public_key();

        assert!(matches!(
            hybrid1.verify_hybrid(message, &hybrid_sig, &classical_pk, wrong_pq_pk),
            Err(PQError::VerificationUnavailable)
        ));
    }

    #[test]
    fn test_key_rotation_schedule_should_rotate() {
        let schedule = KeyRotationSchedule {
            last_rotation: 0,
            rotation_interval: 1000,
        };

        assert!(!schedule.should_rotate(0).unwrap());
        assert!(!schedule.should_rotate(999).unwrap());
        assert!(schedule.should_rotate(1_000).unwrap());
        assert!(schedule.should_rotate(5_000).unwrap());
    }

    #[test]
    fn test_key_rotation_schedule_record_rotation() {
        let mut schedule = KeyRotationSchedule {
            last_rotation: 0,
            rotation_interval: 1000,
        };

        schedule.record_rotation(2_000).unwrap();

        assert_eq!(schedule.last_rotation, 2_000);
        assert!(!schedule.should_rotate(2_999).unwrap());
        assert!(schedule.should_rotate(3_000).unwrap());
        // A chain that is behind the recorded rotation must not underflow: this
        // is the subtraction that used to panic.
        assert!(!schedule.should_rotate(10).unwrap());
    }

    #[test]
    fn test_pq_manager_key_rotation() {
        let mut manager = PQManager::new(PQScheme::Dilithium3).unwrap();
        let original_key = manager.public_key().clone();

        // Not due yet: the default interval is 100_000 blocks.
        manager.rotate_keys(0).unwrap();
        assert_eq!(original_key.0, manager.public_key().0);

        manager.rotate_keys(100_000).unwrap();

        let new_key = manager.public_key();
        // The rotation bookkeeping advanced ...
        assert_eq!(manager.rotation_schedule.last_rotation, 100_000);
        // ... but the key material did not change, because key generation is a
        // placeholder that returns zeroed buffers of the nominal size (issue
        // #285). The second assertion pins that: the day real key generation
        // lands, it fails, and this test must assert `assert_ne!` instead.
        assert_eq!(original_key.0, new_key.0);
        assert!(
            new_key.0.iter().all(|byte| *byte == 0),
            "key generation is still a placeholder: keys must be zeroed"
        );
    }

    #[test]
    fn test_pq_manager_needs_rotation() {
        let manager = PQManager::new(PQScheme::Dilithium3).unwrap();

        assert!(!manager.needs_rotation(0).unwrap());
        assert!(!manager.needs_rotation(99_999).unwrap());
        assert!(manager.needs_rotation(100_000).unwrap());
    }

    #[test]
    fn test_pq_validator_identity_creation() {
        let identity = PQValidatorIdentity::new(42, PQScheme::Falcon512).unwrap();

        assert_eq!(identity.validator_id, 42);
        assert_eq!(identity.pq_manager.scheme, PQScheme::Falcon512);
        // The rotation counter lives on the key manager's schedule, not on the
        // identity.
        assert_eq!(identity.pq_manager.rotation_schedule.last_rotation, 0);
    }

    #[test]
    fn test_pq_validator_identity_sign_message() {
        let identity = PQValidatorIdentity::new(1, PQScheme::Dilithium3).unwrap();
        let message = b"Validator message";

        let signature = identity.sign_validator_message(message).unwrap();

        // Should have both classical and PQ signatures
        assert_eq!(signature.classical.0.len(), 64);
        assert!(!signature.post_quantum.0.is_empty());
    }

    #[test]
    fn test_pq_validator_identity_key_rotation() {
        let mut identity = PQValidatorIdentity::new(1, PQScheme::Dilithium3).unwrap();

        // Not due yet, so the identity keeps its keys ...
        assert!(!identity.rotate_keys_if_needed(0).unwrap());
        assert_eq!(identity.pq_manager.rotation_schedule.last_rotation, 0);

        // ... and rotates once the default interval has elapsed.
        assert!(identity.rotate_keys_if_needed(100_000).unwrap());
        assert_eq!(identity.pq_manager.rotation_schedule.last_rotation, 100_000);
        // The hybrid signer is rebuilt around the rotated keys, so it still
        // signs.
        assert!(identity.sign_validator_message(b"after rotation").is_ok());
    }

    #[test]
    fn test_pq_account_manager_initial_state() {
        let manager = PQAccountManager {
            account: sp_core::H160::from_low_u64_be(123),
            pq_manager: None,
            hybrid_enabled: false,
        };

        assert_eq!(manager.account, sp_core::H160::from_low_u64_be(123));
        assert!(manager.pq_manager.is_none());
        assert!(!manager.hybrid_enabled);
    }

    #[test]
    fn test_pq_account_manager_enable_different_schemes() {
        let mut manager = PQAccountManager {
            account: sp_core::H160::default(),
            pq_manager: None,
            hybrid_enabled: false,
        };

        // Test Dilithium
        manager.enable_pq(PQScheme::Dilithium3).unwrap();
        assert!(manager.pq_manager.is_some());
        assert!(manager.hybrid_enabled);
        assert_eq!(
            manager.pq_manager.as_ref().unwrap().scheme,
            PQScheme::Dilithium3
        );

        // Test changing scheme
        manager.enable_pq(PQScheme::Falcon512).unwrap();
        assert_eq!(
            manager.pq_manager.as_ref().unwrap().scheme,
            PQScheme::Falcon512
        );
    }

    #[test]
    fn test_pq_account_manager_sign_transaction() {
        let mut manager = PQAccountManager {
            account: sp_core::H160::default(),
            pq_manager: None,
            hybrid_enabled: false,
        };

        // Without PQ enabled
        assert!(manager.sign_transaction(b"test tx").unwrap().is_none());

        // With PQ enabled
        manager.enable_pq(PQScheme::Dilithium3).unwrap();
        let signature = manager
            .sign_transaction(b"test tx")
            .unwrap()
            .expect("a PQ-enabled account signs");
        assert_eq!(signature.0.len(), 3293);
    }

    #[test]
    fn test_pq_account_manager_verify_transaction() {
        let mut manager = PQAccountManager {
            account: sp_core::H160::default(),
            pq_manager: None,
            hybrid_enabled: false,
        };

        manager.enable_pq(PQScheme::Dilithium3).unwrap();

        let tx = b"test transaction";
        let signature = manager.sign_transaction(tx).unwrap().unwrap();
        let public_key = manager.pq_manager.as_ref().unwrap().public_key().clone();

        assert!(matches!(
            manager.verify_transaction_signature(tx, &signature, &public_key),
            Err(PQError::VerificationUnavailable)
        ));
    }

    #[test]
    fn test_pq_account_manager_verify_wrong_signature() {
        let mut manager = PQAccountManager {
            account: sp_core::H160::default(),
            pq_manager: None,
            hybrid_enabled: false,
        };

        manager.enable_pq(PQScheme::Dilithium3).unwrap();

        let tx = b"test transaction";
        let wrong_sig = PQSignature(vec![0; 3293]); // Zero signature
        let public_key = manager.pq_manager.as_ref().unwrap().public_key().clone();

        // A zero signature and a real one are both refused: the account-level
        // verifier has no post-quantum verifier behind it either.
        assert!(matches!(
            manager.verify_transaction_signature(tx, &wrong_sig, &public_key),
            Err(PQError::VerificationUnavailable)
        ));
    }

    #[test]
    fn test_pq_error_types() {
        let err = PQError::KeyGenerationFailed;
        assert!(matches!(err, PQError::KeyGenerationFailed));

        let err = PQError::SigningFailed;
        assert!(matches!(err, PQError::SigningFailed));

        let err = PQError::VerificationFailed;
        assert!(matches!(err, PQError::VerificationFailed));

        let err = PQError::InvalidKey;
        assert!(matches!(err, PQError::InvalidKey));

        let err = PQError::RotationFailed;
        assert!(matches!(err, PQError::RotationFailed));

        let err = PQError::StorageError;
        assert!(matches!(err, PQError::StorageError));

        let err = PQError::CodecError;
        assert!(matches!(err, PQError::CodecError));
    }

    #[test]
    fn test_pq_keypair_struct() {
        let public_key = PQPublicKey(vec![1, 2, 3, 4]);
        let private_key = PQPrivateKey(vec![5, 6, 7, 8]);

        let keypair = PQKeyPair {
            public_key,
            private_key,
        };

        assert_eq!(keypair.public_key.0, vec![1, 2, 3, 4]);
        assert_eq!(keypair.private_key.0, vec![5, 6, 7, 8]);
    }

    #[test]
    fn test_pq_signature_struct() {
        let data = vec![1, 2, 3, 4, 5];
        let signature = PQSignature(data.clone());

        assert_eq!(signature.0, data);
    }

    #[test]
    fn test_hybrid_signature_struct() {
        let classical = sp_core::sr25519::Signature::from_raw([42; 64]);
        let post_quantum = PQSignature(vec![1, 2, 3]);

        let hybrid = HybridSignature {
            classical,
            post_quantum,
        };

        assert_eq!(hybrid.classical.0, [42; 64]);
        assert_eq!(hybrid.post_quantum.0, vec![1, 2, 3]);
    }

    #[test]
    fn test_key_rotation_schedule_struct() {
        let schedule = KeyRotationSchedule {
            last_rotation: 12345,
            rotation_interval: 67890,
        };

        assert_eq!(schedule.last_rotation, 12345);
        assert_eq!(schedule.rotation_interval, 67890);
    }

    #[test]
    fn test_pq_validator_identity_struct() {
        // The identity has no `pq_public_key` / `last_rotation` fields: it owns
        // a key manager and a hybrid signer instead. Assert the surface that
        // exists.
        let identity = PQValidatorIdentity::new(42, PQScheme::Sphincs256).unwrap();

        assert_eq!(identity.validator_id, 42);
        assert_eq!(identity.pq_manager.scheme, PQScheme::Sphincs256);
        assert_eq!(identity.pq_manager.public_key().0.len(), 64);
        assert_eq!(
            identity.hybrid_signer.pq_manager.scheme,
            PQScheme::Sphincs256
        );
    }

    #[test]
    fn test_pq_account_manager_struct() {
        // This replaces a test of `PQAccountConfig`, a type that does not exist
        // in this crate (it has never compiled against one).
        let account = sp_core::H160::from_low_u64_be(12345);
        let mut manager = PQAccountManager {
            account,
            pq_manager: None,
            hybrid_enabled: false,
        };

        assert_eq!(manager.account, account);
        assert!(manager.pq_manager.is_none());
        assert!(!manager.hybrid_enabled);

        manager.enable_pq(PQScheme::Falcon512).unwrap();
        assert_eq!(
            manager.pq_manager.as_ref().unwrap().scheme,
            PQScheme::Falcon512
        );
        assert!(manager.hybrid_enabled);
    }

    #[test]
    fn test_all_pq_schemes_have_different_sizes() {
        let dilithium = PQManager::generate_keypair(PQScheme::Dilithium3).unwrap();
        let falcon = PQManager::generate_keypair(PQScheme::Falcon512).unwrap();
        let sphincs = PQManager::generate_keypair(PQScheme::Sphincs256).unwrap();

        // All should have different public key sizes
        assert_ne!(dilithium.public_key.0.len(), falcon.public_key.0.len());
        assert_ne!(falcon.public_key.0.len(), sphincs.public_key.0.len());
        assert_ne!(dilithium.public_key.0.len(), sphincs.public_key.0.len());
    }

    #[test]
    fn test_pq_signatures_have_correct_sizes() {
        let dilithium = PQManager::new(PQScheme::Dilithium3).unwrap();
        let falcon = PQManager::new(PQScheme::Falcon512).unwrap();
        let sphincs = PQManager::new(PQScheme::Sphincs256).unwrap();

        let message = b"test message";

        let dilithium_sig = dilithium.sign(message).unwrap();
        let falcon_sig = falcon.sign(message).unwrap();
        let sphincs_sig = sphincs.sign(message).unwrap();

        // Check signature sizes match expected
        assert_eq!(dilithium_sig.0.len(), 3293); // Dilithium-3
        assert_eq!(falcon_sig.0.len(), 666); // Falcon-512
        assert_eq!(sphincs_sig.0.len(), 29792); // Sphincs+-256
    }
}
