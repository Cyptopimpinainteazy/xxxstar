# X3 Lang Trading Core v1 Hardening Plan

**Target branch:** `codex/x3-trading-core-v1-hardening`

**Goal:** Repair the twelve audit failures in Trading Core v1 and prove the complete compiled-bytecode execution path on the exact GitHub head.

## Global constraints

- Preserve the approved Trading Core v1 syntax and honest fixture/live boundary.
- Every behavior change starts with a focused regression test that fails on GitHub Actions.
- Compiled policy values must be carried into verified IR and cannot be replaced by caller-selected limits.
- Unknown values, malformed trading sequences, unresolved assets, and unverified capabilities fail closed.
- Atomicity covers the host boundary through an explicit transaction lifecycle.
- Receipt verification must validate economic consistency and trusted attestation, not merely a self-recomputed checksum.
- GitHub Actions is the authoritative test runner because the controller workspace has no Rust toolchain.
- Do not merge until nested workspace format, test, Clippy, end-to-end, and exact-head checks pass.

## Task 1: Establish nested-workspace CI

- Add a dedicated workflow job using `x3-lang/Cargo.toml` for focused tests, full tests, formatting, and Clippy.
- Make failures visible on pull requests and pushes to the hardening branch.

## Task 2: Compiler mode and semantic resolution

- Propagate `CompilationMode` through lowering and compilation.
- Resolve debt types at borrow time, track binding values/types, reject unresolved expressions, remove declaration-order dependence, and attach real source spans.

## Task 3: Stateful trading IR and verifier

- Bind the complete compiled risk policy into IR.
- Add explicit committed-cost representation.
- Verify begin/commit shape, debt lifecycle, binding lifecycle/types, guard ordering, receipt presence, and all economic invariants before emission.

## Task 4: Transactional trading runtime

- Connect decoded bytecode to `TradingVm` execution.
- Add host begin/prepare/commit/rollback semantics.
- Enforce capability chain/version/private-submission requirements, gas/slippage/fee/deadline limits, and state commitments.
- Track binding amounts and use checked numeric conversions.

## Task 5: Correct profit and receipt guarantees

- Accrue all committed costs before settlement-asset profit evaluation.
- Correct fee/balance accounting and cross-asset normalization rejection.
- Add economic receipt replay validation and signed/attested receipt verification with explicit trust input.

## Task 6: Genuine end-to-end proof

- Test source parse -> semantic analysis -> stateful verification -> lowering -> encode -> decode -> transactional host execution -> receipt verification.
- Cover host rollback, nonzero gas/venue/flash costs, tampering/re-signing attempts, malformed IR, unresolved bindings, and production capability failures.

## Task 7: Exact-head delivery gate

- Run nested workspace format, all-target/all-feature tests, Clippy with warnings denied, and repository security/readiness checks.
- Record the final SHA and workflow URLs.
- Keep the pull request unmerged until every required exact-head check succeeds.
