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

## Rust Production Surface

The Rust workspace at `x3-lang/` ships a full compiler, IR, bytecode
emitter, verifier, and dry-run VM for the production x3-lang intent
DSL. The CLI binary `x3c` exposes the full pipeline:

```bash
# Parse to JSON AST
cargo run -p x3-tools --bin x3c -- parse <FILE.x3>

# Semantic check (symbols, routes, atomic balance, replay, expiry, allow-list)
cargo run -p x3-tools --bin x3c -- check <FILE.x3>

# Lower to typed IR
cargo run -p x3-tools --bin x3c -- lower <FILE.x3> --out ir.json

# Build bytecode
cargo run -p x3-tools --bin x3c -- build <FILE.x3> --out program.x3b

# Simulate (dry-run VM)
cargo run -p x3-tools --bin x3c -- simulate <FILE.x3>

# Run on dry-run VM
cargo run -p x3-tools --bin x3c -- run <FILE.x3>

# Disassemble bytecode
cargo run -p x3-tools --bin x3c -- explain <program.x3b>

# Emit a reference example fixture
cargo run -p x3-tools --bin x3c -- test-fixture --out example.x3
```

### Rust production safety gates

The Rust path runs four gates that the MVP Python surface does not:

1. **Semantic verifier** (`compiler/src/semantic.rs`): validates
   symbols, route depth, atomic balance, rollback presence, replay
   protection (nonce), expiry, bridge adapter allow-list, and chain
   compatibility. Catches what bytecode verification cannot.
2. **Bytecode verifier** (`vm/src/verifier.rs`): validates opcode
   operands, jump boundaries, and payload structure. Walks past
   compiler-stream metadata correctly.
3. **Dry-run VM** (`vm/src/executor.rs`): executes the bytecode
   end-to-end against a synthetic bridge adapter and asserts the
   rollback invariants.
4. **BTC/UTXO feature gate** (`vm/src/btc_adapter.rs`): the Bitcoin
   adapter fails closed on every cross-VM call unless the
   `bitcoin-adapter` feature is explicitly enabled on a node that has
   wired up a real Bitcoin light client. The default build returns
   `X3_BTC_ADAPTER_DISABLED` so a misconfigured production
   environment cannot silently route through a stub.

### Rust test coverage

- 11 E2E fixture tests (`compiler/tests/test_e2e_fixtures.rs`)
  exercise the full pipeline through the production intent surface:
  transfer, atomic swap, EVM call, X3 internal call, BTC/UTXO
  route, plus negative cases (invalid route, unknown chain,
  malformed input).
- 24 parser coverage tests (`compiler/tests/test_parser_coverage.rs`)
  hit every dispatch arm of the inline tokenizer/parser.
- 9 CLI integration tests (`crates/x3-tools/tests/cli.rs`) validate
  the binary's parse/check/lower/build/run/explain/test-fixture
  subcommands and exit codes.
- VM verifier and bridge adapter unit tests
  (`vm/src/btc_adapter.rs`, `vm/src/verifier.rs`).

Run the full suite with:

```bash
cd x3-lang
cargo fmt --all -- --check
cargo clippy -p x3-lang-compiler -p x3-lang-vm -p x3-lang-ast \
            -p x3-lang-common -p x3-lang-lexer -p x3-tools \
            --all-targets --all-features -- -D warnings
cargo test -p x3-lang-compiler -p x3-lang-vm -p x3-lang-ast \
           -p x3-lang-common -p x3-lang-lexer -p x3-tools --all-features
```

Notes and next steps

- This is intentionally tiny: it supports `intent`, `from`, `to`, `path` (with `swap` and `bridge`), and `constraints` entries used in the MVP spec.
- The runner wires parser → typechecker → planner → simulator and reports constraint results.
- **Rust compiler and VM:** `docs/x3-lang/README.md` documents the Rust workspace and root integration crate status. The Rust path is not production-ready and should not be presented as mainnet-ready.
- Next work: add emitters for EVM/Solana/X3, richer simulation, and actual rollback/atomic semantics.
