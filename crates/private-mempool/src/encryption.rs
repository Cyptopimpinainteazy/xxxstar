//! Encryption utilities for the private mempool.
//!
//! Provides helpers for encrypting transactions to the committee's threshold
//! key and reconstructing plaintexts from decryption shares.
//!
//! # Cryptographic Scheme
//!
//! 1. Sender generates an ephemeral Ristretto scalar/point keypair.
//! 2. `ephemeral_scalar * committee_group_key` (a Ristretto ECDH) → shared point.
//! 3. HKDF-SHA256(shared point's compressed bytes) → AES-256-GCM key.
//! 4. AES-256-GCM encrypt(plaintext, nonce) → ciphertext.
//!
//! Decryption requires `t`-of-`n` validators to each compute a partial
//! decryption (their [`crate::threshold::SecretShare`] times the ephemeral
//! point) and [`combine_shares`] those partials via Lagrange interpolation
//! in the exponent — see [`crate::threshold`] for why this, and not raw
//! X25519 ECDH, is what makes the threshold guarantee real.

use crate::threshold::{self, SecretShare};
use crate::{DecryptionShare, EncryptedTransaction, MempoolError};

/// Encrypt a transaction payload for the committee.
///
/// `committee_group_key` is the committee's Ristretto group public key —
/// see [`crate::threshold::group_public_key`].
///
/// # Invariant: PRIV-EXEC-001
pub fn encrypt_for_committee(
    plaintext: &[u8],
    committee_group_key: &[u8; 32],
    sender_pk: &[u8; 32],
    fee_commitment: &[u8; 32],
    dkg_epoch: u64,
) -> Result<EncryptedTransaction, MempoolError> {
    let group_key = decompress_point(committee_group_key)?;

    // Ephemeral Ristretto keypair; the ECDH shared point is
    // ephemeral_scalar * group_key = ephemeral_scalar * (committee_secret * G).
    let ephemeral_scalar = Scalar::random(&mut OsRng);
    let ephemeral_pk = (ephemeral_scalar * RISTRETTO_BASEPOINT_POINT)
        .compress()
        .to_bytes();
    let shared_point = (ephemeral_scalar * group_key).compress().to_bytes();

    let aes_key = hkdf_derive(&shared_point)?;
    let nonce = generate_nonce();
    let ciphertext = aes_gcm_encrypt(plaintext, &aes_key, &nonce)?;
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

/// Compute one validator's partial decryption of `tx`, from that
/// validator's own [`SecretShare`] of the committee secret. No single
/// validator's `share` reveals the shared secret; see [`combine_shares`].
///
/// # Invariant: PRIV-EXEC-003
pub fn compute_decryption_share(
    share: &SecretShare,
    ephemeral_pk: &[u8; 32],
) -> Result<DecryptionShare, MempoolError> {
    let ephemeral_point = decompress_point(ephemeral_pk)?;
    let partial = (share.scalar * ephemeral_point).compress().to_bytes();
    Ok(DecryptionShare {
        validator_index: share.index,
        share: partial.to_vec(),
        // DLEQ proof that this partial was computed honestly from the
        // validator's committed share is not implemented — see
        // https://github.com/x3-chain/x3-chain/issues (filed alongside this
        // fix) for the confidential-gpu DKG this depends on. A malicious
        // validator can currently submit a bogus partial and the caller
        // only finds out because the resulting AES-GCM tag fails to verify.
        proof: Vec::new(),
    })
}

/// Combine `threshold`-or-more decryption shares into the shared ECDH
/// point, via real Lagrange interpolation in the exponent — not by XORing
/// bytes. Any `threshold`-sized subset of honest shares produces the same
/// result; fewer than `threshold` produces an unrelated point, so
/// [`decrypt_transaction`] fails closed (AES-GCM tag mismatch) rather than
/// silently returning garbage.
///
/// # Invariant: PRIV-EXEC-003
pub fn combine_shares(
    shares: &[DecryptionShare],
    threshold: u32,
) -> Result<[u8; 32], MempoolError> {
    if (shares.len() as u32) < threshold {
        return Err(MempoolError::EncryptionError(format!(
            "Need {} shares but only got {}",
            threshold,
            shares.len()
        )));
    }

    let mut seen_indices = std::collections::HashSet::with_capacity(shares.len());
    let mut points = Vec::with_capacity(shares.len());
    for share in shares {
        if share.validator_index == 0 {
            return Err(MempoolError::EncryptionError(
                "decryption share has index 0, which is reserved for the secret itself".to_string(),
            ));
        }
        if !seen_indices.insert(share.validator_index) {
            return Err(MempoolError::EncryptionError(format!(
                "duplicate decryption share for validator index {}",
                share.validator_index
            )));
        }
        points.push((share.validator_index, decompress_point_slice(&share.share)?));
    }

    Ok(threshold::combine_points(&points).compress().to_bytes())
}

/// Decrypt a transaction using the reconstructed shared secret.
pub fn decrypt_transaction(
    tx: &EncryptedTransaction,
    shared_secret: &[u8; 32],
) -> Result<Vec<u8>, MempoolError> {
    let aes_key = hkdf_derive(shared_secret)?;
    aes_gcm_decrypt(&tx.ciphertext, &aes_key, &tx.nonce).map_err(MempoolError::EncryptionError)
}

/// Decompress a point that will be used as key material (a committee group
/// key or an ephemeral public key), rejecting the group identity element.
///
/// The identity is a valid Ristretto encoding, but `scalar * identity ==
/// identity` for every scalar — if it were accepted as a committee group
/// key, every transaction's "shared secret" would collapse to that one
/// constant, publicly-known value regardless of the ephemeral scalar,
/// silently discarding confidentiality for the entire mempool with no
/// error and no dependence on any validator's share.
fn decompress_point(bytes: &[u8; 32]) -> Result<RistrettoPoint, MempoolError> {
    let point = CompressedRistretto(*bytes)
        .decompress()
        .ok_or_else(|| MempoolError::EncryptionError("invalid Ristretto point".to_string()))?;
    if point == RistrettoPoint::identity() {
        return Err(MempoolError::EncryptionError(
            "point is the group identity element, which cannot be used as key material".to_string(),
        ));
    }
    Ok(point)
}

fn decompress_point_slice(bytes: &[u8]) -> Result<RistrettoPoint, MempoolError> {
    let array: [u8; 32] = bytes.try_into().map_err(|_| {
        MempoolError::EncryptionError("decryption share is not 32 bytes".to_string())
    })?;
    decompress_point(&array)
}

// ──────────────────────────────────────────────────────────────
// Cryptographic primitives (real implementation)
// ──────────────────────────────────────────────────────────────

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use curve25519_dalek::ristretto::{CompressedRistretto, RistrettoPoint};
use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::traits::Identity;
use hkdf::Hkdf;
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::Sha256;

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
    use crate::threshold::{group_public_key, split_secret};

    #[test]
    fn encrypt_decrypt_roundtrip_with_a_real_threshold_committee() {
        let plaintext = b"Hello, private world!";

        // A real DKG (elsewhere) would hand each validator one of these
        // shares and publish only the group key; nothing here ever holds
        // `committee_secret` except this line simulating that ceremony.
        let committee_secret = Scalar::random(&mut OsRng);
        let committee_group_key = group_public_key(&committee_secret);
        let shares = split_secret(committee_secret, 3, 5);

        let sender_pk = [0xAA; 32];
        let fee_commitment = [0xBB; 32];

        let tx = encrypt_for_committee(
            plaintext,
            &committee_group_key,
            &sender_pk,
            &fee_commitment,
            1,
        )
        .unwrap();

        // Any 3 of the 5 validators — not a fixed subset — reconstruct the
        // same shared secret.
        let partials: Vec<DecryptionShare> = [shares[1], shares[2], shares[4]]
            .iter()
            .map(|s| compute_decryption_share(s, &tx.ephemeral_pk).unwrap())
            .collect();

        let shared_secret = combine_shares(&partials, 3).unwrap();
        let decrypted = decrypt_transaction(&tx, &shared_secret).unwrap();
        assert_eq!(&decrypted, plaintext);
    }

    #[test]
    fn below_threshold_shares_fail_to_decrypt() {
        let plaintext = b"top secret trade";
        let committee_secret = Scalar::random(&mut OsRng);
        let committee_group_key = group_public_key(&committee_secret);
        let shares = split_secret(committee_secret, 3, 5);

        let tx =
            encrypt_for_committee(plaintext, &committee_group_key, &[0; 32], &[0; 32], 1).unwrap();

        // Only 2 of the required 3 shares.
        let partials: Vec<DecryptionShare> = [shares[0], shares[1]]
            .iter()
            .map(|s| compute_decryption_share(s, &tx.ephemeral_pk).unwrap())
            .collect();

        // combine_shares' own length check is bypassed by lying about the
        // threshold, to prove the *cryptographic* guarantee also holds: an
        // under-threshold combination is not just rejected by a length
        // check, it actually reconstructs the wrong point.
        let wrong_secret = combine_shares(&partials, 2).unwrap();
        let result = decrypt_transaction(&tx, &wrong_secret);
        assert!(
            result.is_err(),
            "AES-GCM must reject a reconstructed-from-too-few-shares key"
        );
    }

    #[test]
    fn combine_shares_enforces_the_stated_threshold() {
        let result = combine_shares(&[], 3);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_the_identity_element_as_a_committee_group_key() {
        let identity_key = RistrettoPoint::identity().compress().to_bytes();
        let result = encrypt_for_committee(b"msg", &identity_key, &[0; 32], &[0; 32], 1);
        assert!(
            result.is_err(),
            "encrypting to the identity element must be rejected, not silently \
             produce a publicly-known shared secret"
        );
    }

    #[test]
    fn rejects_the_identity_element_as_an_ephemeral_key() {
        let committee_secret = Scalar::random(&mut OsRng);
        let share = split_secret(committee_secret, 1, 1).remove(0);
        let identity_pk = RistrettoPoint::identity().compress().to_bytes();
        let result = compute_decryption_share(&share, &identity_pk);
        assert!(result.is_err());
    }

    #[test]
    fn combine_shares_rejects_a_zero_validator_index() {
        let committee_secret = Scalar::random(&mut OsRng);
        let committee_group_key = group_public_key(&committee_secret);
        let shares = split_secret(committee_secret, 2, 2);
        let tx =
            encrypt_for_committee(b"msg", &committee_group_key, &[0; 32], &[0; 32], 1).unwrap();

        let mut partials: Vec<DecryptionShare> = shares
            .iter()
            .map(|s| compute_decryption_share(s, &tx.ephemeral_pk).unwrap())
            .collect();
        partials[0].validator_index = 0;

        let result = combine_shares(&partials, 2);
        assert!(result.is_err());
    }

    #[test]
    fn combine_shares_rejects_duplicate_validator_indices() {
        let committee_secret = Scalar::random(&mut OsRng);
        let committee_group_key = group_public_key(&committee_secret);
        let shares = split_secret(committee_secret, 2, 3);
        let tx =
            encrypt_for_committee(b"msg", &committee_group_key, &[0; 32], &[0; 32], 1).unwrap();

        // Two entries both claiming to be validator 1 — a second, possibly
        // malicious, share silently overriding another validator's weight
        // in the Lagrange combination instead of being rejected outright.
        let mut partials: Vec<DecryptionShare> = vec![
            compute_decryption_share(&shares[0], &tx.ephemeral_pk).unwrap(),
            compute_decryption_share(&shares[1], &tx.ephemeral_pk).unwrap(),
        ];
        let mut impostor = compute_decryption_share(&shares[2], &tx.ephemeral_pk).unwrap();
        impostor.validator_index = partials[0].validator_index;
        partials.push(impostor);

        let result = combine_shares(&partials, 2);
        assert!(result.is_err());
    }
}
