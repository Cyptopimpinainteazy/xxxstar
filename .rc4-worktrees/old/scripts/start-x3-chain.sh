#!/bin/bash
# Start the real X3 Chain node (development mode).
# Falls back to the mock RPC server ONLY when the binary is missing,
# and prints a loud warning so developers are never silently on mock.
#
# For production / testnet: NEVER fall back — call this script directly.
# For a mock-only dev session: use scripts/start-mock-rpc-dev.sh

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BINARY="${X3_NODE_BIN:-$PROJECT_ROOT/target/release/x3-chain-node}"
LOG_DIR="${LOG_DIR:-/tmp/x3-chain-logs}"
mkdir -p "$LOG_DIR"

# ── Flags (overridable by environment) ─────────────────────────────────────
CHAIN="${CHAIN:-dev}"
RPC_PORT="${RPC_PORT:-9933}"
WS_PORT="${WS_PORT:-9944}"
METRICS_PORT="${METRICS_PORT:-9616}"
ALLOW_MOCK="${ALLOW_MOCK:-false}"

echo "========================================================="
echo "  X3 Chain Node Launcher"
echo "  Binary : $BINARY"
echo "  Chain  : $CHAIN"
echo "  RPC    : http://localhost:$RPC_PORT"
echo "  WS     : ws://localhost:$WS_PORT"
echo "  Metrics: http://localhost:$METRICS_PORT/metrics"
echo "========================================================="
echo ""

if [[ -f "$BINARY" ]]; then
    echo "✅ Real x3-chain-node binary found — launching."
    exec "$BINARY" \
        --chain="$CHAIN" \
        --rpc-port="$RPC_PORT" \
        --ws-port="$WS_PORT" \
        --prometheus-port="$METRICS_PORT" \
        --prometheus-external \
        --rpc-cors=all \
        --tmp \
        2>&1 | tee "$LOG_DIR/x3-chain-node.log"
else
    if [[ "$ALLOW_MOCK" != "true" ]]; then
        echo ""
        echo "❌ FATAL: x3-chain-node binary NOT found at:"
        echo "   $BINARY"
        echo ""
        echo "   Build it first:"
        echo "     cargo build --release -p x3-chain-node"
        echo ""
        echo "   Or, for dev-only mock (NOT for testnet/mainnet):"
        echo "     ALLOW_MOCK=true ./scripts/start-x3-chain.sh"
        echo "   Or directly:"
        echo "     ./scripts/start-mock-rpc-dev.sh"
        echo ""
        exit 1
    fi

    echo ""
    echo "⚠️  ⚠️  ⚠️  ⚠️  ⚠️  ⚠️  ⚠️  ⚠️  ⚠️  ⚠️  ⚠️  ⚠️  ⚠️  ⚠️  ⚠️"
    echo "  WARNING: Real binary not found — starting MOCK RPC server."
    echo "  This is NOT a real blockchain. No consensus. No finality."
    echo "  NEVER use this for testnet, staging, or mainnet."
    echo "⚠️  ⚠️  ⚠️  ⚠️  ⚠️  ⚠️  ⚠️  ⚠️  ⚠️  ⚠️  ⚠️  ⚠️  ⚠️  ⚠️  ⚠️"
    echo ""

    exec node "$PROJECT_ROOT/scripts/mock-rpc-server.js" \
        2>&1 | tee "$LOG_DIR/mock-rpc-server.log"
fi
