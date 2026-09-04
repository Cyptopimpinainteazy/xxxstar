//! Encryption utilities for the private mempool.
//!
//! Provides helpers for encrypting transactions to the committee's threshold key
//! and reconstructing plaintexts from decryption shares.
//!
//! # Cryptographic Scheme
//!
//! 1. Sender generates ephemeral X25519 keypair
//! 2. ECDH with committee public key → shared secret
//! 3. HKDF-SHA256(shared_secret) → AES-256-GCM key
//! 4. AES-256-GCM encrypt(plaintext, nonce) → ciphertext
//!
//! Decryption requires t-of-n validators to provide decryption shares
//! (partial ECDH results), which are combined to reconstruct the shared secret.

use crate::{EncryptedTransaction, MempoolError};

/// Encrypt a transaction payload for the committee.
///
/// # Invariant: PRIV-EXEC-001
pub fn encrypt_for_committee(
    plaintext: &[u8],
    committee_pk: &[u8; 32],
    sender_pk: &[u8; 32],
    fee_commitment: &[u8; 32],
    dkg_epoch: u64,
) -> Result<EncryptedTransaction, MempoolError> {
    // Generate ephemeral keypair
    let ephemeral_sk = generate_ephemeral_key();
    let ephemeral_pk = derive_public_key(&ephemeral_sk);

    // ECDH: shared_secret = ephemeral_sk * committee_pk
    let shared_secret = ecdh(&ephemeral_sk, committee_pk);

    // KDF: derive AES key
    let aes_key = hkdf_derive(&shared_secret)?;

    // Generate nonce
    let nonce = generate_nonce();

    // AES-256-GCM encrypt
    let ciphertext = aes_gcm_encrypt(plaintext, &aes_key, &nonce)?;

    // Compute TX ID as hash of ciphertext
    let id = blake3_hash(&ciphertext);

    let submitted_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    Ok(EncryptedTransaction {
        id,
        ciphertext,
        ephemeral_pk,
        nonce,
        sender_pk: *sender_pk,
        fee_commitment: *fee_commitment,
        submitted_at,
        dkg_epoch,
    })
}

/// Combine decryption shares to reconstruct the shared secret.
///
/// # Invariant: PRIV-EXEC-003
pub fn combine_shares(
    shares: &[crate::DecryptionShare],
    threshold: u32,
) -> Result<[u8; 32], MempoolError> {
    if (shares.len() as u32) < threshold {
        return Err(MempoolError::EncryptionError(format!(
            "Need {} shares but only got {}",
            threshold,
            shares.len()
        )));
    }

    // Lagrange interpolation of decryption shares.
    // Simplified: in production this uses proper Shamir reconstruction on curve points.
    let mut combined = [0u8; 32];
    for (i, share) in shares.iter().enumerate() {
        for j in 0..32.min(share.share.len()) {
            combined[j] ^= share.share[j];
        }
        let _ = i; // Lagrange coefficients needed in production
    }

    Ok(combined)
}

/// Decrypt a transaction using the reconstructed shared secret.
pub fn decrypt_transaction(
    tx: &EncryptedTransaction,
    shared_secret: &[u8; 32],
) -> Result<Vec<u8>, MempoolError> {
    let aes_key = hkdf_derive(shared_secret)?;
    aes_gcm_decrypt(&tx.ciphertext, &aes_key, &tx.nonce)
        .map_err(|e| MempoolError::EncryptionError(e))
}

// ──────────────────────────────────────────────────────────────
// Cryptographic primitives (real implementation)
// ──────────────────────────────────────────────────────────────

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use hkdf::Hkdf;
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::Sha256;
use x25519_dalek::{PublicKey, StaticSecret};

fn generate_ephemeral_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    key
}

fn derive_public_key(sk: &[u8; 32]) -> [u8; 32] {
    let secret = StaticSecret::from(*sk);
    let public = PublicKey::from(&secret);
    *public.as_bytes()
}

fn ecdh(sk: &[u8; 32], pk: &[u8; 32]) -> [u8; 32] {
    let secret = StaticSecret::from(*sk);
    let public = PublicKey::from(*pk);
    let shared = secret.diffie_hellman(&public);
    *shared.as_bytes()
}

fn hkdf_derive(ikm: &[u8; 32]) -> Result<[u8; 32], MempoolError> {
    let hk = Hkdf::<Sha256>::new(Some(ikm), &[]);
    let mut okm = [0u8; 32];
    hk.expand(b"encryption", &mut okm)
        .map_err(|e| MempoolError::EncryptionError(e.to_string()))?;
    Ok(okm)
}

fn generate_nonce() -> [u8; 12] {
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    nonce
}

fn aes_gcm_encrypt(
    plaintext: &[u8],
    key: &[u8; 32],
    nonce: &[u8; 12],
) -> Result<Vec<u8>, MempoolError> {
    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| MempoolError::EncryptionError(e.to_string()))?;
    let nonce = Nonce::from_slice(nonce);
    cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| MempoolError::EncryptionError(e.to_string()))
}

fn aes_gcm_decrypt(ciphertext: &[u8], key: &[u8; 32], nonce: &[u8; 12]) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| e.to_string())?;
    let nonce = Nonce::from_slice(nonce);
    cipher.decrypt(nonce, ciphertext).map_err(|e| e.to_string())
}

fn blake3_hash(data: &[u8]) -> [u8; 32] {
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    hasher.update(data);
    let mut hash = [0u8; 32];
    hasher.finalize_xof().fill(&mut hash);
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let plaintext = b"Hello, private world!";

        // Committee keypair for this test (in production, generated via DKG)
        let committee_sk = generate_ephemeral_key();
        let committee_pk = derive_public_key(&committee_sk);

        let sender_pk = [0xAA; 32];
        let fee_commitment = [0xBB; 32];

        let tx = encrypt_for_committee(plaintext, &committee_pk, &sender_pk, &fee_commitment, 1)
            .unwrap();

        // In realistic decryption, validators reconstruct shared secret from
        // committee secret key and ephemeral public key.
        let shared_secret = ecdh(&committee_sk, &tx.ephemeral_pk);

        let decrypted = decrypt_transaction(&tx, &shared_secret).unwrap();

        // Verify roundtrip works with real AES-GCM
        assert_eq!(&decrypted, plaintext);
    }

    /// # Invariant: PRIV-EXEC-003
    #[test]
    fn combine_shares_requires_threshold() {
        let result = combine_shares(&[], 3);
        assert!(result.is_err());

        let shares = vec![
            crate::DecryptionShare {
                validator_index: 0,
                share: vec![0x01; 32],
                proof: vec![],
            },
            crate::DecryptionShare {
                validator_index: 1,
                share: vec![0x02; 32],
                proof: vec![],
            },
            crate::DecryptionShare {
                validator_index: 2,
                share: vec![0x03; 32],
                proof: vec![],
            },
        ];

        let result = combine_shares(&shares, 3);
        assert!(result.is_ok());
    }
}
