#!/usr/bin/env bash
set -euo pipefail

RUN_ID=${1:-unknown}
LOG_DIR=${2:-artifacts/logs}
mkdir -p "$LOG_DIR"

# Structured logging helper
log_json() {
  local level=$1
  local msg=$2
  local ts=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
  printf '{"ts":"%s","run_id":"%s","level":"%s","msg":"%s"}\n' "$ts" "$RUN_ID" "$level" "$msg" | tee -a "$LOG_DIR/structured.log"
}

log_json "INFO" "Starting CHAIN-CONSENSUS-001 state-root replay test for run $RUN_ID"

# Run the test with error handling
if cargo test --test state_root_replay -- --nocapture 2>&1 | tee "$LOG_DIR/state_root_replay.log"; then
  log_json "SUCCESS" "Test passed for run $RUN_ID"
  exit 0
else
  log_json "ERROR" "Test failed for run $RUN_ID"
  exit 1
fi
