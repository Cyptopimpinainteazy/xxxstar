use anyhow::Result;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier};
use sha2::{Digest, Sha256};

/// Derive an ed25519 signing key deterministically from a 64-byte BIP39 seed.
/// This uses the first 32 bytes of the seed as the ed25519 secret seed.
pub fn derive_ed25519_keypair(seed: &[u8; 64]) -> Result<SigningKey> {
    let seed32: [u8; 32] = seed[0..32]
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid seed length"))?;
    Ok(SigningKey::from_bytes(&seed32))
}

/// Return the public key as hex string
pub fn public_key_hex(kp: &SigningKey) -> String {
    hex::encode(kp.verifying_key().as_bytes())
}

/// Return a simple address: SHA256(pubkey) hex (prototype format)
pub fn address_from_keypair(kp: &SigningKey) -> String {
    let h = Sha256::digest(kp.verifying_key().as_bytes());
    hex::encode(h)
}

/// Sign message with the keypair
pub fn sign_message(kp: &SigningKey, msg: &[u8]) -> Vec<u8> {
    kp.sign(msg).to_bytes().to_vec()
}

/// Verify signature and return bool
pub fn verify_signature(kp: &SigningKey, msg: &[u8], sig: &[u8]) -> bool {
    let Ok(sig_arr) = <&[u8; 64]>::try_from(sig) else {
        return false;
    };
    let sig_obj = Signature::from_bytes(sig_arr);
    kp.verifying_key().verify(msg, &sig_obj).is_ok()
}
