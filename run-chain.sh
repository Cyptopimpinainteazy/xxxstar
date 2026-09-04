#!/bin/bash
# X3 Chain Launcher — clean dev validator with fixed base path
set -euo pipefail

BIN="./target/release/x3-chain-node"
BASE="/tmp/x3-chain-dev"
RPC_PORT=9933

# Kill any existing x3-chain-node
pkill -9 -f x3-chain-node 2>/dev/null || true
sleep 1

# Clean stale base path to avoid NetworkKeyNotFound
rm -rf "$BASE"

echo "=== Starting X3 Chain Validator ==="
echo "  Binary: $BIN"
echo "  Base:   $BASE"
echo "  RPC:    http://localhost:$RPC_PORT"
echo "  Logs:   $BASE/node.log"
echo ""

mkdir -p "$BASE"

exec "$BIN" \
  --chain=dev \
  --validator \
  --base-path="$BASE" \
  --rpc-port="$RPC_PORT" \
  --rpc-cors=all \
  --no-telemetry \
  2>&1 | tee "$BASE/node.log"