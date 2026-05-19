# CURRENT MAINNET STATUS

**Status:** ⚠️ MAINNET READINESS BLOCKED / UNDER REVIEW
**Overall Score:** N/A
**Last Verified Commit:** `HEAD`
**Current Evidence:** inconsistent readiness artifacts, stale gate outputs, and a new canary-first launch posture

---

## Status Summary
The current repo is aligned to an internal RC-1 scope, but the evidence is not yet sufficient to support a clean public mainnet readiness claim.

- The safe launch posture is a public canary/testnet reveal, not a broad mainnet announcement.
- Existing RC-1 gate artifacts are provisional and must be regenerated.
- `proof/reports/feature_status.json` is the authoritative machine-readable readiness source and currently reports `BLOCKED` with `built_count: 0`, `missing_count: 37`, `unwired_count: 12`, and `untested_count: 7`.
- `proof/reports/gap_gate_mainnet_20260426_194429.txt` reports 113 gaps, including 24 S0 blockers.
- The authoritative readiness story is now the canary plan and gate-state reconciliation, supported by the mainnet proof gate plan.
- Public messaging must defer external bridges, PQ, AI, GPU, and advanced DEX until audited.

---

## Current Facts
- The internal RC-1 launch scope is:
  - X3Native + X3Evm + X3Svm internal execution
  - internal cross-VM routing and atomic commit/rollback semantics
  - supply ledger invariants, rollback/refund behavior, and a spot AMM/LP path
- Bridges remain disabled-by-default until formal audit passage.
- PQ, AI optimizer, GPU acceleration, and advanced DEX remain staged for post-canary phases.
- Proof gate infrastructure exists, but the outputs are currently inconsistent and require a refresh.

---

## Current Blockers
A credible readiness claim is blocked by:

- stale or unresolved proof gate outputs
- missing regenerated readiness reports
- evidence gaps in critical receipts and blocker tracking
- current feature report counts that show extensive missing/unwired coverage
- the gap gate file reporting 113 unresolved gaps, including 24 S0 blockers
- contradictory internal status artifacts across docs and reports

---

## Archived / historical reports

The following historical launch artifacts have been moved to the docs archive:

- `docs/archive/status/PROOF_EXECUTION_REPORT.md`
- `docs/archive/status/GENESIS_CEREMONY_RUNBOOK.md`
- `docs/archive/status/PHASE_4_STRATEGIC_OVERVIEW.md`

---

## Authoritative sources
The current source of truth for readiness and launch posture is:

- `docs/MAINNET_CANARY_PLAN.md`
- `docs/MAINNET_READINESS_CHECKLIST.md`
- `docs/MAINNET_LAUNCH_CHECKLIST.md`
- `docs/MAINNET_GATE_PROOFS_PLAN.md`
- `.x3/X3_MAINNET_GATES.md`

---

## Next steps
1. Re-run the readiness pipeline and regenerate all gate artifacts.
2. Refresh the canary plan and readiness checklist with the latest status.
3. Publish a single canonical readiness scoreboard.
4. Lock public messaging to the RC-1 canary scope and explicit deferred features.

---

## Current Commands
```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo build --release -p x3-chain-node
cargo build --release -p x3-cli
cargo build --release -p x3-proof
cargo test -p pallet-x3-cross-vm-router
cargo test -p pallet-x3-supply-ledger
cargo test -p pallet-x3-atomic-kernel
cargo test -p x3-ixl
cargo test -p x3-proof
cargo run -p x3-readiness -- testnet-report --out reports/testnet_readiness_report.md
x3-proof mainnet-rc-report --out reports/mainnet_rc_report.md
```

---

## References
- `docs/MAINNET_CANARY_PLAN.md`
- `docs/MAINNET_READINESS_CHECKLIST.md`
- `docs/MAINNET_LAUNCH_CHECKLIST.md`
- `.x3/X3_MAINNET_GATES.md`
- `docs/MAINNET_READINESS_DELTA.md`
- `proof/reports/gap_gate_mainnet_20260426_194429.txt`
- `proof/reports/todo_gate_mainnet_20260426_194331.txt`
- `reports/testnet_readiness_report.md`

*Last updated: 2026-05-09*
*Source: manual reconciliation of active gate artifacts*

---

## Launch Conditions

Launch is not authorized by the current evidence.

> **Scope note:** This report is scoped to internal v0.4 RC-1 readiness only. It does not imply public mainnet readiness for external gateways, PQ cryptography, advanced DEX, AI optimization, or GPU validator-critical paths.

**RC-1 Scope:** See [../MAINNET_RC1_SCOPE.md](../MAINNET_RC1_SCOPE.md)

**RC-1 Feature Debt:** See [../docs/RC1_FEATURE_DEBT.md](../docs/RC1_FEATURE_DEBT.md)

---

*Last updated: 2026-05-09*
*Source: manual reconciliation of active gate artifacts*
*Commit: 2e0c3bdac9de8b60*