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
    [*Commit:*], [`d082f18de`],
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
- The clean embedded runtime build now works from an empty nested target without `SKIP_WASM_BUILD`:
  the pinned toolchain adds `wasm32v1-none`, the vendored `sp-wasm-interface` no longer leaks
  `parity-scale-codec/std`, and `x3-cross-vm-bridge` std-only crypto (`blst`, `ed25519-dalek`) is
  optional and std-gated. `scripts/ci/check_runtime_wasm_non_stub.sh` passes on the release artifact.
- Three of the four Critical findings in the 2026-09-06 independent audit are closed in current source:
  fabricated wallet RPC is dev-only, `report_misbehavior`/`slash_bond` require root, and the readiness
  registry now has a real-test consistency check.
- HIGH-API-1 is closed in source: the gateway `/health` and `/readyz` endpoints run a real DB
  round-trip and return 503 when it fails, `/livez` preserves process liveness, and the registry no
  longer cites fabricated `/health/<feature>` paths.
- HIGH-TX-2 is closed in source: `x3_submitCrossVmTransaction` is registered only on
  Development/Local chain specs, no longer auto-submits wrapped register/mint under a
  threshold-1 council call, and every Live chain spec requires at least two council members.
- HIGH-CK-2 is closed in source: the simulated primitives are renamed `kyber_research` and
  `dilithium_research`, and crate/package docs explicitly forbid real-security use until audited
  PQC bindings are integrated.
- HIGH-VM-1 is closed in source: `x3_htlc` is added to the SVM workspace with six HTLC-specific
  unit tests (plus Anchor's `test_id`) covering timelock bounds, preimage hashlock checks,
  expiry, and PDA derivation.
- A fresh four-validator reserved full-mesh drill converged to one finalized chain on cold start
  and survived loss of each individual validator with the remaining three finalizing one chain.
- CRITICAL-CK-1 is remediated on all repository refs: `master` was rewritten and force-pushed, 49
  secret-bearing GitHub branches and the old RC tag were deleted, and local reflog/object GC removed
  unreachable secret objects. Remaining work is coordination only: re-create pre-push clones and
  request GitHub-side GC if old blobs persist in the service's object cache.

#pagebreak()

= Fresh evidence ledger

#table(columns: (3.4fr, 1fr, 2.8fr), stroke: 0.4pt + rgb("#ccc"), inset: 5pt,
  [*Command / check*], [*Exit*], [*Observed result*],
  [`git status` / `git log -1`], [0], [`master` clean at `d082f18de`],
  [`cargo check --workspace --locked` with `SKIP_WASM_BUILD=1`], [0], [Workspace compiles; cached runtime WASM embedded],
  [`cargo test --workspace` with `SKIP_WASM_BUILD=1`], [0], [All active suites pass; 3 known manual node tests ignored],
  [`cargo clippy --workspace --all-targets -- -D warnings` with `SKIP_WASM_BUILD=1`], [0], [No workspace warnings as errors],
  [`cargo build --release -p x3-chain-runtime` with `SKIP_WASM_BUILD` unset and empty nested target], [0], [WASM rebuilt from source with `wasm32v1-none`; real compact/compressed blob emitted],
  [`bash scripts/ci/check_runtime_wasm_non_stub.sh`], [0], [No stubbed `wasm_binary.rs`; non-trivial WASM artifact present],
  [`cargo test -p x3-gateway router_serves_real_http_over_ephemeral_port_without_db`], [0], [`/livez` 200; `/health` and `/readyz` 503 with `db:false` against a dead pool],
  [`cargo test --manifest-path X3-contracts/svm/Cargo.toml -p x3_htlc`], [0], [7 HTLC tests pass (timelock, hashlock, expiry, PDAs)],
  [`cargo test -p quantum-crypto`], [0], [22 tests pass with renamed research/simulated modules],
  [`cargo test -p x3-chain-node --lib`], [0], [48 pass; 3 ignored; live-node RPC integration included],
  [`TESTNET_BASE=/tmp/x3-mesh-4 python3 scripts/testnet/run-mesh.py cycles --count 4 --cycles 1`], [0], [Fresh four-validator full mesh converged in 14s with one finalized head],
  [`TESTNET_BASE=/tmp/x3-mesh-4 python3 scripts/testnet/run-mesh.py kills --count 4`], [0], [4/4 single-validator losses left 3/3 survivors finalizing one common chain],
  [`git log --oneline master -- sepolia-deployer-wallet.txt deployment/keys/validator-0*-summary.txt`], [0], [0 secret-path commits remain in rewritten `master`],
  [`git push --force-with-lease ... origin master`], [0], [Rewritten master history pushed at `58982d258`],
  [`git push origin --delete <49 secret-bearing branches>`], [0], [All GitHub branches containing pre-rewrite secret commits deleted; 5 clean branches remain],
  [`git fsck --full --no-reflogs` after `git gc --prune=now`], [0], [No dangling/unreachable secret-bearing objects remain locally],
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
  [CRITICAL-CK-1], [Closed in source], [`master` rewritten at `58982d258`; old Sepolia/validator material revoked; 49 secret-bearing GitHub branches plus RC tag deleted; unreachable objects pruned locally],
  [HIGH-TX-2], [Closed in source], [`x3_submitCrossVmTransaction` is Development/Local-only and no longer auto-mints wrapped assets; Live chain specs require at least two council members],
  [HIGH-CN-2], [Closed in source], [`LAUNCH_SCOPE.md` now explicitly lists permissionless validator staking/bonding/nomination as NOT IMPLEMENTED and deferred to M3],
  [HIGH-CK-2], [Closed in source], [Simulated modules renamed `kyber_research`/`dilithium_research`; docs and Cargo description forbid production use],
  [HIGH-VM-1], [Closed in source], [Six HTLC unit tests plus Anchor `test_id` pass in the SVM workspace],
  [HIGH-API-1], [Closed in source], [`/health` and `/readyz` run a real DB round-trip and return 503 when down; `/livez` is liveness; registry `health_endpoint` fiction removed],
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
