//! Drives every JSON parity vector through `x3_parity_core::simulate_flashloan`
//! and asserts the result matches `expected`. Failure here means EVM/SVM
//! stacks must NOT be built against this commit — the math source of truth
//! has drifted from the published vectors.

use std::path::PathBuf;

use x3_parity_core::{simulate_flashloan, VectorDoc};

fn vectors_path() -> PathBuf {
    // Manifest dir = X3-contracts/shared/parity-core
    // Vectors      = X3-contracts/shared/test-vectors/flashloan_repay_or_revert.json
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.push("test-vectors");
    p.push("flashloan_repay_or_revert.json");
    p
}

#[test]
fn flashloan_repay_or_revert_vectors_match_simulator() {
    let path = vectors_path();
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    let doc: VectorDoc =
        serde_json::from_str(&raw).expect("flashloan vectors are valid JSON matching the schema");

    assert_eq!(doc.spec_version, 1, "spec_version drift");
    assert_eq!(doc.spec, "flashloan/repay_or_revert");
    assert!(!doc.vectors.is_empty(), "no vectors found");

    let mut failures = Vec::new();
    for v in &doc.vectors {
        let out = simulate_flashloan(v.amount_u128(), doc.fee_bps, v.borrower_kind);

        let expected_ok = v.expected.result == "ok";
        if out.ok != expected_ok {
            failures.push(format!(
                "{}: ok mismatch (got {}, want {})",
                v.id, out.ok, expected_ok
            ));
            continue;
        }

        if expected_ok {
            let want_delta = v.expected_pool_delta_i128();
            if out.pool_delta != want_delta {
                failures.push(format!(
                    "{}: pool_delta mismatch (got {}, want {})",
                    v.id, out.pool_delta, want_delta
                ));
            }
        } else {
            let want_reason = v.expected.revert_reason.as_deref().unwrap_or("");
            if out.revert_reason != want_reason {
                failures.push(format!(
                    "{}: revert_reason mismatch (got {:?}, want {:?})",
                    v.id, out.revert_reason, want_reason
                ));
            }
            if out.pool_delta != 0 {
                failures.push(format!(
                    "{}: revert leaked pool_delta {}",
                    v.id, out.pool_delta
                ));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "parity simulator disagrees with {} vectors:\n  - {}",
            failures.len(),
            failures.join("\n  - ")
        );
    }
}
