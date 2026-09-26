//! Adapters that fabricate chain evidence must not declare it.
//!
//! `AdapterScoreboard` reports `readiness_score()` verbatim, so each flag is a claim shown to
//! whoever reads the scoreboard. The EVM and SVM adapters built their lock, claim and refund proofs
//! from `mock_tx_id(..)`, a "Simulated block number" and literal "mock proof" payloads, and
//! `finality_status` returned a block hash derived from the number 42 — while declaring
//! `event_proof_extraction`, `finality_proof`, `rpc_indexer_support` and `proof_ledger_integration`
//! true. Measured 2026-09-26; this pins the correction for the two VM types the roadmap targets next
//! (live X3<->EVM, live X3<->SVM). The remaining adapters are on
//! `security/adapter-readiness-claims-baseline.txt`, which may only shrink.

use x3_atomic_swap::adapter::X3VmAdapter;
use x3_atomic_swap::evm_htlc::{EvmAdapter, EvmHtlcContract};
use x3_atomic_swap::svm_htlc::{SvmAdapter, SvmHtlcProgram};

/// The four flags that assert chain evidence exists, which mock proofs cannot supply.
const EVIDENCE_FLAGS: [&str; 4] = [
    "event_proof_extraction",
    "finality_proof",
    "rpc_indexer_support",
    "proof_ledger_integration",
];

fn assert_reports_evidence_missing(name: &str, missing: Vec<&'static str>) {
    for flag in EVIDENCE_FLAGS {
        assert!(
            missing.contains(&flag),
            "{name} fabricates chain evidence, so it must report {flag} missing; got {missing:?}"
        );
    }
}

#[test]
fn the_evm_adapter_does_not_claim_the_evidence_it_fabricates() {
    let adapter = EvmAdapter::new(EvmHtlcContract::new([0x11u8; 20]));
    assert_reports_evidence_missing(
        "evm-htlc-adapter",
        adapter.readiness_score().missing_items(),
    );
}

#[test]
fn the_svm_adapter_does_not_claim_the_evidence_it_fabricates() {
    let adapter = SvmAdapter::new(SvmHtlcProgram::new([0x22u8; 32]));
    assert_reports_evidence_missing(
        "svm-htlc-adapter",
        adapter.readiness_score().missing_items(),
    );
}
