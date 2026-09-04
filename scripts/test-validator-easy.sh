#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

CMD="$SCRIPT_DIR/start-validator-easy.sh"
CHAIN_SPEC="$PROJECT_ROOT/deployment/chain-specs/x3-testnet-raw.json"
BASE_PATH="/tmp/x3-validator-test"
LOG_DIR="/tmp/x3-validator-test-logs"

if [[ ! -f "$CHAIN_SPEC" ]]; then
  echo "Chain spec not found: $CHAIN_SPEC — falling back to 'dev' chain"
  CHAIN_SPEC="dev"
fi

if [[ ! -f "$PROJECT_ROOT/target/release/x3-chain-node" ]]; then
  echo "Building x3-chain-node release binary..."
  cargo build --release -p x3-chain-node
fi

chmod +x "$CMD"
chmod +x "$SCRIPT_DIR/stop-validator-easy.sh"

export X3_CHAIN_SPEC="$CHAIN_SPEC"
export X3_BASE_PATH="$BASE_PATH"
export X3_LOG_DIR="$LOG_DIR"

trap 'echo "Cleaning up..."; bash "$SCRIPT_DIR/stop-validator-easy.sh" --hard; rm -rf "$BASE_PATH" "$LOG_DIR"' EXIT

set -x

# Use compiled WASM execution. Interpreted mode is deprecated and now falls back to compiled execution.
bash "$CMD" --base-path "$BASE_PATH" --log-dir "$LOG_DIR" --daemonize --wasm-execution compiled

for i in {1..20}; do
  if curl -s "http://127.0.0.1:9933" -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}' 2>/dev/null | grep -q 'result'; then
    echo "RPC is healthy"
    exit 0
  fi
  sleep 1
done

echo "Validator RPC did not become healthy"
exit 1
