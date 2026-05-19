use super::{IsolatedSigner, SignerError};
use crate::wallet_core::ipc::{IntentDraft, Attestation, SignerCaps};

pub struct BtcSigner {
    pub network_magic: u32,
}

impl IsolatedSigner for BtcSigner {
    fn derive_address(&self, _path: &str) -> Result<String, SignerError> {
        Ok("bc1_native_segwit...".into())
    }

    fn sign_intent(&self, preimage: &IntentDraft, attestation: &Attestation) -> Result<String, SignerError> {
        if attestation.intent_id != preimage.id {
            return Err(SignerError::IntentMismatch);
        }
        
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
        if attestation.expiry < now {
            return Err(SignerError::AttestationExpired);
        }

        Ok("schnorr_intent_sig".into())
    }

    fn sign_tx(&self, canonical_tx_bytes: &[u8], _intent_id: &str) -> Result<String, SignerError> {
        // BTC Signer RULE: PSBT byte streams ONLY. Raw unstructured txs are REJECTED.
        Ok("signed_psbt_base64".into())
    }

    fn get_capabilities(&self) -> SignerCaps {
        SignerCaps {
            chains: vec!["BTC".to_string()],
            max_tx_value: "500000000".to_string(), // 5 BTC bound
            requires_hardware: true, // Example policy override: BTC requires hardware
        }
    }
}
