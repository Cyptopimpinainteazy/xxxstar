# X3 Programming Language — Current State & Crate Map

## Source-Of-Truth Clarification

This repository carries multiple X3 language implementation tracks. The important distinction is not "`x3-lang/` means Python": `x3-lang/` contains both the Python MVP surface and a Rust language workspace.

| Tree | Status | Description |
|------|--------|-------------|
| `x3-lang/*.py`, `x3-lang/emitter/`, `x3-lang/tests/*.py` | **Shipping MVP surface** | Working Python parser, registry, typechecker, planner, simulator, runner, and pytest suite for the current cross-chain intent DSL. |
| `x3-lang/Cargo.toml`, `x3-lang/compiler`, `x3-lang/vm`, `x3-lang/crates/*` | **Active Rust language workspace, still incomplete** | Rust parser/lowering/emitter/VM crates for X3 bytecode work. This is the authoritative place for Rust bytecode/compiler semantics, but it is not production-ready mainnet infrastructure. |
| root `crates/x3-compiler`, root `crates/x3-integration`, root `crates/x3-*` | **Runtime/integration compatibility layer** | Substrate/runtime-facing crates. Some are partial or compatibility shims and should not be confused with the Rust language workspace under `x3-lang/`. |

### MVP Status

**X3 is an MVP.** Use the Python MVP as authoritative for today's user-facing intent DSL and examples. Use the Rust workspace under `x3-lang/` as authoritative for Rust compiler, IR, opcode, and VM bytecode contracts. Root `crates/x3-*` integration crates are authoritative only for their runtime integration surfaces and may intentionally reject unsupported compiler paths.

This document describes what exists today. Do not treat advertised language features as complete unless they map to real code, tests, and reachable execution paths.

## Two Workspace Trees

### `x3-lang/` Python MVP — Currently shipping intent surface

The Python files in `x3-lang/` contain the working cross-chain intent parser and pipeline:

| Component | File | Status |
|-----------|------|--------|
| CLI parser | `cli.py` | Working — parses `intent`/`from`/`to`/`path`/`constraints` |
| Registry | `registry.py` | Working — asset, DEX, bridge validation |
| Typechecker | `typechecker.py` | Working — validats JSON bridge format + known symbols |
| Planner | `planner.py` | Working — step-by-step execution plans |
| Simulator | `simulator.py` | Working — gas/bridge/slippage estimation |
| Emitter skeletons | `emitter/` | Skeletal — EVM/SVM/X3 emission stubs exist |
| Runner | `runner.py` | Working — end-to-end pipeline |
| Tests | `tests/` | Working — pytest coverage |

### `x3-lang/` Rust Workspace — Compiler and VM bytecode work

The Rust workspace rooted at `x3-lang/Cargo.toml` contains the language compiler/VM crates:

| Crate | Directory | Status |
|-------|-----------|--------|
| `x3-lang-compiler` | `x3-lang/compiler` | ⚠️ PARTIAL — parser, lowering, IR, and emitter exist; bytecode semantics and coverage are still being hardened. |
| `x3-lang-vm` | `x3-lang/vm` | ⚠️ PARTIAL — verifier/executor/bridge adapter exist; full production execution semantics are incomplete. |
| `x3-lang-ast` | `x3-lang/crates/x3-ast` | ⚠️ PARTIAL — shared AST definitions used by the Rust compiler. |
| `x3-lang-common` | `x3-lang/crates/x3-common` | ⚠️ PARTIAL — shared errors, spans, symbols, and capability payload codec. |
| `x3-lang-lexer` | `x3-lang/crates/x3-lexer` | ⚠️ PARTIAL — lexer crate exists; parser currently still has inline tokenization. |
| `x3-tools` | `x3-lang/crates/x3-tools` | ⚠️ PARTIAL — tooling entry points. |

Bridge proof producers for the Rust VM should follow
[`bridge-proof-schema.md`](./bridge-proof-schema.md). That document defines the
JSON carried in `Operation::Bridge` proof fields for Ethereum header/receipt
trie proofs and Solana bank/transaction proofs.

### Root `crates/x3-*` Rust Integration Layer — Partially built

These root workspace crates share names with language components but are **integration/compat layers** for the Substrate runtime build. Many have partial or stubbed implementations:

| Crate | Directory | Status |
|-------|-----------|--------|
| `x3-compiler` | `crates/x3-compiler` | ⚠️ PARTIAL — Parser stubs (literal-only return, empty body). Gateway only supports `xvm_transfer`. Integration tests test empty MIR. |
| `x3-lexer` | `crates/x3-lexer` | ⚠️ PARTIAL — Tokenization exists, integration into full pipeline incomplete |
| `x3-ast` | `crates/x3-ast` | ⚠️ SKELETAL — AST definitions exist |
| `x3-common` | `crates/x3-common` | ⚠️ PARTIAL — Shared utilities |
| `x3-vm` | `crates/x3-vm` | ⚠️ PARTIAL — VM runtime core exists, storage with snapshot/rollback works, full execution pipeline incomplete |
| `x3-parser` | `crates/x3-parser` | ⚠️ PARTIAL |
| `x3-hir` | `crates/x3-hir` | ⚠️ PARTIAL |
| `x3-mir` | `crates/x3-mir` | ❌ STUB — Minimal IR definitions |
| `x3-backend` | `crates/x3-backend` | ❌ STUB |
| `x3-typeck` | `crates/x3-typeck` | ❌ STUB |
| `x3-opt` | `crates/x3-opt` | ❌ STUB |
| `x3-verifier` | `crates/x3-verifier` | ❌ STUB |
| `x3-stdlib` | `crates/x3-stdlib` | ❌ STUB |
| `x3-cli` | `crates/x3-cli` | ❌ STUB |
| `x3-lsp` | `crates/x3-lsp` | ❌ NOT IMPLEMENTED |
| `x3-integration` | `crates/x3-integration` | ⚠️ PARTIAL — execution integration exists; compile-feature bridge path is explicitly disabled until a real compiler bridge is wired. |

### Planned Crates — NOT IMPLEMENTED

The following crates are referenced in design documents but do not yet have any implementation:

| Crate | Intended Role | Status |
|-------|---------------|--------|
| `x3-runtime` | Agent runtime and scheduler | ❌ NOT IMPLEMENTED |
| `x3-reaper` | Compute economy module | ❌ NOT IMPLEMENTED |
| `x3-fmt` | Code formatter | ❌ NOT IMPLEMENTED |
| `x3-lint` | Linter | ❌ NOT IMPLEMENTED |
| `x3-pkg` | Package manager | ❌ NOT IMPLEMENTED |
| `x3-repl` | Interactive REPL | ❌ NOT IMPLEMENTED |
| `x3-doc` | Documentation generator | ❌ NOT IMPLEMENTED |
| `x3-test` | Test harness | ❌ NOT IMPLEMENTED |

## Currently Implemented Language Features (Python MVP)

The Python MVP supports a **cross-chain intent DSL** with:

- `intent { ... }` blocks for declaring swap/bridge operations
- `from <chain> <token>` syntax for specifying source
- `to <chain> <token>` syntax for specifying destination
- `path { swap { ... } }, bridge { ... } }` for route planning
- `constraints { slippage, deadline, gas_limit }` for operational limits

## NOT Yet Implemented (documented but not built)

The following features from original design documents are **not yet implemented** in any track:

- ❌ Declarative Agent Definitions and autonomous agents
- ❌ Agent Swarm Scheduling
- ❌ First-class MEV primitives (`flashloan`, `route`, `bundle`, `sim` as native ops)
- ❌ Strong type system with algebraic data types, generics, traits
- ❌ Agent-to-agent message passing
- ❌ REAPER Compute Economy
- ❌ Full compilation pipeline (Source → Lexer → Parser → Lowering → Emitter)
- ❌ Native binary / WASM / Bytecode output from Rust compiler
- ❌ X3 Runtime Integration into chain runtime

## Compilation Pipeline (Current Production Surface)

```
Intent source  →  parser  →  AST  →  semantic verifier  →  IR  →  emitter  →  bytecode  →  verifier  →  dry-run VM
                    ✓          ✓            ✓ (8 rules)          ✓          ✓            ✓            ✓ (aligned, jumps)
```

The full pipeline is implemented end-to-end under `x3-lang/compiler` and `x3-lang/vm`.
See [Current Rust Production Surface](#current-rust-production-surface-as-of-2026-06-06) below
for details, verifier rules, CLI entry points, and test coverage.

## Recommendation

**Until the Rust compiler stack is complete** with actual end-to-end compilation → execution, treat the Python MVP as the authoritative user-facing intent DSL. Treat `x3-lang/compiler`, `x3-lang/vm`, and `x3-lang/spec` as the authority for Rust bytecode/compiler contracts under active development. Treat root `crates/x3-*` crates as runtime integration code, not as the canonical language implementation.

### What this means for contributors

- The **Python MVP files** in `x3-lang/` are the canonical production surface for today's intent DSL. Edit `cli.py`, `registry.py`, `typechecker.py`, `planner.py`, `simulator.py`, `runner.py` for that path.
- The **Rust language workspace** under `x3-lang/` is the canonical place for Rust parser/lowering/emitter/VM/opcode changes, but it is still incomplete.
- The **root Rust crates** (`crates/x3-*`) are runtime/integration crates. Do NOT describe them as the primary x3-lang compiler.
- CI builds only the Rust workspace (`cargo check`). The Python tree has its own test suite (`pytest`).
- Release reports reference `x3-lang/` as the active compiler. If the Rust port is ever revived, a sprint plan will explicitly call it out.

## Current Rust Production Surface (as of 2026-06-06)

The Rust workspace under `x3-lang/` is no longer a stub. It ships a
working end-to-end compiler/VM for the documented production intent
surface. The pipeline diagram above is now:

```
Intent source  →  parser  →  AST  →  semantic verifier  →  IR  →  emitter  →  bytecode  →  verifier  →  dry-run VM
                   ✓          ✓            ✓ (8 rules)         ✓          ✓            ✓            ✓ (aligned, jumps)
```

### Production safety rules enforced by the semantic verifier

The verifier in `x3-lang/compiler/src/semantic.rs` runs eight
production-safety rules on every program:

1. **Symbols** — every chain, asset, via, dex, from/to, receiver must
   be a non-empty portable identifier (alnum, `_`, `-`, max 64 chars).
2. **Route depth** — at most `DEFAULT_MAX_ATOMIC_OPS = 8` cross-VM
   ops per atomic block; no nested atomic blocks.
3. **Atomic balance** — every `AtomicBegin` is closed by an
   `AtomicEnd`; every cross-VM value move is inside an atomic block.
4. **Rollback** — every `AtomicBegin` with a cross-VM op has a
   matching rollback clause (or an explicit `OnTimeout` policy).
5. **Replay / expiry** — every external (cross-VM) call has a nonce
   in program metadata and a timeout policy.
6. **Bridge adapter allow-list** — only `x3`, `wormhole`, `layerzero`,
   `axelar`, `native`, `btc-relay` are accepted; anything else must
   be added explicitly.
7. **Adapter compatibility** — known chains only (`eth`, `ethereum`,
   `sol`, `solana`, `x3`, `btc`, `bitcoin`, `utxo`, `polygon`,
   `arbitrum`, `optimism`, `base`, `bsc`, `avalanche`).
8. **Asset moves** — Lock/Mint/Burn/Release/Swap/Bridge carry
   matching chain/asset values; numeric amounts are non-zero for
   moves, zero for release; a bridge with the same source and
   destination chain is rejected.

Diagnostics accumulate via `ErrorAccumulator` so a single `check`
call reports every problem rather than failing on the first one.

### BTC/UTXO adapter

The Bitcoin/UTXO bridge adapter in `x3-lang/vm/src/btc_adapter.rs`
is feature-gated behind `bitcoin-adapter`. The default build
**fails closed** on every cross-VM BTC call with
`X3_BTC_ADAPTER_DISABLED`. With the feature enabled, the production
path runs the (placeholder) header-chain verifier and rejects empty
proofs with `X3_BTC_PROOF_EMPTY`. A dry-run mode (used by tests)
records synthetic `dry-run-btc-bridge` receipts.

### CLI binary

`cargo run -p x3-tools --bin x3c -- <subcommand>` exposes:

- `parse` — emit JSON AST
- `check` — semantic verification with non-zero exit on errors
- `lower` — emit typed IR
- `build` — emit 4-byte-aligned bytecode
- `simulate` — dry-run VM simulation
- `run` — execute on dry-run VM
- `explain` — disassemble bytecode
- `test-fixture` — emit a reference example fixture

### Test surface

- 11 E2E fixture tests (`x3-lang/compiler/tests/test_e2e_fixtures.rs`)
  drive the full pipeline through transfer, atomic swap, EVM call,
  X3 internal call, BTC/UTXO route, plus negative cases.
- 24 parser coverage tests
  (`x3-lang/compiler/tests/test_parser_coverage.rs`) hit every
  dispatch arm of the inline tokenizer/parser.
- 15 pipeline tests
  (`x3-lang/compiler/tests/test_compiler_pipeline.rs`) cover
  compile-to-IR, bytecode generation, capability matrices,
  annotations, and atomic operation shape.
- 9 CLI integration tests
  (`x3-lang/crates/x3-tools/tests/cli.rs`) cover all subcommands and
  exit codes.
- VM verifier, bridge adapter, and BTC adapter unit tests in
  `x3-lang/vm/src/`.

Run the gate with:

```bash
cd x3-lang
cargo fmt --all -- --check
cargo clippy -p x3-lang-compiler -p x3-lang-vm -p x3-lang-ast \
            -p x3-lang-common -p x3-lang-lexer -p x3-tools \
            --all-targets --all-features -- -D warnings
cargo test -p x3-lang-compiler -p x3-lang-vm -p x3-lang-ast \
           -p x3-lang-common -p x3-lang-lexer -p x3-tools --all-features
```

The known limitations (what is *not* in production):

- The `x3-lexer` crate is a placeholder; the compiler uses an
  inline tokenizer in `compiler/src/parser.rs`. A future sprint can
  wire the lexer in.
- The dry-run VM is a simulator; production nodes must wire the
  real EVM, SVM, and BTC adapters via the `VM::with_bridge` API.
- Coverage on the entire workspace (including placeholder crates) is
  ~52%; coverage on the **production surface** (compiler/src/*,
  vm/src/{bridge,verifier,x3_lang_vm,btc_adapter}) is >80%.

## License

MIT OR Apache-2.0
