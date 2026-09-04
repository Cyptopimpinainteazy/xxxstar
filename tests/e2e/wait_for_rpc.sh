#!/usr/bin/env bash
set -euo pipefail

 usage() {
  cat <<EOF
Usage: wait_for_rpc.sh <URL> <RPC_METHOD> [--timeout seconds] [--interval seconds]

Example:
  wait_for_rpc.sh http://127.0.0.1:9933 system_health --timeout 300 --interval 2
EOF
}

URL=${1:-}
METHOD=${2:-}
TIMEOUT=${3:-}
INTERVAL=${4:-}

if [ -z "$URL" ] || [ -z "$METHOD" ]; then
  usage
  exit 2
fi

# defaults
TIMEOUT_SECONDS=300
INTERVAL_SECONDS=2

if [ -n "$TIMEOUT" ]; then
  TIMEOUT_SECONDS=$TIMEOUT
fi
if [ -n "$INTERVAL" ]; then
  INTERVAL_SECONDS=$INTERVAL
fi

START=$(date +%s)

log_info() { printf "[INFO] %s\n" "$1"; }
log_error() { printf "[ERROR] %s\n" "$1"; }

log_info "Waiting for RPC $METHOD at $URL (timeout ${TIMEOUT_SECONDS}s)"

while true; do
  NOW=$(date +%s)
  ELAPSED=$((NOW - START))
  if [ $ELAPSED -ge $TIMEOUT_SECONDS ]; then
    log_error "Timeout waiting for RPC at $URL"
    exit 1
  fi

  # Build JSON-RPC request
  body=$(jq -nc --arg m "$METHOD" '{jsonrpc: "2.0", id: 1, method: $m, params: []}')

  if resp=$(curl -sS -X POST -H "Content-Type: application/json" -d "$body" "$URL" 2>/dev/null || true); then
    ok=$(echo "$resp" | jq 'has("result") or has("error")' || echo false)
    if [ "$ok" = "true" ]; then
      log_info "RPC $METHOD at $URL responded"
      exit 0
    fi
  fi

  sleep $INTERVAL_SECONDS
  # exponential backoff up to 8s
  if [ $INTERVAL_SECONDS -lt 8 ]; then
    INTERVAL_SECONDS=$((INTERVAL_SECONDS * 2))
    if [ $INTERVAL_SECONDS -gt 8 ]; then
      INTERVAL_SECONDS=8
    fi
  fi
done
