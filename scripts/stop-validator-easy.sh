#!/usr/bin/env bash
set -euo pipefail

LOG_DIR="${X3_LOG_DIR:-/tmp/x3-validator-logs}"
MODE="${1:-soft}"

echo "Stopping validator started by start-validator-easy.sh..."

case "$MODE" in
  --hard|-h|hard)
    pkill -9 -f "x3-chain-node.*--validator" 2>/dev/null || true
    ;;
  *)
    pkill -f "x3-chain-node.*--validator" 2>/dev/null || true
    sleep 1
    for i in {1..10}; do
      if pgrep -f "x3-chain-node.*--validator" >/dev/null 2>&1; then
        sleep 1
      else
        break
      fi
    done
    if pgrep -f "x3-chain-node.*--validator" >/dev/null 2>&1; then
      echo "Validators did not stop cleanly after 10s, killing forcibly..."
      pkill -9 -f "x3-chain-node.*--validator" 2>/dev/null || true
    fi
    ;;
esac

if pgrep -f "x3-chain-node.*--validator" >/dev/null 2>&1; then
  echo "ERROR: Some validator processes still remain"
  exit 1
fi

echo "Validator processes stopped."

echo "Logs remain in: $LOG_DIR"
