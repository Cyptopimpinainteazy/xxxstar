# E2E Testing Guide

## Overview

The x3-chain E2E test suite provides deterministic, reproducible integration tests for blockchain consensus, GPU swarm coordination, cross-chain settlement, and collateral transitions.

**Key Feature:** Triple-run determinism verification ensures test flakiness is detected as real bugs, not environmental noise.

## Quick Start

### Single E2E Run (Development)

```bash
cd <workspace>

# Set deterministic mode (optional, improves reproducibility)
export E2E_DETERMINISTIC_TRIPLE_RUN=1

# Start test environment (brings up Docker Compose, sets up RPC)
./tests/e2e/start_test_environment.sh up

# In another terminal, run tests
cargo test --test state_root_replay -- --nocapture

# Collect logs and state artifacts
mkdir -p artifacts/logs
docker compose logs --no-color > artifacts/logs/docker-compose.log

# Stop environment
./tests/e2e/start_test_environment.sh down
```

### Triple-Run Determinism Check (Local Simulation)

Replicate the CI triple-run locally to verify determinism before pushing:

```bash
#!/bin/bash
set -euo pipefail

# Run three iterations with deterministic config
for i in 1 2 3; do
  echo "=== Run $i of 3 ==="
  export DOCKER_COMPOSE_PROJECT="e2e-local-run$i"
  export E2E_DETERMINISTIC_TRIPLE_RUN=1
  
  # Start environment (loads deterministic config)
  ./tests/e2e/start_test_environment.sh up
  
  # Wait for RPC readiness (adaptive polling, no sleep)
  ./tests/e2e/wait_for_rpc.sh "http://127.0.0.1:9933/" system_health 300 || exit 1
  
  # Run test
  mkdir -p "artifacts/run-$i/logs"
  cargo test --test state_root_replay -- --nocapture 2>&1 | tee "artifacts/run-$i/test.log"
  
  # Collect artifacts
  docker compose -p "$DOCKER_COMPOSE_PROJECT" logs --no-color > "artifacts/run-$i/logs/docker.log"
  
  # Cleanup
  ./tests/e2e/start_test_environment.sh down
  
  echo "Run $i complete"
done

# Compare hashes
./tests/e2e/compare_artifacts.sh artifacts/run-1 artifacts/run-2 artifacts/run-3
```

**Expected output:**
```
[compare] Artifacts identical across runs
```

If mismatch occurs:
```
[compare] Mismatch detected — writing diff report to artifacts/compare_report.json
```

## Configuration

### Deterministic Mode (`E2E_DETERMINISTIC_TRIPLE_RUN=1`)

When enabled, `start_test_environment.sh` loads `tests/e2e/fixtures/deterministic_config.toml`:

```toml
[determinism]
seed = "x3-e2e-deterministic-seed-001"
genesis_timestamp = 1707388800  # Feb 8 2025 00:00:00 UTC (fixed)
block_time_millis = 6000
initial_validators = ["5GrwvaEF5zXb26Fz9rcQkQKgVCqvEHVYTMbVqzXHR9WsFPbK"]
```

This ensures:
- All random sequences are reproducible (fixed seed)
- Block times don't drift (6 seconds exactly)
- Genesis state is identical across runs

**When to use:**
- Running tests that must be reproducible (CI before merge)
- Debugging timing-sensitive failures
- Verifying determinism locally before pushing

**When not to use:**
- Performance profiling (locks block time to 6s, may not match production)
- Stress testing (fixed seed limits randomness coverage)

### Environment Variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `E2E_DETERMINISTIC_TRIPLE_RUN` | unset (off) | Enable deterministic config loading |
| `DOCKER_COMPOSE_PROJECT` | `e2e-local` | Docker Compose project namespace (prevents conflicts) |
| `DOCKER_COMPOSE_FILE` | `docker-compose.test.yml` | Test environment definition |
| `X3_E2E_DETERMINISTIC_SEED` | auto-populated | RNG seed (exported from fixture) |
| `X3_E2E_GENESIS_TIMESTAMP` | auto-populated | Block timestamp (exported from fixture) |
| `X3_E2E_BLOCK_TIME_MILLIS` | auto-populated | Block interval in ms (exported from fixture) |
| `RUN_ID` | uuid | UUID for log correlation |
| `LOG_LEVEL` | `info` | Log verbosity (info, debug, trace) |

## RPC Readiness Polling

### Shell Wrapper: `wait_for_rpc.sh`

Replaces sleep-based waits with intelligent exponential backoff:

```bash
./tests/e2e/wait_for_rpc.sh <endpoint> <rpc_method> <timeout_seconds>
```

**Example:**
```bash
# Wait for system_health RPC to respond (default healthy after 5 attempts)
./tests/e2e/wait_for_rpc.sh "http://127.0.0.1:9933/" system_health 300

# Exit codes:
#   0 = Success (RPC responded and predicate passed)
#   1 = Timeout (RPC never healthy within 300 seconds)
#   2 = Argument error
```

**Backoff behavior:**
- Attempt 1: 500ms
- Attempt 2: 750ms (500 × 1.5)
- Attempt 3: 1.1s (750 × 1.5 + jitter)
- Attempt 4: 1.7s
- Attempt 5: 2.5s
- ...scales to max 8s between retries

**Typical result:** 3–5 attempts over 1–3 seconds to reach healthy node.

### Rust Helper: `wait_for_rpc_health()`

For Rust tests, use the native async helper:

```rust
use e2e_tests::wait_for_rpc::{wait_for_rpc_health, RetryPolicy};
use reqwest::Client;
use std::time::Duration;

#[tokio::test]
async fn test_with_rpc_readiness() {
    let client = Client::new();
    let endpoint = "http://127.0.0.1:9933/";
    
    // Custom retry policy
    let retry = RetryPolicy {
        initial_backoff: Duration::from_millis(500),
        max_backoff: Duration::from_secs(8),
        backoff_multiplier: 1.5,
        max_elapsed: Duration::from_secs(300),
    };
    
    // Wait until RPC responds with isSyncing=false
    wait_for_rpc_health(
        endpoint,
        "system_health",
        |v| v.get("result")
            .and_then(|r| r.get("isSyncing"))
            .map(|b| !b.as_bool().unwrap_or(true))
            .unwrap_or(false),
        &client,
        retry,
    )
    .await
    .expect("RPC never became healthy");
}
```

## Artifact Collection & Comparison

### What Gets Compared

The `compare_artifacts.sh` script checks these canonical outputs across runs:

```
artifacts/run-{1,2,3}/
├── logs/                          # Full docker-compose logs
├── state-root.json                # Final blockchain state hash
├── rpc-responses.json             # Timestamped RPC call results
├── docker-ps.log                  # Container state snapshot
└── dmesg.log                       # Kernel messages (for timing info)
```

**Hash comparison logic:**
1. Compute SHA256 of `logs/` directory (tar-sorted, deterministic)
2. Compute SHA256 of `state-root.json` file
3. Compute SHA256 of `rpc-responses.json` file
4. Pairwise compare: if all three runs hash identically, test passes
5. If mismatch: write report to `artifacts/compare_report/report.json`

### Artifact Sanitization

Before uploading artifacts to CI, `sanitize_artifacts_v2.sh` removes secrets:

```bash
./scripts/ci/sanitize_artifacts_v2.sh artifacts/run-1
```

**Redaction patterns:**
- Hex strings (addresses, keys): `0x[0-9a-fA-F]{20,}` → `0x[REDACTED]`
- 64-char keys: `[0-9a-fA-F]{64}` → `[REDACTED_KEY]`
- Bearer tokens: `Bearer [A-Za-z0-9._-]{10,}` → `[REDACTED_TOKEN]`
- Secret lines: `.*\b(PRIVATE|SECRET|PASSWORD|API_KEY)\b.*` → `[REDACTED_LINE]`

**File deletion:**
- `*.pem`, `.env`, `*.key`, `.secret`, `*credentials*`

**Dry-run mode (safe to test):**
```bash
./scripts/ci/sanitize_artifacts_v2.sh artifacts/run-1 --dry-run 1
# Reports matches without modifying files
```

## CI Integration

### GitHub Actions Workflow

The `.github/workflows/e2e-state-root.yml` workflow:

1. **Build job:**
   - Checks out code
   - Sets up Rust toolchain
   - Caches cargo registry/git

2. **Test matrix (3 parallel runs):**
   - Each run gets unique `DOCKER_COMPOSE_PROJECT` to prevent conflicts
   - Loads deterministic config via `E2E_DETERMINISTIC_TRIPLE_RUN=1`
   - Starts environment, waits for RPC, runs test
   - Collects `artifacts/run-{1,2,3}/` with logs and state

3. **Comparison job (depends on all 3 runs):**
   - Downloads all three run artifacts
   - Runs `compare_artifacts.sh run-1 run-2 run-3`
   - Fails if hashes don't match
   - Saves diff report for triage

4. **Trybuild job (independent):**
   - Tests proc-macro validator with `INVARIANT_REGISTRY_STRICT=1`
   - Validates format and registry checks

### Running Locally like CI

Create a test branch and push to trigger the workflow:

```bash
git checkout -b ci/e2e-triple-run-validation
git push origin ci/e2e-triple-run-validation
```

Then visit: https://github.com/Cyptopimpinainteazy/x3-chain/actions

Monitor the workflow:
- **e2e-state-root (run 1):** Builds and tests (⏱️ ~30 min)
- **e2e-state-root (run 2):** Parallel test (⏱️ ~30 min)
- **e2e-state-root (run 3):** Parallel test (⏱️ ~30 min)
- **compare-artifacts:** Validates hash equality (⏱️ ~2 min)
- **proc-macro-trybuild:** Format & registry validation (⏱️ ~5 min)

**Success criteria:**
- All three runs complete without timeouts
- `compare-artifacts` job passes (no hash mismatches)
- `proc-macro-trybuild` job passes

**If mismatch detected:**
1. Download artifact `e2e-state-root-artifacts-run-{1,2,3}`
2. Compare diff report at `artifacts/compare_report/report.json`
3. Investigate timing-sensitive code paths

## Troubleshooting

### "RPC never became healthy" Error

**Symptom:** `wait_for_rpc_health` timeout after 5 minutes.

**Causes:**
- Node didn't start (check `docker ps`, docker-compose logs)
- RPC port blocked (default 9933, check `netstat -tuln | grep 9933`)
- Node crashed early (check docker logs for panics)

**Fix:**
```bash
# Check if node is running
docker ps | grep x3-chain-node

# View logs
docker compose logs x3-chain-node | tail -100

# Try manual RPC call
curl -s http://127.0.0.1:9933/ \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}' | jq .

# If curl hangs, port is not listening — restart environment
./tests/e2e/start_test_environment.sh down
./tests/e2e/start_test_environment.sh up
```

### Hash Mismatch on Triple-Run

**Symptom:** `compare_artifacts.sh` reports hashes don't match across runs.

**Likely causes:**
- Timing-dependent code path (test must be more deterministic)
- Random seed not being applied (check `X3_E2E_DETERMINISTIC_SEED` is exported)
- Async task ordering (use fixed seed in test setup)
- Hardware differences (GPU backend, CPU throttling during CI)

**Investigation:**
1. Download diff report: `artifacts/compare_report/report.json`
2. Compare logs pairwise: `diff -u run-1/logs/test.log run-2/logs/test.log`
3. Look for timestamps, random values, or ordering changes
4. Add logging to pinpoint divergence point

### Docker Compose Port Conflicts

**Symptom:** `bind: address already in use` for port 9933 or 5432.

**Fix:**
```bash
# Ensure unique project names per run
export DOCKER_COMPOSE_PROJECT="e2e-$(uuidgen)-run1"

# Or stop all existing e2e containers
docker compose -p e2e-local down -v && docker ps | grep e2e && docker rm -f $(docker ps | grep e2e | awk '{print $1}')

# Restart clean environment
./tests/e2e/start_test_environment.sh up
```

## Performance Baselines

Expected test runtime (deterministic mode):

| Phase | Time | Notes |
|-------|------|-------|
| Docker Compose up | 15–30s | Pull images, start containers |
| RPC readiness poll | 1–3s | Node initializes and responds |
| Test execution | 5–10m | Depends on test complexity |
| Artifact collection | 10–30s | tar, sha256, copy logs |
| **Total per run** | **6–11m** | Parallel 3 runs: ~10m wall-clock |

**Optimization tips:**
- Pre-build Docker images in CI cache to skip pull
- Use `wait_for_rpc.sh` instead of hardcoded sleeps (saves 1–2 min per run)
- Keep test dataset small (state-root JSON < 10MB)
- Use compression for artifact uploads

## See Also

- **ADR 0002:** `docs/adr/0002-e2e-determinism-triple-run.md`
- **Invariant Registry:** `tests/invariants/registry.toml`
- **Proc-Macro Validator:** `crates/invariant-macros/src/lib.rs`
- **CI Workflow:** `.github/workflows/e2e-state-root.yml`
