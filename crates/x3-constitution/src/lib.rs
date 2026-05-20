//! # X3 Constitution
//!
//! The machine-verifiable constitution of the X3 blockchain jurisdiction.
//!
//! ## Articles
//!
//! - **Article I — Sovereignty**: Authority derives from mathematical correctness.
//! - **Article II — Execution**: All computation must be deterministic, bounded,
//!   and provably invariant-compliant.
//! - **Article III — Safety Invariants**: Core invariants are immutable except via
//!   formally-proven refinement.
//! - **Article IV — Governance**: Governance may propose, but not bypass proof requirements.
//! - **Article V — Amendments**: Amendments must prove refinement, meta-invariant
//!   preservation, termination, and safety.
//! - **Article VI — Enforcement**: Violations trigger automatic slashing, halt, and
//!   forensic replay.
//!
//! The canonical hash of this module is checkpointed on-chain and compared on every
//! invariant-touching operation.

pub mod amendment;
pub mod articles;
pub mod engine;
pub mod error;
pub mod invariants;
pub mod types;

pub use amendment::{AmendmentProof, AmendmentVerifier};
pub use articles::{Article, ConstitutionManifest};
pub use engine::ConstitutionEngine;
pub use error::ConstitutionError;
pub use invariants::{CoreInvariant, InvariantSet, InvariantViolation};
pub use types::{ConstitutionHash, InvariantBounds};
