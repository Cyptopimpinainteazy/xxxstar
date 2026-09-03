# X3Lang Architecture Authority

Status: authoritative for X3Lang 1.0 core-hardening work
Date: 2026-09-02

## Decision

X3 currently contains three distinct language/tooling surfaces. They have different authority domains and must not be treated as interchangeable implementations.

1. **Python intent DSL (`x3-lang/*.py`)** — authoritative for the currently shipping user-facing intent DSL and its Python execution/planning surface.
2. **Rust language workspace (`x3-lang/Cargo.toml`)** — authoritative for Rust X3 source parsing, semantic verification, IR, bytecode, opcode contract, and X3 VM semantics. This is the production-authoritative path for X3Lang 1.0 compiler hardening.
3. **Root Rust crates (`crates/x3-*`)** — authoritative only for Substrate/runtime integration and compatibility surfaces. They are not the canonical X3Lang compiler or bytecode language definition.

The X3Lang 1.0 core-hardening work SHALL target the Rust language workspace under `x3-lang/` and SHALL NOT add new independent grammar, numeric-type, IR, opcode, bytecode, or VM semantics to the root compatibility stack.

## Why this path is authoritative

The repository already documents `x3-lang/compiler` as the production X3Lang Rust compiler and `x3-lang/vm` as its bytecode execution partner. The compiler emits the IR/bytecode consumed by the separate X3Lang VM workspace, and `x3-tools` depends on both compiler and VM. The root `crates/x3-compiler/Cargo.toml` itself states that the production X3Lang compiler is `x3-lang/compiler` and describes the root crate as a chain-integration pipeline.

The root Cargo workspace nevertheless includes a second parser/type-check/lowering/backend chain (`crates/x3-parser`, `crates/x3-typeck`, `crates/x3-hir`, `crates/x3-mir`, `crates/x3-backend`, `crates/x3-opt`, `crates/x3-verifier`, `crates/x3-compiler`). That stack remains necessary for runtime integration callers, but it is not allowed to become a second language specification.

## Responsibility map

| Responsibility | Authoritative path | Other implementation | Authority rule |
|---|---|---|---|
| User-facing intent DSL today | `x3-lang/*.py` | Rust parsers | Python owns current MVP syntax only; Rust 1.0 work must not silently redefine the shipping Python DSL. |
| Rust lexing/tokenization | `x3-lang/compiler/src/parser.rs` today, with `x3-lang/crates/x3-lexer` as the intended shared lexer | `crates/x3-lexer` | Rust language workspace owns X3Lang bytecode compiler token semantics. Inline tokenizer replacement is a later bounded migration. |
| Rust parsing | `x3-lang/compiler/src/parser.rs` | `crates/x3-parser` | `x3-lang/compiler` is canonical for X3Lang 1.0. Root parser is compatibility/integration code. |
| Rust AST | `x3-lang/crates/x3-ast` plus compiler-local structures where still present | `crates/x3-ast` | X3Lang workspace AST is canonical for compiler/bytecode semantics. |
| Semantic verification | `x3-lang/compiler/src/semantic.rs` | `crates/x3-semantics`, `crates/x3-typeck` | X3Lang semantic verifier is canonical for the bytecode compiler. Proven tests/policies from root crates may be migrated, not duplicated. |
| Numeric literal/type policy | X3Lang 1.0 conformance tests plus accepted numeric-coercion RFC | `crates/x3-typeck` current RFC/test path | Policy is shared at documentation level; executable authority moves to the canonical X3Lang compiler conformance suite during hardening. |
| Lowering and IR | `x3-lang/compiler/src/lowering.rs`, `x3-lang/compiler/src/ir.rs` | `crates/x3-hir`, `crates/x3-mir` | X3Lang workspace owns language IR/bytecode lowering. Root HIR/MIR remain integration representations only. |
| Register allocation | `x3-lang/compiler/src/regalloc.rs` | root compiler/backend path | X3Lang compiler owns bytecode register allocation. |
| Bytecode emission | `x3-lang/compiler/src/emitter.rs` and shared opcode spec | `crates/x3-backend` | X3Lang compiler owns X3 bytecode encoding. |
| Bytecode verification | `x3-lang/vm` verifier | `crates/x3-verifier` | X3Lang VM verifier is authoritative for X3Lang bytecode safety. Root verifier may verify runtime integration artifacts but may not redefine bytecode validity. |
| VM execution | `x3-lang/vm` | `crates/x3-vm` | `x3-lang/vm` owns X3Lang bytecode execution semantics; root VM owns runtime integration surfaces only. |
| Substrate/runtime compiler bridge | `crates/x3-integration`, `crates/x3-compiler` | direct `x3-lang` use | Root integration layer remains authoritative for runtime wiring until an explicit cross-workspace bridge is implemented. |

## Caller inventory

### X3Lang Rust workspace callers

| Caller | Dependency/use | Classification |
|---|---|---|
| `x3-lang/vm/Cargo.toml` | depends on `x3-lang-compiler` | production language/VM path |
| `x3-lang/crates/x3-tools/Cargo.toml` | depends on `x3-lang-compiler` and `x3-lang-vm` | production developer/compiler tooling path |
| `x3-lang/compiler/tests/*` | compiles source and exercises IR/bytecode/VM | canonical compiler conformance/E2E tests |
| `docs/x3-lang/README.md` | declares `x3-lang/compiler`, `x3-lang/vm`, and `x3-lang/spec` authoritative for Rust compiler/bytecode contracts | repository source-of-truth documentation |

### Root compatibility/integration callers

| Caller | Dependency/use | Classification |
|---|---|---|
| `crates/x3-compiler` | depends on root lexer/parser/AST/HIR/MIR/typeck/backend/opt/verifier | runtime/integration compatibility compiler |
| `crates/x3-integration` | optional/root compiler bridge and VM integration | Substrate/runtime integration boundary |
| root workspace members using `crates/x3-parser`, `x3-typeck`, `x3-backend`, or `x3-verifier` | integration dependencies | not canonical X3Lang language semantics |
| `crates/x3-crosschain-intent` | carries integration-side intent types/adapters and avoids direct cross-workspace compiler dependency | compatibility boundary |

## Migration and deprecation matrix

| Component | Authoritative path | Non-authoritative path | Action | Removal prerequisite |
|---|---|---|---|---|
| Rust parser | `x3-lang/compiler/src/parser.rs` | `crates/x3-parser` | **migrate-tests** | Root parser callers either migrate to explicit integration grammar APIs or an adapter consumes canonical X3Lang parser output. |
| Rust AST | `x3-lang/crates/x3-ast` | `crates/x3-ast` | **adapt** | Stable conversion layer exists for every runtime integration type still needed by root crates. |
| Semantic verifier | `x3-lang/compiler/src/semantic.rs` | `crates/x3-semantics` | **migrate-tests** | Safety rules needed by X3Lang have canonical conformance coverage. |
| Numeric/type checking | canonical X3Lang semantic/type layer | `crates/x3-typeck` | **migrate-tests** | Numeric RFC behavior and any reusable type rules are covered by canonical X3Lang tests. |
| HIR/MIR integration forms | X3Lang IR for language compilation | `crates/x3-hir`, `crates/x3-mir` | **keep** | These are integration representations; removal is not required for X3Lang 1.0 if they do not define language semantics. |
| Root `x3-compiler` | `x3-lang/compiler` for language compilation | `crates/x3-compiler` | **adapt** | Runtime bridge can call/consume the canonical compiler across the workspace boundary without cyclic or incompatible dependencies. |
| Root backend/verifier | `x3-lang/compiler` + `x3-lang/vm` | `crates/x3-backend`, `crates/x3-verifier` | **keep** | Keep only for runtime integration artifacts; any X3 bytecode-specific rules must be delegated to canonical definitions. |
| Python MVP parser/typechecker | Python MVP | Rust X3Lang compiler | **keep** | A separately approved user-facing DSL migration/compatibility plan exists. |

## Rules for new work

1. New X3 bytecode opcodes, encodings, verifier rules, and gas rules are defined only in the `x3-lang` canonical spec/workspace.
2. New X3Lang grammar or semantic rules are implemented only in the canonical `x3-lang` compiler path and its conformance suite.
3. Root compatibility crates may adapt or translate canonical language artifacts, but may not independently reinterpret their meaning.
4. Tests from root parser/type-checker crates that describe intended language behavior should be migrated into canonical X3Lang conformance fixtures before equivalent root behavior is deprecated.
5. Cross-workspace dependency constraints are integration problems, not justification for a second language specification. Use explicit serialized/typed boundaries where a direct Cargo dependency is impossible.
6. No root compatibility crate is removed during Core Hardening solely to simplify the tree. Removal requires caller migration evidence and its own reviewable change.

## Numeric-coercion policy reconciliation

The existing RFC scopes its current implementation discussion to root `crates/x3-parser` and `crates/x3-typeck`. For X3Lang 1.0, the semantic policy is adopted at the language level and must be proven in the canonical `x3-lang` conformance suite before the RFC is promoted from draft:

- bare positive integer literals default to `u64`;
- negative values retain unary-negation semantics;
- implicit signed/unsigned coercion is rejected;
- direct function-argument incompatibility uses the dedicated argument-type-mismatch diagnostic.

Root type-checker tests are evidence to migrate, not the final executable authority for X3Lang 1.0.

## Boundary with the Python MVP

The Python MVP remains the shipping user-facing intent DSL while the Rust compiler is hardened. X3Lang 1.0 Core Hardening does not authorize breaking Python syntax, removing Python tooling, or declaring the Rust compiler a drop-in replacement for the MVP. A future compatibility/migration plan must explicitly define source compatibility and cutover criteria.

## Verification checklist

Before implementing semantic changes, contributors must be able to answer all of the following with one path each:

- Canonical Rust parser: `x3-lang/compiler/src/parser.rs`
- Canonical Rust semantic verifier: `x3-lang/compiler/src/semantic.rs`
- Canonical Rust IR/lowering: `x3-lang/compiler/src/ir.rs` and `lowering.rs`
- Canonical bytecode emitter: `x3-lang/compiler/src/emitter.rs`
- Canonical bytecode verifier/executor: `x3-lang/vm`
- Canonical runtime integration layer: root `crates/x3-integration` / `crates/x3-compiler`
- Canonical shipping MVP DSL: `x3-lang/*.py`

If a proposed change cannot identify which one of these authority domains it belongs to, it must not introduce new language semantics until that ambiguity is resolved.