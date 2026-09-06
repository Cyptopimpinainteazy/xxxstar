#import "../style.typ": *
#import "../components.typ": *
#import "../data.typ": *
#import "../charts.typ": *

= Scoring Methodology

== Formula

The overall readiness score is a weighted average across 16 subsystem categories, each scored 0–100:

$ "Overall" = (sum_i w_i dot s_i) / (sum_i w_i) $

where $w_i$ is the category's weight (weights sum to 100) and $s_i$ is its 0–100 score. This computation is performed by Typst directly from the literal table in `source/data.typ` at build time (not hand-calculated) — see `data.typ`'s `overall-score` binding.

A missing or placeholder safety-critical component pulls its whole category down heavily by design: documentation alone earns no credit anywhere in this model. A category scores well only when the underlying capability is implemented, wired into the real execution path, tested, and evidenced — matching the VERIFIED bar defined in Chapter 0's status legend.

#figure(
  subsystem-score-chart(subsystem-scores),
  caption: [All 16 weighted subsystem scores, sorted descending. Red < 40, amber 40–69, green ≥ 70.]
) <fig-subsystem-scores>

== Weights, Scores, and Evidence Source

#table(
  columns: (1fr, auto, auto, 2fr),
  fill: (x, y) => if y == 0 { c-brand } else if calc.even(y) { c-bg-panel } else { white },
  [*Category*], [*Weight*], [*Score*], [*Primary evidence driving the score*],
  [Consensus safety], [13], [55], [Real Aura/GRANDPA with proof-gated equivocation slashing (strong), offset heavily by the unauthenticated `report_misbehavior` call (CRIT-02).],
  [Cryptography & key management], [10], [38], [Leaked git-history secrets (CRIT-01), VRF mock-randomness feature-gating flaw (HIGH-03), a dead but exported signature-verification stub (MED-04) — base replay protection is solid but does not offset three separate key-handling defects.],
  [Test quality], [8], [58], [1,160+ `#[test]` annotations, 169/169 EVM Foundry tests, and multiple live property-based test runs (23, 33, 81 passing) offset by CI covering only 12 of ~55 pallets and zero Rust tests on the 6 SVM programs.],
  [Transaction correctness], [8], [70], [Router and settlement-engine logic is genuinely tested and passing live; the RPC bridge-ingress "council" framing issue (HIGH-01) is the main deduction, and it is currently inert by design.],
  [Tokenomics & economic safety], [7], [50], [Supply invariant and LP time-locks are strong and verified live; the DEX's floating-point pricing engine (HIGH-02) is a severe, isolated defect that drags the category down.],
  [State integrity], [6], [60], [Supply-ceiling enforcement is strong; migrations for 10 pallets are no-op scaffolding mislabeled as "proper" (MED-01), and cited reproducible-build evidence is fabricated-looking (HIGH-05).],
  [Smart-contract / VM safety], [6], [62], [EVM contracts are well-tested with correct access-control and reentrancy patterns; SVM/Anchor programs have effectively zero test coverage (MED-08).],
  [Cross-chain safety], [6], [45], [The disable-by-default gate is genuinely enforced with a real circuit breaker (strong); the live relay path when enabled trusts a single hardcoded dev key (HIGH-04) and "PoAE" terminology overstates what is proven (LOW-04).],
  [Operational readiness], [6], [50], [`mainnet_release_gate.py` and `snapshot-restore.sh` are genuinely fail-closed, well-built tools; multi-validator and mainnet-feature builds remain unverified per the repository's own status docs.],
  [Networking resilience], [5], [55], [Real, non-fabricated-looking 8/8 cold-start and 7/7 kill-survival mesh evidence exists, but is loopback-only and was not re-executed this session (MED-12).],
  [Governance & upgrade safety], [5], [40], [Validator set is root-controlled with no permissionless staking, understated in top-level docs (HIGH-07); the bridge-mint "council" call (HIGH-01) also reflects a governance-framing weakness.],
  [Proof-gate enforcement], [5], [65], [The required CI aggregate gate is genuinely fail-closed on inspection (not a bypass); coverage is honestly self-documented as partial (12/~55 pallets), and the secret-scan gate is narrower than the incident it should have caught (MED-11).],
  [Observability], [4], [28], [`FEATURE_REGISTRY.toml` cites specific health endpoints as evidence that do not exist anywhere in code (HIGH-06); at least one service hardcodes a "healthy" database status regardless of reality (MED-09).],
  [Deployment reproducibility], [4], [30], [Cited srtool reproducible-build evidence is orphaned and unverifiable in this environment (HIGH-05); default-feature `cargo check` is genuinely reproducible and clean.],
  [Performance evidence], [4], [35], [One honestly-scoped, real measured figure exists (110.6 finTPS, loopback-only); broader benchmarking is soft-fail-only in CI and not treated as a gate.],
  [Documentation accuracy], [3], [45], [The repository is unusually self-critical in most places (`LAUNCH_SCOPE.md`, `FAILURES_AND_TODOS.md`) but contains at least three concretely disprovable claims found in this audit (Chapter 7).],
)

#callout(kind: "info", title: "Worked Example — Cryptography & Key Management")[
  Weight 10, score 38. The category starts from a base of genuinely solid replay/domain-separation protection (`SignedExtra` binds every extrinsic to genesis hash and spec/tx version — a real, correct, standard mechanism worth roughly 70 points on its own). It is reduced to 38 by three compounding defects rather than one: a confirmed, unremediated secrets leak in git history (CRIT-01, the single largest deduction), a VRF module whose safe/mock boundary depends on an accidental feature-flag interaction rather than an explicit opt-in (HIGH-03), and a dead-but-publicly-exported signature-verification stub that would silently rubber-stamp approvals if ever wired up (MED-04). None of these are hypothetical: each has a concrete file:line citation and failure scenario in Chapter 5/6.
]
