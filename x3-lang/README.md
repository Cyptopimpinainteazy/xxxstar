# x3-lang v1.0 — Production Language & VM Workspace

**Status: 🚧 ACTIVE DEVELOPMENT — Per-feature readiness below (derived from `FEATURE_REGISTRY.toml`)**

> Readiness scores are the canonical source. Run `scripts/check-readiness-consistency.sh` to validate.
> See `CURRENT_MAINNET_STATUS.md` for the full system scoreboard.

This folder contains the Python MVP surface and the production Rust language workspace.

## Source-of-truth rules

- Python files: authoritative for the shipping intent DSL and MVP pipeline.
- `x3-lang/Cargo.toml`, `compiler/`, `vm/`, `crates/x3-ast`, `crates/x3-common`: authoritative for Rust compiler, IR, opcode, and VM bytecode semantics.

## Python MVP files
- `cli.py`, `registry.py`, `typechecker.py`, `planner.py`, `simulator.py`, `emitter/`, `schema.json`, `runner.py`, `tests/`, `examples/arb_solana_eth.x3`

## Rust workspace files
- `Cargo.toml`, `compiler/`, `vm/`, `spec/`, `crates/x3-ast`, `crates/x3-common`, `crates/x3-lexer`, `crates/x3-tools`

## Trading Core v1

Trading Core v1 adds chain-qualified assets, linear debts, explicit swap
routes, risk-policy guards, debt-closure and net-profit verification,
capability-controlled atomic VM execution, and deterministic receipts.

Canonical example: `examples/trading_core_v1.x3`.

Supported today:

- parse, type-check, verify, lower, encode, decode, and disassemble trading declarations;
- checked decimal-to-base-unit accounting with explicit rounding controls;
- borrow/swap/repay/net-profit/all-debts/commit verification;
- fixture-host execution with atomic rollback and receipt verification;
- `x3c receipt inspect <receipt.json>` and `x3c receipt verify <receipt.json>`.

Explicit v1 boundaries:

- no live Aave, Uniswap, SushiSwap, bridge, or private-relay adapters;
- no route discovery or oracle aggregation;
- fixture capabilities are rejected in production/mainnet execution mode;
- no live-DEX or mainnet claim is made without real adapter evidence.

## Canonical native trading runtime

`crates/x3-integration::compiler_bridge::compile_source` now uses this workspace's
compiler. Compilation mode can be selected explicitly with `compile_source_with_mode`;
`compile_artifact` returns source/artifact hashes and the compiler build version.
The old contract grammar is available through `compile_legacy_source` and produces
legacy X3BC. Canonical compilation never falls back to that grammar.

`crates/x3-execution` owns the trading IR, sequence verifier, accounting and
transactional interpreter used by both developer tooling and validator WASM.
The validator's initial canonical profile accepts version-bound trading instructions
and the exact private-submission gate. Unsupported instructions, metadata semantics,
unknown policy fields, mismatched versions, and missing policy bindings are rejected.
Other canonical VM instructions have not been enabled on this runtime path.

`pallet-x3-kernel::submit_comit_v2` executes that profile against the canonical
ledger through the native host. Governance must first bind exact native asset
identities to registered assets; the gas identity must match `NativeFeeAssetId`.
A lender or pool owner signs its own registration. These registrations opt actual
ledger balances into zero-fee same-dispatch lending or zero-fee constant-product
swaps. An unfunded lender/pool cannot provide liquidity. The host uses the existing
AMM arithmetic and checks the projected affected-ledger commitment before commit.
Trade rejection, insufficient fee payment and later prepare-root rejection roll
back the entire dispatch, including balances, fees, nonce and events.

The host has no external DEX, bridge, oracle aggregation, MEV attestation, protected
ordering or persistent strategy-loss capability. Programs requiring those capabilities
are refused. Private submission depends on the runtime's configured channel and is
unavailable by default. Native execution receipts bind the artifact, affected ledger,
operations, typed deltas and costs; they are not independent finality attestations.
The cost input is the kernel execution fee; outer transaction-payment charges
and tips are not yet reconciled into this economic receipt.

`examples/native_trading.x3` is the deterministic native test route. The tests seed
actual FRAME ledger storage with explicit test assets and reserves. Live deployment
requires matching chain asset metadata and funded, signed capability registrations.
This is implementation and component-test evidence; live validator tests, calibrated
runtime weights, restart tests and security review remain release requirements.

## VM Opcode Surface — Per-Feature Readiness

The atomic_router feature owns VM control-flow execution. Its readiness score is **85%** (derived from `FEATURE_REGISTRY.toml`).

| Opcode Group | Status | Detail |
|---|---|---|
| Arithmetic (ADD, SUB, POW) | ✅ EXECUTING | Register-to-register with gas metering |
| Memory (LOAD, STORE) | ✅ EXECUTING | 16-byte aligned loads/stores |
| Asset ops (LOCK, MINT, BURN, RELEASE, SWAP) | ✅ EXECUTING | Compiler-stream payload required |
| BRIDGE transfers | ✅ EXECUTING | Cross-chain transfer with proof verification |
| EMIT (EVM call) | ✅ EXECUTING | Via bridge adapter |
| CALL_HOST (SVM call) | ✅ EXECUTING | Via bridge adapter |
| Capability dispatch (GPU/SIMULATE/etc.) | ✅ EXECUTING | 13+ host capabilities |
| CALL / RET | ✅ EXECUTING | Call stack with return address |
| NOP, HALT | ✅ EXECUTING | |
| IF, LOOP | ✅ EXECUTING | Register-conditioned skip/jump with bounded loops |
| REQUIRE | ✅ EXECUTING | Panics on zero condition, caught by ON_FAIL handler |
| ON_FAIL, ON_TIMEOUT | ✅ EXECUTING | Handler push → dispatch on trap, + deadline enforcement |
| ATOMIC_BEGIN, ATOMIC_END, ATOMIC_ROLLBACK | ✅ EXECUTING | Snapshot/commit/rollback with full state restore |

> **Note**: All opcodes execute in the VM. ON_FAIL now wires real failure-handler dispatch — a trapped opcode transfers control to the most recent handler target instead of immediately returning `Err`. The 85% readiness score reflects remaining production hardening: multi-validator atomic execution tests, external bridge integration, and CI gate wiring — not missing VM functionality.

## Bridge Production Backend ✅

`init_production_backend()` in [`vm/src/bridge.rs`](vm/src/bridge.rs:2990) supports 4 verifier families:
- `evm-light-client` (EthereumLightClientVerifier)
- `svm-light-client` (SolanaLightClientVerifier)
- `evm-rpc` (EthereumRpcFinalityVerifier)
- `svm-rpc` (SolanaRpcFinalityVerifier)

All configured via environment variables. Never silently falls back to dry-run.

## Quick start

```bash
python3 x3-lang/cli.py x3-lang/examples/arb_solana_eth.x3
cargo test --manifest-path x3-lang/Cargo.toml
```

## Rust Production Surface

```bash
cargo run -p x3-tools --bin x3c -- parse/check/lower/build/simulate/run/explain <FILE.x3>
```

**x3-lang status: All 26 opcodes execute. 10+ E2E control-flow tests pass. Bridge backend wired. Ready for production hardening. 🚧**
