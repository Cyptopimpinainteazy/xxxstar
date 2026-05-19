//! # PoAE Proof Types
//!
//! Defines the Proof of Atomic Execution (PoAE) data structures for:
//! - On-chain storage in `pallet-x3-atomic-kernel`
//! - Off-chain verification by external chain verifiers (EVM contracts, SVM programs)
//! - RPC serialization for frontends and indexers

use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::H256;

/// A declared access set for a single bundle leg.
///
/// Per the audit: "Every extrinsic submitted through the parallel path must
/// include a DeclaredAccess: reads/writes lists. The proposer validates that
/// shards are conflict-free on writes."
#[derive(
    Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub struct DeclaredAccess {
    /// Storage keys / account identifiers this leg will read.  
    pub reads: frame_support::BoundedVec<H256, sp_runtime::traits::ConstU32<64>>,
    /// Storage keys / account identifiers this leg will write.
    pub writes: frame_support::BoundedVec<H256, sp_runtime::traits::ConstU32<64>>,
}

/// VM type for a bundle leg.
#[derive(
    Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub enum VmType {
    /// Ethereum Virtual Machine leg.
    Evm,
    /// Solana Virtual Machine leg.
    Svm,
    /// X3 native leg.
    X3,
    /// Cross-VM leg (spans multiple VMs).
    Cross,
}

/// A single atomic trade leg within a bundle.
#[derive(
    Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub struct BundleLeg {
    /// Target VM for this leg.
    pub vm_type: VmType,
    /// Input token identifier.
    pub token_in: H256,
    /// Output token identifier.  
    pub token_out: H256,
    /// Amount to swap (in smallest token units).
    pub amount_in: u128,
    /// Minimum acceptable output (slippage guard).
    pub min_amount_out: u128,
    /// Unix timestamp after which this leg is invalid.
    pub deadline: u64,
    /// Declared read/write accounts (enables parallel scheduling).
    pub access: DeclaredAccess,
}

/// Proof of Atomic Execution — the canonical artifact for cross-chain settlement.
///
/// ## How to Verify (external chain)
///
/// 1. Decode the proof from the X3 RPC endpoint or on-chain storage.
/// 2. Verify `finality_cert` is a valid GRANDPA justification (or Flash
///    Finality certificate) for `finalized_block` on X3 chain.
/// 3. Verify `receipt_root` matches the claimed execution outcomes.
/// 4. Verify `legs_hash` matches the original submission (non-repudiation).
/// 5. If all checks pass: release side-effects on the external chain.
///
/// ## Anchor Chain
///
/// ```text
/// Bundle submission tx
///   → included in block B
///     → B finalized by GRANDPA (justification J)
///       → finality_cert = H256(J)
///         → PoaeProof stored on X3 chain
///           → external verifier reads proof via RPC or state proof
///             → settles cross-chain payment
/// ```
#[derive(
    Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub struct PoaeProof {
    /// Unique bundle identifier (derived deterministically from submitter+block+legs).
    pub bundle_id: H256,
    /// Merkle root of execution receipts from X3 Kernel (one receipt per leg).
    pub receipt_root: H256,
    /// Block number on X3 chain where the bundle was finalized.
    pub finalized_block: u64,
    /// Hash of the finality certificate (GRANDPA justification or Flash cert).
    pub finality_cert: H256,
    /// Hash of the original bundle legs (non-repudiation: proves what was executed).
    pub legs_hash: H256,
    /// Number of legs successfully executed.
    pub leg_count: u32,
}

impl PoaeProof {
    /// Compute a stable proof hash for use in external verifier contracts.
    ///
    /// In EVM Solidity: `keccak256(abi.encode(bundle_id, receipt_root, finalized_block, finality_cert))`
    /// Here we use a SCALE-encoded SHA-256 for on-chain use.
    pub fn proof_hash(&self) -> H256 {
        use sp_io::hashing::sha2_256;
        let mut data = self.bundle_id.as_bytes().to_vec();
        data.extend_from_slice(self.receipt_root.as_bytes());
        data.extend_from_slice(&self.finalized_block.to_le_bytes());
        data.extend_from_slice(self.finality_cert.as_bytes());
        data.extend_from_slice(self.legs_hash.as_bytes());
        H256(sha2_256(&data))
    }

    /// Validate the basic structural consistency of a proof.
    /// Does NOT verify the finality certificate cryptographically
    /// (that requires access to the GRANDPA authority set).
    pub fn validate_structure(&self) -> bool {
        self.bundle_id != H256::zero()
            && self.receipt_root != H256::zero()
            && self.finalized_block > 0
            && self.finality_cert != H256::zero()
            && self.leg_count > 0
    }
}
