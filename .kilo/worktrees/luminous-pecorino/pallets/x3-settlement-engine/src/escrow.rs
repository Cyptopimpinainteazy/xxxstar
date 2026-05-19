//! Cross-VM Escrow Module
//!
//! Manages atomic escrow operations across EVM, SVM, and X3VM.
//! Because X3 hosts all three VMs, internal swaps are atomic within a single block.

use crate::types::{EscrowLeg, EscrowLegState, ExternalChainId};
use codec::{Decode, DecodeWithMemTracking, Encode};
use core::fmt::Debug;
use scale_info::TypeInfo;
use sp_core::H256;
use sp_std::{vec, vec::Vec};

/// Escrow operation to be executed atomically
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo)]
pub enum EscrowOp<AccountId, Balance> {
    /// Lock assets into escrow
    Lock {
        depositor: AccountId,
        amount: Balance,
        chain: ExternalChainId,
        escrow_data: Vec<u8>,
    },
    /// Release assets to recipient
    Release {
        recipient: AccountId,
        amount: Balance,
    },
    /// Refund assets to depositor
    Refund {
        depositor: AccountId,
        amount: Balance,
    },
}

/// Batch of escrow operations (for atomic execution)
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo)]
pub struct EscrowBatch<AccountId, Balance> {
    /// Intent ID this batch belongs to
    pub intent_id: H256,
    /// Operations to execute
    pub operations: Vec<EscrowOp<AccountId, Balance>>,
    /// Whether all ops must succeed (atomic)
    pub atomic: bool,
}

/// Cross-VM escrow manager
pub struct CrossVmEscrow;

impl CrossVmEscrow {
    /// Check if escrow operation is valid for given chain
    pub fn validate_escrow_op<AccountId, Balance>(
        op: &EscrowOp<AccountId, Balance>,
        chain: &ExternalChainId,
    ) -> bool {
        match op {
            EscrowOp::Lock {
                chain: op_chain, ..
            } => {
                // Verify chain matches
                op_chain == chain
            }
            EscrowOp::Release { .. } | EscrowOp::Refund { .. } => true,
        }
    }

    /// Generate escrow address/script for chain
    pub fn generate_escrow_address(
        chain: &ExternalChainId,
        secret_hash: &H256,
        _maker: &[u8],
        _taker: &[u8],
        timeout: u64,
    ) -> Vec<u8> {
        match chain {
            ExternalChainId::Bitcoin | ExternalChainId::BitcoinTestnet => {
                // Generate BTC HTLC P2SH address
                Self::generate_btc_htlc_address(secret_hash, timeout)
            }
            ExternalChainId::Ethereum
            | ExternalChainId::Arbitrum
            | ExternalChainId::Base
            | ExternalChainId::Polygon
            | ExternalChainId::Optimism
            | ExternalChainId::Avalanche
            | ExternalChainId::Bnb
            | ExternalChainId::EvmChain(_) => {
                // Generate EVM HTLC contract address
                Self::generate_evm_htlc_address(secret_hash)
            }
            ExternalChainId::Solana | ExternalChainId::SolanaDevnet => {
                // Generate Solana escrow PDA
                Self::generate_svm_escrow_pda(secret_hash)
            }
            ExternalChainId::X3Native => {
                // Internal X3 escrow (account-based)
                Self::generate_x3_escrow_account(secret_hash)
            }
        }
    }

    fn generate_btc_htlc_address(secret_hash: &H256, _timeout: u64) -> Vec<u8> {
        // P2SH address prefix + hash of HTLC script
        let mut addr = vec![0x05]; // Mainnet P2SH
        addr.extend_from_slice(&secret_hash.as_bytes()[..20]);
        addr
    }

    fn generate_evm_htlc_address(secret_hash: &H256) -> Vec<u8> {
        // CREATE2 address: keccak256(0xff ++ factory ++ salt ++ init_code_hash)[12:]
        let mut data = Vec::with_capacity(85);
        data.push(0xff);
        data.extend_from_slice(&[0u8; 20]); // Factory address placeholder
        data.extend_from_slice(secret_hash.as_bytes()); // Salt
        data.extend_from_slice(&[0u8; 32]); // Init code hash placeholder

        let hash = sp_io::hashing::keccak_256(&data);
        hash[12..32].to_vec()
    }

    fn generate_svm_escrow_pda(secret_hash: &H256) -> Vec<u8> {
        // Solana PDA: hash(seeds, program_id, bump)
        let mut seeds = Vec::with_capacity(64);
        seeds.extend_from_slice(b"escrow");
        seeds.extend_from_slice(secret_hash.as_bytes());

        let hash = sp_io::hashing::sha2_256(&seeds);
        hash.to_vec()
    }

    fn generate_x3_escrow_account(secret_hash: &H256) -> Vec<u8> {
        // X3 escrow: deterministic account from secret hash
        let mut data = Vec::with_capacity(40);
        data.extend_from_slice(b"x3escrow");
        data.extend_from_slice(secret_hash.as_bytes());

        let hash = sp_io::hashing::blake2_256(&data);
        hash.to_vec()
    }
}

/// EVM HTLC contract interface
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo)]
pub struct EvmHtlcParams {
    /// Secret hash (32 bytes)
    pub secret_hash: H256,
    /// Recipient address (20 bytes)
    pub recipient: [u8; 20],
    /// Refund address (20 bytes)
    pub refund_address: [u8; 20],
    /// Timeout timestamp
    pub timeout: u64,
    /// Token address (zero for ETH)
    pub token: [u8; 20],
    /// Amount
    pub amount: u128,
}

impl EvmHtlcParams {
    /// Encode as EVM call data (for createHTLC)
    pub fn encode_calldata(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(196);

        // Function selector: createHTLC(bytes32,address,address,uint256,uint256)
        data.extend_from_slice(&[0x4b, 0x2f, 0x33, 0x6d]);

        // secret_hash (bytes32)
        data.extend_from_slice(self.secret_hash.as_bytes());

        // recipient (address) - padded to 32 bytes
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(&self.recipient);

        // refund_address (address) - padded to 32 bytes
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(&self.refund_address);

        // timeout (uint256)
        let mut timeout_bytes = [0u8; 32];
        timeout_bytes[24..].copy_from_slice(&self.timeout.to_be_bytes());
        data.extend_from_slice(&timeout_bytes);

        // token (address) - padded to 32 bytes
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(&self.token);

        // amount (uint256)
        let mut amount_bytes = [0u8; 32];
        amount_bytes[16..].copy_from_slice(&self.amount.to_be_bytes());
        data.extend_from_slice(&amount_bytes);

        data
    }

    /// Encode claim call data
    pub fn encode_claim_calldata(secret: &H256) -> Vec<u8> {
        let mut data = Vec::with_capacity(36);

        // Function selector: claimHTLC(bytes32,bytes32)
        data.extend_from_slice(&[0x84, 0xcc, 0x31, 0x5c]);

        // secret (bytes32)
        data.extend_from_slice(secret.as_bytes());

        data
    }

    /// Encode refund call data
    pub fn encode_refund_calldata() -> Vec<u8> {
        // Function selector: refundHTLC(bytes32)
        vec![0x72, 0x49, 0xfb, 0xb6]
    }
}

/// Solana escrow program interface
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo)]
pub struct SvmEscrowParams {
    /// Secret hash
    pub secret_hash: H256,
    /// Recipient pubkey (32 bytes)
    pub recipient: [u8; 32],
    /// Refund pubkey (32 bytes)
    pub refund_authority: [u8; 32],
    /// Timeout slot
    pub timeout_slot: u64,
    /// Token mint (32 bytes, or zero for SOL)
    pub token_mint: [u8; 32],
    /// Amount in lamports/tokens
    pub amount: u64,
}

impl SvmEscrowParams {
    /// Generate escrow PDA
    pub fn escrow_pda(&self) -> [u8; 32] {
        let mut seeds = Vec::with_capacity(96);
        seeds.extend_from_slice(b"escrow");
        seeds.extend_from_slice(self.secret_hash.as_bytes());
        seeds.extend_from_slice(&self.recipient);

        let hash = sp_io::hashing::sha2_256(&seeds);
        let mut pda = [0u8; 32];
        pda.copy_from_slice(&hash);
        pda
    }

    /// Encode initialize instruction
    pub fn encode_initialize_ix(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(128);

        // Instruction discriminator
        data.push(0); // Initialize = 0

        // Secret hash
        data.extend_from_slice(self.secret_hash.as_bytes());

        // Timeout
        data.extend_from_slice(&self.timeout_slot.to_le_bytes());

        // Amount
        data.extend_from_slice(&self.amount.to_le_bytes());

        data
    }

    /// Encode claim instruction
    pub fn encode_claim_ix(secret: &H256) -> Vec<u8> {
        let mut data = Vec::with_capacity(33);

        // Instruction discriminator
        data.push(1); // Claim = 1

        // Secret preimage
        data.extend_from_slice(secret.as_bytes());

        data
    }

    /// Encode refund instruction
    pub fn encode_refund_ix() -> Vec<u8> {
        vec![2] // Refund = 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evm_htlc_calldata_encoding() {
        let params = EvmHtlcParams {
            secret_hash: H256::repeat_byte(0xAB),
            recipient: [0x11; 20],
            refund_address: [0x22; 20],
            timeout: 1700000000,
            token: [0; 20],                    // Native ETH
            amount: 1_000_000_000_000_000_000, // 1 ETH
        };

        let calldata = params.encode_calldata();
        assert!(calldata.len() >= 196);
        assert_eq!(&calldata[0..4], &[0x4b, 0x2f, 0x33, 0x6d]);
    }

    #[test]
    fn test_evm_htlc_claim_and_refund_selector_prefixes() {
        let claim_secret = H256::repeat_byte(0x01);
        let claim_calldata = EvmHtlcParams::encode_claim_calldata(&claim_secret);
        assert_eq!(&claim_calldata[0..4], &[0x84, 0xcc, 0x31, 0x5c]);

        let refund_calldata = EvmHtlcParams::encode_refund_calldata();
        assert_eq!(refund_calldata, vec![0x72, 0x49, 0xfb, 0xb6]);
    }

    #[test]
    fn test_svm_escrow_pda_generation() {
        let params = SvmEscrowParams {
            secret_hash: H256::repeat_byte(0xCD),
            recipient: [0x33; 32],
            refund_authority: [0x44; 32],
            timeout_slot: 200_000_000,
            token_mint: [0; 32],   // Native SOL
            amount: 1_000_000_000, // 1 SOL
        };

        let pda = params.escrow_pda();
        assert_ne!(pda, [0; 32]);
    }

    #[test]
    fn test_escrow_address_generation() {
        let secret_hash = H256::repeat_byte(0xEF);

        // BTC
        let btc_addr = CrossVmEscrow::generate_escrow_address(
            &ExternalChainId::Bitcoin,
            &secret_hash,
            &[],
            &[],
            1000,
        );
        assert!(!btc_addr.is_empty());

        // EVM
        let evm_addr = CrossVmEscrow::generate_escrow_address(
            &ExternalChainId::Ethereum,
            &secret_hash,
            &[],
            &[],
            1000,
        );
        assert_eq!(evm_addr.len(), 20);

        // SVM
        let svm_addr = CrossVmEscrow::generate_escrow_address(
            &ExternalChainId::Solana,
            &secret_hash,
            &[],
            &[],
            1000,
        );
        assert_eq!(svm_addr.len(), 32);
    }
}
