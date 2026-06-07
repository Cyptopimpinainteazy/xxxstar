//! Identity management — ephemeral and persistent identities.

use sha2::{Digest, Sha256};
use x3_proof::types::AgentIdentity;

/// Identity manager — creates and validates agent identities.
pub struct IdentityManager;

impl IdentityManager {
    /// Create a persistent identity from a 32-byte public key.
    pub fn persistent(pubkey: [u8; 32]) -> AgentIdentity {
        AgentIdentity {
            pubkey,
            ephemeral: false,
        }
    }

    /// Create an ephemeral identity from a 32-byte public key.
    pub fn ephemeral(pubkey: [u8; 32]) -> AgentIdentity {
        AgentIdentity {
            pubkey,
            ephemeral: true,
        }
    }

    /// Derive an ephemeral key from a primary key and a nonce.
    /// The derivation is deterministic: same inputs always produce same output.
    pub fn derive_ephemeral(primary_pubkey: &[u8; 32], nonce: u64) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"x3-ephemeral-v1");
        hasher.update(primary_pubkey);
        hasher.update(&nonce.to_le_bytes());
        let result = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&result);
        key
    }

    /// Verify that an ephemeral key was derived from a given primary key and nonce.
    pub fn verify_ephemeral_derivation(
        primary_pubkey: &[u8; 32],
        ephemeral_pubkey: &[u8; 32],
        nonce: u64,
    ) -> bool {
        let expected = Self::derive_ephemeral(primary_pubkey, nonce);
        expected == *ephemeral_pubkey
    }

    /// Compute the canonical hash of an identity.
    pub fn identity_hash(identity: &AgentIdentity) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"x3-identity-v1");
        hasher.update(&identity.pubkey);
        hasher.update(&[identity.ephemeral as u8]);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ephemeral_derivation() {
        let primary = [1u8; 32];
        let eph = IdentityManager::derive_ephemeral(&primary, 42);
        assert!(IdentityManager::verify_ephemeral_derivation(
            &primary, &eph, 42
        ));
        assert!(!IdentityManager::verify_ephemeral_derivation(
            &primary, &eph, 43
        ));
    }

    #[test]
    fn test_different_keys_different_derivations() {
        let k1 = [1u8; 32];
        let k2 = [2u8; 32];
        let e1 = IdentityManager::derive_ephemeral(&k1, 0);
        let e2 = IdentityManager::derive_ephemeral(&k2, 0);
        assert_ne!(e1, e2);
    }
}
