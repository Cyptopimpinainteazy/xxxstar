#!/bin/bash
# Validator Coordinator Script
# Starts all 3 X3 validators in parallel with proper coordination

set -e

WORKSPACE="${WORKSPACE:-/home/lojak/Desktop/X3_ATOMIC_STAR}"
SCRIPTS_DIR="$WORKSPACE/scripts"
LOG_DIR="/tmp/x3-testnet-logs"

# Enhanced logging
log_info() { echo "[INFO] $(date '+%Y-%m-%d %H:%M:%S') - $1"; }
log_error() { echo "[ERROR] $(date '+%Y-%m-%d %H:%M:%S') - $1"; }
log_success() { echo "[✅] $(date '+%Y-%m-%d %H:%M:%S') - $1"; }

mkdir -p "$LOG_DIR"

echo "╔════════════════════════════════════════════════════════════════╗"
echo "║         X3 VALIDATOR NETWORK COORDINATOR                      ║"
echo "║  Starting 3 validators for consensus network                  ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo

# Verify bootstrap script exists
if [ ! -f "$SCRIPTS_DIR/bootstrap-validator.sh" ]; then
  log_error "Bootstrap script not found: $SCRIPTS_DIR/bootstrap-validator.sh"
  exit 1
fi

log_success "Bootstrap script found"

# Clean up any existing validators
log_info "Cleaning up existing validator processes..."
pkill -f "x3-chain-node.*validator" 2>/dev/null || true
rm -rf /tmp/x3-validator-{1,2,3}
sleep 2

log_success "Cleanup complete"
echo

# Start validators in parallel
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🚀 Starting validators..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo

PIDS=()
PORTS=(30333 30334 30335)

for i in {1,2,3}; do
  PORT=${PORTS[$((i-1))]}
  log_info "Starting Validator $i (P2P port: $PORT)..."
  
  # Start validator in background
  bash "$SCRIPTS_DIR/bootstrap-validator.sh" $i $PORT &
  PID=$!
  PIDS+=($PID)
  
  echo "   Started (PID: $PID)"
  sleep 2  # Stagger starts to avoid conflicts
done

echo
log_success "All validators started"
echo

# Wait for validators to initialize
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "⏳ Validators initializing (waiting 10 seconds)..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
sleep 10
echo

# Verify all processes are running
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 Validator Status"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo

ALL_RUNNING=true

for i in {0,1,2}; do
  PID=${PIDS[$i]}
  VAL_NUM=$((i+1))
  RPC_PORT=$((9933 + i))
  WS_PORT=$((9944 + i))
  
  if kill -0 $PID 2>/dev/null; then
    echo "✅ Validator $VAL_NUM (PID: $PID)"
    echo "   RPC: http://127.0.0.1:$RPC_PORT"
    echo "   WS: ws://127.0.0.1:$WS_PORT"
    
    # Quick RPC check
    if curl -s http://127.0.0.1:$RPC_PORT -X POST \
      -H "Content-Type: application/json" \
      -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}' 2>/dev/null | grep -q "result"; then
      echo "   Status: RPC Responding ✅"
    else
      echo "   Status: RPC Not Yet Responding (starting...)"
    fi
  else
    echo "❌ Validator $VAL_NUM (PID: $PID) - FAILED TO START"
    ALL_RUNNING=false
  fi
  echo
done

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ "$ALL_RUNNING" = true ]; then
  log_success "All validators running"
  
  echo
  echo "📂 Log Files:"
  echo "   Validator 1: tail -f $LOG_DIR/validator1.log"
  echo "   Validator 2: tail -f $LOG_DIR/validator2.log"
  echo "   Validator 3: tail -f $LOG_DIR/validator3.log"
  echo
  
  echo "🔍 Monitor Network:"
  echo "   Watch peers: watch -n 2 'ps aux | grep validator | grep -v grep | wc -l'"
  echo "   Check consensus: curl -s http://127.0.0.1:9933 -X POST -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"method\":\"system_health\",\"params\":[],\"id\":1}' | jq"
  echo
  
  echo "🛑 Stop validators:"
  echo "   kill ${PIDS[0]} ${PIDS[1]} ${PIDS[2]}"
  echo
  
  # Keep coordinator running
  echo "═══════════════════════════════════════════════════════════════"
  echo "✨ Validator Network Ready!"
  echo "Coordinat or keeping validators supervised..."
  echo "Press Ctrl+C to stop all validators"
  echo "═══════════════════════════════════════════════════════════════"
  
  # Wait for all processes
  for PID in "${PIDS[@]}"; do
    wait $PID 2>/dev/null || true
  done
  
else
  log_error "Some validators failed to start"
  exit 1
fi
