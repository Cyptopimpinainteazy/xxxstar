//! Proof verification backends for cross-chain bridge operations.
//!
//! Every cross-chain bridge operation requires cryptographic proof that the
//! source-chain event actually occurred. This module holds three verifiers:
//!
//! - **EVM** (`evm`): receipt inclusion, delegated to
//!   [`x3_verification_router::evm_receipt::verify_merkle_patricia_proof`] — the
//!   canonical Merkle Patricia verifier, the one the relayer is wired to. This crate
//!   does not carry its own trie walk.
//! - **SVM** (`svm`): stake-account validator quorum signatures.
//! - **BTC** (`btc`): SPV block header chain with UTXO confirmation.
//!
//! # Who calls this, and who does not
//!
//! Nothing in this workspace calls these functions. The previous version of this
//! comment said "the intent compiler's `VerifyProof` instruction calls into this
//! module through the `ProofVerifier` trait", which was not true: `ProofVerifier` does
//! not appear in the intent compiler, and the trait of that name lives in
//! `x3-orchestrator`, which does not depend on this crate (TICKET-065). The verifiers
//! here are reachable as library API and covered by their own tests; a caller that
//! needs one has to call it.

mod btc;
mod evm;
mod svm;

pub use btc::{verify_btc_spv_proof, BtcBlockHeader, BtcProofError, BtcSpvProof};
/// Re-export public types and functions.
pub use evm::{verify_evm_receipt_proof, EvmLog, EvmProofError, EvmReceiptProof, RlpDecodedLog};
pub use svm::{
    verify_svm_validator_quorum, SvmProofError, SvmValidatorQuorumProof, ValidatorEntry,
};
