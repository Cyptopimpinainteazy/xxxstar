//! ABI encoding/decoding helpers for cross-VM HTLC operations.
//!
//! Provides lightweight EVM ABI encoding without pulling in full ethers-rs.
//! For SVM, provides Borsh-style instruction packing.
//! For X3VM, provides ABI-compatible calldata encoding.

use crate::types::*;

// ─── EVM ABI Encoding ─────────────────────────────────────────────────────────

/// AtlasHTLC.sol function selectors (first 4 bytes of keccak256).
pub mod evm_selectors {
    /// createHTLC(address,bytes32,uint256,address,uint256) → 0x4b2f336d
    pub const CREATE_HTLC: [u8; 4] = [0x4b, 0x2f, 0x33, 0x6d];
    /// claimHTLC(bytes32,bytes32) → 0x84cc315c
    pub const CLAIM_HTLC: [u8; 4] = [0x84, 0xcc, 0x31, 0x5c];
    /// refundHTLC(bytes32) → 0x7249fbb6
    pub const REFUND_HTLC: [u8; 4] = [0x72, 0x49, 0xfb, 0xb6];
    /// getHTLC(bytes32) → 0x905d22a5
    pub const GET_HTLC: [u8; 4] = [0x90, 0x5d, 0x22, 0xa5];
}

/// Encode EVM ABI uint256 from u128.
pub fn encode_uint256(value: u128) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[16..32].copy_from_slice(&value.to_be_bytes());
    buf
}

/// Encode EVM ABI bytes32.
pub fn encode_bytes32(data: &[u8; 32]) -> [u8; 32] {
    *data
}

/// Encode address (20 bytes) into 32-byte ABI slot (left-padded).
pub fn encode_address(addr: &[u8]) -> [u8; 32] {
    let mut buf = [0u8; 32];
    let start = 32usize.saturating_sub(addr.len());
    buf[start..32].copy_from_slice(&addr[..addr.len().min(32)]);
    buf
}

/// Build createHTLC calldata for AtlasHTLC.sol.
pub fn encode_create_htlc(
    recipient: &[u8],
    hash_lock: &[u8; 32],
    timelock: u64,
    token_address: &[u8],
    amount: u128,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(4 + 5 * 32);
    data.extend_from_slice(&evm_selectors::CREATE_HTLC);
    data.extend_from_slice(&encode_address(recipient));
    data.extend_from_slice(&encode_bytes32(hash_lock));
    data.extend_from_slice(&encode_uint256(timelock as u128));
    data.extend_from_slice(&encode_address(token_address));
    data.extend_from_slice(&encode_uint256(amount));
    data
}

/// Build claimHTLC calldata for AtlasHTLC.sol.
pub fn encode_claim_htlc(htlc_id: &[u8], secret: &[u8; 32]) -> Vec<u8> {
    let mut data = Vec::with_capacity(4 + 2 * 32);
    data.extend_from_slice(&evm_selectors::CLAIM_HTLC);
    // Pad htlc_id to 32 bytes
    let mut id_buf = [0u8; 32];
    let len = htlc_id.len().min(32);
    id_buf[32 - len..].copy_from_slice(&htlc_id[..len]);
    data.extend_from_slice(&id_buf);
    data.extend_from_slice(&encode_bytes32(secret));
    data
}

/// Build refundHTLC calldata.
pub fn encode_refund_htlc(htlc_id: &[u8]) -> Vec<u8> {
    let mut data = Vec::with_capacity(4 + 32);
    data.extend_from_slice(&evm_selectors::REFUND_HTLC);
    let mut id_buf = [0u8; 32];
    let len = htlc_id.len().min(32);
    id_buf[32 - len..].copy_from_slice(&htlc_id[..len]);
    data.extend_from_slice(&id_buf);
    data
}

/// Build getHTLC calldata.
pub fn encode_get_htlc(htlc_id: &[u8]) -> Vec<u8> {
    let mut data = Vec::with_capacity(4 + 32);
    data.extend_from_slice(&evm_selectors::GET_HTLC);
    let mut id_buf = [0u8; 32];
    let len = htlc_id.len().min(32);
    id_buf[32 - len..].copy_from_slice(&htlc_id[..len]);
    data.extend_from_slice(&id_buf);
    data
}

/// Decode HTLC status from getHTLC return data.
/// Returns (status: u8, confirmations: u32).
pub fn decode_htlc_status(return_data: &[u8]) -> Result<(HtlcStatus, u32), CoordinatorError> {
    if return_data.len() < 64 {
        return Err(CoordinatorError::Internal(
            "getHTLC return data too short".into(),
        ));
    }

    // First word: status enum (0=Pending, 1=Funded, 2=Claimed, 3=Refunded)
    let status_byte = return_data[31];
    let status = match status_byte {
        0 => HtlcStatus::Pending,
        1 => HtlcStatus::Funded,
        2 => HtlcStatus::Claimed,
        3 => HtlcStatus::Refunded,
        _ => HtlcStatus::Expired,
    };

    // Second word: block number when created (we use to estimate confirmations)
    let mut block_bytes = [0u8; 8];
    block_bytes.copy_from_slice(&return_data[56..64]);
    let _created_block = u64::from_be_bytes(block_bytes);

    // Confirmations are computed externally (latest_block - created_block)
    // Return 0 here; the adapter calculates from chain tip
    Ok((status, 0))
}

// ─── SVM Instruction Encoding ─────────────────────────────────────────────────

/// Anchor discriminator for create_htlc instruction.
pub mod svm_discriminators {
    /// sha256("global:create_htlc")[..8]
    pub const CREATE_HTLC: [u8; 8] = [0x67, 0x4a, 0x1e, 0x9d, 0x01, 0xf3, 0x5c, 0x8a];
    /// sha256("global:claim_htlc")[..8]
    pub const CLAIM_HTLC: [u8; 8] = [0x2b, 0x49, 0xcc, 0x7f, 0x8e, 0x34, 0x12, 0x67];
    /// sha256("global:refund_htlc")[..8]
    pub const REFUND_HTLC: [u8; 8] = [0x91, 0x37, 0xea, 0x01, 0x4d, 0x9b, 0x55, 0x23];
}

/// Encode SVM create_htlc instruction data.
pub fn encode_svm_create_htlc(hash_lock: &[u8; 32], timelock: u64, amount: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(8 + 32 + 8 + 8);
    data.extend_from_slice(&svm_discriminators::CREATE_HTLC);
    data.extend_from_slice(hash_lock);
    data.extend_from_slice(&timelock.to_le_bytes());
    data.extend_from_slice(&amount.to_le_bytes());
    data
}

/// Encode SVM claim_htlc instruction data.
pub fn encode_svm_claim_htlc(secret: &[u8; 32]) -> Vec<u8> {
    let mut data = Vec::with_capacity(8 + 32);
    data.extend_from_slice(&svm_discriminators::CLAIM_HTLC);
    data.extend_from_slice(secret);
    data
}

/// Encode SVM refund_htlc instruction data.
pub fn encode_svm_refund_htlc() -> Vec<u8> {
    svm_discriminators::REFUND_HTLC.to_vec()
}

// ─── X3VM ABI Encoding ────────────────────────────────────────────────────────

/// X3-lang function selectors (from htlc.x3 ABI).
pub mod x3_selectors {
    /// create_htlc(address,bytes32,u64,u64)
    pub const CREATE_HTLC: [u8; 4] = [0x01, 0x00, 0x00, 0x00];
    /// claim_htlc(bytes32,bytes32)
    pub const CLAIM_HTLC: [u8; 4] = [0x02, 0x00, 0x00, 0x00];
    /// refund_htlc(bytes32)
    pub const REFUND_HTLC: [u8; 4] = [0x03, 0x00, 0x00, 0x00];
    /// get_htlc_status(bytes32)
    pub const GET_STATUS: [u8; 4] = [0x04, 0x00, 0x00, 0x00];
}

/// Encode X3VM create_htlc call.
pub fn encode_x3_create_htlc(
    recipient: &[u8; 32],
    hash_lock: &[u8; 32],
    timelock: u64,
    amount: u64,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(4 + 32 + 32 + 8 + 8);
    data.extend_from_slice(&x3_selectors::CREATE_HTLC);
    data.extend_from_slice(recipient);
    data.extend_from_slice(hash_lock);
    data.extend_from_slice(&timelock.to_le_bytes());
    data.extend_from_slice(&amount.to_le_bytes());
    data
}

/// Encode X3VM claim_htlc call.
pub fn encode_x3_claim_htlc(htlc_id: &[u8; 32], secret: &[u8; 32]) -> Vec<u8> {
    let mut data = Vec::with_capacity(4 + 32 + 32);
    data.extend_from_slice(&x3_selectors::CLAIM_HTLC);
    data.extend_from_slice(htlc_id);
    data.extend_from_slice(secret);
    data
}

/// Encode X3VM refund_htlc call.
pub fn encode_x3_refund_htlc(htlc_id: &[u8; 32]) -> Vec<u8> {
    let mut data = Vec::with_capacity(4 + 32);
    data.extend_from_slice(&x3_selectors::REFUND_HTLC);
    data.extend_from_slice(htlc_id);
    data
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evm_create_htlc_calldata_length() {
        let recipient = [0xAA; 20];
        let hash_lock = [0xBB; 32];
        let token = [0xCC; 20];
        let data = encode_create_htlc(&recipient, &hash_lock, 3600, &token, 1_000_000);
        // 4 selector + 5 * 32 = 164 bytes
        assert_eq!(data.len(), 164);
        assert_eq!(&data[..4], &evm_selectors::CREATE_HTLC);
    }

    #[test]
    fn evm_claim_calldata_correct() {
        let htlc_id = [0x11; 32];
        let secret = [0x22; 32];
        let data = encode_claim_htlc(&htlc_id, &secret);
        assert_eq!(data.len(), 68); // 4 + 32 + 32
        assert_eq!(&data[..4], &evm_selectors::CLAIM_HTLC);
        assert_eq!(&data[4..36], &htlc_id);
        assert_eq!(&data[36..68], &secret);
    }

    #[test]
    fn uint256_encoding() {
        let val = 1_000_000u128;
        let encoded = encode_uint256(val);
        // Should be big-endian, zero-padded
        assert_eq!(encoded[0..16], [0u8; 16]);
        let mut expected = [0u8; 16];
        expected.copy_from_slice(&val.to_be_bytes());
        assert_eq!(&encoded[16..32], &expected);
    }

    #[test]
    fn address_encoding() {
        let addr = [0xFF; 20];
        let encoded = encode_address(&addr);
        assert_eq!(&encoded[0..12], &[0u8; 12]); // left-padded
        assert_eq!(&encoded[12..32], &addr);
    }

    #[test]
    fn svm_create_length() {
        let hash = [0x33; 32];
        let data = encode_svm_create_htlc(&hash, 3600, 1_000_000);
        assert_eq!(data.len(), 56); // 8 + 32 + 8 + 8
    }

    #[test]
    fn x3_create_length() {
        let recipient = [0; 32];
        let hash = [0; 32];
        let data = encode_x3_create_htlc(&recipient, &hash, 3600, 1_000_000);
        assert_eq!(data.len(), 84); // 4 + 32 + 32 + 8 + 8
    }

    #[test]
    fn decode_htlc_status_funded() {
        let mut data = vec![0u8; 64];
        data[31] = 1; // status = Funded
        let (status, confs) = decode_htlc_status(&data).unwrap();
        assert_eq!(status, HtlcStatus::Funded);
        assert_eq!(confs, 0);
    }

    #[test]
    fn decode_htlc_status_claimed() {
        let mut data = vec![0u8; 64];
        data[31] = 2; // status = Claimed
        let (status, _) = decode_htlc_status(&data).unwrap();
        assert_eq!(status, HtlcStatus::Claimed);
    }

    #[test]
    fn decode_htlc_status_too_short() {
        let data = vec![0u8; 10];
        assert!(decode_htlc_status(&data).is_err());
    }
}
