#!/bin/bash
# Standalone Validator Bootstrap Script
# Starts a single X3 validator node with proper configuration and logging

set -e

# Configuration
WORKSPACE="${WORKSPACE:-/home/lojak/Desktop/X3_ATOMIC_STAR}"
VALIDATOR_NUM="${1:-1}"
BASE_PORT="${2:-30333}"
RPC_PORT=$((9933 + VALIDATOR_NUM - 1))
WS_PORT=$((9944 + VALIDATOR_NUM - 1))
VALIDATOR_NAME="X3-Validator-$VALIDATOR_NUM"
BASE_PATH="/tmp/x3-validator-$VALIDATOR_NUM"
LOG_FILE="/tmp/x3-testnet-logs/validator$VALIDATOR_NUM.log"

# Enhanced logging
log_info() { echo "[INFO] $(date '+%Y-%m-%d %H:%M:%S') - $1"; }
log_error() { echo "[ERROR] $(date '+%Y-%m-%d %H:%M:%S') - $1"; }
log_success() { echo "[✅] $(date '+%Y-%m-%d %H:%M:%S') - $1"; }

# Verify workspace
if [ ! -d "$WORKSPACE" ]; then
  log_error "Workspace not found: $WORKSPACE"
  exit 1
fi

# Verify node binary
NODE_BINARY="$WORKSPACE/target/release/x3-chain-node"
if [ ! -f "$NODE_BINARY" ]; then
  log_error "Node binary not found at: $NODE_BINARY"
  exit 1
fi

if [ ! -x "$NODE_BINARY" ]; then
  log_error "Node binary not executable: $NODE_BINARY"
  exit 1
fi

log_success "Node binary verified: $NODE_BINARY"

# Create logging directory
mkdir -p /tmp/x3-testnet-logs

# Prepare base path
log_info "Setting up validator at: $BASE_PATH"
rm -rf "$BASE_PATH"
mkdir -p "$BASE_PATH"

# Generate chain specification if needed
CHAIN_SPEC="$WORKSPACE/chain-spec.json"
if [ ! -f "$CHAIN_SPEC" ]; then
  log_info "Creating chain specification..."
  "$NODE_BINARY" build-spec --disable-default-bootnode > "$CHAIN_SPEC" 2>&1 || {
    log_error "Failed to build chain specification"
    exit 1
  }
  log_success "Chain specification created"
fi

# Prepare keys if needed (for authorities)
log_info "Preparing keys for validator..."
mkdir -p "$BASE_PATH/chains/x3-chain/keystore"

# Start validator
log_info "Starting $VALIDATOR_NAME..."
log_info "Configuration:"
log_info "  - Name: $VALIDATOR_NAME"
log_info "  - Base Path: $BASE_PATH"
log_info "  - P2P Port: $BASE_PORT"
log_info "  - RPC Port: $RPC_PORT"
log_info "  - WS Port: $WS_PORT"
log_info "  - Log File: $LOG_FILE"
echo

{
  echo "=== Validator $VALIDATOR_NUM Started: $(date) ==="
  echo "Command: $NODE_BINARY"
  echo "Arguments:"
  echo "  --validator"
  echo "  --name=$VALIDATOR_NAME"
  echo "  --base-path=$BASE_PATH"
  echo "  --chain=$CHAIN_SPEC"
  echo "  --port=$BASE_PORT"
  echo "  --rpc-port=$RPC_PORT"
  echo "  --ws-port=$WS_PORT"
  echo "  --rpc-external"
  echo "  --ws-external"
  echo "  --rpc-methods=Unsafe"
  echo ""
  
  "$NODE_BINARY" \
    --validator \
    --name="$VALIDATOR_NAME" \
    --base-path="$BASE_PATH" \
    --chain="$CHAIN_SPEC" \
    --port="$BASE_PORT" \
    --rpc-port="$RPC_PORT" \
    --ws-port="$WS_PORT" \
    --rpc-external \
    --ws-external \
    --rpc-methods=Unsafe \
    2>&1
  
  echo "=== Validator $VALIDATOR_NUM Exited: $(date) ==="
} >> "$LOG_FILE" 2>&1 &

VALIDATOR_PID=$!

# Wait for startup
log_info "Waiting for validator to initialize..."
sleep 5

# Verify process started
if ! kill -0 $VALIDATOR_PID 2>/dev/null; then
  log_error "Validator process failed to start"
  log_info "Last log entries:"
  tail -20 "$LOG_FILE"
  exit 1
fi

log_success "Validator $VALIDATOR_NUM started (PID: $VALIDATOR_PID)"

# Verify RPC responsiveness
log_info "Verifying RPC port $RPC_PORT..."
for i in {1..10}; do
  if curl -s http://127.0.0.1:$RPC_PORT -X POST \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"chain_getLatestHeader","params":[],"id":1}' | grep -q "result"; then
    log_success "RPC port $RPC_PORT is responding"
    break
  fi
  
  if [ $i -eq 10 ]; then
    log_error "RPC port $RPC_PORT not responding after 10 attempts"
  fi
  
  sleep 1
done

# Output status
echo
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Validator Bootstrap Complete"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Validator: $VALIDATOR_NAME (PID: $VALIDATOR_PID)"
echo "P2P Port: $BASE_PORT"
echo "RPC: http://127.0.0.1:$RPC_PORT"
echo "WebSocket: ws://127.0.0.1:$WS_PORT"
echo "Log File: $LOG_FILE"
echo ""
echo "Monitor with:"
echo "  tail -f $LOG_FILE"
echo ""
echo "Query status:"
echo "  curl -s http://127.0.0.1:$RPC_PORT -X POST -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"method\":\"system_health\",\"params\":[],\"id\":1}' | jq"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Keep running
wait $VALIDATOR_PID
