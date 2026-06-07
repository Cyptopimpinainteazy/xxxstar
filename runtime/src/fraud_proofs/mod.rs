// runtime/src/fraud_proofs/mod.rs
//
// Fraud-proof verification logic for X3 cross-VM scheduler commitments.

pub mod committee;
pub mod freeze;
pub mod pallet;
pub mod scheduler_v1;
pub mod types;
pub mod verifier;
pub mod witness_v1;

// startup_gate is std-only (runs before consensus join, not inside the WASM runtime).
#[cfg(feature = "std")]
pub mod startup_gate;

pub use committee::{
    select_committee, try_select_committee, CommitteeError, DEFAULT_COMMITTEE_SIZE,
    MAX_COMMITTEE_SIZE,
};
pub use freeze::{engaged_state, freeze_summary, is_frozen_from_bytes, FreezeReason, FreezeState};
pub use pallet::pallet as fraud_proof_pallet;
pub use scheduler_v1::{recompute_from_bytes, scheduler_commitment_from_bytes};
pub use types::{DisputedBlockMeta, FraudProofV1, HeaderRef, PROOF_TYPE_SCHED_MISMATCH_V1};
pub use verifier::{compute_proof_id, verify_scheduler_mismatch_v1, VerifyError};
pub use witness_v1::{
    AccessKeyV1, AccessListV1, SchedulerCommitments, SchedulerWitnessV1, WitnessError,
    WITNESS_VERSION,
};
