#!/bin/bash
#############################################################################
# scripts/stop-testnet.sh
#
# X3 Atomic Star — Testnet Stop Script.
#
# Canonical script to gracefully stop all x3-chain-node processes related
# to a testnet started by scripts/testnet-full-launch.sh.
#
# This is the inverse of the launch script. It:
#   1. Sends SIGTERM to all x3-chain-node processes
#   2. Waits up to 10 seconds for graceful shutdown
#   3. Force-kills (SIGKILL) any remaining processes
#   4. Cleans up temporary directories (optional)
#
# Usage:
#   ./scripts/stop-testnet.sh              — soft stop (keep data)
#   ./scripts/stop-testnet.sh --clean      — stop and remove data dirs
#   ./scripts/stop-testnet.sh --hard       — SIGKILL immediately
#
# Environment variables:
#   X3_BASE_PATH  — base data directory (default: /tmp/x3-testnet)
#   X3_LOG_DIR    — log directory (default: /tmp/x3-testnet-logs)
#   X3_KEEP_LOGS  — if set, do not remove log files
#############################################################################

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

BASE_PATH="${X3_BASE_PATH:-/tmp/x3-testnet}"
LOG_DIR="${X3_LOG_DIR:-/tmp/x3-testnet-logs}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${YELLOW}🛑 Stopping X3 testnet...${NC}"

MODE="${1:-soft}"

case "$MODE" in
    --hard|-h)
        echo "  Force-killing all x3-chain-node processes..."
        pkill -9 -f "x3-chain-node" 2>/dev/null || true
        ;;
    *)
        echo "  Sending SIGTERM to x3-chain-node processes..."
        pkill -f "x3-chain-node" 2>/dev/null || true

        # Wait for graceful shutdown
        WAIT=0
        while pgrep -f "x3-chain-node" >/dev/null 2>&1; do
            sleep 1
            WAIT=$((WAIT + 1))
            if [[ $WAIT -ge 10 ]]; then
                echo -e "  ${YELLOW}⚠️  Processes did not exit within 10s, sending SIGKILL...${NC}"
                pkill -9 -f "x3-chain-node" 2>/dev/null || true
                break
            fi
        done
        ;;
esac

# Verify all processes stopped
if pgrep -f "x3-chain-node" >/dev/null 2>&1; then
    echo -e "  ${RED}❌ Some x3-chain-node processes still running${NC}"
    exit 1
fi

echo -e "  ${GREEN}✅ All x3-chain-node processes stopped${NC}"

# Clean up data dirs if --clean flag
if [[ "$MODE" == "--clean" ]]; then
    echo "  Cleaning data directories..."
    rm -rf "$BASE_PATH" 2>/dev/null || echo "  (base path $BASE_PATH not found)"
    if [[ -z "${X3_KEEP_LOGS:-}" ]]; then
        rm -rf "$LOG_DIR" 2>/dev/null || echo "  (log dir $LOG_DIR not found)"
    fi
    echo -e "  ${GREEN}✅ Data directories cleaned${NC}"
fi

echo -e "${GREEN}✅ Testnet stopped successfully${NC}"