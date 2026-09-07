#import "../style.typ": *
#import "../components.typ": *
#import "../data.typ": *

= Final Launch Gates

Gates are objective and evidence-based. High-risk security, fund-safety, consensus-safety, key-management, and state-integrity gates are marked *non-waivable* — no operational pressure should override them.

== Gate: Internal Devnet (currently achievable)

#table(
  columns: (2fr, 2fr, auto),
  fill: (x, y) => if y == 0 { c-brand } else if calc.even(y) { c-bg-panel } else { white },
  [*Requirement*], [*Evidence artifact*], [*Waivable?*],
  [Workspace compiles clean (default features)], [`cargo check --workspace` exit 0 — VERIFIED this audit], [No],
  [Single-node dev chain boots and authors blocks], [`--chain dev --tmp --validator --alice` per README], [No],
  [Core pallet test suites pass], [Live-verified: cross-vm-router 81/81, settlement-engine 23/23, supply-ledger 33/33, dex 14/14, lp-locker 19/19], [No],
)

*Status: MET.* This gate is already satisfied by evidence gathered in this audit.

== Gate: Private Multi-Node Testnet

#table(
  columns: (2fr, 2fr, auto),
  fill: (x, y) => if y == 0 { c-brand } else if calc.even(y) { c-bg-panel } else { white },
  [*Requirement*], [*Evidence artifact*], [*Waivable?*],
  [CRIT-03 fixed], [`wallet_*` RPC methods removed or refuse to start on non-dev chain specs], [*No*],
  [CRIT-02 fixed], [Test proving unauthenticated `report_misbehavior` is rejected], [*No*],
  [CRIT-01 remediated], [`git log --all` clean on rewritten history + rotated keys], [*No*],
  [Multi-validator convergence re-proven on non-loopback hosts], [MED-12 verification], [No],
)

*Status: NOT MET.* All three Critical findings block this gate.

== Gate: Public Testnet

#table(
  columns: (2fr, 2fr, auto),
  fill: (x, y) => if y == 0 { c-brand } else if calc.even(y) { c-bg-panel } else { white },
  [*Requirement*], [*Evidence artifact*], [*Waivable?*],
  [All Private Multi-Node Testnet gates met], [above], [*No*],
  [HIGH-05 srtool evidence real and current], [`.log`/`.json` for the deployed commit], [No],
  [HIGH-06 health endpoints real or removed from registry], [`curl` verification], [No],
  [HIGH-02 DEX float-pricing fixed (if DEX carries any real value)], [Property test vs. decimal reference], [*No, if DEX is live*],
  [Secret-scan gate broadened (MED-11)], [CI catches a planted test secret], [No],
)

*Status: NOT MET.*

== Gate: Incentivized Testnet

#table(
  columns: (2fr, 2fr, auto),
  fill: (x, y) => if y == 0 { c-brand } else if calc.even(y) { c-bg-panel } else { white },
  [*Requirement*], [*Evidence artifact*], [*Waivable?*],
  [All Public Testnet gates met], [above], [*No*],
  [Multisig execution wired (if incentive claims rely on it)], [MED-03 integration test], [No, if relied upon],
  [SVM programs have adversarial test coverage (if SVM path is active)], [MED-08 test suite], [*No, if active*],
  [Sustained-load performance figures published, honestly scoped], [Chapter 12's methodology extended to multi-host], [No],
)

*Status: NOT MET.*

== Gate: Release Candidate

#table(
  columns: (2fr, 2fr, auto),
  fill: (x, y) => if y == 0 { c-brand } else if calc.even(y) { c-bg-panel } else { white },
  [*Requirement*], [*Evidence artifact*], [*Waivable?*],
  [All Incentivized Testnet gates met], [above], [*No*],
  [Bridge single-key trust points replaced (HIGH-01, HIGH-04)], [Threshold-signature integration test], [*No, if bridges enabled*],
  [Governance/upgrade safety score ≥ 70 in this audit's model], [Re-run scoring methodology], [No],
)

*Status: NOT MET.*

== Gate: Mainnet

#table(
  columns: (2fr, 2fr, auto),
  fill: (x, y) => if y == 0 { c-brand } else if calc.even(y) { c-bg-panel } else { white },
  [*Requirement*], [*Evidence artifact*], [*Waivable?*],
  [All Release Candidate gates met], [above], [*No*],
  [External, licensed security audit complete for runtime, EVM contracts, and SVM programs], [Published audit report], [*No*],
  [Permissionless validator staking/bonding decision made and documented (even if deferred)], [Governance doc + code, or explicit documented deferral], [No],
  [Overall readiness score ≥ 85 in this audit's weighted model, re-scored against then-current commit], [Re-run Chapter 2's methodology], [*No*],
)

*Status: NOT MET.* The repository's own `LAUNCH_SCOPE.md` does not claim mainnet readiness, and this audit agrees.

#callout(kind: "critical", title: "Failure Response for a Non-Waivable Gate")[
  If a non-waivable gate fails during a promotion attempt, the correct response is to halt promotion and file the failure against the specific finding ID blocking it — never to relabel the gate's threshold, disable the check, or promote anyway "just this once." Every non-waivable gate above maps to a Critical or fund-safety-relevant High finding in `findings.json`.
]
