# x3-lang v0.1 — MVP Surface and Rust Workspace

This folder contains both the current Python MVP surface and the Rust language workspace. It is not a Python-only tree.

Source-of-truth rules:

- The Python files in this directory are authoritative for the currently shipping user-facing intent DSL and MVP pipeline.
- `x3-lang/Cargo.toml`, `compiler/`, `vm/`, `crates/x3-ast`, `crates/x3-common`, and related Rust crates are authoritative for Rust compiler, IR, opcode, and VM bytecode semantics under active development.
- Root repository crates such as `../crates/x3-compiler` and `../crates/x3-integration` are runtime/integration compatibility crates, not the canonical x3-lang language workspace.

Python MVP files:
- `cli.py` — tiny CLI that parses a small `intent` DSL and emits JSON.
- `registry.py` — small asset, DEX, and bridge registry for validation.
- `typechecker.py` — validates the JSON bridge format and known symbols.
- `planner.py` — converts parsed intent JSON into a step-by-step execution plan.
- `simulator.py` — estimates gas, bridge fees, slippage, and expected profit.
- `emitter/` — simple EVM/SVM/X3 emission skeletons for generated plans.
- `schema.json` — JSON Schema for the bridge format.
- `runner.py` — end-to-end pipeline from DSL to planning, simulation, emission, and constraint evaluation.
- `tests/` — pytest coverage for parser, schema validation, typechecker, planner, simulator, and runner.
- `examples/arb_solana_eth.x3` — example intent demonstrating Solana->X3->Ethereum flow.

Rust workspace files:
- `Cargo.toml` — workspace manifest for the Rust language crates.
- `compiler/` — parser, lowering, IR, bytecode emitter, and compiler tests.
- `vm/` — bytecode verifier, executor, bridge adapter, and VM tests.
- `spec/` — opcode and language capability specifications.
- `crates/x3-ast`, `crates/x3-common`, `crates/x3-lexer`, `crates/x3-tools` — shared Rust language crates.

Quick start

1. Run the parser against the example:

```bash
python3 x3-lang/cli.py x3-lang/examples/arb_solana_eth.x3
```

2. Run the full pipeline:

```bash
python3 x3-lang/runner.py x3-lang/examples/arb_solana_eth.x3
```

3. Run tests:

```bash
python3 -m pytest -q x3-lang/tests -q
```

Rust workspace checks:

```bash
cargo test --manifest-path x3-lang/Cargo.toml
```

Rust VM bridge proof schema:

- `docs/x3-lang/bridge-proof-schema.md` documents the JSON proof packets carried in Rust `Operation::Bridge` payloads for Ethereum header/receipt trie verification and Solana bank/transaction proof verification.

Notes and next steps

- This is intentionally tiny: it supports `intent`, `from`, `to`, `path` (with `swap` and `bridge`), and `constraints` entries used in the MVP spec.
- The runner wires parser → typechecker → planner → simulator and reports constraint results.
- **Rust compiler and VM:** `docs/x3-lang/README.md` documents the Rust workspace and root integration crate status. The Rust path is not production-ready and should not be presented as mainnet-ready.
- Next work: add emitters for EVM/Solana/X3, richer simulation, and actual rollback/atomic semantics.
