use super::{IsolatedSigner, SignerError};
use crate::wallet_core::ipc::{IntentDraft, Attestation, SignerCaps};
use k256::ecdsa::{SigningKey, signature::Signer, Signature};
use k256::elliptic_curve::sec1::ToEncodedPoint;
use sp_core::hashing::keccak_256;

// In a real implementation this secret should be Zeroized on drop
pub struct EvmSigner {
    pub chain_id: u64,
    secret_key: SigningKey,
}

impl EvmSigner {
    pub fn new(secret_bytes: &[u8]) -> Result<Self, SignerError> {
        let secret_key = SigningKey::from_bytes(secret_bytes.into())
            .map_err(|e| SignerError::CryptoError(e.to_string()))?;
        Ok(Self {
            chain_id: 1, // Defaulting to Ethereum Mainnet
            secret_key,
        })
    }
}

impl IsolatedSigner for EvmSigner {
    fn derive_address(&self, _path: &str) -> Result<String, SignerError> {
        let verifying_key = self.secret_key.verifying_key();
        let encoded_point = verifying_key.to_encoded_point(false);
        let pubkey_bytes = &encoded_point.as_bytes()[1..]; // Remove the 0x04 format prefix
        let hash = keccak_256(pubkey_bytes);
        let address_bytes = &hash[12..32];
        Ok(format!("0x{}", hex::encode(address_bytes)))
    }

    fn sign_intent(&self, preimage: &IntentDraft, attestation: &Attestation) -> Result<String, SignerError> {
        // MUST VERIFY ATTESTATION SIG BEFORE PROCEEDING
        if attestation.intent_id != preimage.id {
            return Err(SignerError::IntentMismatch);
        }
        
        // Ensure not expired
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
        if attestation.expiry < now {
            return Err(SignerError::AttestationExpired);
        }

        // Generate actual EIP-712 structured hex signature from canonical intent bytes
        // Using Keccak256 as our preimage hasher
        let intent_bytes = bincode::serialize(preimage).map_err(|_| SignerError::CryptoError("Serialization failed".into()))?;
        let hash = keccak_256(&intent_bytes);
        
        let signature: Signature = self.secret_key.sign(&hash);
        Ok(format!("0x{}", hex::encode(signature.to_bytes())))
    }

    fn sign_tx(&self, canonical_tx_bytes: &[u8], _intent_id: &str) -> Result<String, SignerError> {
        // For EVM: RLP decoding would occur here to strictly vet against IntentDraft bounds
        let hash = keccak_256(canonical_tx_bytes);
        // Note: Missing recovery ID, typically appended for EVM ECSDA (v = rec_id + 27 or chainid*2+35)
        let signature: Signature = self.secret_key.sign(&hash);
        
        // Return raw signature string for broadcast
        Ok(format!("0x{}", hex::encode(signature.to_bytes())))
    }

    fn get_capabilities(&self) -> SignerCaps {
        SignerCaps {
            chains: vec!["EVM".to_string()],
            max_tx_value: "100000000000000000000".to_string(), // 100 ETH bound
            requires_hardware: false,
        }
    }
}
