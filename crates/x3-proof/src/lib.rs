//! # X3 Proof Engine
//!
//! Deterministic execution proof generation and verification for the X3 jurisdiction.
//!
//! Every execution within X3 produces a cryptographic proof chain that can be
//! independently verified by any observer. Proofs are the foundation of the
//! court system — disputes are resolved by deterministic replay, not voting.
//!
//! ## Architecture
//!
//! - **ExecutionProof**: Captures a single atomic execution step
//! - **StateProof**: Captures a state transition with before/after hashes
//! - **ProofChain**: An ordered sequence of proofs forming a complete execution trace
//! - **ProofEngine**: Generates proofs during VM execution
//! - **ProofVerifier**: Independently verifies proof chains against replay

pub mod chain;
pub mod engine;
pub mod epoch;
pub mod error;
pub mod finality_registry;
pub mod hasher;
pub mod types;
pub mod verifier;

pub use chain::ProofChain;
pub use engine::ProofEngine;
pub use engine::ProofEngineConfig;
pub use epoch::{
    EpochProof, RecursiveProofAggregator, ZkBlockProof, ZkBlockVerifier, ZkProofError,
    BLOCKS_PER_EPOCH,
};
pub use error::ProofError;
pub use finality_registry::{FinalityError, FinalityRecord, FinalityRegistry};
pub use hasher::DeterministicHasher;
pub use types::*;
pub use verifier::ProofVerifier;
