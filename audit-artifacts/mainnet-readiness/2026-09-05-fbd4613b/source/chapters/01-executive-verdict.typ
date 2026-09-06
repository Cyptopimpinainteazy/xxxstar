#import "../style.typ": *
#import "../components.typ": *
#import "../data.typ": *
#import "../charts.typ": *

= Executive Verdict

#grid(columns: (1fr, 1fr, 1fr), gutter: 10pt)[
  #metric-tile([#overall-score / 100], "Overall readiness score", color: score-band-color(overall-score))
][
  #metric-tile([NO-GO], "Public testnet", color: c-critical)
][
  #metric-tile([NO-GO], "Mainnet", color: c-critical)
]

#v(0.6em)

The repository's own `LAUNCH_SCOPE.md` self-labels the project as a **"v0.4 Internal Testnet Candidate"** — it does not claim public testnet readiness, and this audit agrees with that self-assessment. The findings below explain what stands between the current state and each subsequent gate.

#pull-quote(
  [X3 Atomic Star is in v0.4 Internal Testnet Candidate phase. This is an internal-only, closed-operator staged testnet. It is NOT a public testnet, NOT a mainnet candidate, and NOT production-ready for external bridging or public-value settlement.],
  source: [`LAUNCH_SCOPE.md`, the repository's own authoritative scope statement]
)

== What This Blockchain Is Trying to Become

X3 Atomic Star is a Substrate-based Layer-1 blockchain with native cross-VM atomic execution across three domains — X3Native (its own asset/consensus layer), X3Evm (Frontier-based EVM compatibility), and X3Svm (Solana-VM-compatible execution). Its central value proposition is the *Universal Asset Kernel*: a canonical-supply invariant (`represented_total ≤ canonical_supply`) enforced across every cross-VM transfer, with atomic settlement and a governance-gated external bridge layer that is deliberately disabled by default pending audit. It is an ambitious, unusually large workspace — 140 Cargo workspace members, 58 pallets, 133 non-pallet crates, a dual EVM/SVM contract stack, and roughly 16 front-end/tooling applications.

== What Actually Exists Today

#figure(
  severity-bar-chart(sev-counts),
  caption: [Findings by severity across all seven audited domains (32 total). Full register in `findings.json` and Appendix A.]
) <fig-severity>

The codebase compiles clean (`cargo check --workspace`, default features, exit 0, 1m52s), `cargo audit` reports zero blocking vulnerabilities, and the Foundry EVM contract suite passes 169/169 tests across 12 suites. Several pallet-level test suites were executed live in this audit and passed: the cross-VM router (81 tests), the settlement engine (23 property-based tests), the supply ledger (33 tests including a fuzz test), the DEX pallet (14 tests), and the LP locker (19 tests). This is real, substantive, working code — not a shell.

At the same time, this audit surfaced #by-severity("Critical").len() Critical and #by-severity("High").len() High-severity findings that were not previously documented with this level of precision anywhere in the repository's own extensive self-audit trail, including one outright security defect (an unauthenticated public call that can slash any validator with zero evidence) and one confirmed, unremediated secrets leak still reachable in git history.

== Top Five Strengths

+ *Real consensus, not a mock.* Aura block production and GRANDPA finality are genuine, unmodified Substrate/Polkadot-SDK crates, wired into every runtime variant, with a correctly proof-gated equivocation-slashing path.
+ *The core atomicity claim is genuinely tested.* The cross-VM router's 6-route matrix (Native↔EVM↔SVM) has 81 passing tests covering replay protection, nonce monotonicity, and expiry refund; the settlement engine's 23 property-based tests assert real invariants (no partial execution, bond release never exceeds reserved) rather than example-based happy paths.
+ *The canonical supply invariant is enforced in depth, not just documented.* `check_invariant()` runs at every mutation site *and* redundantly every block, with a real fail-closed halt policy — 33/33 tests pass including a fuzz test.
+ *External bridges are genuinely disabled, not just documented as disabled.* `ExternalBridgesEnabled` is a real runtime storage gate, checked at dispatch time, with an automatic circuit breaker that trips to `false` on invariant violation — one of the better-engineered controls in the repository.
+ *The EVM contract suite is well-tested.* 169 Foundry tests across 12 suites, including fuzz tests on flashloan fee math and governance voting, all passing; reentrancy guards and access-control patterns (`Ownable`, `AccessControl`) are used correctly in the contracts reviewed.

== Top Five Most Dangerous Weaknesses

+ *An unauthenticated public call can slash any validator with zero evidence* (CRIT-02, `pallets/x3-consensus/src/lib.rs:262`) — a live griefing/DoS vector against validator liveness, reachable in every runtime variant.
+ *Validator authoring seeds and a plaintext EVM private key remain reachable in git history* (CRIT-01) — the remediation commit made today only untracked the files; the secrets themselves are still fetchable from any prior clone.
+ *The DEX's entire pricing engine uses floating-point arithmetic* (HIGH-02, `crates/x3-dex/src/amm_pools.rs`) — a determinism and precision hazard in validator-executed financial state-transition code, masked by tests that only check the float implementation against itself.
+ *A bridge RPC path presents a single-key mint as a "council" vote* (HIGH-01, `node/src/rpc.rs:1058-1093`) — `pallet_collective::propose{threshold: 1}` executes immediately with no real multi-party vote; currently inert only because bridges are disabled by default.
+ *Cited "reproducible build" evidence proves nothing* (HIGH-05) — the srtool checksums the repository's own status doc points to are checksums of log files that do not exist, from a different machine, four months stale.

== Most Serious Security or Integrity Concern

The unauthenticated `report_misbehavior` call (CRIT-02) is the single most serious finding in this audit. It sits directly beside a *correctly engineered* equivocation-slashing path that requires a real cryptographic proof — meaning the codebase clearly knows the right pattern, but a second, parallel entrypoint to the same dangerous operation (slashing a validator's stake) was left without that protection. This is exactly the kind of defect that a broad, mechanical stub-scanner (grepping for `todo!()`, `unimplemented!()`, `mock`) would never catch, because the code is fully implemented, compiles, and has no obvious placeholder marker — it simply lacks an authorization check appropriate to its power. It should be fixed before any network carrying real stake is exposed to a wider set of accounts than a small trusted internal group.

== Fastest Credible Path Forward

The two Critical findings (CRIT-01, CRIT-02) are both narrow, well-understood fixes — a history rewrite plus key rotation, and an authorization check mirroring a pattern the codebase already implements correctly elsewhere — and neither requires new architecture. Fixing both, plus the top three High findings that block any bridge-enabled or DEX-carrying deployment (HIGH-01, HIGH-02, HIGH-04), would move this repository from "internal testnet candidate with live security defects" to "internal testnet candidate with only the already-disclosed, already-gated limitations remaining" within a small, well-scoped engineering effort. See Chapter 13 for the full prioritized recovery plan.

== Next 7 / 30 / 60 / 90 Days

#table(
  columns: (auto, 1fr),
  fill: (x, y) => if y == 0 { c-brand } else if calc.even(y) { c-bg-panel } else { white },
  [*Window*], [*What should happen*],
  [Next 7 days], [Fix CRIT-02 (require real evidence for `report_misbehavior`); begin the git-history rewrite for CRIT-01 and rotate every affected key offline via `subkey`; correct the two disproven documentation claims (TREASURY_POLICY.md, FEATURE_REGISTRY.toml health endpoints) so the audit trail stops citing false evidence.],
  [Next 30 days], [Replace DEX pricing math with integer/fixed-point arithmetic (HIGH-02); decouple VRF's `dev` feature from `std` (HIGH-03); re-run `scripts/run-srtool.sh` on a machine with Docker and commit real evidence (HIGH-05); re-verify the 8/8 mesh-resilience result across 3+ physically separate hosts, not loopback.],
  [Next 60 days], [Wire the multisig propose/sign/execute engine into real dispatchable calls (MED-03); build a real Anchor test suite for the 6 untested SVM programs (MED-08); add dependency-checked `/ready` endpoints to every service currently reporting liveness-only "health" (MED-09/MED-10); broaden the secret-scan CI gate to cover history, not just the working tree (MED-11).],
  [Next 90 days], [Engage an external, licensed security audit firm for the runtime pallets, EVM contracts, and SVM/Anchor programs — the repository's own `LAUNCH_SCOPE.md` correctly identifies this as not yet done and as the primary blocker to any public-beta claim; run a real, honestly-labeled multi-host performance benchmark to replace the loopback-only 110.6 finTPS figure.],
)

#pagebreak(weak: true)
#align(center + horizon)[
  #block(width: 90%, fill: c-bg-panel, inset: 18pt, radius: 4pt, stroke: 1pt + c-brand)[
    #text(font: title-font, size: 13pt, weight: "bold", fill: c-brand)[If You Read Nothing Else]
    #v(0.5em)
    #set text(size: 10.5pt)
    X3 Atomic Star's core cross-VM atomicity claim is real, tested, and well-engineered — the settlement engine, supply ledger, and internal router are the strongest parts of this codebase. Everything that crosses a real external chain boundary (EVM/SVM/Bitcoin bridges) is correctly disabled by default and honestly documented as audit-ready-only, not active. Two Critical defects — an unauthenticated validator-slashing call and unremediated leaked git-history secrets — must be fixed before this network is exposed to any account outside a small trusted group. The DEX's floating-point pricing engine must be replaced before it ever carries real value. None of this requires new architecture; all of it is well-scoped, understood work. An external, licensed security audit — not yet engaged — remains the primary gate before any public-facing claim.
  ]
]
