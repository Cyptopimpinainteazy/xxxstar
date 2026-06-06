#!/usr/bin/env bash
set -euo pipefail

TESTNET_CONFIG="${TESTNET_CONFIG:-docs/testnet-config/testnet-config.json}"
BASE_RPC_PORT="${BASE_RPC_PORT:-9944}"
COUNT="${COUNT:-7}"
MIN_PEERS="${MIN_PEERS-}"
PEER_TIMEOUT_SEC="${PEER_TIMEOUT_SEC-}"
FINALITY_WINDOW_SEC="${FINALITY_WINDOW_SEC-}"
MAX_FINALITY_LAG_MULT="${MAX_FINALITY_LAG_MULT-}"
CHECK_TELEMETRY="${CHECK_TELEMETRY-}"
PROM_HOST="${PROM_HOST-}"
PROM_PORT="${PROM_PORT-}"
GRAFANA_HOST="${GRAFANA_HOST-}"
GRAFANA_PORT="${GRAFANA_PORT-}"
RUN_LOAD="${RUN_LOAD-}"
RUN_MULTIPROCESS="${RUN_MULTIPROCESS-}"
LOAD_DURATION_SEC="${LOAD_DURATION_SEC-}"
MIN_FINALIZED_TPS="${MIN_FINALIZED_TPS-}"
MAX_ERROR_RATE="${MAX_ERROR_RATE-}"

config_get() {
  local config_json="$1"
  local key="$2"
  node -e '
const cfg = JSON.parse(process.argv[1]);
const key = process.argv[2];
const value = cfg[key];
if (value === undefined || value === null) {
  process.exit(0);
}
if (typeof value === "boolean") {
  process.stdout.write(value ? "1" : "0");
} else {
  process.stdout.write(String(value));
}
' "$config_json" "$key"
}

load_config() {
  if [[ ! -f "$TESTNET_CONFIG" ]]; then
    return 0
  fi

  local config_json
  config_json="$(node -e '
const fs = require("fs");
const path = process.argv[1];
try {
  const raw = JSON.parse(fs.readFileSync(path, "utf8"));
  const verification = raw.verification || {};
  process.stdout.write(JSON.stringify(verification));
} catch (err) {
  console.error(err.message);
  process.exit(1);
}
' "$TESTNET_CONFIG")"

  [[ -n "$MIN_PEERS" ]] || MIN_PEERS="$(config_get "$config_json" "min_peers")"
  [[ -n "$PEER_TIMEOUT_SEC" ]] || PEER_TIMEOUT_SEC="$(config_get "$config_json" "peer_timeout_sec")"
  [[ -n "$FINALITY_WINDOW_SEC" ]] || FINALITY_WINDOW_SEC="$(config_get "$config_json" "finality_window_sec")"
  [[ -n "$MAX_FINALITY_LAG_MULT" ]] || MAX_FINALITY_LAG_MULT="$(config_get "$config_json" "max_finality_lag_mult")"
  [[ -n "$CHECK_TELEMETRY" ]] || CHECK_TELEMETRY="$(config_get "$config_json" "check_telemetry")"
  [[ -n "$PROM_HOST" ]] || PROM_HOST="$(config_get "$config_json" "prom_host")"
  [[ -n "$PROM_PORT" ]] || PROM_PORT="$(config_get "$config_json" "prom_port")"
  [[ -n "$GRAFANA_HOST" ]] || GRAFANA_HOST="$(config_get "$config_json" "grafana_host")"
  [[ -n "$GRAFANA_PORT" ]] || GRAFANA_PORT="$(config_get "$config_json" "grafana_port")"
  [[ -n "$RUN_LOAD" ]] || RUN_LOAD="$(config_get "$config_json" "run_load")"
  [[ -n "$RUN_MULTIPROCESS" ]] || RUN_MULTIPROCESS="$(config_get "$config_json" "run_multiprocess")"
  [[ -n "$LOAD_DURATION_SEC" ]] || LOAD_DURATION_SEC="$(config_get "$config_json" "load_duration_sec")"
  [[ -n "$MIN_FINALIZED_TPS" ]] || MIN_FINALIZED_TPS="$(config_get "$config_json" "min_finalized_tps")"
  [[ -n "$MAX_ERROR_RATE" ]] || MAX_ERROR_RATE="$(config_get "$config_json" "max_error_rate")"
}

usage() {
  cat <<EOF_USAGE
Usage: $(basename "$0") [--check-telemetry] [--run-load] [--run-multiprocess]

Options:
  --check-telemetry   Check Prometheus and Grafana health endpoints.
  --run-load          Run load test with baseline assertions.
  --run-multiprocess  Run multiprocess load test (implies --run-load).
Environment:
  TESTNET_CONFIG      JSON file with optional verification settings.
EOF_USAGE
}

load_config

MIN_PEERS="${MIN_PEERS:-3}"
PEER_TIMEOUT_SEC="${PEER_TIMEOUT_SEC:-120}"
FINALITY_WINDOW_SEC="${FINALITY_WINDOW_SEC:-12}"
MAX_FINALITY_LAG_MULT="${MAX_FINALITY_LAG_MULT:-2}"
CHECK_TELEMETRY="${CHECK_TELEMETRY:-0}"
PROM_HOST="${PROM_HOST:-127.0.0.1}"
PROM_PORT="${PROM_PORT:-9090}"
GRAFANA_HOST="${GRAFANA_HOST:-127.0.0.1}"
GRAFANA_PORT="${GRAFANA_PORT:-3001}"
RUN_LOAD="${RUN_LOAD:-0}"
RUN_MULTIPROCESS="${RUN_MULTIPROCESS:-0}"
LOAD_DURATION_SEC="${LOAD_DURATION_SEC:-1200}"
MIN_FINALIZED_TPS="${MIN_FINALIZED_TPS:-0}"
MAX_ERROR_RATE="${MAX_ERROR_RATE:-0.01}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --check-telemetry)
      CHECK_TELEMETRY=1
      shift
      ;;
    --run-load)
      RUN_LOAD=1
      shift
      ;;
    --run-multiprocess)
      RUN_LOAD=1
      RUN_MULTIPROCESS=1
      shift
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

echo "[verify] Checking peers and finality..."
BASE_RPC_PORT="$BASE_RPC_PORT" COUNT="$COUNT" MIN_PEERS="$MIN_PEERS" \
  PEER_TIMEOUT_SEC="$PEER_TIMEOUT_SEC" FINALITY_WINDOW_SEC="$FINALITY_WINDOW_SEC" \
  MAX_FINALITY_LAG_MULT="$MAX_FINALITY_LAG_MULT" \
  scripts/testnet/status-7-validators.sh --check-peers --check-finality

echo "[verify] Peer/finality checks passed."

if [[ "$CHECK_TELEMETRY" == "1" ]]; then
  echo "[verify] Checking telemetry endpoints..."
  if ! curl -s "http://${PROM_HOST}:${PROM_PORT}/-/healthy" >/dev/null 2>&1; then
    echo "Prometheus health check failed at http://${PROM_HOST}:${PROM_PORT}/-/healthy"
    exit 1
  fi
  if ! curl -s "http://${GRAFANA_HOST}:${GRAFANA_PORT}/api/health" >/dev/null 2>&1; then
    echo "Grafana health check failed at http://${GRAFANA_HOST}:${GRAFANA_PORT}/api/health"
    exit 1
  fi
  echo "[verify] Telemetry checks passed."
fi

if [[ "$RUN_LOAD" == "1" ]]; then
  echo "[verify] Running load test..."
  if [[ "$RUN_MULTIPROCESS" == "1" ]]; then
    python3 scripts/testnet/run-multiprocess-load.py \
      --duration-sec "$LOAD_DURATION_SEC" \
      --require-baseline \
      --min-duration-sec "$LOAD_DURATION_SEC" \
      --min-finalized-tps "$MIN_FINALIZED_TPS" \
      --max-error-rate "$MAX_ERROR_RATE"
  else
    DURATION_SEC="$LOAD_DURATION_SEC" REQUIRE_BASELINE=1 MIN_DURATION_SEC="$LOAD_DURATION_SEC" \
      MIN_FINALIZED_TPS="$MIN_FINALIZED_TPS" MAX_ERROR_RATE="$MAX_ERROR_RATE" \
      node scripts/testnet/load-remarks-tps.js
  fi
  echo "[verify] Load test passed."
fi
