//! X3 Atomic Star Interchain Orchestrator.
//!
//! Provides cross-VM message routing, proof verification, replay protection,
//! and canonical supply invariants across EVM, SVM, and X3VM execution
//! domains. Designed as a grant-ready MultiVM orchestration layer that wires
//! into the broader X3 stack (`x3-bridge`, `x3-vm`, `x3-dex`,
//! `x3-supply-ledger`).

pub mod adapters;
pub mod error;
pub mod executor;
pub mod invariant;
pub mod message;
pub mod proof;
pub mod registry;
pub mod replay;
pub mod router;
pub mod types;

pub use error::{OrchestratorError, Result};
pub use executor::VmExecutor;
pub use invariant::CanonicalSupplySnapshot;
pub use message::CrossVmMessage;
pub use proof::{ExecutionProof, MockProofVerifier, ProofVerifier};
pub use registry::{AdapterRegistry, ChainAdapter};
pub use replay::ReplayGuard;
pub use router::OrchestratorRouter;
pub use types::{ChainId, MessageStatus, VmKind};
