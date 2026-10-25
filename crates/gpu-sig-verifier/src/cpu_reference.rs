//! CPU reference for secp256k1 ECDSA verification.
//!
//! The caller supplies the public key, 32-byte message digest, and compact
//! (r || s) signature. This is deliberately separate from the legacy
//! signature-only API, which must continue to fail closed.
use k256::ecdsa::{signature::hazmat::PrehashVerifier, Signature, VerifyingKey};

/// Verify a compact ECDSA signature over an already-computed 32-byte digest.
///
/// Public keys must be SEC1-encoded (compressed or uncompressed). This function
/// does not hash the message, recover a signer, or accept DER signatures.
pub fn verify_secp256k1_digest(
    public_key_sec1: &[u8],
    digest: &[u8],
    signature_compact: &[u8],
) -> bool {
    if digest.len() != 32 || signature_compact.len() != 64 {
        return false;
    }
    let Ok(key) = VerifyingKey::from_sec1_bytes(public_key_sec1) else {
        return false;
    };
    let Ok(signature) = Signature::from_slice(signature_compact) else {
        return false;
    };
    key.verify_prehash(digest, &signature).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::ecdsa::{signature::hazmat::PrehashSigner, SigningKey};

    #[test]
    fn valid_signature_and_tampering() {
        let signing_key = SigningKey::from_bytes((&[7u8; 32]).into()).unwrap();
        let public_key = signing_key.verifying_key().to_sec1_point(true);
        let digest = [42u8; 32];
        let signature: Signature = signing_key.sign_prehash(&digest).unwrap();
        assert!(verify_secp256k1_digest(
            public_key.as_bytes(),
            &digest,
            &signature.to_bytes()
        ));

        let mut changed_digest = digest;
        changed_digest[0] ^= 1;
        assert!(!verify_secp256k1_digest(
            public_key.as_bytes(),
            &changed_digest,
            &signature.to_bytes()
        ));
        assert!(!verify_secp256k1_digest(
            public_key.as_bytes(),
            &digest[..31],
            &signature.to_bytes()
        ));
        assert!(!verify_secp256k1_digest(
            public_key.as_bytes(),
            &digest,
            &signature.to_bytes()[..63]
        ));
        assert!(!verify_secp256k1_digest(
            &[0u8; 33],
            &digest,
            &signature.to_bytes()
        ));

        // A correctly formed signature from a different key must not pass.
        let other_key = SigningKey::from_bytes((&[8u8; 32]).into()).unwrap();
        let other_public_key = other_key.verifying_key().to_sec1_point(true);
        assert!(!verify_secp256k1_digest(
            other_public_key.as_bytes(),
            &digest,
            &signature.to_bytes()
        ));

        // Corrupting r or s must never silently verify.
        let mut corrupted_signature = signature.to_bytes();
        corrupted_signature[0] ^= 1;
        assert!(!verify_secp256k1_digest(
            public_key.as_bytes(),
            &digest,
            &corrupted_signature
        ));
    }
}
