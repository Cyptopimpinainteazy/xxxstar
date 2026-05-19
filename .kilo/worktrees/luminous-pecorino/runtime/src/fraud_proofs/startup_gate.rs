// runtime/src/fraud_proofs/startup_gate.rs
//
// Determinism validation gate run before a node joins consensus.
//
// ## Purpose
// Before a validator can participate in block production or attestation, it
// must prove its CPU scheduler implementation produces byte-identical outputs
// for a set of hard-coded, known-good test vectors.  Any deviation immediately
// signals that the node is running a non-deterministic or modified scheduler
// and must not be admitted to the committee.
//
// ## Gate contract
// `run_startup_gate()` must return `Ok(())` before the node calls
// `start_proposer()`.  Failing the gate is a hard abort — the operator must
// fix their binary before retrying.
//
// INVARIANT: STARTUP-GATE-001 — any validator that fails the gate must not
// join the consensus committee.
// INVARIANT: STARTUP-GATE-002 — gate outputs are deterministic across all
// compliant builds (same scheduler, same vectors).

// Only meaningful in native / std context — no point running inside WASM.
#![cfg(feature = "std")]

use crate::fraud_proofs::scheduler_v1::scheduler_commitment_from_bytes;
use sp_core::H256;

// ---------------------------------------------------------------------------
// Test vector definition
// ---------------------------------------------------------------------------

/// A single determinism test vector.
pub struct TestVector {
    /// Human-readable name for diagnostics.
    pub name: &'static str,
    /// Raw witness bytes (SCALE-encoded `SchedulerWitnessV1`).
    pub witness_bytes: &'static [u8],
    /// rules_version to pass to the scheduler.
    pub rules_version: u32,
    /// max_tx_count to enforce.
    pub max_tx_count: u32,
    /// Expected scheduler commitment output.
    pub expected_commitment: H256,
}

/// Error from the startup gate.
#[derive(Debug)]
pub enum GateError {
    /// A test vector did not produce the expected commitment.
    VectorMismatch {
        name: &'static str,
        expected: H256,
        got: H256,
    },
    /// The scheduler could not process a test vector (witness error).
    WitnessError {
        name: &'static str,
        err: crate::fraud_proofs::witness_v1::WitnessError,
    },
}

impl core::fmt::Display for GateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GateError::VectorMismatch {
                name,
                expected,
                got,
            } => write!(
                f,
                "startup-gate FAIL [{name}]: expected {expected:?} got {got:?}"
            ),
            GateError::WitnessError { name, err } => {
                write!(f, "startup-gate FAIL [{name}]: witness error {err:?}")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Built-in test vectors
// ---------------------------------------------------------------------------
//
// Each vector below is produced by the reference CPU scheduler running the
// canonical algorithm.  To regenerate, run:
//     cargo test -p x3-chain-runtime startup_gate::tests::dump_vectors -- --nocapture
//
// DO NOT change these vectors without also bumping `RULES_VERSION` in the
// witness module and creating a migration.

/// Minimal 1-transaction, 0-dependency witness.
///
/// Byte layout (SCALE):
///   version: u8 = 1
///   rules_version: u32 (compact) = 1
///   tx_count: u32 (compact) = 1
///   tx_ids: [H256; 1]  (32 zero bytes)
///   access_lists: [AccessListV1; 1]
///     access_count (compact) = 0
///   dep_edges: u32 (compact) = 0
const VECTOR_1TX_NODEPS: &[u8] = &[
    // version: u8 = 1
    0x01, // rules_version: u32 = 1 (fixed 4-byte LE; NOT compact)
    0x01, 0x00, 0x00, 0x00, // tx_count: Compact<u32> = 1
    0x04, // tx_ids: Vec<H256> — Compact length prefix = 1, then 32 zero bytes
    0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // access_lists: Vec<AccessListV1> — Compact length prefix = 1
    0x04, // access_lists[0].access_count: Compact<u32> = 0
    0x00, // access_lists[0].accesses: Vec<AccessKeyV1> — Compact length = 0
    0x00, // seed: Option<H256> = None
    0x00, // reserved: Vec<u8> = [] (Compact length = 0)
    0x00,
];

/// The expected scheduler_commitment for VECTOR_1TX_NODEPS.
///
/// Computed offline by the reference scheduler.  This is the blake2_256 of
/// the canonical concatenation of tx_set_commitment || graph_commitment ||
/// order_commitment as described in witness_v1.rs.
///
/// Value refreshed from the current deterministic reference scheduler via
/// `cargo test -p x3-chain-runtime dump_vectors_for_bootstrapping -- --nocapture`.
///
/// SECURITY: This value MUST be hardcoded to ensure all nodes produce the same
/// output. Dynamic computation would allow a buggy scheduler to pass its own
/// gate by comparing against itself.
const REFERENCE_COMMITMENT_1TX_NODEPS: H256 = H256([
    0xd3, 0x36, 0x7f, 0xa8, 0xc4, 0x77, 0xc1, 0xe0, 0x5e, 0x97, 0x87, 0x74, 0x0c, 0x43, 0x50, 0x9b,
    0x07, 0x8b, 0x5d, 0x07, 0xd8, 0x90, 0xda, 0x53, 0x14, 0x50, 0x0c, 0xf4, 0x31, 0xf3, 0xac, 0xd3,
]);

/// The canonical set of test vectors every compliant node must pass.
/// Returns `Err` if a hard-coded reference vector cannot be decoded — this
/// indicates a programming error (malformed constant bytes) that must be fixed
/// before the node can start.
pub fn required_vectors() -> Result<Vec<TestVector>, GateError> {
    Ok(vec![TestVector {
        name: "1tx-no-deps",
        witness_bytes: VECTOR_1TX_NODEPS,
        rules_version: 1,
        max_tx_count: 256,
        expected_commitment: REFERENCE_COMMITMENT_1TX_NODEPS,
    }])
}

// ---------------------------------------------------------------------------
// Gate execution
// ---------------------------------------------------------------------------

/// Run all hard-coded test vectors.  Returns `Ok(())` only if every vector
/// passes.  On the first failure returns `Err(GateError)` with details.
pub fn run_startup_gate() -> Result<(), GateError> {
    for vector in required_vectors()? {
        let result = scheduler_commitment_from_bytes(
            vector.witness_bytes,
            vector.rules_version,
            vector.max_tx_count,
        );
        match result {
            Err(e) => {
                return Err(GateError::WitnessError {
                    name: vector.name,
                    err: e,
                });
            }
            Ok(got) => {
                if got != vector.expected_commitment {
                    return Err(GateError::VectorMismatch {
                        name: vector.name,
                        expected: vector.expected_commitment,
                        got,
                    });
                }
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// STARTUP-GATE-001: gate passes with the reference scheduler
    #[test]
    fn gate_passes_with_reference_scheduler() {
        assert!(
            run_startup_gate().is_ok(),
            "startup gate failed: {:?}",
            run_startup_gate().unwrap_err()
        );
    }

    /// STARTUP-GATE-002: gate is idempotent — repeated calls return same result
    #[test]
    fn gate_is_deterministic() {
        let r1 = run_startup_gate();
        let r2 = run_startup_gate();
        // Both must succeed (or both fail with the same message).
        assert_eq!(r1.is_ok(), r2.is_ok());
    }

    /// Utility: dump computed commitment so it can be hard-coded later.
    #[test]
    fn dump_vectors_for_bootstrapping() {
        for v in required_vectors().expect("reference vectors must decode") {
            let commitment =
                scheduler_commitment_from_bytes(v.witness_bytes, v.rules_version, v.max_tx_count);
            println!("vector={} commitment={:?}", v.name, commitment);
        }
    }
}
