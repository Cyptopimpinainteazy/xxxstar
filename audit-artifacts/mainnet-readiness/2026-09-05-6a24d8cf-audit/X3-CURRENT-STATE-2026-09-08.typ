#set document(title: "X3 Atomic Star: The Road to Mainnet — Current-State Refresh", author: "Codex / AI-assisted analysis")
#set page(paper: "us-letter", margin: (x: 2.0cm, y: 2.0cm), numbering: "1", number-align: center)
#set text(font: "Liberation Sans", size: 10.2pt, lang: "en")
#set heading(numbering: none)
#set par(justify: true, leading: 0.62em)

#let crit(body) = block(fill: rgb("#8f2d2d"), inset: 8pt, radius: 3pt, text(fill: white, weight: "bold")[#body])
#let good(body) = block(fill: rgb("#1f6f43"), inset: 8pt, radius: 3pt, text(fill: white, weight: "bold")[#body])
#let warn(body) = block(fill: rgb("#c9982a"), inset: 8pt, radius: 3pt, text(fill: white, weight: "bold")[#body])

= Cover

#v(2.6cm)
#align(center)[
  #text(size: 12pt, fill: rgb("#555"))[X3 / MAINNET-READINESS AUDIT ARTIFACT]
  #v(0.5cm)
  #text(size: 29pt, weight: "bold")[X3 ATOMIC STAR]
  #v(0.15cm)
  #text(size: 20pt, weight: "bold")[THE ROAD TO MAINNET]
  #v(0.35cm)
  #text(size: 12pt, style: "italic")[Current-state refresh at `master`]
  #v(1.3cm)
  #table(columns: (auto, auto), stroke: none, inset: 4pt, align: left,
    [*Commit:*], [`f74ed4f29`],
    [*Refresh date:*], [2026-09-08],
    [*Predecessor:*], [2026-09-05 field manual frozen at `6a24d8cf`],
    [*Generator:*], [Typst `X3-CURRENT-STATE-2026-09-08.typ`],
  )
  #v(1.5cm)
  #crit[PUBLIC TESTNET: NO-GO \ MAINNET: NO-GO]
  #v(0.8cm)
  #block(fill: rgb("#eef1f5"), inset: 9pt, radius: 4pt, width: 86%)[
    This document supersedes the Sep 5 cover claim for current-master evidence only.
    The historical 127-page field manual remains a frozen artifact for commit `6a24d8cf`.
    This refresh is not an independent security certification.
  ]
]

#pagebreak()

= What changed since the Sep 5 field manual

The Sep 5 manual was correct for its audited source state but is no longer current. Since then the
workspace moved from blocked builds and unverified atomic paths to passing local gates plus an
implemented and tested atomic lifecycle. The most important current facts:

- Workspace Rust check, full tests, and clippy pass with the repository's dev-node evidence path.
- The JavaScript test and production-build suites pass; the Python root test suite passes (181 tests).
- `cargo audit` reports 0 blocking findings.
- GitHub Dependabot critical alerts are 0 after PR #124; remaining alerts are 41 high / 85 medium / 27 low.
- The atomic-kernel overlay diff/reverter, `x3leg:` receipt path, `x3fin:` finalization, and forced
  rollback control are present and covered by workspace tests.
- Three of the four Critical findings in the 2026-09-06 independent audit are closed in current source:
  fabricated wallet RPC is dev-only, `report_misbehavior`/`slash_bond` require root, and the readiness
  registry now has a real-test consistency check.
- The fourth Critical, recoverable secrets in git history, remains open.

One environmental note: the pre-existing nested WASM build failure (`duplicate lang item` in the
Substrate `wasm32-unknown-unknown` builder) still occurs when `SKIP_WASM_BUILD` is unset. The
repository's runtime build script now embeds a real cached runtime WASM when `SKIP_WASM_BUILD=1`, which
let the full test suite and live-node tests execute in this environment. A clean, from-scratch embedded
WASM compile without that workaround remains a release-gate item.

#pagebreak()

= Fresh evidence ledger

#table(columns: (3.4fr, 1fr, 2.8fr), stroke: 0.4pt + rgb("#ccc"), inset: 5pt,
  [*Command / check*], [*Exit*], [*Observed result*],
  [`git status` / `git log -1`], [0], [`master` clean at `f74ed4f29`],
  [`cargo check --workspace --locked` with `SKIP_WASM_BUILD=1`], [0], [Workspace compiles; cached runtime WASM embedded],
  [`cargo test --workspace` with `SKIP_WASM_BUILD=1`], [0], [All active suites pass; 3 known manual node tests ignored],
  [`cargo clippy --workspace --all-targets -- -D warnings` with `SKIP_WASM_BUILD=1`], [0], [No workspace warnings as errors],
  [`cargo audit`], [0], [0 blocking; 27 allow-listed warnings],
  [`npm test`], [0], [All configured JS test packages pass],
  [`npm run build`], [0], [All configured JS builds pass],
  [`pnpm audit` in `apps/x3-studio`], [0], [0 vulnerabilities],
  [`.venv/bin/python -m pytest tests -q`], [0], [181 passed],
  [GitHub open critical Dependabot alerts], [0], [Verified after PR #124 merge],
)

Relevant passing Rust suites observed in the workspace test run include:

- `cross_vm_real_chain_test.rs`: node binary, RPC methods, WebSocket connection, signed extrinsic
  submission, block production, and finality.
- `live_internal_mainnet_e2e.rs`: timeout expiry, duplicate/reordered delivery, bridge-proof crypto,
  and full accounting paths.
- `atomic_swap_orchestrator`: receipt-root commitment, deterministic bundle IDs, and finalization
  request construction.
- `x3-bridge-adapters`: overlay diff round-trips with the atomic-kernel format and runtime-dispatcher
  execution through the client-backed escrow config.
- `pallet-x3-cross-vm-router` and supply-ledger invariant tests.

#pagebreak()

= Current finding status

The table below carries forward the 2026-09-06 independent audit's register (`fbd4613b`) and updates it
only where current source and this session's checks support a status change. Unchanged rows remain
open unless a new commit explicitly closes them; none are silently deleted.

#table(columns: (1.2fr, 1fr, 4.4fr), stroke: 0.4pt + rgb("#ccc"), inset: 5pt,
  [*Finding*], [*Status in refresh*], [*Basis*],
  [CRITICAL-TX-1], [Closed in source], [`wallet_*` RPC registration is gated by `ChainType::Development` in `node/src/rpc.rs:890` and `node/src/service.rs:1054`],
  [CRITICAL-CN-1], [Closed in source], [`report_misbehavior` and `slash_bond` enforce `ensure_root`; tests cover root-only calls],
  [CRITICAL-TOK-1], [Closed in source], [`scripts/check-readiness-consistency.sh` now verifies `required_tests` against real functions; registry fiction was purged],
  [CRITICAL-CK-1], [Open], [`git log --all` still returns commits for `sepolia-deployer-wallet.txt` and validator seed summaries; history rewrite + rotation remains required],
  [HIGH-TX-2], [Open], [No new quorum/threshold change verified in current source],
  [HIGH-CN-2], [Open], [Validator set remains root-controlled; no permissionless staking implemented],
  [HIGH-CK-2], [Open], [Quantum-crypto placeholder is not real Kyber/Dilithium lattice crypto],
  [HIGH-VM-1], [Open], [No new Solana HTLC test coverage verified in this refresh],
  [HIGH-API-1], [Open], [No current evidence that gateway health and per-feature health paths changed],
  [MEDIUM / LOW / INFO rows], [Open unless code inspection confirms closure], [Carried from the 2026-09-06 register],
)

#crit[Open Critical:]
The only remaining Critical carried into this refresh is CRITICAL-CK-1. Because genuine validator and
deployer secrets are recoverable from git history, this alone keeps public testnet and mainnet at NO-GO
until history is rewritten and all affected keys are rotated offline.

#pagebreak()

= Current scorecard

This refresh replaces the old single-number readiness display with a small evidence scorecard. It does
not invent a new aggregate percentage because no complete 64-feature independent re-audit was performed
in this session. Each row is a concrete, current claim.

#table(columns: (2.9fr, 1.1fr, 3.2fr), stroke: 0.4pt + rgb("#ccc"), inset: 5pt,
  [*Criterion*], [*Status*], [*Evidence / limit*],
  [Workspace Rust build], [PASS], [`cargo check --workspace --locked` with `SKIP_WASM_BUILD=1`],
  [Rust tests], [PASS], [`cargo test --workspace`; active suites pass, manual node tests ignored],
  [Rust lint], [PASS], [`cargo clippy --workspace --all-targets -- -D warnings`],
  [Rust dependency advisory], [PASS], [`cargo audit` exit 0; 0 blocking],
  [JS tests / builds], [PASS], [`npm test`, `npm run build`, pnpm studio audit],
  [Python tests], [PASS], [181 passed],
  [GitHub critical advisories], [PASS], [0 critical after PR #124],
  [Atomic lifecycle integration], [PARTIAL PASS], [Overlay, receipt, finalization, rollback, and live node tests pass; no independent multi-validator review],
  [Clean from-scratch WASM compile], [BLOCKED], [Nested duplicate-core failure with `SKIP_WASM_BUILD` unset; cached-WASM workaround used],
  [Git history secret purge], [FAIL], [CRITICAL-CK-1 remains open],
  [Multi-node finality / recovery drill], [NOT VERIFIED], [No fresh four-validator independent run in this refresh],
  [Independent security audit], [NOT VERIFIED], [Not commissioned],
  [External bridge enabled], [NOT APPLICABLE / DISABLED], [Genesis governance flag keeps external routes disabled],
)

#pagebreak()

= Decisions and next steps

#crit[Public testnet: NO-GO. Mainnet: NO-GO.]

The NO-GO is not because the workspace fails to build; the local gates are now green. It is because a
current Critical finding (recoverable secrets in git history) remains open and because public operation
still needs independent security review and multi-validator operational proof.

Next steps in priority order:

1. Rewrite git history and rotate every affected key offline; verify `git log --all` no longer returns
   the secret paths.
2. Engage an independent professional security audit of the runtime and contract stacks.
3. Close the remaining High findings, starting with unilateral wrapped-asset authority and the fake
   post-quantum placeholder naming.
4. Run a fresh four-validator finality/partition/recovery drill with retained raw evidence.
5. Repair the clean embedded WASM build without the cached-artifact workaround and promote it to a gate.
6. Refresh the full 64-feature scorecard only after the independent audit and multi-node drill provide
   current evidence for each row.

#pagebreak()

= Regeneration

Compile this PDF from the artifact directory with:

```text
typst compile --root . X3-CURRENT-STATE-2026-09-08.typ X3-ROAD-TO-MAINNET.pdf
```

The historical audit files remain beside this source. Checksums and manifest entries should be
regenerated after this PDF is published.
