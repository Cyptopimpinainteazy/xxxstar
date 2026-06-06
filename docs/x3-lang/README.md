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

## Compilation Pipeline (Planned Architecture)

```
Source (.x3)
    │
    ▼
┌─────────┐
│  Lexer  │ ─── Tokenization — PARTIAL in Rust, working in Python
└────┬────┘
     │
     ▼
┌─────────┐
│ Parser  │ ─── AST Construction — PARTIAL in Rust, working in Python
└────┬────┘
     │
     ▼
┌─────────┐
│ Lowering│ ─── IR & Optimization — NOT IMPLEMENTED
└────┬────┘
     │
     ▼
┌─────────┐
│ Emitter │ ─── Bytecode Generation — STUB only
└────┬────┘
     │
     ▼
Native Binary / WASM / Bytecode — NOT IMPLEMENTED
```

## Recommendation

**Until the Rust compiler stack is complete** with actual end-to-end compilation → execution, treat the Python MVP as the authoritative user-facing intent DSL. Treat `x3-lang/compiler`, `x3-lang/vm`, and `x3-lang/spec` as the authority for Rust bytecode/compiler contracts under active development. Treat root `crates/x3-*` crates as runtime integration code, not as the canonical language implementation.

### What this means for contributors

- The **Python MVP files** in `x3-lang/` are the canonical production surface for today's intent DSL. Edit `cli.py`, `registry.py`, `typechecker.py`, `planner.py`, `simulator.py`, `runner.py` for that path.
- The **Rust language workspace** under `x3-lang/` is the canonical place for Rust parser/lowering/emitter/VM/opcode changes, but it is still incomplete.
- The **root Rust crates** (`crates/x3-*`) are runtime/integration crates. Do NOT describe them as the primary x3-lang compiler.
- CI builds only the Rust workspace (`cargo check`). The Python tree has its own test suite (`pytest`).
- Release reports reference `x3-lang/` as the active compiler. If the Rust port is ever revived, a sprint plan will explicitly call it out.

## License

MIT OR Apache-2.0
