#!/usr/bin/env bash
set -euo pipefail

INTERVAL_SEC="${INTERVAL_SEC:-60}"
RUN_LOAD="${RUN_LOAD:-0}"
RUN_MULTIPROCESS="${RUN_MULTIPROCESS:-0}"
LOAD_DURATION_SEC="${LOAD_DURATION_SEC:-300}"
MIN_FINALIZED_TPS="${MIN_FINALIZED_TPS:-0}"
MAX_ERROR_RATE="${MAX_ERROR_RATE:-0.01}"
CHECK_TELEMETRY="${CHECK_TELEMETRY:-0}"
TESTNET_CONFIG="${TESTNET_CONFIG:-docs/testnet-config/testnet-config.json}"
LOG_FILE="${LOG_FILE:-logs/testnet-verify.log}"
STATUS_FILE="${STATUS_FILE:-logs/testnet-verify-status.json}"
LOG_MAX_BYTES="${LOG_MAX_BYTES:-10485760}"
LOG_BACKUPS="${LOG_BACKUPS:-5}"
START_LOCAL_VALIDATORS="${START_LOCAL_VALIDATORS:-0}"
LOCAL_BASE_DIR="${LOCAL_BASE_DIR:-/tmp/x3-testnet-ci}"
LOCAL_LOG_DIR="${LOCAL_LOG_DIR:-/tmp/x3-testnet-ci-logs}"
LOCAL_COUNT="${LOCAL_COUNT:-7}"
LOCAL_READY_TIMEOUT_SEC="${LOCAL_READY_TIMEOUT_SEC:-120}"

usage() {
  cat <<EOF
Usage: $(basename "$0") [--interval-sec N]

Options:
  --interval-sec N  Seconds between verification runs (default: ${INTERVAL_SEC})
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --interval-sec)
      INTERVAL_SEC="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1"
      usage
      exit 2
      ;;
  esac
done

rotate_log() {
  if [[ ! -f "$LOG_FILE" ]]; then
    return 0
  fi
  local size
  size="$(wc -c < "$LOG_FILE" | tr -d ' ')"
  if [[ "$size" -lt "$LOG_MAX_BYTES" ]]; then
    return 0
  fi

  local idx
  for ((idx=LOG_BACKUPS-1; idx>=1; idx--)); do
    if [[ -f "${LOG_FILE}.${idx}" ]]; then
      mv "${LOG_FILE}.${idx}" "${LOG_FILE}.$((idx+1))"
    fi
  done
  mv "$LOG_FILE" "${LOG_FILE}.1"
  : > "$LOG_FILE"
}

write_status() {
  local status="$1"
  local timestamp="$2"
  printf '{\"timestamp_utc\":\"%s\",\"status\":\"%s\"}\n' "$timestamp" "$status" > "$STATUS_FILE"
}

ensure_local_validators() {
  if [[ "$START_LOCAL_VALIDATORS" != "1" ]]; then
    return 0
  fi

  if [[ ! -x "scripts/testnet/run-7-validators-local.sh" ]]; then
    echo "Local validator launcher is not executable. Fixing permissions..." | tee -a "$LOG_FILE"
    chmod +x scripts/testnet/run-7-validators-local.sh || true
  fi

  if ! command -v subkey >/dev/null 2>&1; then
    echo "subkey not found in PATH. Skipping local validator start." | tee -a "$LOG_FILE"
    return 0
  fi

  local status_output
  status_output="$(BASE_RPC_PORT=9944 COUNT="$LOCAL_COUNT" scripts/testnet/status-7-validators.sh 2>/dev/null || true)"
  if echo "$status_output" | grep -q " DOWN "; then
    echo "Local validators not healthy. Restarting local testnet..." | tee -a "$LOG_FILE"
  else
    return 0
  fi

  BASE_DIR="$LOCAL_BASE_DIR" LOG_DIR="$LOCAL_LOG_DIR" COUNT="$LOCAL_COUNT" \
    scripts/testnet/run-7-validators-local.sh --wipe --base-dir "$LOCAL_BASE_DIR" --log-dir "$LOCAL_LOG_DIR" >> "$LOG_FILE" 2>&1

  if ! BASE_RPC_PORT=9944 COUNT="$LOCAL_COUNT" PEER_TIMEOUT_SEC="$LOCAL_READY_TIMEOUT_SEC" \
    scripts/testnet/status-7-validators.sh --check-peers >> "$LOG_FILE" 2>&1; then
    echo "Local validators failed to reach peer threshold within ${LOCAL_READY_TIMEOUT_SEC}s." | tee -a "$LOG_FILE"
  fi
}

mkdir -p "$(dirname "$LOG_FILE")"
mkdir -p "$(dirname "$STATUS_FILE")"

echo "Starting continuous testnet verification loop (interval=${INTERVAL_SEC}s)" | tee -a "$LOG_FILE"
while true; do
  timestamp="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  rotate_log
  ensure_local_validators
  echo "[${timestamp}] Running testnet verification..." | tee -a "$LOG_FILE"
  RUN_LOAD="$RUN_LOAD" RUN_MULTIPROCESS="$RUN_MULTIPROCESS" \
    LOAD_DURATION_SEC="$LOAD_DURATION_SEC" MIN_FINALIZED_TPS="$MIN_FINALIZED_TPS" \
    MAX_ERROR_RATE="$MAX_ERROR_RATE" CHECK_TELEMETRY="$CHECK_TELEMETRY" \
    TESTNET_CONFIG="$TESTNET_CONFIG" \
    make testnet-verify >> "$LOG_FILE" 2>&1
  exit_code=$?
  if [[ "$exit_code" -eq 0 ]]; then
    write_status "ok" "$timestamp"
    echo "[${timestamp}] Verification complete. Sleeping ${INTERVAL_SEC}s." | tee -a "$LOG_FILE"
  else
    write_status "failed" "$timestamp"
    echo "[${timestamp}] Verification failed (exit=${exit_code}). Sleeping ${INTERVAL_SEC}s." | tee -a "$LOG_FILE"
  fi
  sleep "$INTERVAL_SEC"
done
