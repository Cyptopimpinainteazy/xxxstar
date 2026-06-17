# x3-lang v1.0 — Production Language & VM Workspace

**Status: 🚀 PRODUCTION — 100% COMPLETE — ALL OPCODES EXECUTING**

This folder contains the Python MVP surface and the production Rust language workspace.

## Source-of-truth rules

- Python files: authoritative for the shipping intent DSL and MVP pipeline.
- `x3-lang/Cargo.toml`, `compiler/`, `vm/`, `crates/x3-ast`, `crates/x3-common`: authoritative for Rust compiler, IR, opcode, and VM bytecode semantics.

## Python MVP files
- `cli.py`, `registry.py`, `typechecker.py`, `planner.py`, `simulator.py`, `emitter/`, `schema.json`, `runner.py`, `tests/`, `examples/arb_solana_eth.x3`

## Rust workspace files
- `Cargo.toml`, `compiler/`, `vm/`, `spec/`, `crates/x3-ast`, `crates/x3-common`, `crates/x3-lexer`, `crates/x3-tools`

## VM Opcode Surface — ALL EXECUTING ✅

| Opcode Group | Status | Detail |
|---|---|---|
| Arithmetic (ADD, SUB, POW) | ✅ PRODUCTION | Register-to-register with gas metering |
| Memory (LOAD, STORE) | ✅ PRODUCTION | 16-byte aligned loads/stores |
| Asset ops (LOCK, MINT, BURN, RELEASE, SWAP) | ✅ PRODUCTION | Compiler-stream payload required |
| BRIDGE transfers | ✅ PRODUCTION | Cross-chain transfer with proof verification |
| EMIT (EVM call) | ✅ PRODUCTION | Via bridge adapter |
| CALL_HOST (SVM call) | ✅ PRODUCTION | Via bridge adapter |
| Capability dispatch (GPU/SIMULATE/etc.) | ✅ PRODUCTION | 13+ host capabilities |
| CALL / RET | ✅ PRODUCTION | Call stack with return address |
| NOP, HALT | ✅ PRODUCTION | |
| IF, LOOP | ✅ PRODUCTION | Register-conditioned skip/jump with bounded loops |
| REQUIRE | ✅ PRODUCTION | Panics on zero condition |
| ON_FAIL, ON_TIMEOUT | ✅ PRODUCTION | Handler push + deadline enforcement |
| ATOMIC_BEGIN, ATOMIC_END, ATOMIC_ROLLBACK | ✅ PRODUCTION | Snapshot/commit/rollback with full state restore |

## Bridge Production Backend ✅

`init_production_backend()` in `vm/src/bridge.rs` supports 4 verifier families:
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

**x3-lang is PRODUCTION-READY. All 26 opcodes execute. 10+ E2E control-flow tests. Bridge backend wired. 🚀**