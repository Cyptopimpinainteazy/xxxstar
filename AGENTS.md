<!-- bmad:context -->
<!-- Verified 2026-10-23 against c8d308a. Managed by bmad-project-context; edits inside this block are replaced on refresh. Keep anything you want preserved outside the markers. -->

## X3 Atomic Star

Substrate L1 with three execution domains — X3Native (X3VM), X3Evm, X3Svm — that settle atomically in one block. Root Cargo workspace: `runtime/` (`x3-chain-runtime`), `node/` (`x3-chain-node`), `pallets/` (60), `crates/`. `x3-lang/` is a separate workspace for the `.x3` language; `X3-contracts/` is a separate Foundry + Anchor workspace. Contributor detail `CONTRIBUTING.md`; gate evidence `launch-gates/`; language detail `x3-lang/README.md`.

## Policy

- Fix real code. Do not update documents instead of implementing working systems.
- Treat code as the source of truth; a roadmap or status markdown the code contradicts is stale.
- Never weaken, skip, or delete a failing test to get green — `make guard` fails on test-cheat.
- Never ship fake adapters, fake relayers, fake proofs, no-op execution paths, placeholder logic, TODO-only work, mocks outside test-only modules, or silent fallbacks in security code.
- Never claim mainnet readiness without `make mainnet-check` passing.
- Never commit secrets, keys, seeds, or RPC credentials. `scripts/mock-rpc-server.js` is dev-only.
- Atomic routes run: parse → semantic check → lower to typed IR → validate → preflight every leg → lock assets → execute → verify receipts → commit all legs → roll back all failed legs → settle → assert invariants. Partial success without rollback is forbidden.

## Where things are

- The atomic-swap engine is one crate, wired through `node/Cargo.toml` and `pallets/x3-settlement-engine`. `crates/x3-atomic-swap/src/` holds the per-chain HTLC legs (`bitcoin_htlc.rs`, `evm_htlc.rs`, `svm_htlc.rs`, `x3vm_htlc.rs`, `cosmwasm_htlc.rs`, …) plus `intent.rs`, `relayer.rs`, `rpc_quorum.rs`, `timeout.rs`, `slashing.rs`, `finality.rs`, `registry.rs` (solver registry) and `scoreboard.rs`. Most "critical systems" are files in it — look here before creating a crate.
- Bridges `crates/x3-bridge-adapters`; finality `crates/x3-finality-oracle`; attestation `crates/x3-validator-attestation`; chain health `crates/x3-chain-health-daemon`; swarm `crates/x3-swarm-core`; proof ledger `proof/` + `crates/x3-proof`; slashing `pallets/x3-slash`.
- Two languages live here. X3BC: `crates/x3-parser` + `crates/x3-compiler`, opcode table `crates/x3bc-core/src/opcode.rs` (re-exported by `crates/x3-backend/src/opcode.rs`), engine `crates/x3-vm` — this one runs on chain. x3-lang intent language: `x3-lang/compiler`, 84 opcodes in `x3-lang/spec/opcodes.yaml`, engine `x3-lang/vm` — this one runs in tests and `x3c` only, never in a block.
- `x3c` is the x3-lang CLI: `cargo run -p x3-tools --bin x3c -- parse|check|lower|build|simulate|run|explain <FILE.x3>`.
- Testnet bring-up `scripts/bootstrap-validator.sh`; release gate `scripts/mainnet_release_gate.py`.

## Running and verifying

- `scripts/local-ci.sh` is the gate runner. GitHub-hosted Actions are billing-blocked, so there is no green CI check to lean on. `--list` for gate slugs, `--only <slug>` for one.
- Inner loop `scripts/local-ci.sh` (fmt, guards, check, lint, unit); full root workspace suite `--deep` (~5–15 min); pre-push `--pre-push`.
- Never pass `--all-features`: `crates/x3-finality-oracle/src/lib.rs:4` raises `compile_error!` when `test-verifier` and `production` are both on. Use `--features test-verifier`.
- Root `cargo test --workspace` skips nested workspaces — `x3-lang/`, `adapters/`, `crates/x3-sidecar`, `crates/x3-sim`, `crates/x3-mc`, `services/*`, `X3-contracts/*`. Run their own gates or you ship a break nobody sees.
- `make test` covers only the x3-lang suites. `scripts/x3-verify.sh` passes JS tests unconditionally (`pnpm test || true`), so it is not a JS gate; the real JS gates are the twelve `test js <project>` entries in `local-ci.sh`. There is no workspace lockfile — each project installs from its own; `apps/x3-studio` is pnpm, the rest npm.
- A gate that could not start is BLOCKED, never PASS. Report BLOCKED with the reason.
- Commit messages must match `[area] description; tests: <command>`; the hook is inert until `scripts/x3-install-git-hooks.sh` runs.

## Conventions that differ from defaults

- There are two of several things, and editing the wrong one looks like success. `crates/x3-ast`, `crates/x3-common` and `crates/x3-lexer` exist in both the root workspace and `x3-lang/crates/`; confirm which workspace you are in before editing.
- Two parsers read one language: the Rust compiler (`x3-lang/compiler/src/parser.rs`) and Python (`x3-lang/cli.py`, `x3-lang/typechecker.py`). They drift in one direction — the compiler and `x3c fmt` accept spellings the Python surface rejects. Change one, run `x3-lang/tests/test_surface_drift.py`, and keep anything `x3c fmt` writes readable by both.
- Money-like values — balances, rewards, slashes, settlement amounts — are exact: no `f64`/`f32` in `pallets/*/src`, `runtime/src`, or consensus paths. A legitimate float writes `// float-exemption: <reason>` on its own line.
- A `#[cfg(feature = "std")]` guard around a check deletes that check in the wasm build; the runtime builds its local crates with `default-features = false`. Every `no_std` crate must pass `cargo check -p <crate> --no-default-features` alone, never batched — cargo unifies features across a batch.
- A pallet's runtime config must name a real `WeightInfo`; `type WeightInfo = ();` returns `Weight::zero()` and drops the pallet from block-weight accounting.

## Known pitfalls

- After any dependency change, regenerate each nested workspace's `Cargo.lock`; a stale nested lock has been fixed three times recently (`ed06a4d9`, `ea3f7cad`, `d85e7016`).
- `Swatinem/rust-cache` must set `cache-bin: false`; its save step unlinks `$CARGO_HOME/bin` and wiped rustup, subkey, cargo-audit and srtool twice on 2026-09-26.
- Never hardcode `/home/<user>/Desktop/` in a script; derive the root from `BASH_SOURCE` or a documented override.
- Keep nightly artifacts out of `target/` — put them in `target-fuzz/`; mixing toolchains makes the next stable gate fail as "compiled by an incompatible version of rustc".
- Never enable `test-verifier` and `production` together on `x3-finality-oracle`.
- Never run `npm run build` as a verification step: it rewrites tracked `dist/` output (12 deleted, 4 modified files across `apps/x3-desktop`, `packages/polkawallet-plugin`, `packages/atomic-swap-sdk`). Tests only.
- `scripts/x3-detect-stubs.sh` and `scripts/x3-detect-test-cheats.sh` are still named by older rule files but never complete — they walk build output and die at the 240 s timeout. Use `python3 scripts/x3_fake_code_scan.py stubs` (or `cheats`), which ratchets against `docs/reports/fake-code-baseline.json`.

## Reporting

End every task with a scoreboard — `<domain>/<subsystem>  <10-block bar>  <percent>%  <honest status>` — then What changed / Still missing / Next best action. Score only the subsystem touched, on evidence: not wired into the real execution path caps at 60%, stubs in the core path cap at 50%. Follow it with the completion block: Files changed / Commands run / Proof result / Remaining blockers / Next 10 tasks / Completion percent.

## Autonomy

When the next valid action is clear from the task, repo state, tests, or the master plan, take it. Do not close a turn asking to proceed, to run tests, or to confirm the next step. Ask only when a genuine decision or blocker cannot be resolved from code, docs, or task state. A report is a checkpoint, not a stop.

## Verification states

Use exactly: NOT RUN / RUNNING / PASS / FAIL / BLOCKED. Only PASS counts as evidence. RUNNING is not PASS, and no claim may cite a suite as evidence until its command exits with a recorded status.

## Task tracking

Use the TODO MCP tools (`todo_get_tasks`, `todo_add_tasks`, `todo_update_tasks`, `todo_delete_tasks`, `todo_clear_category`, `todo_move_category`) on `todo-mcp` or `todo-extension`. Never edit `.todo` directly.

<!-- /bmad:context -->
