/// Bitcoin HTLC Bridge — Hash Time-Locked Contract atomic swaps enabling BTC ↔ X3 trustless trading
/// Implements HTLC construction, preimage validation, and timeout refunds
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_std::vec::Vec;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct HTLCContract {
    pub contract_id: [u8; 32],
    pub initiator: Vec<u8>,     // Bitcoin address
    pub counterparty: [u8; 32], // X3 account
    pub amount_satoshis: u64,
    pub amount_x3: u128,
    pub hash_lock: [u8; 32], // SHA256(preimage)
    pub time_lock: u64,      // Unix timestamp
    pub state: HTLCState,
    pub created_block: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub enum HTLCState {
    Open,
    Redeemed,
    Refunded,
    Expired,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct Preimage {
    pub value: Vec<u8>,
    pub length: u32,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct BitcoinAddress {
    pub address_type: AddressType,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub enum AddressType {
    P2PKH,  // Pay to Public Key Hash
    P2SH,   // Pay to Script Hash
    P2WPKH, // Pay to Witness Public Key Hash (SegWit v0)
    P2TR,   // Pay to Taproot (SegWit v1)
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct BitcoinTxProof {
    pub tx_hash: [u8; 32],
    pub merkle_proof: Vec<[u8; 32]>,
    pub block_header: [u8; 80],
    pub confirmations: u32,
}

pub const MIN_TIMELOCK_SECONDS: u64 = 3600; // Minimum 1 hour
pub const MAX_TIMELOCK_SECONDS: u64 = 2592000; // Maximum 30 days
pub const BTC_CONFIRMED_BLOCKS: u32 = 6; // Standard 6 confirmations

pub struct BitcoinHTLC;

impl BitcoinHTLC {
    /// Create HTLC contract for atomic swap initiation
    pub fn create_contract(
        initiator: Vec<u8>,
        counterparty: [u8; 32],
        amount_satoshis: u64,
        amount_x3: u128,
        hash_lock: [u8; 32],
        time_lock_seconds: u64,
        current_block: u64,
    ) -> Result<HTLCContract, &'static str> {
        if amount_satoshis == 0 || amount_x3 == 0 {
            return Err("Amounts must be positive");
        }
        if hash_lock == [0; 32] {
            return Err("Hash lock cannot be zero");
        }
        if time_lock_seconds < MIN_TIMELOCK_SECONDS || time_lock_seconds > MAX_TIMELOCK_SECONDS {
            return Err("Time lock outside acceptable range");
        }
        if initiator.is_empty() {
            return Err("Invalid initiator address");
        }

        let contract_id = Self::generate_contract_id(&initiator, &counterparty, amount_satoshis);

        Ok(HTLCContract {
            contract_id,
            initiator,
            counterparty,
            amount_satoshis,
            amount_x3,
            hash_lock,
            time_lock: time_lock_seconds,
            state: HTLCState::Open,
            created_block: current_block,
        })
    }

    /// Redeem HTLC by revealing preimage
    pub fn redeem(
        contract: &mut HTLCContract,
        preimage: &Preimage,
        current_block: u64,
    ) -> Result<[u8; 32], &'static str> {
        if contract.state != HTLCState::Open {
            return Err("Contract is not in Open state");
        }

        // Check timelock not exceeded
        if current_block > contract.created_block + (contract.time_lock / 15) {
            // ~15 seconds per Bitcoin block
            return Err("Time lock expired");
        }

        // Validate preimage matches hash lock
        let computed_hash = Self::sha256(&preimage.value);
        if computed_hash != contract.hash_lock {
            return Err("Preimage does not match hash lock");
        }

        contract.state = HTLCState::Redeemed;
        Ok(computed_hash)
    }

    /// Refund HTLC after timeout (only initiator can call)
    pub fn refund(
        contract: &mut HTLCContract,
        refunder: Vec<u8>,
        current_block: u64,
    ) -> Result<(), &'static str> {
        if contract.state != HTLCState::Open {
            return Err("Contract is not in Open state");
        }

        // Only initiator can refund
        if refunder != contract.initiator {
            return Err("Only initiator can refund");
        }

        // Require timelock expiry
        let blocks_passed = current_block.saturating_sub(contract.created_block);
        let timelock_blocks = contract.time_lock / 15; // ~15 seconds per block

        if blocks_passed < timelock_blocks {
            return Err("Time lock not yet expired");
        }

        contract.state = HTLCState::Refunded;
        Ok(())
    }

    /// Verify Bitcoin transaction proof (simplified merkle proof verification)
    pub fn verify_btc_tx(tx_hash: [u8; 32], proof: &BitcoinTxProof) -> Result<bool, &'static str> {
        if tx_hash == [0; 32] {
            return Err("Transaction hash cannot be zero");
        }
        if proof.confirmations < BTC_CONFIRMED_BLOCKS {
            return Err("Insufficient confirmations");
        }

        // Verify merkle proof: check if tx_hash is in merkle tree rooted at block header
        let merkle_root = Self::compute_merkle_root(&tx_hash, &proof.merkle_proof);

        // Extract merkle root from block header (bytes 36-68)
        let header_merkle_root = &proof.block_header[36..68];
        let mut extracted = [0u8; 32];
        extracted.copy_from_slice(header_merkle_root);

        Ok(merkle_root == extracted)
    }

    /// Validate Bitcoin address format
    pub fn validate_address(address: &BitcoinAddress) -> Result<bool, &'static str> {
        match address.address_type {
            AddressType::P2PKH => {
                // P2PKH: 25 bytes (1 byte version + 20 bytes hash + 4 bytes checksum)
                if address.bytes.len() != 25 {
                    return Err("P2PKH address invalid length");
                }
            }
            AddressType::P2SH => {
                // P2SH: 25 bytes
                if address.bytes.len() != 25 {
                    return Err("P2SH address invalid length");
                }
            }
            AddressType::P2WPKH => {
                // P2WPKH: 22 bytes (1 version + 1 length + 20 bytes pubkey hash)
                if address.bytes.len() != 22 {
                    return Err("P2WPKH address invalid length");
                }
            }
            AddressType::P2TR => {
                // P2TR: 34 bytes (1 version + 1 length + 32 bytes taproot output key)
                if address.bytes.len() != 34 {
                    return Err("P2TR address invalid length");
                }
            }
        }

        Ok(true)
    }

    /// Compute HTLC script (Bitcoin Script)
    pub fn compute_htlc_script(
        initiator_pubkey: Vec<u8>,
        counterparty_pubkey: Vec<u8>,
        hash_lock: [u8; 32],
        time_lock: u64,
    ) -> Result<Vec<u8>, &'static str> {
        if initiator_pubkey.is_empty() || counterparty_pubkey.is_empty() {
            return Err("Public keys cannot be empty");
        }

        // Standard HTLC script (simplified):
        // OP_IF
        //   OP_SHA256 <hash_lock> OP_EQUALVERIFY OP_DUP OP_HASH160 <counterparty_pubkey>
        // OP_ELSE
        //   <time_lock> OP_CHECKLOCKTIMEVERIFY OP_DROP OP_DUP OP_HASH160 <initiator_pubkey>
        // OP_ENDIF
        // OP_EQUALVERIFY OP_CHECKSIG

        let mut script = Vec::new();
        script.push(0x63); // OP_IF
        script.push(0xa8); // OP_SHA256
        script.extend_from_slice(&hash_lock);
        script.push(0x87); // OP_EQUALVERIFY
        script.push(0x76); // OP_DUP
        script.push(0xa1); // OP_HASH160
        script.extend_from_slice(&counterparty_pubkey);
        script.push(0x67); // OP_ELSE
        script.extend_from_slice(&time_lock.to_le_bytes());
        script.push(0xb1); // OP_CHECKLOCKTIMEVERIFY
        script.push(0x75); // OP_DROP
        script.push(0x76); // OP_DUP
        script.push(0xa1); // OP_HASH160
        script.extend_from_slice(&initiator_pubkey);
        script.push(0x68); // OP_ENDIF
        script.push(0x88); // OP_EQUALVERIFY
        script.push(0xac); // OP_CHECKSIG

        Ok(script)
    }

    /// Hash preimage using SHA256
    fn sha256(data: &[u8]) -> [u8; 32] {
        sp_io::hashing::sha2_256(data)
    }

    /// Generate deterministic contract ID
    fn generate_contract_id(initiator: &[u8], counterparty: &[u8; 32], amount: u64) -> [u8; 32] {
        let mut data = Vec::with_capacity(initiator.len() + counterparty.len() + 8);
        data.extend_from_slice(initiator);
        data.extend_from_slice(counterparty);
        data.extend_from_slice(&amount.to_le_bytes());
        sp_io::hashing::sha2_256(&data)
    }

    /// Compute merkle root from leaf and proof path
    fn compute_merkle_root(leaf: &[u8; 32], proofs: &[[u8; 32]]) -> [u8; 32] {
        let mut hash = *leaf;
        for proof_node in proofs {
            hash = Self::hash_pair(&hash, proof_node);
        }
        hash
    }

    /// Hash pair of 32-byte values using Bitcoin merkle double-SHA256
    fn hash_pair(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
        let (left, right) = if a <= b { (a, b) } else { (b, a) };

        let mut combined = [0u8; 64];
        combined[..32].copy_from_slice(left);
        combined[32..].copy_from_slice(right);

        let first = sp_io::hashing::sha2_256(&combined);
        sp_io::hashing::sha2_256(&first)
    }

    /// Get contract state
    pub fn get_contract_state(contract: &HTLCContract) -> (HTLCState, u64, u64) {
        (
            contract.state.clone(),
            contract.amount_satoshis,
            contract.amount_x3 as u64,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_contract() {
        let initiator = b"1A1z7agoat".to_vec();
        let counterparty = [1; 32];
        let hash_lock = [2; 32];

        let contract = BitcoinHTLC::create_contract(
            initiator,
            counterparty,
            1000000,
            500u128 * 100000000, // 5 X3 tokens
            hash_lock,
            86400, // 1 day
            0,
        )
        .unwrap();

        assert_eq!(contract.amount_satoshis, 1000000);
        assert_eq!(contract.state, HTLCState::Open);
    }

    #[test]
    fn test_create_contract_zero_amount() {
        let result = BitcoinHTLC::create_contract(
            b"1A1z7agoat".to_vec(),
            [1; 32],
            0,
            100u128,
            [2; 32],
            86400,
            0,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_create_contract_zero_hash_lock() {
        let result = BitcoinHTLC::create_contract(
            b"1A1z7agoat".to_vec(),
            [1; 32],
            1000000,
            100u128,
            [0; 32],
            86400,
            0,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_timelock_too_short() {
        let result = BitcoinHTLC::create_contract(
            b"1A1z7agoat".to_vec(),
            [1; 32],
            1000000,
            100u128,
            [2; 32],
            600, // 10 minutes (too short)
            0,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_redeem_correct_preimage() {
        let preimage = b"secret".to_vec();
        let hash_lock = BitcoinHTLC::sha256(&preimage);
        let expected_secret_sha256 = [
            0x2b, 0xb8, 0x0d, 0x53, 0x7b, 0x1d, 0xa3, 0xe3, 0x8b, 0xd3, 0x03, 0x61, 0xaa, 0x85,
            0x56, 0x86, 0xbd, 0xe0, 0xea, 0xcd, 0x71, 0x62, 0xfe, 0xf6, 0xa2, 0x5f, 0xe9, 0x7b,
            0xf5, 0x27, 0xa2, 0x5b,
        ];
        assert_eq!(hash_lock, expected_secret_sha256);

        let mut contract = BitcoinHTLC::create_contract(
            b"1A1z7agoat".to_vec(),
            [1; 32],
            1000000,
            100u128,
            hash_lock,
            86400,
            0,
        )
        .unwrap();

        let preimage_obj = Preimage {
            value: preimage,
            length: 6,
        };

        let redeemed_hash = BitcoinHTLC::redeem(&mut contract, &preimage_obj, 1).unwrap();
        assert_eq!(redeemed_hash, expected_secret_sha256);
        assert_eq!(contract.state, HTLCState::Redeemed);
    }

    #[test]
    fn test_redeem_wrong_preimage() {
        let hash_lock = BitcoinHTLC::sha256(b"correct_secret");

        let mut contract = BitcoinHTLC::create_contract(
            b"1A1z7agoat".to_vec(),
            [1; 32],
            1000000,
            100u128,
            hash_lock,
            86400,
            0,
        )
        .unwrap();

        let wrong_preimage = Preimage {
            value: b"wrong_secret".to_vec(),
            length: 12,
        };

        let result = BitcoinHTLC::redeem(&mut contract, &wrong_preimage, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_refund_after_timeout() {
        let mut contract = BitcoinHTLC::create_contract(
            b"1A1z7agoat".to_vec(),
            [1; 32],
            1000000,
            100u128,
            [2; 32],
            86400, // 1 day = ~5760 blocks @ 15 sec each
            0,
        )
        .unwrap();

        // Refund after sufficient blocks
        BitcoinHTLC::refund(&mut contract, b"1A1z7agoat".to_vec(), 6000).unwrap();
        assert_eq!(contract.state, HTLCState::Refunded);
    }

    #[test]
    fn test_refund_before_timeout() {
        let mut contract = BitcoinHTLC::create_contract(
            b"1A1z7agoat".to_vec(),
            [1; 32],
            1000000,
            100u128,
            [2; 32],
            86400,
            0,
        )
        .unwrap();

        let result = BitcoinHTLC::refund(&mut contract, b"1A1z7agoat".to_vec(), 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_p2pkh_address() {
        let address = BitcoinAddress {
            address_type: AddressType::P2PKH,
            bytes: vec![0; 25],
        };

        assert!(BitcoinHTLC::validate_address(&address).unwrap());
    }

    #[test]
    fn test_validate_p2wpkh_address() {
        let address = BitcoinAddress {
            address_type: AddressType::P2WPKH,
            bytes: vec![0; 22],
        };

        assert!(BitcoinHTLC::validate_address(&address).unwrap());
    }

    #[test]
    fn test_compute_htlc_script() {
        let script =
            BitcoinHTLC::compute_htlc_script(vec![1, 2, 3], vec![4, 5, 6], [7; 32], 86400).unwrap();

        assert!(!script.is_empty());
        assert!(script.len() > 50);
    }

    #[test]
    fn test_get_contract_state() {
        let contract = BitcoinHTLC::create_contract(
            b"1A1z7agoat".to_vec(),
            [1; 32],
            1000000,
            500u128,
            [2; 32],
            86400,
            0,
        )
        .unwrap();

        let (state, amount_sat, amount_x3) = BitcoinHTLC::get_contract_state(&contract);
        assert_eq!(state, HTLCState::Open);
        assert_eq!(amount_sat, 1000000);
        assert_eq!(amount_x3, 500);
    }

    #[test]
    fn test_verify_btc_tx_insufficient_confirmations() {
        let proof = BitcoinTxProof {
            tx_hash: [1; 32],
            merkle_proof: vec![[2; 32]],
            block_header: [0; 80],
            confirmations: 3, // Less than 6 required
        };

        let result = BitcoinHTLC::verify_btc_tx([1; 32], &proof);
        assert!(result.is_err());
    }

    #[test]
    fn test_sha256() {
        let hash_hello = BitcoinHTLC::sha256(b"hello");
        let expected_hello = [
            0x2c, 0xf2, 0x4d, 0xba, 0x5f, 0xb0, 0xa3, 0x0e, 0x26, 0xe8, 0x3b, 0x2a, 0xc5, 0xb9,
            0xe2, 0x9e, 0x1b, 0x16, 0x1e, 0x5c, 0x1f, 0xa7, 0x42, 0x5e, 0x73, 0x04, 0x33, 0x62,
            0x93, 0x8b, 0x98, 0x24,
        ];
        assert_eq!(hash_hello, expected_hello);

        let hash_abc = BitcoinHTLC::sha256(b"abc");
        let expected_abc = [
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
            0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
            0xf2, 0x00, 0x15, 0xad,
        ];
        assert_eq!(hash_abc, expected_abc);
    }

    #[test]
    fn test_contract_id_deterministic() {
        let id1 = BitcoinHTLC::generate_contract_id(b"1A1z7agoat", &[1; 32], 1000000);
        let id2 = BitcoinHTLC::generate_contract_id(b"1A1z7agoat", &[1; 32], 1000000);
        assert_eq!(id1, id2);
    }
}
