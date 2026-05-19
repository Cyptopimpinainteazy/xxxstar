#!/bin/bash
# DEV-ONLY: Start the mock Substrate RPC server.
#
# This simulates the X3 Chain node on ports 9933 (HTTP) and 9944 (WebSocket)
# WITHOUT real consensus, real block production, or real state.
#
# ╔══════════════════════════════════════════════════════════════╗
# ║  ⚠️  MOCK SERVER — NOT SUITABLE FOR TESTNET OR MAINNET  ⚠️  ║
# ║                                                              ║
# ║  For real local dev chain:  ./scripts/start-x3-chain.sh     ║
# ║  For 3-validator testnet:   ./scripts/testnet-full-launch.sh ║
# ╚══════════════════════════════════════════════════════════════╝
#
# This file is intentionally named "start-mock-rpc-dev.sh" so that
# it is never confused with the production start script.

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOG_DIR="${LOG_DIR:-/tmp/x3-chain-logs}"
mkdir -p "$LOG_DIR"

echo "========================================================"
echo "  ⚠️  X3 MOCK RPC SERVER (DEV ONLY)"
echo "  HTTP RPC : http://localhost:9933"
echo "  WS RPC   : ws://localhost:9944"
echo "  Logs     : $LOG_DIR/mock-rpc-server.log"
echo "========================================================"
echo ""
echo "  This is NOT a real blockchain node."
echo "  It returns simulated responses for local UI/SDK testing."
echo "  It will NOT produce real blocks, consensus, or finality."
echo ""

exec node "$PROJECT_ROOT/scripts/mock-rpc-server.js" \
    2>&1 | tee "$LOG_DIR/mock-rpc-server.log"
