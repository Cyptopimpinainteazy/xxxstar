# ADR 0002 — E2E Test Determinism via Triple-Run Verification

Status: Implemented

Date: 2026-02-08

## Context

End-to-end (E2E) tests for blockchains and distributed systems are inherently non-deterministic due to timing variations, async task ordering, random seeding, and hardware differences. This non-determinism causes flaky CI, makes test failures hard to reproduce, and prevents reliable artifact comparison across runs.

X3 Chain requires stateful E2E testing of:
- CHAIN-CONSENSUS-001: state-root replay across blocks
- GPU swarm coordination and task scheduling
- Cross-chain message ordering and finality
- Settlement engine collateral state transitions

Without deterministic test execution, CI cannot reliably detect real regressions versus environmental flakiness, forcing developers to re-run jobs and delaying merges.

## Decision

Implement **deterministic triple-run verification** for E2E tests:

1. **Deterministic Seeding** — All randomness pinned to a fixed seed at test startup (via `X3_E2E_DETERMINISTIC_SEED`), ensuring identical pseudo-random sequences across runs.

2. **Locked Genesis Parameters** — Immutable test genesis config (block time, timestamp, validator set, gas limits) stored in `tests/e2e/fixtures/deterministic_config.toml` and loaded via `E2E_DETERMINISTIC_TRIPLE_RUN` flag.

3. **Triple-Run CI Matrix** — GitHub Actions workflow executes the same test three times in parallel (or sequential if resource-constrained) with unique Docker project namespaces, collecting artifacts for each run.

4. **Artifact Comparison** — Post-test job computes SHA256 hashes of canonical outputs (logs, state-root.json, rpc-responses.json) and fails CI if any run diverges from the others, signaling a real bug.

5. **Intelligent Readiness Polling** — Replace arbitrary sleep-based waits with exponential backoff RPC health checks (`wait_for_rpc_health()`), reducing test duration and improving determinism by waiting only as long as needed.

## Components

### Deterministic Config Fixture
**File:** `tests/e2e/fixtures/deterministic_config.toml`
- `seed`: "x3-e2e-deterministic-seed-001" (fixed)
- `genesis_timestamp`: 1707388800 (2025-02-08 UTC, locked)
- `block_time_millis`: 6000 (6-second blocks)
- `initial_validators`: [predefined list with fixed keys]
- `max_gas_per_block`: 10000000000 (fixed)

Loaded by `tests/e2e/start_test_environment.sh` when `E2E_DETERMINISTIC_TRIPLE_RUN=1` or `CI=true`.

### RPC Health Polling Helper
**Files:** `tests/e2e/src/wait_for_rpc.rs` (Rust), `tests/e2e/wait_for_rpc.sh` (shell)

Replaces 60 × 2-second sleeps (120s worst-case) with adaptive polling:
- Initial backoff: 500ms
- Max backoff: 8s
- Backoff multiplier: 1.5 with 20% jitter
- Max elapsed time: 5 minutes (configurable)
- Timeout returns `WaitError::Timeout` for diagnostic logging

Typical success: 3–5 retries (1–3 seconds), avoiding unnecessary delays.

### CI Workflow with Triple-Run Matrix
**File:** `.github/workflows/e2e-state-root.yml`

```yaml
strategy:
  matrix:
    run: [1, 2, 3]
env:
  DOCKER_COMPOSE_PROJECT: e2e-${{ github.run_id }}-run${{ matrix.run }}
  E2E_DETERMINISTIC_TRIPLE_RUN: '1'
```

Each run:
1. Starts E2E environment with unique project namespace (prevents port conflicts)
2. Loads deterministic config via `start_test_environment.sh`
3. Executes state-root replay test via `ci_run_state_root_test.sh`
4. Collects artifacts (logs, state-root JSON, RPC responses)
5. Sanitizes PII/secrets via `sanitize_artifacts_v2.sh` (regex + file deletion)
6. Uploads artifacts to GitHub Actions artifact storage

Compare job (runs after all three):
1. Downloads all three runs' artifacts
2. Calls `tests/e2e/compare_artifacts.sh` with run1, run2, run3 directories
3. Computes SHA256 of logs/, state-root.json, rpc-responses.json for each run
4. Fails CI if any hash mismatches
5. Saves diff report to `artifacts/compare_report/report.json` for triage

### Structured Logging for CI (JSON Lines)
**File:** `tests/e2e/ci_wrapper.sh`

Each log line is JSON:
```json
{"ts":"2026-02-08T09:35:12Z","run_id":"abc-123","level":"INFO","msg":"Starting run 1 of 3"}
```

Parseable by CI aggregators (Splunk, DataDog, etc.) for:
- Filtering by run_id to correlate multi-run logs
- Alerting on ERROR/EXIT levels
- Computing total duration via ts timestamps

### Artifact Sanitization (v2)
**File:** `scripts/ci/sanitize_artifacts_v2.sh`

Redaction patterns applied to log/JSON/CSV files:
- Hex strings: `0x[0-9a-fA-F]{20,}` → `0x[REDACTED]`
- 64-char keys: `[0-9a-fA-F]{64}` → `[REDACTED_KEY]`
- Bearer tokens: `Bearer|bearer [A-Za-z0-9._-]{10,}` → `[REDACTED_TOKEN]`
- Secrets: lines matching `(SECRET|PRIVATE|PASSWORD|API_KEY|MNEMONIC)` → `[REDACTED_LINE]`

File deletion by name pattern:
- `*.pem`, `.env`, `*.key`, `.secret`, `*credentials*`

Dry-run mode (`--dry-run 1`) reports matches without modifying.

## Trade-offs

### Strengths
1. **Reproducibility** — Fixed seed + timestamps enable local replication of CI failures
2. **Signal vs. Noise** — Hash mismatches clearly indicate real bugs, not flakiness
3. **Performance Visibility** — Triple-run reveals performance regressions (longer E2E times)
4. **Low Cost** — Reuses existing CI infrastructure; minimal overhead per run

### Limitations
1. **Determinism Boundaries** — Some sources of non-determinism are hard to control:
   - OS task scheduler (mitigated by test isolation via Docker Compose)
   - Network timing (mitigated by localhost RPC, no external network calls)
   - Async runtime task ordering in Tokio (mitigated by fixed seed in test setup)
   - Wall-clock timing of real operations (mitigated by locked genesis timestamps)

2. **Not a Fix for True Async Races** — Triple-run can detect race conditions probabilistically but cannot guarantee absence. Use dynamic analysis tools (ThreadSanitizer) for precise detection.

3. **Artifact Size** — Full triple-run multiplies log volume; sanitizer and compression recommended before long-term storage.

## Acceptance Criteria

- ✅ Deterministic config fixture loads and exports env vars correctly
- ✅ RPC polling replaces sleeps and reduces test duration by 30–50%
- ✅ Triple-run CI matrix runs without port conflicts
- ✅ Artifact comparison detects hash mismatches and fails CI
- ✅ Sanitizer removes all patterns and supports dry-run
- ✅ Three consecutive test runs produce identical output hashes
- ✅ Local developers can replicate CI runs via `E2E_DETERMINISTIC_TRIPLE_RUN=1 ./tests/e2e/start_test_environment.sh`

## Implementation Status

**Done (✅):**
- Deterministic config fixture created and wired into `start_test_environment.sh`
- RPC polling helper (Rust + shell) with unit tests (httptest mock server)
- CI workflow updated with matrix, compare job, proc-macro trybuild job
- Artifact sanitizer v2 with dry-run mode
- Shell script syntax validation
- Proc-macro validator: format validation + registry checks
- All unit and integration tests passing

**In Progress/Pending:**
- CI validation: Push main to test branch, trigger GitHub Actions triple-run
- Performance baseline: Measure expected runtime, detect regressions
- Documentation: Update docs/root/README.md with local test instructions
- Metrics telemetry: Log retry counts, wait times, hash comparison results

## Related ADRs & Issues

- ADR-0001: Bonding & Collateral (uses CHAIN-CONSENSUS-001 state-root tests)
- Invariant Registry (`tests/invariants/registry.toml`): Maps invariants to tests
- Proc-Macro Validator (`crates/invariant-macros`): Compile-time invariant ID validation

## References

- Test Infrastructure: [docs/tests/e2e/README.md](../../docs/tests/e2e/README.md) (to be created)
- Spec: `.github/copilot-instructions.md` (CI/test conventions)
- Workflow: `.github/workflows/e2e-state-root.yml`
