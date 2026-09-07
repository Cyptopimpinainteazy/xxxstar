#import "../style.typ": *
#import "../components.typ": *
#import "../data.typ": *
#import "../charts.typ": *

= Feature Completeness Scorecard

This chapter summarizes the 67-row feature matrix produced by the seven domain investigations. The full matrix — every feature, its claimed behavior, actual implementation with file:line, runtime wiring, tests, status, evidence quality, and missing work — is `feature-matrix.csv` alongside this document, and is reproduced in full in Appendix D.

#figure(
  feature-status-chart(feature-status-counts),
  caption: [Status distribution across all 67 features explicitly evaluated in this audit's seven domains. This is not a complete inventory of the repository's \~140 crates and \~58 pallets — it is the set this audit's seven parallel investigations directly traced with file:line evidence.]
) <fig-feature-status>

== Reading the Chart Honestly

29 of 67 traced features are VERIFIED — a real majority, and consistent with the repository's own genuinely substantial engineering effort. But three buckets deserve scrutiny before treating that number as reassuring:

- *13 PARTIAL* features are directionally correct but overstated or incomplete somewhere — for example, the validator-set/staking language in top-level docs (F004, F052) describes real slashing bookkeeping but omits that admission is root-controlled, not permissionless.
- *9 PLACEHOLDER* features are the ones that matter most: the fabricated wallet RPC (F067), DEX pricing math (F056), the VRF pallet's randomness source (F035), the migrations for 10 pallets (F025), and the dead `TransactionSigner` module (F033) all fall here. Every one of these compiles, has no `todo!()`, and would pass a naive stub-scanner.
- *4 DISCONNECTED* features (F017 private-mempool, F032 multisig execution, F059 health endpoints) are real code that a user's actual transaction or query would never reach.

== Feature Completeness by Subsystem

#table(
  columns: (1.6fr, auto, auto, auto, auto, auto, auto, auto),
  fill: (x, y) => if y == 0 { c-brand } else if calc.even(y) { c-bg-panel } else { white },
  [*Subsystem*], [*Rows*], [*Verified*], [*Partial*], [*Unverified*], [*Placeholder*], [*Disconn.*], [*Blocked*],
  ..{
    let subsystems = ("Consensus/Networking", "Transaction Lifecycle", "State/Storage", "Cryptography/Keys", "Contracts/VM/Cross-Chain", "Tokenomics", "API/Ops/ProofGates/Perf")
    let cells = ()
    for s in subsystems {
      let rows = feature-rows.filter(r => r.subsystem == s)
      let count(bucket) = rows.filter(r => status-bucket(r.status) == bucket).len()
      cells.push([#s])
      cells.push([#rows.len()])
      cells.push([#count("Verified")])
      cells.push([#count("Partial")])
      cells.push([#count("Implemented, unverified")])
      cells.push([#count("Placeholder")])
      cells.push([#count("Disconnected")])
      cells.push([#(count("Blocked") + count("Informational"))])
    }
    cells
  }
)

== Scoring Formula for Completion Percentages

Rather than assign an intuitive completion percentage per feature, this audit uses the discrete status vocabulary defined in Chapter 0 throughout — VERIFIED, PARTIAL, IMPLEMENTED BUT UNVERIFIED, PLACEHOLDER, DISCONNECTED, MISSING, BLOCKED. A discrete label resists the false precision of a number like "73% complete" for something that is either wired into the real execution path or is not. Where this audit does compute a number — the 0–100 subsystem scores in Chapter 2 — the formula and every input are shown explicitly, never asserted alone.

#callout(kind: "info", title: "Code Exists vs. Code Runs")[
  The clearest way to see this repository's central pattern: almost everything *exists*. The gap this audit found is almost never "this file is empty" — it is "this file compiles, is well-written, has passing tests, and is either not wired to anything (DISCONNECTED), guarded by an accidental feature-flag interaction (HIGH-03), or quietly uses the wrong kind of arithmetic for what it claims to do (HIGH-02)." A mechanical `grep -r "TODO\|unimplemented"` scan — which this repository's own tooling already runs extensively — would not surface a single one of this audit's Critical or High findings, because none of them look like a stub.
]
