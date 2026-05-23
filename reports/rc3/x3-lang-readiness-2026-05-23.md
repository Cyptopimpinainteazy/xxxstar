# X3-Lang Production Readiness Rundown (2026-05-23)

## 1) Executive Summary
x3-lang is not sign-off ready in this workspace snapshot. The current state is artifact-heavy and source-light: [x3-lang](../../x3-lang) exists with a large build cache footprint, but no auditable source tree for core x3-lang crates is present in the tracked workspace.

Production verdict: NO-GO  
Readiness score: 29/100 (confidence: medium on blockers, low on functional correctness)

Evidence anchors:
- [x3-lang](../../x3-lang)
- [x3-lang/target/debug/deps/x3_lang-e612796fdcb885d3.d](../../x3-lang/target/debug/deps/x3_lang-e612796fdcb885d3.d#L1)
- [x3-lang/target/debug/deps/x3_parser-4178972c8a274ccb.d](../../x3-lang/target/debug/deps/x3_parser-4178972c8a274ccb.d#L1)
- [x3-lang/target/debug/deps/x3_typeck-52b93eb4c3e85607.d](../../x3-lang/target/debug/deps/x3_typeck-52b93eb4c3e85607.d#L1)
- [x3-lang/target/debug/deps/x3_vm-fee92248d1c62123.d](../../x3-lang/target/debug/deps/x3_vm-fee92248d1c62123.d#L1)
- [.github/workflows/docs-consistency.yml](../../.github/workflows/docs-consistency.yml#L1)

Fact vs inference:
- Fact: artifact manifests reference many x3-lang subsystem source paths.
- Fact: those referenced source paths are not available in this workspace snapshot for direct audit.
- Inference: system likely existed and compiled previously, but current completion cannot be proven without source continuity.

## 2) Functional Completion Matrix

| Capability Area | Status | Confidence | Evidence | Gap |
|---|---|---|---|---|
| Workspace root crate presence | Partial | Medium | [x3-lang/target/debug/deps/x3_lang-e612796fdcb885d3.d](../../x3-lang/target/debug/deps/x3_lang-e612796fdcb885d3.d#L1) | Root crate source not available for review |
| Lexing/parsing pipeline | Partial | Medium | [x3-lang/target/debug/deps/x3_parser-4178972c8a274ccb.d](../../x3-lang/target/debug/deps/x3_parser-4178972c8a274ccb.d#L1) | Parser behavior, errors, edge-case handling cannot be verified |
| HIR/semantic lowering | Partial | Medium | [x3-lang/target/debug/deps/x3_hir-601e33a34a78dd5c.d](../../x3-lang/target/debug/deps/x3_hir-601e33a34a78dd5c.d#L1), [x3-lang/target/debug/deps/x3_semantics-87a116c7a5c975f9.d](../../x3-lang/target/debug/deps/x3_semantics-87a116c7a5c975f9.d#L1) | No source-level invariant review possible |
| Type system and coercion policy | Partial | Low-Medium | [x3-lang/target/debug/deps/x3_typeck-52b93eb4c3e85607.d](../../x3-lang/target/debug/deps/x3_typeck-52b93eb4c3e85607.d#L1), [docs/rfc/RFC-t5-6-numeric-coercion-policy.md](../../docs/rfc/RFC-t5-6-numeric-coercion-policy.md) | Policy-to-implementation trace unavailable |
| MIR/backend/codegen | Partial | Medium | [x3-lang/target/debug/deps/x3_mir-4e60173766291c6d.d](../../x3-lang/target/debug/deps/x3_mir-4e60173766291c6d.d#L1), [x3-lang/target/debug/deps/x3_backend-cce456effa49751a.d](../../x3-lang/target/debug/deps/x3_backend-cce456effa49751a.d#L1) | Output correctness and ABI compatibility unproven |
| Optimizer pass stack | Partial | Medium | [x3-lang/target/debug/deps/x3_opt-18d87cfa2f7789f3.d](../../x3-lang/target/debug/deps/x3_opt-18d87cfa2f7789f3.d#L1) | Safety and semantic-preservation tests unverified |
| VM runtime and hostcalls | Partial | Medium | [x3-lang/target/debug/deps/x3_vm-fee92248d1c62123.d](../../x3-lang/target/debug/deps/x3_vm-fee92248d1c62123.d#L1) | Runtime security and determinism not auditable |
| EVM/SVM integration | Partial | Medium | [x3-lang/target/debug/deps/x3_evm_integration-bb47e8b0be2904e7.d](../../x3-lang/target/debug/deps/x3_evm_integration-bb47e8b0be2904e7.d#L1), [x3-lang/target/debug/deps/x3_svm_integration-3ade91ce3d77befa.d](../../x3-lang/target/debug/deps/x3_svm_integration-3ade91ce3d77befa.d#L1) | Cross-VM correctness and gas semantics unproven |
| Verifier/rules/gas checks | Partial | Medium | [x3-lang/target/debug/deps/x3_verifier-b246cc023bf91b34.d](../../x3-lang/target/debug/deps/x3_verifier-b246cc023bf91b34.d#L1) | Rule enforcement cannot be validated without source/tests |

## 3) Non-Functional Readiness

| Dimension | Rating | Fact | Inference |
|---|---|---|---|
| Security readiness | Weak | Verifier and VM components are present in artifacts | Critical controls may exist, but are non-auditable in current snapshot |
| Reliability/operability | Weak-Moderate | Build artifacts indicate previously compilable stack | Runtime and recovery behavior cannot be trusted without source and E2E checks |
| Performance readiness | Weak | Optimizer and VM modules exist in dep manifests | No benchmark evidence, no regression budget traceability |
| Maintainability | Weak | Missing source continuity for core crates | Team cannot safely patch/release from this backup |
| Compliance/docs discipline | Moderate | Docs consistency workflow exists | Documentation governance exists, but cannot replace code-level assurance |

Additional fact:
- x3-lang footprint is large and mostly build output, which increases operational risk for a recovery snapshot.

## 4) Test and Verification Coverage
Current verifiable test posture for x3-lang is insufficient.

Facts:
- No direct x3-lang source tests are auditable from available source tree.
- Dependency manifests do not expose explicit test-source paths for x3-lang crates in this snapshot.
- Adjacent systems do have visible test activity (sidecar/scanner from earlier audit context), but that does not close x3-lang verification gaps.

Evidence anchors:
- [x3-lang/target/debug/deps/x3_parser-4178972c8a274ccb.d](../../x3-lang/target/debug/deps/x3_parser-4178972c8a274ccb.d#L1)
- [x3-lang/target/debug/deps/x3_typeck-52b93eb4c3e85607.d](../../x3-lang/target/debug/deps/x3_typeck-52b93eb4c3e85607.d#L1)
- [x3-lang/target/debug/deps/x3_opt-18d87cfa2f7789f3.d](../../x3-lang/target/debug/deps/x3_opt-18d87cfa2f7789f3.d#L1)

Required minimum test evidence before sign-off:
1. Parser golden tests for syntax/error spans.
2. Type coercion policy conformance tests tied to numeric coercion RFC.
3. Optimizer semantic-preservation tests (input program equivalence).
4. VM determinism and gas-accounting property tests.
5. Cross-VM integration tests covering EVM and SVM adapters.

## 5) Git Continuity and Change Impact
Continuity risk is high for x3-lang in this workspace.

Facts:
- Recent branch history shows active engineering on node/sidecar/integration hardening.
- x3-lang core source is absent from tracked/visible audit scope, while artifact traces reference many missing crate paths.
- Documentation consistency workflow exists and is wired in CI triggers.

Evidence anchors:
- [node/src/service.rs](../../node/src/service.rs#L1)
- [crates/x3-sidecar/src/main.rs](../../crates/x3-sidecar/src/main.rs#L1)
- [x3-lang/target/debug/deps/x3_compiler-29bb0577883d1188.d](../../x3-lang/target/debug/deps/x3_compiler-29bb0577883d1188.d#L1)
- [.github/workflows/docs-consistency.yml](../../.github/workflows/docs-consistency.yml#L1)

Impact:
- Platform integration work can continue, but production-grade x3-lang completion cannot be claimed from this snapshot alone.

## 6) Risk Register and Critical Path Gaps

| Risk | Severity | Fact vs Inference | Evidence | Owner Action |
|---|---|---|---|---|
| Missing x3-lang source continuity | Critical | Fact | [x3-lang](../../x3-lang), [x3-lang/target/debug/deps/x3_compiler-29bb0577883d1188.d](../../x3-lang/target/debug/deps/x3_compiler-29bb0577883d1188.d#L1) | Restore source tree from authoritative remote/backup |
| Cannot prove policy compliance for type coercion | High | Fact | [docs/rfc/RFC-t5-6-numeric-coercion-policy.md](../../docs/rfc/RFC-t5-6-numeric-coercion-policy.md), [x3-lang/target/debug/deps/x3_typeck-52b93eb4c3e85607.d](../../x3-lang/target/debug/deps/x3_typeck-52b93eb4c3e85607.d#L1) | Reconstruct source and add RFC-trace tests |
| VM/runtime security unverifiable | High | Fact | [x3-lang/target/debug/deps/x3_vm-fee92248d1c62123.d](../../x3-lang/target/debug/deps/x3_vm-fee92248d1c62123.d#L1) | Run verifier plus fuzz/property suite on restored code |
| Optimizer regression risk | High | Inference from missing tests | [x3-lang/target/debug/deps/x3_opt-18d87cfa2f7789f3.d](../../x3-lang/target/debug/deps/x3_opt-18d87cfa2f7789f3.d#L1) | Add semantic-equivalence gates in CI |
| Artifact-heavy snapshot may mask drift | Medium | Fact | [x3-lang](../../x3-lang) | Rebuild from clean checkout and compare outputs |

Critical path:
1. Recover source.
2. Rebuild deterministically.
3. Re-run x3-lang test matrix.
4. Revalidate cross-VM runtime invariants.
5. Reassess release gate.

## 7) Priority Matrix with Acceptance Criteria

| Priority | Item | Acceptance Criteria | Current State |
|---|---|---|---|
| P0 | Source restoration for x3-lang crates | Core crates compile from source in clean environment with no missing paths | Blocked |
| P0 | End-to-end compiler pipeline verification | Parse -> semantic -> typeck -> MIR/backend -> VM execution test suite green | Blocked |
| P0 | Cross-VM assurance linkage | EVM/SVM integration tests pass with deterministic gas and state transitions | Blocked |
| P1 | RFC conformance checks | Numeric coercion RFC mapped to executable tests and passing | Blocked |
| P1 | Security verification | Verifier plus VM hardening tests and fuzz/property tests pass | Blocked |
| P2 | Documentation-to-code traceability | T5 docs/rfcs linked to passing test artifacts in CI | Partial |

## 8) Recommended Next Actions
1. Restore missing x3-lang source directories and manifests from the authoritative repository state before any release decision.
2. Produce a clean, reproducible build from source and archive fresh dep manifests plus build logs for audit.
3. Add and run a mandatory x3-lang release test pack covering parser/typeck/optimizer/vm/verifier and EVM/SVM adapters.
4. Add CI jobs dedicated to x3-lang compile, tests, and regression checks alongside existing docs consistency checks.
5. Re-run this readiness audit immediately after source restoration with line-level evidence from actual implementation files.

Minimum additional evidence required before sign-off:
- All source files for crates referenced by dep manifests in [x3-lang/target/debug/deps](../../x3-lang/target/debug/deps).
- x3-lang workspace manifests and lockfiles.
- x3-lang test suites and latest CI test reports.
- Symbol-level traces for numeric coercion policy implementation.
- EVM/SVM integration test harness and passing results.

## 9) Definition of Done
x3-lang is considered complete and production-ready only when all conditions below are met:
1. Source completeness: all core x3-lang crates are present and auditable from source, not only build artifacts.
2. Build reproducibility: clean rebuild succeeds from source with deterministic outputs and no missing-path references.
3. Functional completion: parser, semantics, type checker, optimizer, backend, verifier, VM, and cross-VM integrations all pass their test suites.
4. Policy compliance: numeric coercion and governance-related RFC behavior is proven by executable tests.
5. Security assurance: verifier and VM hardening tests plus fuzz/property tests pass with no critical findings.
6. CI continuity: x3-lang gates are active in CI and block merges on regression.
7. Audit evidence: each gate has stable evidence links and machine-generated logs for release sign-off.

Until then, completion status remains partial/blocked and release posture is NO-GO.
