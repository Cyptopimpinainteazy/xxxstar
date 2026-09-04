# X3Lang 1.0 Core Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish one authoritative X3Lang front end and a testable compiler-hardening baseline with stable diagnostics, executable conformance fixtures, numeric-policy enforcement, IR verification, and safety-intent preservation.

**Architecture:** Reconcile the duplicate X3 language stacks before adding semantics. Preserve the existing `x3-lang/compiler -> x3-lang/vm` execution pipeline, but reuse or migrate proven top-level `x3-parser`/`x3-typeck` behavior instead of maintaining two conflicting language definitions. Harden boundaries with structured diagnostics and verification rather than rewriting the compiler wholesale.

**Tech Stack:** Rust 2021, Cargo workspace, `proptest`, existing X3 lexer/AST/compiler/VM crates, YAML opcode specification, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-09-02-x3lang-1-0-readiness-design.md`

## Global Constraints

- Do not rewrite stable compiler components solely for aesthetics.
- Compiler and VM must consume one canonical execution contract for consensus-sensitive opcode/version behavior.
- Invalid source and malformed compiler states must fail closed with structured errors, not uncontrolled panics.
- Bare integer literals remain `u64` by default for the 1.0 baseline.
- No implicit signed/unsigned numeric coercion is allowed in the 1.0 baseline.
- Direct function-argument incompatibility must map to the dedicated argument-type-mismatch diagnostic semantics.
- Safety-critical swap/bridge intent fields must survive source -> AST -> semantics -> IR -> bytecode.
- VM execution remains authoritative for gas accounting; gas work is a later plan, not part of this first slice.
- Do not claim X3Lang 1.0 readiness until trusted CI actually executes and passes all required gates.

---

## File Structure for This Slice

The first implementation slice deliberately avoids deleting either language stack until callers are mapped and behavior is covered by tests.

- `docs/x3-lang/architecture-authority.md` — records the production-authoritative front-end decision, ownership boundaries, migration map, and deprecation policy.
- `x3-lang/compiler/src/diagnostic.rs` — compiler-facing stable diagnostic code/severity/span representation.
- `x3-lang/compiler/src/verify.rs` — IR structural and safety-invariant verifier.
- `x3-lang/compiler/src/lib.rs` — exports diagnostics/verifier and wires verification into the compile pipeline.
- `x3-lang/compiler/src/semantic.rs` — maps semantic failures to stable diagnostics without changing approved numeric semantics.
- `x3-lang/compiler/tests/test_diagnostics.rs` — diagnostic code/span contract tests.
- `x3-lang/compiler/tests/test_ir_verifier.rs` — malformed/valid IR verifier tests.
- `x3-lang/compiler/tests/test_intent_preservation.rs` — source-to-IR safety-field preservation tests.
- `x3-lang/tests/conformance/manifest.json` — machine-readable conformance case inventory and expected outcomes.
- `x3-lang/tests/conformance/valid/*` and `invalid/*` — canonical source fixtures.
- `x3-lang/compiler/tests/test_conformance.rs` — manifest-driven compiler conformance harness.
- `docs/rfc/RFC-t5-6-numeric-coercion-policy.md` — promote from draft only after tests prove the approved baseline.

### Task 1: Reconcile the Two X3 Front-End Stacks

**Files:**
- Create: `docs/x3-lang/architecture-authority.md`
- Inspect: `x3-lang/Cargo.toml`
- Inspect: `x3-lang/compiler/Cargo.toml`
- Inspect: `x3-lang/compiler/src/parser.rs`
- Inspect: `x3-lang/compiler/src/semantic.rs`
- Inspect: `crates/x3-parser/Cargo.toml`
- Inspect: `crates/x3-typeck/Cargo.toml`
- Inspect: all repository callers importing `x3_parser`, `x3_typeck`, `x3_lang_compiler`, or the corresponding package names

**Interfaces:**
- Consumes: existing compiler/front-end crates and repository call graph.
- Produces: an explicit authority decision naming the canonical parser, AST, semantic/type-check path, compiler entry point, and migration/deprecation treatment for the non-authoritative stack.

- [ ] **Step 1: Build a caller inventory**

Run repository searches for these exact identifiers/package names:

```bash
git grep -nE 'x3[_-](parser|typeck|lang[_-]compiler)|x3-lang-compiler' -- ':!target' ':!x3-lang/target'
```

Record every production caller, test-only caller, and orphaned crate in a table in `docs/x3-lang/architecture-authority.md`.

- [ ] **Step 2: Compare the semantic contracts**

Document, with source paths, which stack currently owns each responsibility:

```text
lexing
parsing
AST
symbol resolution
type checking
numeric literal policy
diagnostics
lowering
IR
bytecode emission
VM entry
```

The document must explicitly identify duplicate responsibilities rather than labeling both stacks authoritative.

- [ ] **Step 3: Choose the authoritative production path**

Use this decision rule:

```text
Prefer the path that currently reaches x3-lang bytecode + VM in production/E2E execution.
Reuse tested semantics from the other path by migration or adapter only when they are missing from the authoritative path.
Do not maintain two independent grammar/type-policy definitions.
```

Write the selected path and rationale in `docs/x3-lang/architecture-authority.md`.

- [ ] **Step 4: Define migration boundaries**

The authority document must include a table with these columns:

```text
Component | Authoritative path | Non-authoritative path | Action | Removal prerequisite
```

For every duplicated parser/type-check component, choose exactly one action: `keep`, `adapt`, `migrate-tests`, or `deprecate`.

- [ ] **Step 5: Verify the document against actual Cargo dependencies**

Run:

```bash
cargo metadata --format-version 1 --no-deps > /tmp/x3-metadata.json
```

Confirm the authority document does not claim a dependency edge that Cargo metadata contradicts.

- [ ] **Step 6: Commit**

```bash
git add docs/x3-lang/architecture-authority.md
git commit -m "docs(x3lang): define authoritative compiler architecture"
```

### Task 2: Add Stable Compiler Diagnostics

**Files:**
- Create: `x3-lang/compiler/src/diagnostic.rs`
- Modify: `x3-lang/compiler/src/lib.rs`
- Modify: `x3-lang/compiler/src/semantic.rs`
- Test: `x3-lang/compiler/tests/test_diagnostics.rs`

**Interfaces:**
- Consumes: existing source-span type used by the compiler parser/semantic layer.
- Produces: `DiagnosticCode`, `DiagnosticSeverity`, and `CompilerDiagnostic` as the stable compiler-facing error contract.

- [ ] **Step 1: Write failing diagnostic contract tests**

Create `x3-lang/compiler/tests/test_diagnostics.rs` with tests equivalent to:

```rust
use x3_lang_compiler::diagnostic::{CompilerDiagnostic, DiagnosticCode, DiagnosticSeverity};

#[test]
fn diagnostic_code_is_stable_text() {
    assert_eq!(DiagnosticCode::UnexpectedToken.as_str(), "X3E0001");
    assert_eq!(DiagnosticCode::UndefinedSymbol.as_str(), "X3E0101");
    assert_eq!(DiagnosticCode::ArgumentTypeMismatch.as_str(), "X3E0202");
    assert_eq!(DiagnosticCode::InvalidNumericCoercion.as_str(), "X3E0301");
    assert_eq!(DiagnosticCode::InvalidCrossChainRoute.as_str(), "X3E0401");
    assert_eq!(DiagnosticCode::UnsafeIr.as_str(), "X3E0501");
}

#[test]
fn diagnostic_preserves_primary_span() {
    let diagnostic = CompilerDiagnostic::error(
        DiagnosticCode::UndefinedSymbol,
        "undefined symbol `missing`",
        7..14,
    );
    assert_eq!(diagnostic.severity, DiagnosticSeverity::Error);
    assert_eq!(diagnostic.primary_span, 7..14);
}
```

Adapt only the span concrete type to the compiler's existing source-span representation.

- [ ] **Step 2: Run the tests and verify RED**

Run:

```bash
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler --test test_diagnostics
```

Expected: FAIL because `diagnostic` and its exported types do not exist.

- [ ] **Step 3: Implement the diagnostic representation**

Create `diagnostic.rs` with these semantic variants and exact stable codes:

```rust
pub enum DiagnosticCode {
    UnexpectedToken,          // X3E0001
    UndefinedSymbol,          // X3E0101
    IncompatibleTypes,        // X3E0201
    ArgumentTypeMismatch,     // X3E0202
    InvalidNumericCoercion,   // X3E0301
    InvalidCrossChainRoute,   // X3E0401
    UnsafeIr,                 // X3E0501
}

pub enum DiagnosticSeverity { Error, Warning, Note }

pub struct CompilerDiagnostic {
    pub code: DiagnosticCode,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub primary_span: std::ops::Range<usize>,
    pub secondary_spans: Vec<std::ops::Range<usize>>,
    pub help: Option<String>,
}
```

If the compiler already has a canonical span type, replace `Range<usize>` consistently in both implementation and tests rather than introducing a second span model.

- [ ] **Step 4: Export the module and map one existing semantic error**

In `lib.rs` export:

```rust
pub mod diagnostic;
```

In `semantic.rs`, convert the direct call-argument incompatibility path to `DiagnosticCode::ArgumentTypeMismatch`. Do not broaden this step into a semantic rewrite.

- [ ] **Step 5: Run targeted tests**

```bash
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler --test test_diagnostics
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add x3-lang/compiler/src/diagnostic.rs x3-lang/compiler/src/lib.rs x3-lang/compiler/src/semantic.rs x3-lang/compiler/tests/test_diagnostics.rs
git commit -m "feat(x3lang): add stable compiler diagnostics"
```

### Task 3: Build the Executable Conformance Harness

**Files:**
- Create: `x3-lang/tests/conformance/manifest.json`
- Create: `x3-lang/tests/conformance/valid/arithmetic/basic_add.x3`
- Create: `x3-lang/tests/conformance/invalid/syntax/unclosed_block.x3`
- Create: `x3-lang/tests/conformance/invalid/numeric_conversion/unsigned_argument_mismatch.x3`
- Create: `x3-lang/compiler/tests/test_conformance.rs`

**Interfaces:**
- Consumes: authoritative compile/check API selected by Task 1 and stable diagnostics from Task 2.
- Produces: manifest-driven conformance runner that can add cases without adding a new Rust test function for each source file.

- [ ] **Step 1: Define the manifest schema through a failing test**

Create a manifest with entries shaped exactly as:

```json
{
  "cases": [
    {"name":"basic_add","path":"valid/arithmetic/basic_add.x3","expect":"accept"},
    {"name":"unclosed_block","path":"invalid/syntax/unclosed_block.x3","expect":"reject","code":"X3E0001"},
    {"name":"unsigned_argument_mismatch","path":"invalid/numeric_conversion/unsigned_argument_mismatch.x3","expect":"reject","code":"X3E0202"}
  ]
}
```

Write `test_conformance.rs` to deserialize this manifest, read each fixture, invoke the authoritative check/compile entry point, and assert accept/reject plus exact diagnostic code for rejected cases.

- [ ] **Step 2: Run and verify RED**

```bash
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler --test test_conformance
```

Expected: FAIL until fixtures and/or diagnostic mapping satisfy the manifest.

- [ ] **Step 3: Add the minimal valid arithmetic fixture**

Use syntax already accepted by an existing compiler fixture/test; do not invent new grammar. The fixture must exercise an integer addition and successful return/observable result supported by current X3Lang syntax.

- [ ] **Step 4: Add the invalid syntax fixture**

Copy a currently valid block/function form from an existing fixture, then remove only its closing delimiter so the failure is specifically an unexpected/incomplete syntax diagnostic.

- [ ] **Step 5: Add the numeric mismatch fixture**

Use an existing function-declaration/call syntax and construct a fixed signed/unsigned parameter mismatch that the approved numeric policy rejects. The expected diagnostic is `X3E0202` (`ArgumentTypeMismatch`), not a generic unification error.

- [ ] **Step 6: Make the harness pass without special-casing filenames**

Any mapping from internal parser/semantic errors to stable diagnostics must be based on error kind, never fixture name or source text.

- [ ] **Step 7: Run conformance and compiler suites**

```bash
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler --test test_conformance
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add x3-lang/tests/conformance x3-lang/compiler/tests/test_conformance.rs x3-lang/compiler/src
git commit -m "test(x3lang): add executable conformance harness"
```

### Task 4: Lock the Numeric-Coercion Policy with Tests

**Files:**
- Modify: `x3-lang/compiler/tests/test_conformance.rs`
- Create: additional fixtures under `x3-lang/tests/conformance/valid/numeric/` and `invalid/numeric_conversion/`
- Modify: `docs/rfc/RFC-t5-6-numeric-coercion-policy.md`

**Interfaces:**
- Consumes: conformance harness and `ArgumentTypeMismatch` diagnostic contract.
- Produces: executable proof for the 1.0 numeric baseline and an RFC whose status matches tested behavior.

- [ ] **Step 1: Add failing policy cases**

Add manifest cases covering all four rules:

```text
bare positive integer defaults successfully where u64 is required
unary-negated integer follows existing unary-negation semantics
u64 literal passed to incompatible signed parameter is rejected
argument incompatibility reports X3E0202 rather than X3E0201
```

Use only syntax/types already supported by the authoritative parser.

- [ ] **Step 2: Run and verify any uncovered rule fails**

```bash
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler --test test_conformance numeric
```

If Cargo's substring filter does not select manifest subcases, run the entire `test_conformance` target and identify the named failing manifest case.

- [ ] **Step 3: Make minimal semantic/diagnostic changes**

Modify only the authoritative semantic/type-check path needed to enforce:

```text
bare integer -> u64 baseline
negative form -> unary negation semantics
no implicit signed/unsigned coercion
call-site mismatch -> X3E0202
```

Do not add suffix syntax or implicit widening/narrowing.

- [ ] **Step 4: Promote the RFC status**

Change the RFC header from:

```text
Status: DRAFT — requires review
```

to:

```text
Status: ACCEPTED for X3Lang 1.0 baseline
```

Add an `Executable conformance` section pointing to `x3-lang/tests/conformance/manifest.json` and `x3-lang/compiler/tests/test_conformance.rs`.

- [ ] **Step 5: Run tests**

```bash
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler --test test_conformance
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add docs/rfc/RFC-t5-6-numeric-coercion-policy.md x3-lang/tests/conformance x3-lang/compiler/tests/test_conformance.rs x3-lang/compiler/src
git commit -m "test(x3lang): lock numeric coercion semantics"
```

### Task 5: Add the IR Verifier Boundary

**Files:**
- Create: `x3-lang/compiler/src/verify.rs`
- Modify: `x3-lang/compiler/src/lib.rs`
- Inspect/Modify: `x3-lang/compiler/src/ir.rs`
- Test: `x3-lang/compiler/tests/test_ir_verifier.rs`

**Interfaces:**
- Consumes: the concrete IR program/module/function/block types in `ir.rs`.
- Produces: `verify_ir(...) -> Result<(), Vec<CompilerDiagnostic>>`, called after lowering and before register allocation/emission.

- [ ] **Step 1: Write verifier tests against concrete IR constructors**

Create tests for:

```text
valid minimal IR -> Ok
branch to missing block -> X3E0501
use of nonexistent value/register abstraction -> X3E0501
block missing required terminator -> X3E0501
malformed swap/bridge instruction missing required safety attribute -> X3E0501
```

Construct IR using the existing concrete structs/enums from `ir.rs`; if fields are private, add the smallest test builder API in `ir.rs` rather than duplicating IR types.

- [ ] **Step 2: Run and verify RED**

```bash
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler --test test_ir_verifier
```

Expected: FAIL because `verify_ir` does not yet exist or does not enforce the tested invariant.

- [ ] **Step 3: Implement structural verification**

In `verify.rs`, implement checks in this order:

```text
1. collect valid block/value identifiers
2. validate uniqueness
3. validate all referenced identifiers
4. validate branch targets
5. validate block termination/control-flow shape
6. validate instruction operand/result compatibility available from current IR metadata
7. validate required swap/bridge safety attributes represented by current IR
```

Every failure returns `CompilerDiagnostic` with `DiagnosticCode::UnsafeIr`.

- [ ] **Step 4: Wire verification into compilation**

In the existing compile pipeline in `lib.rs`, call verification immediately after lowering and before register allocation/emission:

```rust
let ir = lower(...)?;
verify_ir(&ir).map_err(CompileError::Verification)?;
let allocated = allocate_registers(ir)?;
```

Adapt names to existing pipeline functions/error enum; preserve the ordering exactly.

- [ ] **Step 5: Run verifier and existing pipeline tests**

```bash
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler --test test_ir_verifier
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler --test test_compiler_pipeline
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add x3-lang/compiler/src/verify.rs x3-lang/compiler/src/lib.rs x3-lang/compiler/src/ir.rs x3-lang/compiler/tests/test_ir_verifier.rs
git commit -m "feat(x3lang): verify IR before bytecode emission"
```

### Task 6: Prove Safety-Critical Intent Preservation

**Files:**
- Create: `x3-lang/compiler/tests/test_intent_preservation.rs`
- Inspect/Modify: `x3-lang/compiler/src/parser.rs`
- Inspect/Modify: `x3-lang/compiler/src/semantic.rs`
- Inspect/Modify: `x3-lang/compiler/src/ir.rs`
- Inspect/Modify: `x3-lang/compiler/src/lowering.rs`
- Inspect/Modify: `x3-lang/compiler/src/emitter.rs`

**Interfaces:**
- Consumes: current swap/bridge syntax and current IR/bytecode representations.
- Produces: regression tests proving existing safety fields are not silently dropped through compilation.

- [ ] **Step 1: Write source-to-IR preservation tests**

Use existing valid swap/bridge fixtures or syntax and assert that the lowered IR retains every currently represented field among:

```text
min_output
slippage policy
route/destination
receiver
timeout/deadline
refund policy
nonce/replay metadata
finality requirement
```

Only assert fields that the current source grammar actually accepts; for missing fields, record them as later runtime-path work rather than inventing syntax in this task.

- [ ] **Step 2: Run and verify RED for any dropped field**

```bash
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler --test test_intent_preservation
```

Expected: either RED for a real preservation gap, or PASS if current claims are already correct. A PASS is acceptable evidence; do not force code churn.

- [ ] **Step 3: Fix only demonstrated preservation gaps**

For each failing field, trace:

```text
parser node -> semantic representation -> IR field -> lowering -> emitter
```

Add the field at the earliest point it is lost and forward it unchanged thereafter. Do not redesign unrelated instructions.

- [ ] **Step 4: Add a bytecode-level assertion where encoding exists**

For safety data encoded in bytecode, decode/inspect emitted bytecode with the existing decoder/test utilities and assert the value is preserved. If a field intentionally remains host/runtime metadata rather than bytecode, document that boundary in the test name/comment and assert preservation to that boundary.

- [ ] **Step 5: Run targeted and E2E suites**

```bash
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler --test test_intent_preservation
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler --test test_atomic_swap_syntax
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler --test test_e2e_fixtures
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add x3-lang/compiler/tests/test_intent_preservation.rs x3-lang/compiler/src
git commit -m "test(x3lang): prove safety intent preservation"
```

### Task 7: Add Bounded Property Tests for Compiler Safety

**Files:**
- Create: `x3-lang/compiler/tests/test_properties.rs`
- Modify only if a property exposes a real defect: authoritative parser/compiler files

**Interfaces:**
- Consumes: authoritative parse/check/compile API and existing `proptest` dev dependency.
- Produces: bounded regression properties for hostile source handling and deterministic compilation.

- [ ] **Step 1: Add parser-no-panic property**

Use `proptest` to generate bounded UTF-8 strings (for example 0..2048 chars), invoke the authoritative parse/check entry point inside `std::panic::catch_unwind`, and assert it never unwinds. Acceptance and structured rejection are both valid outcomes.

- [ ] **Step 2: Add deterministic-compilation property**

Generate from a small grammar of currently valid arithmetic/control-flow snippets rather than arbitrary text. Compile the exact same generated source twice with identical options and assert canonical emitted bytes are identical.

- [ ] **Step 3: Run and verify properties**

```bash
PROPTEST_CASES=256 cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler --test test_properties
```

Expected: PASS, or a minimized reproducible failing case if a defect exists.

- [ ] **Step 4: If a property fails, convert the minimized case to a fixed regression test first**

Add the minimized source to the conformance fixtures or a named unit test, confirm it fails without the fix, then make the smallest production change needed.

- [ ] **Step 5: Re-run**

```bash
PROPTEST_CASES=256 cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler --test test_properties
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add x3-lang/compiler/tests/test_properties.rs x3-lang/compiler/tests x3-lang/compiler/src x3-lang/tests/conformance
git commit -m "test(x3lang): add bounded compiler properties"
```

### Task 8: Wire the Core-Hardening Gate and Produce Evidence

**Files:**
- Create or Modify: the existing X3Lang-specific GitHub Actions workflow discovered by repository search
- Modify: `x3-lang/PLAN.md`
- Create: `reports/x3lang/x3lang-core-hardening-evidence.md`

**Interfaces:**
- Consumes: all tests from Tasks 2-7.
- Produces: one reproducible command set and CI gate for the first hardening milestone, plus an evidence report that distinguishes code failures from CI infrastructure failures.

- [ ] **Step 1: Discover the existing X3Lang workflow before editing**

Run:

```bash
git grep -nE 'x3-lang|x3lang' .github/workflows
```

Extend the most specific existing workflow. Do not create a duplicate workflow if one already owns X3Lang readiness.

- [ ] **Step 2: Add the core-hardening command set**

The workflow must run at minimum:

```bash
cargo fmt --manifest-path x3-lang/Cargo.toml --all -- --check
cargo clippy --manifest-path x3-lang/Cargo.toml --workspace --all-targets --all-features -- -D warnings
cargo test --manifest-path x3-lang/Cargo.toml --workspace
PROPTEST_CASES=256 cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler --test test_properties
```

If repository toolchain constraints require invoking these from `x3-lang/`, use equivalent `cd x3-lang && cargo ...` commands while preserving the same gate.

- [ ] **Step 3: Run the gate locally**

Run the exact commands from Step 2 and capture versions:

```bash
rustc --version
cargo --version
git rev-parse HEAD
```

- [ ] **Step 4: Write the evidence report**

`reports/x3lang/x3lang-core-hardening-evidence.md` must include:

```text
commit SHA
Rust/Cargo versions
commands executed
pass/fail result for each command
known remaining X3Lang 1.0 milestones
CI execution status
```

If GitHub Actions still fails before steps start, say exactly that and do not mark CI green.

- [ ] **Step 5: Update the completion plan**

In `x3-lang/PLAN.md`, mark only work proven by the commands/evidence. Leave deterministic gas, full runtime rollback, full bytecode safety, and developer tooling pending unless separately proven.

- [ ] **Step 6: Final verification**

```bash
git diff --check
cargo fmt --manifest-path x3-lang/Cargo.toml --all -- --check
cargo clippy --manifest-path x3-lang/Cargo.toml --workspace --all-targets --all-features -- -D warnings
cargo test --manifest-path x3-lang/Cargo.toml --workspace
PROPTEST_CASES=256 cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler --test test_properties
```

Expected: all local commands PASS. CI must be reported separately based on actual GitHub execution evidence.

- [ ] **Step 7: Commit**

```bash
git add .github/workflows x3-lang/PLAN.md reports/x3lang/x3lang-core-hardening-evidence.md
git commit -m "ci(x3lang): gate core compiler hardening"
```

## Completion Boundary

This plan completes **Milestone 0 + Milestone 1/Core Compiler Hardening** from the approved X3Lang 1.0 design. It does not claim completion of the later bytecode-verifier, deterministic-gas, full runtime rollback/replay/finality, or polished developer-tooling milestones. Those should be separate implementation plans after this slice is verified.
