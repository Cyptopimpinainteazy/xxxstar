#!/usr/bin/env bash
set -euo pipefail

# X3 Chain E2E CI Test State-Root Replay Wrapper
# Standardized test execution w/ structured logging, exit codes, and artifact collection
#
# Exit Codes:
#   0 = SUCCESS
#   1 = TEST_FAILURE (test ran but assertions failed)
#   2 = INFRASTRUCTURE_ERROR (env, docker, RPC, timeout)
#   3 = CONFIGURATION_ERROR (invalid args or missing config)

run_id="${RUN_ID:-$(uuidgen 2>/dev/null || echo local)}"
start_ts="$(date -u +%s)"
test_mode="${TEST_MODE:-single}"  # single or triple
log_level="${LOG_LEVEL:-info}"     # info, debug, trace
artifact_dir="${ARTIFACT_DIR:-artifacts}"

mkdir -p "$artifact_dir/logs"

# Structured logging: JSON lines with timestamp, level, component, message
log_json() {
    local level=$1 component=$2 msg=$3
    local ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf '{"ts":"%s","run_id":"%s","level":"%s","component":"%s","msg":"%s"}\n' "$ts" "$run_id" "$level" "$component" "$msg" | tee -a "$artifact_dir/logs/trace.jsonl"
}

log_info() { log_json "INFO" "wrapper" "$1"; }
log_error() { log_json "ERROR" "wrapper" "$1"; }
log_debug() { [ "$log_level" = "debug" ] && log_json "DEBUG" "wrapper" "$1" || true; }

exit_with_status() {
    local code=$1 msg=$2
    log_json "STATUS" "wrapper" "exit_code=$code: $msg"
    exit "$code"
}

log_info "E2E state-root replay test wrapper started (run_id=$run_id, mode=$test_mode)"

# Determine test invocation based on mode
if [ "$test_mode" = "triple" ]; then
    log_info "Triple-run mode: executing 3 deterministic runs in sequence"
    for i in 1 2 3; do
        log_info "Starting run $i of 3..."
        export RUN_NUM=$i
        export DOCKER_COMPOSE_PROJECT="e2e-${run_id}-run${i}"
        export E2E_DETERMINISTIC_TRIPLE_RUN=1
        
        if ! cargo test --test state_root_replay -- --nocapture 2>&1 | tee -a "$artifact_dir/logs/run-$i.log"; then
            exit_with_status 1 "State-root replay test FAILED on run $i"
        fi
        log_info "Run $i completed successfully"
    done
    log_info "All three runs completed successfully"
else
    log_info "Single-run mode: executing one E2E test iteration"
    export RUN_NUM=1
    export DOCKER_COMPOSE_PROJECT="e2e-${run_id}-single"
    export E2E_DETERMINISTIC_TRIPLE_RUN=1
    
    if ! cargo test --test state_root_replay -- --nocapture 2>&1 | tee -a "$artifact_dir/logs/test.log"; then
        exit_with_status 1 "State-root replay test FAILED"
    fi
fi

# Collect diagnostics
log_info "Collecting diagnostic artifacts..."
docker compose -p "${DOCKER_COMPOSE_PROJECT:-e2e-local}" logs --no-color > "$artifact_dir/logs/docker.log" 2>/dev/null || true
docker ps -a > "$artifact_dir/logs/docker-ps.txt" 2>/dev/null || true

# Sanitize artifacts before exit
log_info "Sanitizing artifacts..."
chmod +x scripts/ci/sanitize_artifacts.sh
if scripts/ci/sanitize_artifacts.sh "$artifact_dir" --dry-run 2>&1 | tee -a "$artifact_dir/logs/sanitizer.log"; then
    scripts/ci/sanitize_artifacts.sh "$artifact_dir" || log_error "Artifact sanitization failed (non-fatal)"
else
    log_error "Sanitizer dry-run failed"
fi

end_ts="$(date -u +%s)"
elapsed=$((end_ts - start_ts))
log_info "Wrapper completed successfully (elapsed=${elapsed}s)"
log_json "METRICS" "wrapper" "duration_sec=$elapsed test_mode=$test_mode run_id=$run_id"

exit_with_status 0 "All tests passed"
