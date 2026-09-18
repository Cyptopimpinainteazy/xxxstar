# X3Lang Root Compiler Bridge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the fail-closed placeholder in `crates/x3-integration` with a real std-only bridge to canonical `x3-lang/compiler`, preserving runtime/WASM isolation and producing deterministic typed compilation artifacts suitable for Jack The Ripper.

**Architecture:** Keep `x3-lang/` as the canonical language workspace and keep root `crates/x3-integration` as the official host/runtime integration boundary. `compile` becomes a std-only feature that optionally depends on `x3-lang-compiler`; the bridge runs canonical mode-aware checking and compilation, returns bytecode plus identity metadata, and never falls back to the root compatibility compiler.

**Tech Stack:** Rust 2021, Cargo workspaces, Substrate `sp-core`, SCALE codec/type-info, canonical `x3-lang-compiler`, canonical `x3-lang-vm` for E2E verification.

**Spec:** `docs/superpowers/specs/2026-09-13-x3lang-root-compiler-bridge-design.md`

## Global Constraints

- Canonical X3Lang semantics remain owned by `x3-lang/compiler` and `x3-lang/vm`.
- Do not merge the root and `x3-lang` Cargo workspaces.
- Do not copy compiler source into root `crates/`.
- `compile` must require `std`; runtime/WASM builds must not pull canonical compiler dependencies.
- Never return empty or synthetic bytecode as success.
- Preserve Dev, Production, and Mainnet modes exactly.
- Preserve canonical rejection behavior, including mainnet/private-submission checks.
- Artifacts must include compiler version, bytecode version, compilation mode, and deterministic source hash.
- No gate may be weakened to make the bridge pass.

---

## File Structure

- Modify `x3-lang/compiler/src/lib.rs` — export canonical compiler/bytecode version constants.
- Modify `crates/x3-integration/Cargo.toml` — add optional canonical compiler dependency, feature wiring, and canonical VM dev dependency.
- Modify `crates/x3-integration/src/lib.rs` — enforce the `std + compile` feature contract and export bridge types.
- Modify `crates/x3-integration/src/error.rs` — add stable compiler-unavailable/rejected error categories.
- Replace `crates/x3-integration/src/compiler_bridge.rs` — implement typed request/artifact API and canonical delegation.
- Replace `crates/x3-integration/tests/compiler_bridge.rs` — TDD matrix for success, failure, modes, determinism, metadata, and fail-closed behavior.
- Create `crates/x3-integration/tests/compiler_bridge_e2e.rs` — prove bytes returned by the root bridge are accepted by canonical VM verifier.
- Modify the existing CI workflow that gates `x3-integration`/X3Lang — add exact bridge feature-matrix and canonical workspace commands without weakening existing checks.

---

### Task 1: Export Canonical Compiler Identity

**Files:**
- Modify: `x3-lang/compiler/src/lib.rs`
- Test: `x3-lang/compiler/tests/test_compiler_pipeline.rs`

**Interfaces:**
- Produces: `pub const COMPILER_VERSION: &str`
- Produces: `pub const BYTECODE_VERSION: u8`

- [ ] **Step 1: Write the failing compiler identity test**

Add to `x3-lang/compiler/tests/test_compiler_pipeline.rs`:

```rust
#[test]
fn compiler_exports_stable_identity() {
    assert!(!x3_lang_compiler::COMPILER_VERSION.is_empty());
    assert_eq!(x3_lang_compiler::BYTECODE_VERSION, 0x01);
}
```

- [ ] **Step 2: Run the focused test and verify it fails**

Run:

```bash
cd x3-lang
cargo test -p x3-lang-compiler compiler_exports_stable_identity -- --exact
```

Expected: compile failure because `COMPILER_VERSION` and `BYTECODE_VERSION` are not exported.

- [ ] **Step 3: Add the canonical constants**

Near the public re-exports in `x3-lang/compiler/src/lib.rs` add:

```rust
pub const COMPILER_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const BYTECODE_VERSION: u8 = 0x01;
```

Replace any local literal used by the top-level bytecode sanity check with `BYTECODE_VERSION` where doing so is mechanically safe; do not change the wire format.

- [ ] **Step 4: Run focused and package tests**

```bash
cd x3-lang
cargo test -p x3-lang-compiler compiler_exports_stable_identity -- --exact
cargo test -p x3-lang-compiler
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add x3-lang/compiler/src/lib.rs x3-lang/compiler/tests/test_compiler_pipeline.rs
git commit -m "feat(x3lang): export compiler identity"
```

---

### Task 2: Wire the Canonical Compiler as a Host-Only Optional Dependency

**Files:**
- Modify: `crates/x3-integration/Cargo.toml`
- Modify: `crates/x3-integration/src/lib.rs`

**Interfaces:**
- Produces Cargo feature: `compile = ["std", "dep:x3-lang-compiler"]`
- Produces compile-time rejection for `compile` without `std`.

- [ ] **Step 1: Make the feature contract explicit in code**

Add near the top of `crates/x3-integration/src/lib.rs`:

```rust
#[cfg(all(feature = "compile", not(feature = "std")))]
compile_error!("x3-x3-integration feature `compile` requires feature `std`");
```

- [ ] **Step 2: Add the optional canonical compiler dependency**

In `crates/x3-integration/Cargo.toml` add:

```toml
x3-lang-compiler = { path = "../../x3-lang/compiler", optional = true }
```

Change:

```toml
compile = []
```

to:

```toml
compile = ["std", "dep:x3-lang-compiler"]
```

Add the canonical VM only as a dev dependency for E2E proof:

```toml
x3-lang-vm = { path = "../../x3-lang/vm" }
```

- [ ] **Step 3: Prove runtime isolation still builds**

Run:

```bash
cargo check -p x3-x3-integration --no-default-features
```

Expected: PASS and no canonical compiler build in the dependency output.

- [ ] **Step 4: Prove host compilation feature resolves**

Run:

```bash
cargo check -p x3-x3-integration --features compile
```

Expected: PASS dependency resolution through `../../x3-lang/compiler`. If Cargo rejects the nested-workspace path package, stop this task and implement the spec's second dependency mechanism: extract a small canonical API crate under `x3-lang/crates/` and depend on that. Do not fall back to root `x3-compiler`.

- [ ] **Step 5: Commit**

```bash
git add crates/x3-integration/Cargo.toml crates/x3-integration/src/lib.rs Cargo.lock
git commit -m "build(x3-integration): add canonical compiler feature"
```

---

### Task 3: Add Stable Typed Compilation Errors and Artifact Types

**Files:**
- Modify: `crates/x3-integration/src/error.rs`
- Replace: `crates/x3-integration/src/compiler_bridge.rs`
- Test: `crates/x3-integration/tests/compiler_bridge.rs`

**Interfaces:**
- Produces: `X3CompilationMode::{Dev, Production, Mainnet}`
- Produces: `X3CompileRequest<'a> { source: &'a str, mode: X3CompilationMode }`
- Produces: `X3CompileArtifact { bytecode, compiler_version, bytecode_version, mode, source_hash }`
- Produces: `X3CompilationErrorKind::{Parse, Semantic, MainnetCapability, Emit, Internal}`
- Produces: `compile(request) -> X3Result<X3CompileArtifact>`
- Preserves: `compile_source(source) -> X3Result<Vec<u8>>` as a Dev wrapper.

- [ ] **Step 1: Replace the old unconditional-failure tests with type-level expectations**

In `crates/x3-integration/tests/compiler_bridge.rs`, import:

```rust
use x3_x3_integration::compiler_bridge::{
    compile, compile_source, X3CompilationMode, X3CompileRequest,
};
```

Add an initial failing test:

```rust
#[test]
fn compile_artifact_exposes_mode_and_identity() {
    let source = include_str!("../../../x3-lang/examples/trading_core_v1.x3");
    let artifact = compile(X3CompileRequest {
        source,
        mode: X3CompilationMode::Dev,
    })
    .expect("canonical source must compile");

    assert!(!artifact.bytecode.is_empty());
    assert_eq!(artifact.bytecode_version, 0x01);
    assert_eq!(artifact.mode, X3CompilationMode::Dev);
    assert!(!artifact.compiler_version.is_empty());
    assert_ne!(artifact.source_hash, [0u8; 32]);
}
```

- [ ] **Step 2: Run it and verify failure**

```bash
cargo test -p x3-x3-integration --features compile compile_artifact_exposes_mode_and_identity -- --exact
```

Expected: compile failure because typed bridge API does not exist.

- [ ] **Step 3: Add stable error categories**

In `crates/x3-integration/src/error.rs` add a SCALE-safe enum:

```rust
#[derive(Clone, Copy, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq)]
pub enum X3CompilationErrorKind {
    Parse,
    Semantic,
    MainnetCapability,
    Emit,
    Internal,
}
```

Extend `X3IntegrationError` with:

```rust
CompilerUnavailable,
CompilationRejected {
    kind: X3CompilationErrorKind,
    message: String,
},
```

Update `Display` and `DispatchError` mapping so both variants have stable human-readable output and `"X3: Compiler unavailable"` / `"X3: Compilation rejected"` dispatch strings.

Keep `CompilationFailed(String)` temporarily for source compatibility, but the new bridge must not use it for canonical rejections.

- [ ] **Step 4: Define bridge request/artifact types**

In `crates/x3-integration/src/compiler_bridge.rs` define:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X3CompilationMode {
    Dev,
    Production,
    Mainnet,
}

pub struct X3CompileRequest<'a> {
    pub source: &'a str,
    pub mode: X3CompilationMode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct X3CompileArtifact {
    pub bytecode: Vec<u8>,
    pub compiler_version: String,
    pub bytecode_version: u8,
    pub mode: X3CompilationMode,
    pub source_hash: [u8; 32],
}
```

- [ ] **Step 5: Re-export compiler error kind**

Update `crates/x3-integration/src/lib.rs`:

```rust
pub use error::{X3CompilationErrorKind, X3IntegrationError, X3Result};
```

- [ ] **Step 6: Run the focused test to confirm it now reaches missing implementation**

```bash
cargo test -p x3-x3-integration --features compile compile_artifact_exposes_mode_and_identity -- --exact
```

Expected: failure from unimplemented/old bridge body, not missing types.

- [ ] **Step 7: Commit**

```bash
git add crates/x3-integration/src/error.rs crates/x3-integration/src/lib.rs crates/x3-integration/src/compiler_bridge.rs crates/x3-integration/tests/compiler_bridge.rs
git commit -m "feat(x3-integration): define typed compiler bridge contract"
```

---

### Task 4: Delegate to Canonical Mode-Aware Checking and Compilation

**Files:**
- Replace: `crates/x3-integration/src/compiler_bridge.rs`
- Test: `crates/x3-integration/tests/compiler_bridge.rs`

**Interfaces:**
- Consumes: `x3_lang_compiler::{check_source_with_mode, compile_with_mode, CompilationMode, COMPILER_VERSION, BYTECODE_VERSION}`
- Produces: real `compile()` and compatibility `compile_source()`.

- [ ] **Step 1: Add tests for determinism, invalid source, and wrapper behavior**

Add:

```rust
#[test]
fn repeated_compilation_is_deterministic() {
    let source = include_str!("../../../x3-lang/examples/trading_core_v1.x3");
    let request = || X3CompileRequest { source, mode: X3CompilationMode::Dev };
    let a = compile(request()).unwrap();
    let b = compile(request()).unwrap();
    assert_eq!(a.bytecode, b.bytecode);
    assert_eq!(a.source_hash, b.source_hash);
}

#[test]
fn invalid_source_never_returns_artifact() {
    let err = compile(X3CompileRequest {
        source: "this is not x3 source",
        mode: X3CompilationMode::Dev,
    })
    .expect_err("invalid source must fail closed");
    assert!(matches!(
        err,
        X3IntegrationError::CompilationRejected { .. }
    ));
}

#[test]
fn compile_source_is_dev_mode_compatibility_wrapper() {
    let source = include_str!("../../../x3-lang/examples/trading_core_v1.x3");
    let bytes = compile_source(source).unwrap();
    let artifact = compile(X3CompileRequest { source, mode: X3CompilationMode::Dev }).unwrap();
    assert_eq!(bytes, artifact.bytecode);
}
```

- [ ] **Step 2: Implement mode mapping**

Add a private mapper:

```rust
fn canonical_mode(mode: X3CompilationMode) -> x3_lang_compiler::CompilationMode {
    match mode {
        X3CompilationMode::Dev => x3_lang_compiler::CompilationMode::Dev,
        X3CompilationMode::Production => x3_lang_compiler::CompilationMode::Production,
        X3CompilationMode::Mainnet => x3_lang_compiler::CompilationMode::Mainnet,
    }
}
```

- [ ] **Step 3: Implement canonical checking before emission**

The `compile()` flow must be:

```rust
let mode = canonical_mode(request.mode);
let (_, _, errors) = x3_lang_compiler::check_source_with_mode(request.source, mode)
    .map_err(map_compiler_error)?;
if !errors.is_empty() {
    let message = errors.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n");
    let kind = if request.mode == X3CompilationMode::Mainnet
        && message.contains("private-submission")
    {
        X3CompilationErrorKind::MainnetCapability
    } else {
        X3CompilationErrorKind::Semantic
    };
    return Err(X3IntegrationError::CompilationRejected { kind, message });
}
```

Then compile with the same `mode`:

```rust
let bytecode = x3_lang_compiler::compile_with_mode(request.source, mode)
    .map_err(map_compiler_error)?;
if bytecode.is_empty() {
    return Err(X3IntegrationError::CompilationRejected {
        kind: X3CompilationErrorKind::Emit,
        message: "canonical compiler returned empty bytecode".into(),
    });
}
```

- [ ] **Step 4: Construct deterministic artifact identity**

Use the existing Substrate hashing dependency:

```rust
let source_hash = sp_core::hashing::blake2_256(request.source.as_bytes());
```

Return:

```rust
Ok(X3CompileArtifact {
    bytecode,
    compiler_version: x3_lang_compiler::COMPILER_VERSION.to_string(),
    bytecode_version: x3_lang_compiler::BYTECODE_VERSION,
    mode: request.mode,
    source_hash,
})
```

Implement:

```rust
pub fn compile_source(source: &str) -> X3Result<Vec<u8>> {
    compile(X3CompileRequest { source, mode: X3CompilationMode::Dev })
        .map(|artifact| artifact.bytecode)
}
```

- [ ] **Step 5: Map canonical errors without inventing semantics**

Implement `map_compiler_error` by formatting the canonical `X3Error` once and categorizing parse/codegen/semantic variants according to the actual enum variants exposed by `x3-lang/crates/x3-common`. If a variant cannot be classified without guessing, map it to `Internal`; do not rewrite the canonical message.

- [ ] **Step 6: Run bridge tests**

```bash
cargo test -p x3-x3-integration --features compile --test compiler_bridge
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/x3-integration/src/compiler_bridge.rs crates/x3-integration/tests/compiler_bridge.rs
git commit -m "feat(x3-integration): delegate compilation to canonical x3lang"
```

---

### Task 5: Prove Mode Preservation and Mainnet Rejection

**Files:**
- Modify: `crates/x3-integration/tests/compiler_bridge.rs`

**Interfaces:**
- Consumes: `compile(X3CompileRequest)`.
- Proves: mode is not collapsed and canonical mainnet restrictions propagate.

- [ ] **Step 1: Add Production-mode identity test**

```rust
#[test]
fn production_mode_is_preserved() {
    let source = include_str!("../../../x3-lang/examples/trading_core_v1.x3");
    let result = compile(X3CompileRequest {
        source,
        mode: X3CompilationMode::Production,
    });
    if let Ok(artifact) = result {
        assert_eq!(artifact.mode, X3CompilationMode::Production);
    }
}
```

- [ ] **Step 2: Add Mainnet capability rejection test using the same canonical trading fixture used by the compiler E2E suite**

```rust
#[test]
fn mainnet_rejection_preserves_private_submission_requirement() {
    let source = include_str!("../../../x3-lang/examples/trading_core_v1.x3");
    let err = compile(X3CompileRequest {
        source,
        mode: X3CompilationMode::Mainnet,
    })
    .expect_err("fixture lacks required mainnet private-submission capability");

    match err {
        X3IntegrationError::CompilationRejected { kind, message } => {
            assert_eq!(kind, X3CompilationErrorKind::MainnetCapability);
            assert!(message.contains("private-submission"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
```

- [ ] **Step 3: Run the mode tests**

```bash
cargo test -p x3-x3-integration --features compile --test compiler_bridge
```

Expected: PASS with the Mainnet test rejecting through canonical diagnostics.

- [ ] **Step 4: Commit**

```bash
git add crates/x3-integration/tests/compiler_bridge.rs
git commit -m "test(x3-integration): prove compiler mode preservation"
```

---

### Task 6: Add Root Bridge -> Canonical VM E2E Proof

**Files:**
- Create: `crates/x3-integration/tests/compiler_bridge_e2e.rs`
- Modify: `crates/x3-integration/Cargo.toml` only if dev dependency was not already added in Task 2.

**Interfaces:**
- Consumes: exact `artifact.bytecode` returned by root bridge.
- Produces: proof that canonical VM verifier accepts those exact bytes.

- [ ] **Step 1: Write the E2E test**

```rust
#![cfg(all(feature = "std", feature = "compile"))]

use x3_x3_integration::compiler_bridge::{compile, X3CompilationMode, X3CompileRequest};
use x3_lang_vm::x3_lang_vm::InstructionStream;

#[test]
fn root_bridge_bytes_are_accepted_by_canonical_vm_verifier() {
    let source = include_str!("../../../x3-lang/examples/trading_core_v1.x3");
    let artifact = compile(X3CompileRequest {
        source,
        mode: X3CompilationMode::Dev,
    })
    .expect("bridge compilation must succeed");

    let stream = InstructionStream::new(artifact.bytecode.clone());
    x3_lang_vm::verifier::verify(&stream)
        .expect("canonical VM verifier must accept exact bridge bytes");
}
```

If `InstructionStream::new` has a different existing constructor, use the actual public constructor from `x3-lang/vm/src/x3_lang_vm.rs`; do not add a duplicate wrapper type just for the test.

- [ ] **Step 2: Run the E2E proof**

```bash
cargo test -p x3-x3-integration --features compile --test compiler_bridge_e2e
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/x3-integration/tests/compiler_bridge_e2e.rs crates/x3-integration/Cargo.toml Cargo.lock
git commit -m "test(x3-integration): prove canonical bridge bytecode e2e"
```

---

### Task 7: Run the Full Feature Matrix and Canonical Regression Gates

**Files:**
- Modify: existing CI workflow that currently gates X3Lang / `x3-integration`.

**Interfaces:**
- Produces exact-head evidence for host compile, runtime isolation, canonical language regression, and E2E proof.

- [ ] **Step 1: Run runtime-safe root checks locally**

```bash
cargo check -p x3-x3-integration --no-default-features
cargo check -p x3-x3-integration
cargo test -p x3-x3-integration --features compile
```

Expected: all PASS.

- [ ] **Step 2: Run canonical X3Lang gates**

```bash
cd x3-lang
cargo fmt --all -- --check
cargo clippy -p x3-lang-compiler -p x3-lang-vm -p x3-lang-ast -p x3-lang-common -p x3-lang-lexer -p x3-tools --all-targets --all-features -- -D warnings
cargo test -p x3-lang-compiler -p x3-lang-vm -p x3-lang-ast -p x3-lang-common -p x3-lang-lexer -p x3-tools --all-features
```

Expected: all PASS, including Trading Core E2E, TradingVm rollback/property tests, and mainnet capability tests.

- [ ] **Step 3: Confirm runtime/WASM dependency isolation**

From repo root run:

```bash
cargo tree -p x3-x3-integration --no-default-features | grep x3-lang-compiler && exit 1 || true
```

Expected: no `x3-lang-compiler` entry.

Then run the repository's existing runtime WASM build/check command unchanged. If the repo uses a wrapper script or exact package command, preserve that existing command rather than introducing a weaker substitute.

- [ ] **Step 4: Add CI commands to the existing relevant workflow**

Add jobs/steps that execute the exact commands from Steps 1-3. Do not remove or loosen existing root, Rust, OSV, fmt, clippy, test, or WASM gates.

- [ ] **Step 5: Commit**

```bash
git add .github/workflows
git commit -m "ci: gate canonical x3lang compiler bridge"
```

---

### Task 8: Final Exact-Head Verification and Jack-Ready Evidence

**Files:**
- Modify: `docs/x3-lang/architecture-authority.md` only if the bridge status text is now stale.
- Create: `audit-artifacts/x3lang-bridge/<head-sha>/README.md` if the repository's existing audit-artifact convention permits committed evidence summaries.

**Interfaces:**
- Produces: reviewer-readable proof that the root bridge is canonical and Jack-consumable.

- [ ] **Step 1: Scan changed files for fake/stub bridge paths**

Run:

```bash
git diff master...HEAD -- crates/x3-integration x3-lang/compiler | grep -Ei 'Ok\(vec!\[\]\)|fake bytecode|synthetic bytecode|not yet wired' && exit 1 || true
```

Expected: no placeholder success path and no stale "not yet wired" bridge text.

- [ ] **Step 2: Re-run focused exact-head tests**

```bash
cargo test -p x3-x3-integration --features compile --test compiler_bridge
cargo test -p x3-x3-integration --features compile --test compiler_bridge_e2e
```

Expected: PASS.

- [ ] **Step 3: Verify the exact artifact contract Jack will consume**

The final public bridge must expose all of:

```rust
X3CompilationMode
X3CompileRequest
X3CompileArtifact
compile
compile_source
```

and `X3CompileArtifact` must include:

```rust
bytecode
compiler_version
bytecode_version
mode
source_hash
```

- [ ] **Step 4: Update authority documentation only if needed**

If `docs/x3-lang/architecture-authority.md` still says the root compiler bridge is unwired, replace only that stale statement with the proven host-side delegation status. Do not change authority ownership: canonical semantics stay in `x3-lang/`.

- [ ] **Step 5: Commit final evidence/docs**

```bash
git add docs/x3-lang/architecture-authority.md audit-artifacts/x3lang-bridge 2>/dev/null || true
git commit -m "docs: record canonical x3lang bridge proof"
```

Skip the commit only if neither file changed.

---

## Completion Gate

Do not call the bridge complete until all of these are simultaneously true on the same head SHA:

```text
root no-default-features check        GREEN
root default check                    GREEN
x3-integration + compile tests        GREEN
bridge determinism tests              GREEN
Dev/Production/Mainnet mode tests     GREEN
Mainnet capability rejection          GREEN
canonical VM E2E verifier proof       GREEN
canonical x3-lang fmt                 GREEN
canonical x3-lang clippy -D warnings  GREEN
canonical x3-lang tests               GREEN
runtime WASM boundary                 GREEN
fake/stub bridge scan                 CLEAN
```

Only after this gate is green should Jack The Ripper add a dependency on the root `x3-x3-integration` interface.