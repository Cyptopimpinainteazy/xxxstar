use super::{IsolatedSigner, SignerError};
use crate::wallet_core::ipc::{IntentDraft, Attestation, SignerCaps};
use base64::{engine::general_purpose, Engine as _};
use k256::ecdsa::{signature::Signer as _, Signature, SigningKey};
use sp_core::hashing::sha2_256;

pub struct BtcSigner {
    pub network_magic: u32,
    signing_key: SigningKey,
}

impl BtcSigner {
    pub fn new(secret_bytes: &[u8], network_magic: u32) -> Result<Self, SignerError> {
        let sk = SigningKey::from_slice(secret_bytes)
            .map_err(|e| SignerError::CryptoError(format!("invalid BTC signer key: {}", e)))?;
        Ok(Self {
            network_magic,
            signing_key: sk,
        })
    }

    fn address_prefix(&self) -> u8 {
        match self.network_magic {
            // Mainnet
            0xD9B4BEF9 => 0x00,
            // Testnet
            0x0709110B => 0x6f,
            _ => 0x00,
        }
    }

    fn parse_psbt_bytes(canonical_tx_bytes: &[u8]) -> Result<Vec<u8>, SignerError> {
        if canonical_tx_bytes.is_empty() {
            return Err(SignerError::CryptoError("empty PSBT payload".into()));
        }

        // Prefer base64-encoded PSBT payloads.
        if let Ok(as_str) = std::str::from_utf8(canonical_tx_bytes) {
            let trimmed = as_str.trim();
            if !trimmed.is_empty() {
                if let Ok(decoded) = general_purpose::STANDARD.decode(trimmed) {
                    return Ok(decoded);
                }
            }
        }

        // Fallback to raw PSBT bytes.
        Ok(canonical_tx_bytes.to_vec())
    }

    fn parse_compact_size(bytes: &[u8], cursor: &mut usize) -> Result<u64, SignerError> {
        if *cursor >= bytes.len() {
            return Err(SignerError::CryptoError("unexpected end while reading compact size".into()));
        }
        let first = bytes[*cursor];
        *cursor += 1;
        match first {
            0x00..=0xfc => Ok(first as u64),
            0xfd => {
                if *cursor + 2 > bytes.len() {
                    return Err(SignerError::CryptoError("unexpected end for compact size u16".into()));
                }
                let v = u16::from_le_bytes([bytes[*cursor], bytes[*cursor + 1]]) as u64;
                *cursor += 2;
                Ok(v)
            }
            0xfe => {
                if *cursor + 4 > bytes.len() {
                    return Err(SignerError::CryptoError("unexpected end for compact size u32".into()));
                }
                let v = u32::from_le_bytes([
                    bytes[*cursor],
                    bytes[*cursor + 1],
                    bytes[*cursor + 2],
                    bytes[*cursor + 3],
                ]) as u64;
                *cursor += 4;
                Ok(v)
            }
            _ => {
                if *cursor + 8 > bytes.len() {
                    return Err(SignerError::CryptoError("unexpected end for compact size u64".into()));
                }
                let v = u64::from_le_bytes([
                    bytes[*cursor],
                    bytes[*cursor + 1],
                    bytes[*cursor + 2],
                    bytes[*cursor + 3],
                    bytes[*cursor + 4],
                    bytes[*cursor + 5],
                    bytes[*cursor + 6],
                    bytes[*cursor + 7],
                ]);
                *cursor += 8;
                Ok(v)
            }
        }
    }

    fn encode_compact_size(v: usize) -> Vec<u8> {
        if v <= 0xfc {
            return vec![v as u8];
        }
        if v <= u16::MAX as usize {
            let mut out = vec![0xfd];
            out.extend_from_slice(&(v as u16).to_le_bytes());
            return out;
        }
        if v <= u32::MAX as usize {
            let mut out = vec![0xfe];
            out.extend_from_slice(&(v as u32).to_le_bytes());
            return out;
        }
        let mut out = vec![0xff];
        out.extend_from_slice(&(v as u64).to_le_bytes());
        out
    }

    fn parse_unsigned_tx_counts(unsigned_tx: &[u8]) -> Result<(usize, usize), SignerError> {
        if unsigned_tx.len() < 8 {
            return Err(SignerError::CryptoError("unsigned tx too short".into()));
        }

        let mut cursor = 4; // version
        let input_count = Self::parse_compact_size(unsigned_tx, &mut cursor)? as usize;
        if input_count == 0 {
            return Err(SignerError::CryptoError("unsigned tx must include at least one input".into()));
        }

        for _ in 0..input_count {
            if cursor + 36 > unsigned_tx.len() {
                return Err(SignerError::CryptoError("unexpected end in tx input outpoint".into()));
            }
            cursor += 36;
            let script_len = Self::parse_compact_size(unsigned_tx, &mut cursor)? as usize;
            if cursor + script_len + 4 > unsigned_tx.len() {
                return Err(SignerError::CryptoError("unexpected end in tx input script/sequence".into()));
            }
            cursor += script_len + 4;
        }

        let output_count = Self::parse_compact_size(unsigned_tx, &mut cursor)? as usize;
        if output_count == 0 {
            return Err(SignerError::CryptoError("unsigned tx must include at least one output".into()));
        }

        for _ in 0..output_count {
            if cursor + 8 > unsigned_tx.len() {
                return Err(SignerError::CryptoError("unexpected end in tx output value".into()));
            }
            cursor += 8;
            let script_len = Self::parse_compact_size(unsigned_tx, &mut cursor)? as usize;
            if cursor + script_len > unsigned_tx.len() {
                return Err(SignerError::CryptoError("unexpected end in tx output script".into()));
            }
            cursor += script_len;
        }

        if cursor + 4 > unsigned_tx.len() {
            return Err(SignerError::CryptoError("unexpected end in tx locktime".into()));
        }
        cursor += 4;

        if cursor != unsigned_tx.len() {
            return Err(SignerError::CryptoError("unsigned tx contains trailing bytes".into()));
        }

        Ok((input_count, output_count))
    }

    fn validate_psbt_and_bounds(psbt: &[u8]) -> Result<(usize, usize, Vec<usize>), SignerError> {
        if psbt.len() < 8 || &psbt[..5] != b"psbt\xff" {
            return Err(SignerError::CryptoError("invalid PSBT magic bytes".into()));
        }

        let mut cursor = 5;
        let mut has_unsigned_tx = false;
        let mut unsigned_tx = Vec::new();
        loop {
            let key_len = Self::parse_compact_size(psbt, &mut cursor)? as usize;
            if key_len == 0 {
                break;
            }
            if cursor + key_len > psbt.len() {
                return Err(SignerError::CryptoError("invalid PSBT global key length".into()));
            }
            let key = &psbt[cursor..cursor + key_len];
            cursor += key_len;

            let value_len = Self::parse_compact_size(psbt, &mut cursor)? as usize;
            if cursor + value_len > psbt.len() {
                return Err(SignerError::CryptoError("invalid PSBT global value length".into()));
            }
            let value = &psbt[cursor..cursor + value_len];
            cursor += value_len;

            if key[0] == 0x00 {
                has_unsigned_tx = true;
                unsigned_tx = value.to_vec();
            }
        }

        if !has_unsigned_tx {
            return Err(SignerError::CryptoError("PSBT missing unsigned transaction".into()));
        }

        let (input_count, output_count) = Self::parse_unsigned_tx_counts(&unsigned_tx)?;

        let mut input_map_end_offsets = Vec::with_capacity(input_count);
        for _ in 0..input_count {
            loop {
                let key_len = Self::parse_compact_size(psbt, &mut cursor)? as usize;
                if key_len == 0 {
                    input_map_end_offsets.push(cursor - 1);
                    break;
                }
                if cursor + key_len > psbt.len() {
                    return Err(SignerError::CryptoError("invalid PSBT input key length".into()));
                }
                cursor += key_len;
                let value_len = Self::parse_compact_size(psbt, &mut cursor)? as usize;
                if cursor + value_len > psbt.len() {
                    return Err(SignerError::CryptoError("invalid PSBT input value length".into()));
                }
                cursor += value_len;
            }
        }

        for _ in 0..output_count {
            loop {
                let key_len = Self::parse_compact_size(psbt, &mut cursor)? as usize;
                if key_len == 0 {
                    break;
                }
                if cursor + key_len > psbt.len() {
                    return Err(SignerError::CryptoError("invalid PSBT output key length".into()));
                }
                cursor += key_len;
                let value_len = Self::parse_compact_size(psbt, &mut cursor)? as usize;
                if cursor + value_len > psbt.len() {
                    return Err(SignerError::CryptoError("invalid PSBT output value length".into()));
                }
                cursor += value_len;
            }
        }

        if cursor != psbt.len() {
            return Err(SignerError::CryptoError("PSBT contains trailing bytes".into()));
        }

        Ok((input_count, output_count, input_map_end_offsets))
    }

    fn inject_partial_signatures(
        psbt: &[u8],
        signature_bytes: &[u8],
        compressed_pubkey: &[u8],
        input_map_end_offsets: &[usize],
    ) -> Vec<u8> {
        let mut entry = Vec::new();
        let mut key = Vec::with_capacity(1 + compressed_pubkey.len());
        key.push(0x02);
        key.extend_from_slice(compressed_pubkey);
        entry.extend(Self::encode_compact_size(key.len()));
        entry.extend_from_slice(&key);
        entry.extend(Self::encode_compact_size(signature_bytes.len()));
        entry.extend_from_slice(signature_bytes);

        let mut out = psbt.to_vec();
        let mut offset = 0usize;
        for insert_at in input_map_end_offsets {
            let adjusted = insert_at + offset;
            out.splice(adjusted..adjusted, entry.iter().copied());
            offset += entry.len();
        }
        out
    }

    fn btc_address_from_pubkey(&self, compressed_pubkey: &[u8]) -> String {
        let hash = sha2_256(compressed_pubkey);
        let mut payload = Vec::with_capacity(25);
        payload.push(self.address_prefix());
        payload.extend_from_slice(&hash[..20]);

        let checksum_1 = sha2_256(&payload);
        let checksum_2 = sha2_256(&checksum_1);
        payload.extend_from_slice(&checksum_2[..4]);

        bs58::encode(payload).into_string()
    }
}

impl IsolatedSigner for BtcSigner {
    fn derive_address(&self, _path: &str) -> Result<String, SignerError> {
        let verifying_key = self.signing_key.verifying_key();
        let pubkey = verifying_key.to_encoded_point(true);
        Ok(self.btc_address_from_pubkey(pubkey.as_bytes()))
    }

    fn sign_intent(&self, preimage: &IntentDraft, attestation: &Attestation) -> Result<String, SignerError> {
        if attestation.intent_id != preimage.id {
            return Err(SignerError::IntentMismatch);
        }
        
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
        if attestation.expiry < now {
            return Err(SignerError::AttestationExpired);
        }

        let intent_bytes = bincode::serialize(preimage)
            .map_err(|_| SignerError::CryptoError("Intent serialization failed".into()))?;
        let digest_1 = sha2_256(&intent_bytes);
        let digest_2 = sha2_256(&digest_1);
        let sig: Signature = self.signing_key.sign(&digest_2);
        Ok(hex::encode(sig.to_bytes()))
    }

    fn sign_tx(&self, canonical_tx_bytes: &[u8], intent_id: &str) -> Result<String, SignerError> {
        // BTC Signer RULE: PSBT byte streams ONLY.
        let psbt_raw = Self::parse_psbt_bytes(canonical_tx_bytes)?;
        let (_inputs, _outputs, input_map_end_offsets) = Self::validate_psbt_and_bounds(&psbt_raw)?;

        // Deterministic signer receipt bound to intent and PSBT bytes.
        let mut binding = Vec::with_capacity(intent_id.len() + 1 + psbt_raw.len());
        binding.extend_from_slice(intent_id.as_bytes());
        binding.push(b':');
        binding.extend_from_slice(&psbt_raw);

        let digest_1 = sha2_256(&binding);
        let digest_2 = sha2_256(&digest_1);
        let sig: Signature = self.signing_key.sign(&digest_2);
        let mut psbt_sig = sig.to_der().as_bytes().to_vec();
        psbt_sig.push(0x01);
        let compressed_pubkey = self.signing_key.verifying_key().to_encoded_point(true);
        let signed_psbt = Self::inject_partial_signatures(
            &psbt_raw,
            &psbt_sig,
            compressed_pubkey.as_bytes(),
            &input_map_end_offsets,
        );

        Ok(general_purpose::STANDARD.encode(signed_psbt))
    }

    fn get_capabilities(&self) -> SignerCaps {
        SignerCaps {
            chains: vec!["BTC".to_string()],
            max_tx_value: "500000000".to_string(), // 5 BTC bound
            requires_hardware: true, // Example policy override: BTC requires hardware
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_signer() -> BtcSigner {
        let seed = [7u8; 32];
        BtcSigner::new(&seed, 0xD9B4BEF9).expect("btc signer init")
    }

    fn sample_psbt_b64() -> String {
        let mut tx = Vec::new();
        tx.extend_from_slice(&1u32.to_le_bytes());
        tx.push(0x01);
        tx.extend_from_slice(&[0u8; 32]);
        tx.extend_from_slice(&0u32.to_le_bytes());
        tx.push(0x00);
        tx.extend_from_slice(&u32::MAX.to_le_bytes());
        tx.push(0x01);
        tx.extend_from_slice(&1000u64.to_le_bytes());
        tx.push(0x00);
        tx.extend_from_slice(&0u32.to_le_bytes());

        let mut psbt = Vec::new();
        psbt.extend_from_slice(b"psbt\xff");
        psbt.push(0x01);
        psbt.push(0x00);
        psbt.push(tx.len() as u8);
        psbt.extend_from_slice(&tx);
        psbt.push(0x00);
        psbt.push(0x00);
        psbt.push(0x00);
        general_purpose::STANDARD.encode(psbt)
    }

    #[test]
    fn btc_signer_derives_address() {
        let signer = test_signer();
        let addr = signer.derive_address("m/84'/0'/0'/0/0").expect("address");
        assert!(!addr.is_empty());
    }

    #[test]
    fn btc_signer_rejects_invalid_psbt() {
        let signer = test_signer();
        let err = signer
            .sign_tx(b"not-base64-and-not-psbt", "intent-test")
            .expect_err("invalid psbt must fail");
        match err {
            SignerError::CryptoError(_) => {}
            _ => panic!("expected crypto error for invalid psbt"),
        }
    }

    #[test]
    fn btc_signer_signs_valid_psbt() {
        let signer = test_signer();
        let b64 = sample_psbt_b64();
        let signed_b64 = signer.sign_tx(b64.as_bytes(), "intent-test").expect("sign psbt");
        assert!(!signed_b64.is_empty());

        let decoded = general_purpose::STANDARD
            .decode(signed_b64)
            .expect("signed psbt decode");
        assert!(decoded.starts_with(b"psbt\xff"));
        let pubkey = signer.signing_key.verifying_key().to_encoded_point(true);
        let mut key = Vec::with_capacity(1 + pubkey.as_bytes().len());
        key.push(0x02);
        key.extend_from_slice(pubkey.as_bytes());
        assert!(decoded.windows(key.len()).any(|w| w == key.as_slice()));
    }
}
