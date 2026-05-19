#!/usr/bin/env bash
set -euo pipefail

BASE_DIR="${BASE_DIR:-/tmp/x3-testnet-local}"

if [[ -f "$BASE_DIR/chainbench-server.pid" ]]; then
  kill "$(cat "$BASE_DIR/chainbench-server.pid")" 2>/dev/null || true
  rm -f "$BASE_DIR/chainbench-server.pid"
fi

pkill -f "scripts/testnet/chainbench_server.py" || true
pkill -f "x3-chain-node" || true

echo "Chainbench stack stopped"
