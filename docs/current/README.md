# X3 Bridge-Testnet Notes

**Evidence snapshot:** Checks initially recorded June 18, 2026, with later
updates in this directory. They are historical results, not a test report for
the current `master` head.

Use [launch scope](../../LAUNCH_SCOPE.md) for the intended release boundary,
[release gates](../../RELEASE_GATES.md) for required checks, and current
GitHub Actions results for exact-commit evidence.

## Bridge-testnet code paths

The repository contains code and scripts for a bridge-enabled testnet:

- Substrate runtime with `testnet` feature support.
- `pallet-x3-crosschain-gateway` wired into runtime construction.
- Settlement engine bridge integration.
- External bridge adapters for Ethereum, Solana, and Bitcoin.
- Verification router with EVM receipt, Solana, Bitcoin SPV, validator quorum, and X3 internal verifier strategies.
- `x3-relayer` crate that compiles locally.
- EVM gateway contracts and Foundry tests under `X3-contracts/evm`.
- Scripts for local multi-validator testnets, bridge e2e verification, and Anvil plus X3 relayer e2e flow.

This is an inventory of work exercised during earlier development. Code paths
listed here may be feature-gated or incomplete in the latest release candidate.

### Checks recorded during earlier development

The following results were reported at prior commits. They have **not** been
rechecked by this documentation edit, and do not establish current CI status:

- `cargo check --workspace` compiles clean (build fixed 2026-06-18 - Phase 0).
- SHA-256/XOR vulnerability fixed in SVM syscall table.
- Keccak256 syscall added to SVM syscall table.
- All governance origin checks confirmed in agent-law.
- Single-node Anvil-backed bridge relay e2e passes and asserts X3 canonical ledger mutation after deposit relay.
- Release `x3-chain-node` and `x3-relayer` build.
- Full EVM Foundry suite passes.
- Runtime proof submission is wired through `x3_submitCrossVmTransaction` into the kernel `submit_cross_vm_operation` extrinsic for the local relay profile.
- **Phase 3 (Product Integration)**: All 4 GAP claims verified pre-wired: DEX in construct_runtime, launchpad→TokenFactory bridge, CrossVM Router rate limits enforced, LP Locker pallet exists.
- **Phase 4 (Test & Code Quality)**: vm_revert expect() fixed, supply O(n) documented, CI gates 12/55 pallets directly, 24 live-chain tests intentionally ignored.
- **GRANDPA finality cert**: `run_flash_finality_voter()` now derives cert hash from GRANDPA block hash when Flash-Finality is inactive. `build_finalization_request()` accepts cert as parameter. Off-chain storage always populated with non-zero cert.
- **Registry-scored readiness: ~54%** (53.74% across 19 current entries in `FEATURE_REGISTRY.toml`, recalculated 2026-09-20). This is the canonical aggregate; it is not a mainnet-ready claim or a measurement of every aspirational subsystem.

## Read These First

- `docs/current/BRIDGED_TESTNET_RUNBOOK.md` — how to build, start, and verify the bridge-enabled testnet.
- `docs/current/CODEBASE_CAPABILITIES.md` — what the repo actually contains by subsystem.
- `docs/current/VERIFICATION_EVIDENCE.md` — local checks run during this documentation pass.
- `docs/current/FAILURES_AND_TODOS.md` — current hard failures, stubs, and repair order.

## Example verification commands

Run these only in a disposable test environment with the intended feature
flags, chain spec and keys. The seven-validator launcher starts real local
processes; its `--wipe` option removes its testnet base directory.

```bash
cargo check -p x3-chain-runtime --features testnet
cargo check -p x3-relayer
cargo test -p x3-verification-router
cargo test -p x3-bridge-adapters

bash scripts/testnet/run-7-validators-local.sh
bash scripts/testnet/verify-bridge-e2e.sh --count 7 --base-rpc-port 9944
```

For an Anvil-backed EVM deposit relay flow:

```bash
cargo build --release -p x3-chain-node -p x3-relayer
bash scripts/e2e-atomic-trade.sh
```

That flow proves local gateway deposit event emission, relayer event pickup, and
node RPC proof submission through the runtime kernel path. The harness asserts
Alice's X3 canonical ledger balance increases after relay. It does not yet prove
production wrapped-asset minting or a non-local proof-validation profile.

## Legacy Docs

Other dated records live in root markdown files, `docs/reports/`,
`reports/`, `.planning/`, `.audit/`, and package-level READMEs.
Use them to trace past work. Recheck commands and claims against the selected
commit before relying on them in deployment or release notes.
