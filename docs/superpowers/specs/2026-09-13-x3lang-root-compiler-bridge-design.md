# X3Lang Root Compiler Bridge Design

Date: 2026-09-13
Status: Proposed
Target repository: `Cyptopimpinainteazy/xxxstar`
Target branch: `design/x3lang-root-compiler-bridge`

## 1. Goal

Make the root `crates/x3-integration` layer able to compile canonical X3Lang source through the authoritative Rust language implementation under `x3-lang/`, without duplicating grammar, IR, bytecode, verifier, or VM semantics and without pulling std-only compiler dependencies into Substrate runtime/WASM builds.

This bridge is the prerequisite for Jack The Ripper to consume X3Lang through an official X3 integration boundary rather than depending directly on compiler internals.

## 2. Existing state

The repository has three distinct language/tooling surfaces:

1. `x3-lang/*.py`: current Python MVP intent surface.
2. `x3-lang/compiler`, `x3-lang/vm`, `x3-lang/crates/*`: canonical Rust X3Lang compiler, bytecode, verifier, and VM semantics.
3. Root `crates/x3-*`: runtime/Substrate integration and compatibility crates.

`crates/x3-integration/src/compiler_bridge.rs` currently fails closed with `X3IntegrationError::CompilationFailed`. This behavior is intentional and prevents fake or empty bytecode from being mistaken for a successful compile.

The canonical Rust compiler already exposes `x3_lang_compiler::compile_source`, trading analysis/lowering, bytecode generation, TradingVm execution, receipt construction, and conformance/E2E tests. The bridge must delegate to that canonical implementation rather than reimplementing compiler behavior.

## 3. Architectural decision

The root runtime crate must not directly depend on the canonical compiler in `no_std` or WASM configurations.

The integration boundary will be split into two layers:

### 3.1 Runtime-safe core

`crates/x3-integration` remains buildable without the canonical compiler. Its core types, execution integration, and runtime-facing APIs continue to support `no_std` where currently required.

The compiler bridge API remains present, but canonical compilation is enabled only when an explicit std-only feature is selected.

### 3.2 Host-side canonical compiler adapter

A std-only adapter will call the canonical `x3-lang-compiler` APIs and return a stable serialized artifact to the root integration layer.

The adapter owns no language semantics. It may:

- accept UTF-8 `.x3` source;
- select compilation mode;
- invoke canonical parse/check/lower/compile APIs;
- return canonical bytecode and metadata;
- convert canonical diagnostics into stable integration errors;
- expose compiler/interface version metadata.

It must not:

- define a second grammar;
- reinterpret canonical IR;
- emit alternate bytecode;
- weaken canonical verifier errors;
- synthesize successful artifacts when compilation fails.

## 4. Dependency strategy

A naive direct member relationship between the root Cargo workspace and `x3-lang/` will not be used. `x3-lang` remains its own canonical workspace.

The bridge will use an explicit host-only dependency boundary. The implementation plan should choose the least invasive Cargo mechanism that satisfies all gates below. Acceptable mechanisms are:

1. a path dependency on `x3-lang/compiler` compiled only behind a std-only integration feature, if Cargo accepts the nested workspace package cleanly;
2. extraction of a small canonical compiler API package into `x3-lang/crates/` that can be consumed as a normal path dependency while remaining owned by the canonical workspace;
3. a process boundary using the canonical `x3c` tool only if direct library consumption proves structurally impossible.

Preference order is 1, then 2, then 3. The design forbids copying compiler sources into root `crates/`.

## 5. Feature model

`crates/x3-integration` will distinguish runtime integration from host compilation.

Proposed feature semantics:

- `default = ["std"]` remains unchanged unless tests prove a required adjustment.
- `std` enables existing host-side root integrations.
- `compile` becomes the public capability flag for source compilation.
- a private/internal dependency feature may be introduced if needed to bind `compile` to the canonical compiler.
- `compile` must require `std`; a `no_std + compile` configuration must fail at compile time with a clear feature error rather than silently falling back to root compatibility compilation.

The old root `x3-compiler` dependency must not be used as the implementation of canonical X3Lang compilation merely to make the feature green.

## 6. Public API

The current compatibility function may remain:

```rust
pub fn compile_source(source: &str) -> X3Result<Vec<u8>>
```

but the preferred new API is a typed compile request/result pair so Jack and other callers do not have to infer compiler mode or artifact identity from raw bytes.

Proposed shape:

```rust
pub enum X3CompilationMode {
    Dev,
    Production,
    Mainnet,
}

pub struct X3CompileRequest<'a> {
    pub source: &'a str,
    pub mode: X3CompilationMode,
}

pub struct X3CompileArtifact {
    pub bytecode: Vec<u8>,
    pub compiler_version: String,
    pub bytecode_version: u32,
    pub mode: X3CompilationMode,
    pub source_hash: [u8; 32],
}

pub fn compile(request: X3CompileRequest<'_>) -> X3Result<X3CompileArtifact>;
```

`compile_source` becomes a small backwards-compatible wrapper around `compile` in development mode only if preserving its current semantics is safe. Otherwise it should require an explicit mode and callers should be migrated in the same PR.

No API may return success with empty bytecode.

## 7. Diagnostic and error contract

Canonical compiler failures must be converted into stable integration errors without discarding useful context.

The root integration layer should distinguish at minimum:

- compiler unavailable because feature support is not built;
- source parse failure;
- semantic/type/trading verification failure;
- mainnet capability rejection;
- bytecode emission failure;
- internal compiler failure.

If changing the existing `X3IntegrationError` enum would create excessive compatibility churn, the first implementation may retain `CompilationFailed(String)` while adding machine-readable category information to the typed artifact/diagnostic API. The implementation plan must prefer typed errors if caller impact is small.

## 8. Security and correctness invariants

The bridge must preserve the following invariants:

1. Canonical authority: all accepted source semantics come from `x3-lang/compiler`.
2. Fail closed: unsupported or unavailable compilation returns an error.
3. No fake bytecode: no empty/synthetic bytecode on success paths.
4. Mode preservation: Dev, Production, and Mainnet cannot be silently collapsed into one mode.
5. Mainnet strictness: canonical mainnet checks, including capability/private-submission requirements, must propagate through the bridge.
6. Version identity: artifacts identify compiler and bytecode versions.
7. Source identity: artifacts carry a deterministic source hash.
8. Runtime isolation: Substrate runtime/WASM builds do not pull canonical compiler networking, tooling, or other std-only dependencies.
9. Determinism: identical source, mode, and compiler version produce identical bytecode and source hash.
10. No semantic translation: the root bridge never rewrites or optimizes canonical language semantics independently.

## 9. Jack The Ripper compatibility target

This work does not add Jack-specific strategy semantics to X3Lang. It only establishes the stable bridge Jack will later consume.

The bridge is considered Jack-ready when a host-side caller can:

1. submit a Trading Core `.x3` strategy;
2. select Production or Mainnet compilation mode;
3. receive canonical bytecode plus version/source identity;
4. receive canonical rejection diagnostics for unsafe or unsupported strategies;
5. reproduce the same artifact deterministically;
6. hand the compiled artifact to a later Jack adapter without importing internal compiler modules.

The follow-on Jack integration will map canonical trading operations/capabilities into Jack's existing flash-liquidity, DEX, simulation, risk, and submission layers. That mapping is explicitly out of scope for this bridge PR.

## 10. Testing gates

The implementation is not complete until all applicable gates pass.

### 10.1 Bridge unit tests

- valid canonical `.x3` source compiles to non-empty aligned bytecode;
- invalid source returns a typed failure;
- semantic failure returns failure and no artifact;
- Dev/Production/Mainnet mode reaches the canonical compiler unchanged;
- mainnet-only capability rejection is preserved;
- repeated compilation is byte-for-byte deterministic;
- source hash is stable;
- returned version metadata is populated.

### 10.2 Feature/build matrix

- `cargo check -p x3-x3-integration --no-default-features` remains green;
- standard root integration build remains green;
- std + compile feature builds and exercises the canonical compiler;
- `no_std + compile` is explicitly rejected or structurally impossible with a clear error;
- root runtime WASM build does not gain canonical compiler dependencies.

### 10.3 Canonical X3Lang regression gates

From `x3-lang/`:

```bash
cargo fmt --all -- --check
cargo clippy -p x3-lang-compiler -p x3-lang-vm -p x3-lang-ast -p x3-lang-common -p x3-lang-lexer -p x3-tools --all-targets --all-features -- -D warnings
cargo test -p x3-lang-compiler -p x3-lang-vm -p x3-lang-ast -p x3-lang-common -p x3-lang-lexer -p x3-tools --all-features
```

Existing Trading Core E2E, TradingVm rollback, receipt, property, and mainnet capability tests must stay green.

### 10.4 End-to-end bridge proof

Add a test that starts from an actual `.x3` fixture and proves:

```text
source
  -> root x3-integration bridge
  -> canonical x3-lang compiler
  -> canonical bytecode
  -> canonical verifier/VM test consumer
```

The test must use the exact bytes returned by the root bridge, not recompile the source separately inside the assertion path.

## 11. CI evidence

The PR should publish or clearly identify exact-head evidence for:

- bridge feature matrix;
- canonical X3Lang fmt/clippy/test gates;
- root workspace affected-package tests;
- runtime WASM boundary check;
- E2E bridge proof;
- no fake/stub compiler path scan in the files changed by the PR.

No gate may be weakened to make the bridge pass.

## 12. Migration strategy

1. Add the canonical host compiler dependency boundary behind the compile feature.
2. Replace the fail-closed bridge body with real canonical delegation.
3. Preserve a typed unavailable error when compilation support is not built.
4. Add typed artifact metadata and mode propagation.
5. Replace the existing test that expects unconditional failure with success/failure matrix tests.
6. Add E2E canonical bytecode proof.
7. Run the root and canonical workspace gates.
8. Only after the bridge is proven should Jack consume it.

## 13. Non-goals

This design does not:

- merge the root and `x3-lang` Cargo workspaces;
- replace the Python MVP;
- remove root compatibility compiler crates;
- add new X3Lang syntax;
- add Jack-specific DEX or flash-loan semantics;
- wire live wallets, RPCs, signers, or private builders;
- change canonical VM/opcode semantics;
- make runtime/WASM code compile source on-chain.

## 14. Acceptance criteria

The bridge is complete when all of the following are true:

- `x3-integration` delegates source compilation to the canonical `x3-lang` compiler in host/std builds;
- unsupported build configurations fail closed;
- runtime/WASM isolation is preserved;
- compiler mode and canonical diagnostics are preserved;
- artifacts include deterministic source/version identity;
- no fake or empty success artifact is possible;
- canonical X3Lang gates remain green;
- root affected-package and WASM gates remain green;
- an E2E test proves root bridge -> canonical bytecode -> canonical verifier/VM consumption;
- the interface is stable enough for Jack to consume without importing canonical compiler internals.
