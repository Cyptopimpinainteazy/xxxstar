# Mainnet Gate + Proofs Plan

This document defines the path to a full mainnet gate with evidence-backed proofs for all major repository features.

## 1. Source of Truth

The authoritative mainnet gate criteria are defined in:
- `.x3/X3_MAINNET_GATES.md`

A mainnet gate is only valid when all P0 items pass and the relevant P1 supporting items are satisfied.

## 2. Current Status

Current repo evidence indicates the mainnet readiness gate is blocked.

Key blockers:
- `proof/reports/gap_gate_mainnet_20260426_194429.txt` reports 113 gaps, including 24 S0 blockers.
- `proof/reports/features_report.md` reports:
  - 0 built
  - 37 missing
  - 12 unwired
  - 7 untested
  - 2 weak
- Catastrophic claims are missing proof receipts for core features including runtime, consensus, finality, validator set, asset kernel, bridge, cross-VM execution, and X3VM.
- Several readiness artifacts are stale or remain placeholder coverage instead of fresh evidence.

## 3. Mainnet Gate Scope

### P0 items to prove

#### Runtime
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo check --workspace`
- `cargo test --workspace --lib --tests -- --test-threads=1`
- production runtime and WASM compile cleanly
- runtime upgrade rehearsal documented
- canonical mainnet genesis spec files generated and reviewed
- no dev accounts / dev authorities remain
- no consensus-critical `unwrap` / `panic!` remains in hot paths

#### Universal Asset Kernel
- supply invariant tests
- native/EVM/SVM/external_locked/pending accounting tests
- arithmetic overflow/precision tests
- rollback safety tests for failed cross-VM flows

#### Cross-VM Atomic Execution
- integration tests for EVM/SVM/X3VM execution paths
- end-to-end atomic commit/rollback tests
- replay protection tests for duplicate/stale/nonce reuse
- timeout/expiry/rollback coverage
- domain separation and settlement proof validation

#### Bridge / Router
- bridge enablement audit gate implemented
- external bridge disabled by default until audit gate passes
- nonce uniqueness, expiry, finality, replay rejection tests
- documented proof validation and error handling

#### DEX / Launchpad
- swap correctness and rollback tests
- liquidity lock accounting tests
- anti-rug and sandwich resistance tests
- fee accounting validation
- slippage/TWAP/order protections
- launchpad cap/vesting/allocation tests

#### Security
- no TODO/FIXME in P0 code paths
- no hardcoded mocks in production paths
- weak randomness audit complete
- unsafe code reviewed and justified
- panic/unwrap audit documented in `reports/panic_unwrap_audit.md`
- equivocation detection and slashing are wired end-to-end, not just logged

#### Ops
- testnet launch checklist complete
- genesis review complete
- monitoring and alerting plan exists
- rollback/recovery plan documented
- validator bootstrap documented and tested
- emergency governance/upgrade procedure documented

### P1 supporting items

- infrastructure topology validated
- `ExternalBridgesEnabled=false` locked in mainnet genesis
- binaries signed, reproducible, and packaged
- governance/audit gate enforcement verified
- formal proof reporting wired into gate workflow
- documentation and launch support published

## 4. Proof Coverage Requirements

A valid mainnet gate must prove coverage for every feature in the repo across these dimensions:

- code compile/test proof (runtime, pallets, crates)
- proof receipts for critical claims in `proof/receipts/claims/`
- proof runner outputs from `proof-forge` and `proof/reports`
- generated report freshness and provenance
- end-to-end gate scripts in `launch-gates/`

## 5. Immediate Execution Plan

1. Start from a clean repo and regenerate proofs:
   - `cargo fmt --all --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo check --workspace`
   - `cargo test --workspace --lib --tests -- --test-threads=1`
   - verify proof artifact freshness by inspecting `proof/reports/features_report.md` and `proof/reports/gap_gate_mainnet_*.txt`
2. Re-run the mainnet gate proof pipeline:
   - `bash launch-gates/multi-node-testnet-proof.sh`
   - `bash launch-gates/verify-p0-blockers.sh`
   - `bash launch-gates/fresh-machine-proof.sh`
   - `bash launch-gates/embarrassment-scan.sh`
3. Rebuild all gate evidence outputs:
   - regenerate `proof/reports/gap_gate_mainnet_*.txt`
   - regenerate `proof/reports/todo_gate_mainnet_*.txt`
   - regenerate `proof/reports/features_report.md`
   - refresh `reports/mainnet_rc_report.md`
   - publish updated proof receipts under `proof/receipts/claims/`
4. Verify canonical docs and reports are consistent:
   - `docs/CURRENT_MAINNET_STATUS.md`
   - `docs/MAINNET_READINESS_CHECKLIST.md`
   - `docs/MAINNET_LAUNCH_CHECKLIST.md`
   - `.x3/X3_MAINNET_GATES.md`
   - `reports/testnet_readiness_report.md`
5. Close the highest-severity proof gaps first:
   - fix S0 gaps from `proof/reports/gap_gate_mainnet_*.txt`
   - prove and publish missing receipts for catastrophic claims
   - remove stale or placeholder report artifacts and replace them with regenerated evidence

## 6. Feature-Level Proof Checklist

Based on current repo evidence, the following feature groups require proof coverage before the mainnet gate can be true:

- runtime / WASM build + runtime upgrade rehearsal
- universal asset kernel invariants
- cross-VM atomic execution and replay safety
- bridge/router audit gate and finality proofs
- DEX/launchpad swap/fee/anti-manipulation proofs
- security gap audit and panic/unwrap proofs
- ops/runbook evidence for validator bootstrap, rollout, and recovery
- proof system receipts for every claim in `proof/receipts/claims/`

## 7. Risk Escalation

If any of the following remain unresolved, mainnet gate cannot be declared:
- S0 proof claim missing receipt
- proof gap file still reports gate failure
- P0 test or build command fails
- stale or placeholder readiness report is published as launch evidence

## 8. Recommended Product Step

Create a single “mainnet gate release candidate” artifact after a clean execution pass.

That artifact should include:
- fresh `proof/reports/gap_gate_mainnet_*.txt`
- fresh `proof/reports/features_report.md`
- fresh `reports/mainnet_rc_report.md`
- a signed `chain-specs/x3-mainnet-plain.json` and `x3-mainnet-raw.json`
- a summary in `docs/CURRENT_MAINNET_STATUS.md`
- a canonical pass/fail statement in `docs/MAINNET_GATE_PROOFS_PLAN.md`

---

## 9. Notes

This repo already contains the gate framework and a formal proof-runner path. The remaining work is not a new requirements spec — it is a disciplined, evidence-first execution run and remediation of the failing proof subjects.

Current evidence shows the mainnet gate remains blocked. The path forward is to complete the missing build/test proofs, close the `proof/reports/features_report.md` blockers, and regenerate all proof receipts and gap reports from a fresh clean execution.
