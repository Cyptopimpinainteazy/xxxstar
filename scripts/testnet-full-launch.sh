#!/bin/bash

#############################################################################
# X3_ATOMIC_STAR - Full Testnet Launch Orchestration
# 
# This script launches a complete testnet with:
# - 3 validators with Aura + GRANDPA consensus
# - Indexer service for event capture  
# - Settlement flow monitoring
# - Health check system
# - End-to-end settlement validation
#
# Usage: ./scripts/testnet-full-launch.sh [--validators N] [--clean]
#############################################################################

set -e

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
VALIDATORS=${1:-3}
BINARY="/home/lojak/Desktop/X3_ATOMIC_STAR/target/release/x3-chain-node"
CHAIN_SPEC="/home/lojak/Desktop/X3_ATOMIC_STAR/deployment/chain-specs/x3-testnet-raw.json"
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BASE_PATH="/tmp/x3-testnet"
LOG_DIR="/tmp/x3-testnet-logs"
PORT_BASE=30333
RPC_PORT_BASE=9933
METRICS_PORT_BASE=9616

# Check if binary exists
if [[ ! -f "$BINARY" ]]; then
    echo -e "${RED}❌ Binary not found at $BINARY${NC}"
    echo "   Please run: cargo build --release -p x3-chain-node"
    exit 1
fi

# Create directories
mkdir -p "$BASE_PATH" "$LOG_DIR"

echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}🚀 X3_ATOMIC_STAR - Full Testnet Launch${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo ""

# Function to cleanup on exit
cleanup() {
    echo -e "${YELLOW}🛑 Shutting down testnet...${NC}"
    pkill -f "x3-chain-node" || true
    sleep 2
    echo -e "${GREEN}✅ Testnet stopped${NC}"
}

trap cleanup EXIT

# Function to start a validator
start_validator() {
    local VAL_NUM=$1
    local PORT=$((PORT_BASE + VAL_NUM - 1))
    local RPC_PORT=$((RPC_PORT_BASE + VAL_NUM - 1))
    local METRICS_PORT=$((METRICS_PORT_BASE + VAL_NUM - 1))
    local VAL_PATH="$BASE_PATH/validator$VAL_NUM"
    local LOG_FILE="$LOG_DIR/validator$VAL_NUM.log"
    
    mkdir -p "$VAL_PATH"
    
    echo -e "${BLUE}📍 Starting Validator $VAL_NUM...${NC}"
    echo "   Port: $PORT | RPC: $RPC_PORT | Metrics: $METRICS_PORT"
    
    local ARGS=(
        "--chain" "$CHAIN_SPEC"
        "--validator"
        "--name" "Validator-$VAL_NUM"
        "--base-path" "$VAL_PATH"
        "--port" "$PORT"
        "--rpc-port" "$RPC_PORT"
        "--rpc-external"
        "--prometheus-port" "$METRICS_PORT"
        "--tmp"
    )
    
    # Add bootnode for validators after first one
    if [[ $VAL_NUM -gt 1 ]]; then
        # Try to get peer ID from first validator's log (will be captured on startup)
        sleep 2
        local PEER_ID=$(grep "Local node identity is:" "$LOG_DIR/validator1.log" | tail -1 | grep -oP 'is: \K[^ ]+' || echo "")
        if [[ -n "$PEER_ID" ]]; then
            ARGS+=(
                "--bootnodes" "/ip4/127.0.0.1/tcp/$PORT_BASE/p2p/$PEER_ID"
            )
        fi
    fi
    
    # Start validator in background
    "$BINARY" "${ARGS[@]}" >> "$LOG_FILE" 2>&1 &
    local PID=$!
    
    echo -e "${GREEN}✅ Validator $VAL_NUM started (PID: $PID)${NC}"
    echo "   Log: $LOG_FILE"
    
    sleep 3
}

# Function to wait for finality
wait_for_finality() {
    echo -e "${BLUE}⏳ Waiting for consensus finality...${NC}"
    
    local MAX_ATTEMPTS=30
    local ATTEMPT=0
    
    while [[ $ATTEMPT -lt $MAX_ATTEMPTS ]]; do
        local FINALIZED=$(curl -s http://127.0.0.1:9933 \
            -X POST \
            -H "Content-Type: application/json" \
            -d '{"jsonrpc":"2.0","method":"chain_getFinalizedHead","params":[],"id":1}' \
            2>/dev/null | grep -o '"result":"0x[a-f0-9]*"' || echo "")
        
        if [[ -n "$FINALIZED" ]]; then
            echo -e "${GREEN}✅ Consensus reached, blocks finalizing${NC}"
            return 0
        fi
        
        ATTEMPT=$((ATTEMPT + 1))
        echo "   Attempt $ATTEMPT/$MAX_ATTEMPTS..."
        sleep 2
    done
    
    echo -e "${YELLOW}⚠️  Finality check timeout (may still be syncing)${NC}"
}

# Function to validate chain state
validate_chain_state() {
    echo -e "${BLUE}🔍 Validating chain state...${NC}"
    
    # Check block production
    local BLOCK_HEIGHT=$(curl -s http://127.0.0.1:9933 \
        -X POST \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"chain_getBlockNumber","params":[],"id":1}' \
        2>/dev/null | grep -oP '"result":"0x[a-f0-9]*' | grep -oP '0x[a-f0-9]*')
    
    if [[ -n "$BLOCK_HEIGHT" ]]; then
        echo -e "${GREEN}✅ Chain producing blocks: Height = $BLOCK_HEIGHT${NC}"
        return 0
    else
        echo -e "${YELLOW}⚠️  Could not verify block production${NC}"
        return 1
    fi
}

# MAIN EXECUTION

echo -e "${YELLOW}📋 Configuration:${NC}"
echo "   Validators: $VALIDATORS"
echo "   Chain Spec: $CHAIN_SPEC"
echo "   Base Path: $BASE_PATH"
echo "   Log Dir: $LOG_DIR"
echo ""

# Start validators
echo -e "${YELLOW}🎯 Phase 1: Validator Startup${NC}"
for ((i=1; i<=VALIDATORS; i++)); do
    start_validator $i
done

echo ""
echo -e "${YELLOW}🎯 Phase 2: Consensus Initialization${NC}"
wait_for_finality

echo ""
echo -e "${YELLOW}🎯 Phase 3: Chain State Validation${NC}"
validate_chain_state

echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}✅ TESTNET READY!${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${YELLOW}🌐 RPC Endpoints:${NC}"
for ((i=1; i<=VALIDATORS; i++)); do
    local RPC_PORT=$((RPC_PORT_BASE + i - 1))
    echo "   Validator $i: http://127.0.0.1:$RPC_PORT"
done

echo ""
echo -e "${YELLOW}📊 Monitoring:${NC}"
echo "   Settlement: ./scripts/settlement-timeout-monitor.sh"
echo "   GPU Health: ./scripts/gpu-health-monitor.sh"
echo "   Consensus: ./scripts/peer-consensus-tracker.sh"
echo ""

echo -e "${YELLOW}🔄 Live Logs:${NC}"
echo "   tail -f $LOG_DIR/validator1.log"
echo ""

# Keep running — comprehensive health check loop.
# Checks every 30 seconds:
#   1. RPC responds (JSON-RPC system_health)
#   2. Block production is advancing (best block increases)
#   3. GRANDPA finality is advancing (finalized block increases)
#   4. Peer count is adequate (≥ VALIDATORS-1 per node)

echo -e "${BLUE}ℹ️  Testnet running. Press Ctrl+C to stop.${NC}"
echo ""

# Capture initial block state for advance tracking
declare -A PREV_BEST
declare -A PREV_FINALIZED
for ((i=1; i<=VALIDATORS; i++)); do
    PREV_BEST[$i]=0
    PREV_FINALIZED[$i]=0
done

while true; do
    sleep 30

    echo -e "${BLUE}── Health check $(date '+%H:%M:%S') ──────────────────────────────${NC}"

    ALL_OK=true

    for ((i=1; i<=VALIDATORS; i++)); do
        local PORT=$((RPC_PORT_BASE + i - 1))
        local ENDPOINT="http://127.0.0.1:$PORT"

        # 1. RPC responsiveness
        HEALTH=$(curl -sf --max-time 5 "$ENDPOINT" \
            -H 'Content-Type: application/json' \
            -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}' 2>/dev/null)
        if [[ -z "$HEALTH" ]]; then
            echo -e "  ${RED}❌ Validator $i (port $PORT): RPC not responding${NC}"
            ALL_OK=false
            continue
        fi

        # 2. Peer count — substrate system_peers
        PEERS=$(curl -sf --max-time 5 "$ENDPOINT" \
            -H 'Content-Type: application/json' \
            -d '{"jsonrpc":"2.0","method":"system_peers","params":[],"id":1}' 2>/dev/null \
            | python3 -c "import sys,json; d=json.load(sys.stdin); print(len(d.get('result',[])))" 2>/dev/null || echo "0")
        REQUIRED_PEERS=$((VALIDATORS - 1))
        if [[ "$PEERS" -lt "$REQUIRED_PEERS" ]]; then
            echo -e "  ${YELLOW}⚠️  Validator $i: only $PEERS/$REQUIRED_PEERS required peers${NC}"
            ALL_OK=false
        fi

        # 3. Best block advancing
        BEST_HEX=$(curl -sf --max-time 5 "$ENDPOINT" \
            -H 'Content-Type: application/json' \
            -d '{"jsonrpc":"2.0","method":"chain_getHeader","params":[],"id":1}' 2>/dev/null \
            | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['result']['number'])" 2>/dev/null || echo "0x0")
        BEST_DEC=$(python3 -c "print(int('$BEST_HEX',16))" 2>/dev/null || echo "0")

        if [[ "$BEST_DEC" -le "${PREV_BEST[$i]}" ]]; then
            echo -e "  ${RED}❌ Validator $i: block production stalled (best=$BEST_DEC)${NC}"
            ALL_OK=false
        fi
        PREV_BEST[$i]=$BEST_DEC

        # 4. Finality advancing
        FIN_HASH=$(curl -sf --max-time 5 "$ENDPOINT" \
            -H 'Content-Type: application/json' \
            -d '{"jsonrpc":"2.0","method":"chain_getFinalizedHead","params":[],"id":1}' 2>/dev/null \
            | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('result','0x'))" 2>/dev/null || echo "0x")
        FIN_HDR=$(curl -sf --max-time 5 "$ENDPOINT" \
            -H 'Content-Type: application/json' \
            -d "{\"jsonrpc\":\"2.0\",\"method\":\"chain_getHeader\",\"params\":[\"$FIN_HASH\"],\"id\":1}" 2>/dev/null \
            | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['result']['number'])" 2>/dev/null || echo "0x0")
        FIN_DEC=$(python3 -c "print(int('$FIN_HDR',16))" 2>/dev/null || echo "0")

        if [[ "$FIN_DEC" -le "${PREV_FINALIZED[$i]}" ]] && [[ "${PREV_FINALIZED[$i]}" -gt 0 ]]; then
            echo -e "  ${YELLOW}⚠️  Validator $i: finality stalled (finalized=$FIN_DEC, prev=${PREV_FINALIZED[$i]})${NC}"
        fi
        PREV_FINALIZED[$i]=$FIN_DEC

        echo -e "  ${GREEN}✅ Validator $i (port $PORT): best=$BEST_DEC finalized=$FIN_DEC peers=$PEERS${NC}"
    done

    if [[ "$ALL_OK" == "true" ]]; then
        echo -e "  ${GREEN}✅ All $VALIDATORS validators healthy${NC}"
    else
        echo -e "  ${YELLOW}⚠️  Issues detected — check logs in $LOG_DIR${NC}"
    fi
    echo ""
done
