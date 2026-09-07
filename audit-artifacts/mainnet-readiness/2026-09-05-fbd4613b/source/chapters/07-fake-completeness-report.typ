#import "../style.typ": *
#import "../components.typ": *
#import "../data.typ": *

= Fake-Completeness Report

This chapter collects every place this audit found a *claim* — in code comments, status documents, or the feature registry — that does not match what the code actually does. Some of these were confirmed as claimed (evidence in earlier chapters); this chapter is specifically the ones that were not.

== Confirmed Doc-vs-Code Mismatches

#table(
  columns: (1.3fr, 2.4fr, 2.1fr),
  fill: (x, y) => if y == 0 { c-brand } else if calc.even(y) { c-bg-panel } else { white },
  [*Claim*], [*Where claimed*], [*What the code actually does*],
  [Node boots deterministically, proven by srtool checksums], [`docs/current/MASTER_CHECKLIST_STATUS.md:64`], [The cited `.sha256` files hash `.log` files that do not exist anywhere in the repo, reference a different machine's path, and are 4+ months stale. Docker (required to regenerate them) is not installed in this environment. See HIGH-05.],
  [10 pallets have "proper migrations.rs with OnRuntimeUpgrade impl"], [`docs/current/MASTER_CHECKLIST_STATUS.md:78`], [All 10 files read contain only a `StorageVersion` bump — zero field/type/schema transformation logic. See MED-01.],
  [Per-feature health endpoints (`/health/atomic-kernel`, `/health/atomic-router`, etc.) support specific readiness scores], [`FEATURE_REGISTRY.toml`], [None of these literal paths exist anywhere in the codebase — zero hits in a repo-wide grep outside the registry file itself. See HIGH-06.],
  [Treasury.sol's `routeFee()` is "callable by anyone"], [`TREASURY_POLICY.md`, `docs/current/MASTER_CHECKLIST_STATUS.md:174`], [The actual Solidity is `external onlyOwner nonReentrant` — the claim overstates the risk (the real, narrower risk is single-EOA ownership, MED-07). See MED-06.],
  [README: CI has "9 worker jobs + 1 aggregate job"], [`README.md`], [`ci.yml` currently defines 20 gate jobs feeding the aggregate. Not a defect in the gate (verified fail-closed), but a stale inventory count. See LOW-07.],
  [VRF mock randomness is isolated to test/dev builds], [`crates/x3-vrf/src/lib.rs` doc comment], [The pallet's own `std` feature unconditionally enables the `dev` (mock) feature, so any native/benchmark build gets predictable randomness by accident, not by explicit choice. See HIGH-03.],
)

== A Meta-Finding: The Audit Trail Sometimes Overstates *and* Understates

Two of the mismatches above cut in opposite directions, which is itself worth naming. The health-endpoint and srtool claims *overstate* readiness (citing evidence that doesn't exist). The Treasury.sol claim *overstates risk* (describing a function as unprotected when it is not). Neither direction is more acceptable than the other: both mean a reader cannot trust the repository's own status documents without independently checking source, which is exactly the posture this audit — and this repository's own `AGENTS.md` — recommends.

#callout(kind: "warning", title: "Why This Matters Beyond These Six Items")[
  This audit sampled a subset of the repository's own claims for independent verification; it did not attempt to re-verify every claim in every status document. The fact that 6 concrete, checkable claims were found wrong in a targeted sample is a signal about the *category* of risk (unverified evidence citations), not a complete inventory of every wrong claim that may exist elsewhere in the same documents.
]

== What Was *Not* Found — And Is Worth Naming

For balance: this audit did not find a hardcoded "success" response standing in for the canonical supply invariant, did not find a mocked cross-VM router masquerading as the real one, and did not find disabled tests silently deleted rather than fixed. The repository's own `AGENTS.md`/`CLAUDE.md` rules forbidding exactly these patterns appear to have been followed in the paths this audit traced. The failures found here are more subtle than a classic "fake it" pattern — they are real, compiling, tested code that is either miswired (DISCONNECTED), using the wrong tool for the job (HIGH-02's floating point), or missing an authorization check appropriate to its power (CRIT-02).
