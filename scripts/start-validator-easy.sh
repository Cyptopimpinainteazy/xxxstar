#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

BINARY="${X3_NODE_BIN:-$PROJECT_ROOT/target/release/x3-chain-node}"
CHAIN_SPEC="${X3_CHAIN_SPEC:-}"
BASE_PATH="${X3_BASE_PATH:-/tmp/x3-validator}"
LOG_DIR="${X3_LOG_DIR:-/tmp/x3-validator-logs}"
NAME="${X3_NAME:-Validator-1}"
PORT="${X3_PORT:-30333}"
RPC_PORT="${X3_RPC_PORT:-9933}"
WS_PORT="${X3_WS_PORT:-$RPC_PORT}"
METRICS_PORT="${X3_METRICS_PORT:-9615}"
WASM_EXECUTION="${X3_WASM_EXECUTION:-}"
NODE_KEY_FILE="${X3_NODE_KEY_FILE:-}"
NODE_KEY_HEX="${X3_NODE_KEY_HEX:-}"
BOOTNODES="${X3_BOOTNODES:-}"
DAEMONIZE="false"
CLEAN="false"

show_help() {
  cat <<EOF
Usage: $0 [options]

Options:
  --name NAME               Validator name (default: Validator-1)
  --chain PATH              Chain spec JSON path
  --base-path PATH          Validator base data path (default: /tmp/x3-validator)
  --log-dir PATH            Log directory (default: /tmp/x3-validator-logs)
  --port PORT               P2P port (default: 30333)
  --rpc-port PORT           RPC port (default: 9933)
  --ws-port PORT            WebSocket port alias (sets --rpc-port)
  --metrics-port PORT       Prometheus port (default: 9615)
  --wasm-execution MODE     WASM execution mode (e.g. compiled)
  --node-key-file PATH      Node key file for stable peer identity
  --node-key HEX            Node key hex string
  --bootnodes MULTIADDRS    Bootnodes multiaddr list for peer discovery
  --daemonize               Start node and exit instead of waiting
  --clean                   Remove previous base-path data before start
  --help                    Show this help message

Example:
  ./scripts/start-validator-easy.sh --chain deployment/chain-specs/x3-testnet-raw.json --daemonize
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --name) NAME="$2"; shift 2;;
    --chain) CHAIN_SPEC="$2"; shift 2;;
    --base-path) BASE_PATH="$2"; shift 2;;
    --log-dir) LOG_DIR="$2"; shift 2;;
    --port) PORT="$2"; shift 2;;
    --rpc-port) RPC_PORT="$2"; shift 2;;
    --ws-port) WS_PORT="$2"; RPC_PORT="$2"; shift 2;;
    --metrics-port) METRICS_PORT="$2"; shift 2;;
    --wasm-execution) WASM_EXECUTION="$2"; shift 2;;
    --node-key-file) NODE_KEY_FILE="$2"; shift 2;;
    --node-key) NODE_KEY_HEX="$2"; shift 2;;
    --bootnodes) BOOTNODES="$2"; shift 2;;
    --daemonize) DAEMONIZE="true"; shift;;
    --clean) CLEAN="true"; shift;;
    --help) show_help; exit 0;;
    *) echo "Unknown option: $1" >&2; show_help; exit 1;;
  esac
done

log() { echo "[x3-validator-easy] $*"; }
error() { echo "[ERROR] $*" >&2; }

generate_node_key_file() {
  local key_path="$1"
  if [[ -s "$key_path" ]]; then
    return
  fi
  log "Node key file '$key_path' not found; generating new Ed25519 node key"
  if command -v xxd >/dev/null 2>&1; then
    xxd -p -l 32 /dev/urandom | tr -d '\n' > "$key_path"
  elif command -v python3 >/dev/null 2>&1; then
    python3 - <<'PY' > "$key_path"
import os, binascii
print(binascii.hexlify(os.urandom(32)).decode())
PY
  else
    error "Cannot generate node key; install xxd or python3"
    exit 1
  fi
  chmod 600 "$key_path"
}

if [[ "$CLEAN" == "true" ]]; then
  log "Removing previous validator data at $BASE_PATH"
  rm -rf "$BASE_PATH"
fi

if [[ ! -f "$BINARY" ]]; then
  log "Node binary not found at $BINARY"
  log "Building x3-chain-node release binary..."
  cargo build --release -p x3-chain-node
  if [[ ! -f "$BINARY" ]]; then
    error "Build completed but binary still missing: $BINARY"
    exit 1
  fi
fi

if [[ -z "$CHAIN_SPEC" ]]; then
  if [[ -f "$PROJECT_ROOT/deployment/chain-specs/x3-testnet-raw.json" ]]; then
    CHAIN_SPEC="$PROJECT_ROOT/deployment/chain-specs/x3-testnet-raw.json"
  elif [[ -f "$PROJECT_ROOT/chain-specs/x3-local3-current-raw.json" ]]; then
    CHAIN_SPEC="$PROJECT_ROOT/chain-specs/x3-local3-current-raw.json"
  elif [[ -f "$PROJECT_ROOT/chain-specs/x3-local3-raw.json" ]]; then
    CHAIN_SPEC="$PROJECT_ROOT/chain-specs/x3-local3-raw.json"
  fi
fi

if [[ -n "$CHAIN_SPEC" && ! -f "$CHAIN_SPEC" ]]; then
  case "$CHAIN_SPEC" in
    dev|local|staging)
      # Allow built-in chain names recognized by the node CLI.
      ;;
    *)
      error "Chain spec JSON not found. Provide one with --chain or create a raw spec under chain-specs/ or deployment/chain-specs/."
      echo "Suggested default: ./deployment/chain-specs/x3-testnet-raw.json"
      exit 1
      ;;
  esac
fi

if [[ -z "$NODE_KEY_FILE" && -z "$NODE_KEY_HEX" ]]; then
  NODE_KEY_FILE="$BASE_PATH/node-key"
fi

mkdir -p "$BASE_PATH" "$LOG_DIR" "$(dirname "$NODE_KEY_FILE")"

if [[ -n "$NODE_KEY_HEX" ]]; then
  log "Using provided node key hex"
else
  log "Using node key file: $NODE_KEY_FILE"
  if [[ ! -s "$NODE_KEY_FILE" ]]; then
    generate_node_key_file "$NODE_KEY_FILE"
  fi
fi

log "Using binary: $BINARY"
log "Using chain spec: $CHAIN_SPEC"
log "Validator name: $NAME"
log "Data path: $BASE_PATH"
log "RPC port: $RPC_PORT"
log "WS port: $WS_PORT"
log "Metrics port: $METRICS_PORT"

if [[ -n "$NODE_KEY_HEX" ]]; then
  log "Node key: (provided via hex)"
else
  log "Node key file: $NODE_KEY_FILE"
fi

LOG_FILE="$LOG_DIR/$NAME-$(date +%Y%m%d-%H%M%S).log"
LOG_FILE="$LOG_DIR/$NAME-$(date +%Y%m%d-%H%M%S).log"

cmd=("$BINARY"
  --chain "$CHAIN_SPEC"
  --validator
  --name "$NAME"
  --base-path "$BASE_PATH"
  --port "$PORT"
  --rpc-port "$RPC_PORT"
  --rpc-cors all
  --rpc-methods safe
  --prometheus-port "$METRICS_PORT"
  --prometheus-external
  --log info)

if [[ -n "${WASM_EXECUTION:-}" ]]; then
  cmd+=(--wasm-execution "$WASM_EXECUTION")
fi

if [[ -n "$NODE_KEY_HEX" ]]; then
  cmd+=(--node-key "$NODE_KEY_HEX")
else
  cmd+=(--node-key-file "$NODE_KEY_FILE")
fi

if [[ -n "$BOOTNODES" ]]; then
  cmd+=(--bootnodes "$BOOTNODES")
fi

log "Starting validator and logging to $LOG_FILE"

{
  echo "=== Starting validator: $(date) ==="
  printf '%q ' "${cmd[@]}"
  echo
  echo
  exec "${cmd[@]}"
} >> "$LOG_FILE" 2>&1 &

PID=$!

log "Validator process PID: $PID"

sleep 4

if ! kill -0 "$PID" 2>/dev/null; then
  error "Validator failed to start. See latest logs:"
  tail -20 "$LOG_FILE" >&2
  exit 1
fi

log "Waiting for RPC port $RPC_PORT to respond..."
for i in {1..20}; do
  if curl -s "http://127.0.0.1:$RPC_PORT" -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}' 2>/dev/null | grep -q 'result'; then
    log "RPC is responding"
    break
  fi
  if [[ "$i" -eq 20 ]]; then
    error "RPC did not respond after 20 seconds. Check logs: $LOG_FILE"
    exit 1
  fi
  sleep 1
done

cat <<EOF

══════════════════════════════════════════════════
✅ Validator bootstrap complete

Validator: $NAME
PID: $PID
P2P port: $PORT
RPC: http://127.0.0.1:$RPC_PORT
WS: ws://127.0.0.1:$WS_PORT
Metrics: http://127.0.0.1:$METRICS_PORT/metrics
Log: $LOG_FILE

Monitor:
  tail -f "$LOG_FILE"

Verify:
  curl -s http://127.0.0.1:$RPC_PORT -X POST -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}' | jq .

Stop:
  kill $PID
════════════════════════════════════════════════════════
EOF

if [[ "$DAEMONIZE" == "true" ]]; then
  log "Daemonized validator; exiting launcher while node continues to run."
  exit 0
fi

wait "$PID"
