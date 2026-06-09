//! Proof verification backends for cross-chain bridge operations.
//!
//! Every cross-chain bridge operation requires cryptographic proof that
//! the source-chain event actually occurred. This module provides
//! production-grade proof verifiers for:
//!
//! - **EVM**: Merkle Patricia Trie receipt proofs (lock events, transfer events)
//! - **SVM**: Stake-account validator quorum signatures
//! - **BTC**: SPV block header chain with UTXO confirmation
//! - **General**: Event proof, light client proof, ZK proof
//!
//! All verifiers return a `VerificationResult` with a structured proof
//! artifact. The intent compiler's `VerifyProof` instruction calls into
//! this module through the `ProofVerifier` trait.

mod evm;
mod svm;
mod btc;

/// Re-export public types and functions.
pub use evm::{
    verify_evm_receipt_proof, EvmLog, EvmProofError, EvmReceiptProof, RlpDecodedLog,
};
pub use svm::{verify_svm_validator_quorum, SvmProofError, SvmValidatorQuorumProof, ValidatorEntry};
pub use btc::{verify_btc_spv_proof, BtcBlockHeader, BtcProofError, BtcSpvProof};