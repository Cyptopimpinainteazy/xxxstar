# Autonomous Security Constitution — Wiring Gap List

Status: **NOT COMPLETE (BLOCK MAINNET)**

This document maps “constitution-grade” expectations (fail-closed security gates, deterministic governance simulation, real rollback, production-faithful CI) to what is presently enforced in the repo.

---

## Summary of Gaps (Top Risk)

1) **Fail-closed CI is not wired** for vulnerability scans and some VM checks.
2) **Production-faithful validation is not wired** due to pervasive `--all-features` usage.
3) **Deterministic AI governance enforcement is not wired** (simulation/sandbox/rollback are placeholders).
4) **Cross-VM bridge correctness is not wired** (mock hostcalls in X3VM bridge).

---

## Requirement → Reality Mapping

### R-01: “Fail closed on security regressions”

**Expected**
- Any high/critical security regression breaks merge/release.

**Observed**
- `.github/workflows/production-deploy.yml` runs:
  - `cargo audit ... || true`
  - `npm audit ... || true`

**Gap**
- Security scan results do not gate deployment.

**Remediation**
- Remove `|| true` and treat findings as blockers (or enforce an allowlist with expiry).

---

### R-02: “Production-faithful test matrix”

**Expected**
- Validation runs with the same feature set and compilation profile as the release artifact.

**Observed**
- `.github/workflows/ci.yml` and `.github/workflows/production-deploy.yml` run tests with `--all-features`.

**Gap**
- Union-of-features builds can mask security posture (e.g., dev-only bypass).

**Remediation**
- Add a build/test matrix (prod vs dev), and treat prod jobs as gating.

---

### R-03: “No dev bypass in production”

**Expected**
- It is mechanically difficult or impossible to ship a release with bypass features.

**Observed**
- `pallets/x3-kernel/src/lib.rs` includes `#[cfg(feature = "dev-bypass")]` authorization bypass.

**Gap**
- CI `--all-features` increases accidental exposure risk.

**Remediation**
- Add explicit CI check preventing `dev-bypass` in release jobs.

---

### R-04: “Deterministic governance simulation and sandbox enforcement”

**Expected**
- Proposals are simulated deterministically; sandbox execution is isolated; rollback checkpoints are real.

**Observed**
- `pallets/governance/src/lib.rs`:
  - `success: true // Assume success for now`
  - sandbox execution is simulated via `risk_level < 50`
  - rollback checkpoint returns `Default::default()`

**Gap**
- Constitution-style governance assurances are not implementationally true.

**Remediation**
- Implement deterministic checks (state diff, invariants) and real checkpointing or remove rollback claims.

---

### R-05: “Cross-VM/cross-chain correctness backed by real execution”

**Expected**
- Hostcalls map to real EVM/SVM actions with receipts and verifiable state transitions.

**Observed**
- `crates/x3-vm/src/bridge.rs` registers mock hostcalls for SVM/EVM/bridge operations.

**Gap**
- Cross-domain atomicity cannot be considered a security property yet.

**Remediation**
- Replace mocks with real adapters; add end-to-end tests that fail closed.

---

## Acceptance Criteria for “Constitution Complete”

Minimum bar to claim constitution wiring is complete:

1) CI fails closed on:
   - vulnerability scans,
   - VM integration tests,
   - constitution-critical invariant tests.
2) CI validates production-faithful builds (no `--all-features` for prod gate).
3) Governance AI:
   - deterministic simulation (no hardcoded success),
   - sandbox enforcement semantics,
   - real rollback checkpoint or explicitly removed requirement.
4) X3VM bridge:
   - no mock hostcalls in production builds,
   - receipts and state transitions are covered by integration tests.

---

## Suggested Implementation Work Items (If you want me to wire this)

- CI: replace `--all-features` with a prod feature matrix; remove `|| true` from scans/tests; add a “forbidden features” check.
- Governance: implement deterministic simulation + checkpointing or refactor constitution spec to match reality.
- X3VM bridge: gate mocks behind dev-only features or replace with real hostcalls/adapters.
