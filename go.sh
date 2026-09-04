#!/bin/bash
# X3 Chain — one-shot: kill, clean, launch, verify, test
set -euo pipefail
cd "$(dirname "$0")"

echo "=== [1/8] Killing stale nodes ==="
pkill -9 -f x3-chain-node 2>/dev/null || true
sleep 2

echo "=== [2/8] Cleaning temp dirs ==="
rm -rf /tmp/x3-chain-* /tmp/node*.log 2>/dev/null || true

echo "=== [3/8] Starting validator ==="
./target/release/x3-chain-node \
  --chain=dev --validator --tmp \
  --rpc-port=9933 --rpc-cors=all \
  --no-telemetry \
  > /tmp/x3-node.log 2>&1 &
NODE_PID=$!
echo "  Node PID: $NODE_PID"

echo "=== [4/8] Waiting for RPC ==="
for i in $(seq 1 30); do
  if curl -s --max-time 2 http://localhost:9933 \
    -H 'Content-Type: application/json' \
    -d '{"id":1,"jsonrpc":"2.0","method":"system_health","params":[]}' \
    > /dev/null 2>&1; then
    echo "  RPC ready after ${i}s"
    break
  fi
  if [ "$i" -eq 30 ]; then
    echo "  FAIL: RPC never came up"
    tail -30 /tmp/x3-node.log
    exit 1
  fi
  sleep 1
done

echo "=== [5/8] Checking block production ==="
for i in $(seq 1 30); do
  HEADER=$(curl -s --max-time 2 http://localhost:9933 \
    -H 'Content-Type: application/json' \
    -d '{"id":1,"jsonrpc":"2.0","method":"chain_getHeader","params":[]}')
  BLOCK_NUM=$(echo "$HEADER" | grep -o '"number":"0x[^"]*"' | cut -d'"' -f4)
  if [ -n "$BLOCK_NUM" ]; then
    BN=$((BLOCK_NUM))
    echo "  Block #: $BN (0x$BLOCK_NUM)"
    if [ "$BN" -gt 1 ]; then
      echo "  ✅ Block production confirmed"
      break
    fi
  fi
  if [ "$i" -eq 30 ]; then
    echo "  ⚠ Block height stuck at #$BN after 30s"
  fi
  sleep 2
done

echo "=== [6/8] System health ==="
curl -s http://localhost:9933 \
  -H 'Content-Type: application/json' \
  -d '{"id":1,"jsonrpc":"2.0","method":"system_health","params":[]}'

echo ""
echo "=== [7/8] Running pallet tests ==="
make test 2>&1 | tail -20

echo ""
echo "=== [8/8] Running all pallet tests ==="
make test-all-pallets 2>&1 | tail -20

echo ""
echo "========================================="
echo "  X3 Chain — RUNNING"
echo "  RPC:     http://localhost:9933"
echo "  Explorer: apps/explorer/x3-chain-explorer.html"
echo "  Logs:    tail -f /tmp/x3-node.log"
echo "========================================="