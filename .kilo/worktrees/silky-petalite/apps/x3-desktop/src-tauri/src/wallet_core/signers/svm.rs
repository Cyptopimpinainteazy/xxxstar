use super::{IsolatedSigner, SignerError};
use crate::wallet_core::ipc::{IntentDraft, Attestation, SignerCaps};
use ed25519_dalek::{SigningKey, Signer, Signature};
use bs58;
use std::convert::TryInto;

pub struct SvmSigner {
    pub genesis_hash: String,
    secret_key: SigningKey,
}

impl SvmSigner {
    pub fn new(secret_bytes: &[u8], genesis_hash: String) -> Result<Self, SignerError> {
        let bytes: [u8; 32] = secret_bytes.try_into().map_err(|_| SignerError::CryptoError("Invalid seed length".into()))?;
        let secret_key = SigningKey::from_bytes(&bytes);
        Ok(Self {
            genesis_hash,
            secret_key,
        })
    }
}

impl IsolatedSigner for SvmSigner {
    fn derive_address(&self, _path: &str) -> Result<String, SignerError> {
        let pubkey = self.secret_key.verifying_key();
        Ok(bs58::encode(pubkey.as_bytes()).into_string())
    }

    fn sign_intent(&self, preimage: &IntentDraft, attestation: &Attestation) -> Result<String, SignerError> {
        if attestation.intent_id != preimage.id {
            return Err(SignerError::IntentMismatch);
        }
        
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
        if attestation.expiry < now {
            return Err(SignerError::AttestationExpired);
        }

        let intent_bytes = bincode::serialize(preimage).map_err(|_| SignerError::CryptoError("Serialization failed".into()))?;
        let signature: Signature = self.secret_key.sign(&intent_bytes);
        
        Ok(bs58::encode(signature.to_bytes()).into_string())
    }

    fn sign_tx(&self, canonical_tx_bytes: &[u8], _intent_id: &str) -> Result<String, SignerError> {
        // Enforce parsing here in real impl
        let signature: Signature = self.secret_key.sign(canonical_tx_bytes);
        Ok(bs58::encode(signature.to_bytes()).into_string())
    }

    fn get_capabilities(&self) -> SignerCaps {
        SignerCaps {
            chains: vec!["SVM".to_string()],
            max_tx_value: "100000000000000000".to_string(), // 100 SOL 
            requires_hardware: false,
        }
    }
}
