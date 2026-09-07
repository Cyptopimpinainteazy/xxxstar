#import "../style.typ": *
#import "../components.typ": *
#import "../data.typ": *

= Appendices

== Appendix A: Complete Findings Register

All #all-findings.len() findings, most-severe first. This table is generated directly from `findings.json` at build time.

#table(
  columns: (auto, auto, 2.4fr, 1.6fr, auto),
  fill: (x, y) => if y == 0 { c-brand } else if calc.even(y) { c-bg-panel } else { white },
  [*ID*], [*Sev*], [*Title*], [*File*], [*Status*],
  ..for f in all-findings {
    (
      [#f.id],
      [#severity-badge(f.severity)],
      [#f.title],
      [#text(font: mono-font, size: 7.6pt)[#f.file]],
      [#status-badge(f.status)],
    )
  }
)

== Appendix B: Evidence Ledger — Commands Executed This Session

#table(
  columns: (2.6fr, auto, 2fr),
  fill: (x, y) => if y == 0 { c-brand } else if calc.even(y) { c-bg-panel } else { white },
  [*Command*], [*Exit*], [*Result*],
  [`git log -1`, `git remote -v`, `rustc --version`, `cargo --version`], [0], [Ground truth: commit `fbd4613bd8769ac7422278fae441af1b302a1c88`, branch `master`, rustc 1.90.0, cargo 1.90.0.],
  [`cargo check --workspace --all-features`], [1], [Fails on deliberate `compile_error!` guard (test-verifier vs production in x3-finality-oracle) — correct defensive design, not a bug.],
  [`cargo check --workspace` (default features)], [0], [Clean, 1m52s. Only future-incompat and unused-patch warnings.],
  [`~/.cargo/bin/cargo-audit audit`], [0], [Zero blocking vulnerabilities; 33 allowed unmaintained/unsound warnings (INFO-02).],
  [`cd X3-contracts/evm && forge test --summary`], [0], [169/169 tests passed across 12 suites.],
  [`cargo test -p pallet-x3-cross-vm-router -- --nocapture`], [0], [81 passed, 0 failed.],
  [`cargo test -p pallet-x3-settlement-engine -- --nocapture`], [0], [23 passed (property-based), 0 failed.],
  [`cargo test -p pallet-x3-supply-ledger -- --nocapture`], [0], [33 passed incl. fuzz test, 0 failed.],
  [`cargo test -p pallet-x3-dex -- --nocapture`], [0], [14 passed, 0 failed (see HIGH-02 for why this doesn't mean the arithmetic is safe).],
  [`cargo test -p pallet-x3-lp-locker -- --nocapture`], [0], [19 passed, 0 failed.],
  [`git log --all --oneline -- sepolia-deployer-wallet.txt`], [0], [2 commits — confirms CRIT-01's history-persistence claim.],
  [`git log --all --oneline -- deployment/keys/validator-01-summary.txt`], [0], [2 commits — confirms CRIT-01.],
  [`grep -rl "benchmarks!" pallets/*/src/`], [0], [5 pallets have FRAME benchmarks (cross-chain-validator, x3-atomic-kernel, x3-settlement-engine, x3-inventory, x3-slash).],
  [`grep -rh "^#\[test\]" crates/*/src pallets/*/src node/src runtime/src \| wc -l`], [0], [1,160 test annotations.],
  [`bash -n` on `start-x3-chain.sh`, `start-validator-easy.sh`, `testnet-full-launch.sh`], [0 each], [All three syntactically valid.],
  [`grep -n "test test test\|3750\|8304" crates/x3-rpc/src/wallet_service_rpc.rs`], [0], [Confirmed the hardcoded default mnemonic and fake USD balances underlying CRIT-03.],
  [`grep -n "wallet_service_rpc\|WalletServiceRpc" node/src/rpc.rs`], [0], [Confirmed `WalletServiceRpc` is instantiated and `module.register_method`-registered on the live node RPC surface (CRIT-03).],
)

== Appendix C: Toolchain & Dependency Inventory

- Rust: 1.90.0 (pinned via `rust-toolchain.toml`, targets `wasm32-unknown-unknown`)
- Node.js: 26.8.1; npm 11.19.0
- Python: 3.14.7
- Foundry: `forge`/`cast`/`anvil` present, versions per `X3-contracts/evm/foundry.lock`
- `cargo-audit` present at `~/.cargo/bin/cargo-audit`
- 1,996 crate dependencies scanned by `cargo audit` (Cargo.lock)
- Notable dependency risk: `solana_rbpf` 0.8.5 carries an unsound-code advisory (RUSTSEC-2026-0191) directly relevant to the SVM execution domain (INFO-02)

== Appendix D: Feature-Completeness Matrix (Full)

The complete 67-row matrix is `feature-matrix.csv` alongside this document — each row includes claimed behavior, actual implementation with file:line, runtime wiring, tests, status, evidence quality, missing work, and the source domain file. It is not reproduced row-by-row here to avoid duplicating a machine-readable artifact into prose; open the CSV directly for the full detail, or see Chapter 4 for the summarized version.

== Glossary

#table(
  columns: (auto, 1fr),
  fill: (x, y) => if y == 0 { c-brand } else if calc.even(y) { c-bg-panel } else { white },
  [*Term*], [*Meaning*],
  [Aura], [Substrate's slot-based block-authoring consensus algorithm used for leader selection.],
  [GRANDPA], [Substrate's BFT finality gadget, run alongside Aura to finalize blocks.],
  [Canonical supply invariant], [`represented_total ≤ canonical_supply` — the rule that no more of an asset can be represented across domains than actually exists in the kernel.],
  [PoAE], [Proof of Atomic Execution — this repository's term for an on-chain audit record of bundle lifecycle state; not a cryptographic proof in the ZK/SNARK sense (LOW-04).],
  [X3Native / X3Evm / X3Svm], [The three execution domains: X3's own native asset/consensus layer, EVM (Frontier) compatibility, and Solana-VM compatibility.],
  [`mainnet-rc1`], [A Cargo feature flag gating which capabilities are permitted to compile together for a mainnet release candidate build.],
  [ExternalBridgesEnabled], [A runtime storage flag, default `false`, gating whether external chain bridge operations may execute.],
)

== Assumptions & Limitations

This audit assumed: the audited commit (`fbd4613bd8769ac7422278fae441af1b302a1c88`) is representative of the codebase at the time of writing, despite observed concurrent modification by another agent system during the session (see Front Matter callout); that command outputs captured in this session are trustworthy (no evidence of tampering was found, but this was not independently cryptographically verified); and that the seven domain-audit sub-investigations' file:line citations are accurate as reported (spot-checked in a sample, not exhaustively re-verified line-by-line for all 67 feature rows and 33 findings).

== Unknowns Requiring Operator Confirmation

- Whether `runtime/genesis-presets/production.json` (flagged by a prior internal audit as containing dev-seed accounts with a large endowment) still does so as of the current `HEAD` — this audit did not independently re-verify that specific file's current content.
- Whether the three leaked validator identities (CRIT-01) have been used for anything beyond disposable internal testing since the leak.
- The current git `HEAD` may differ from the audited commit given the observed concurrent-modification activity — confirm `git log -1` matches `fbd4613bd8769ac7422278fae441af1b302a1c88` before treating any file:line citation in this document as current.

== Regeneration Instructions

See `README.md` alongside this document for the exact command to regenerate `booklet.pdf` from source.
