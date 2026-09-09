#!/usr/bin/env bash
# verify-wrapped-council-flow.sh
#
# Exercise the two-member wrapped-asset council flow on a temporary dev chain:
#   1. Alice proposes Council::propose(threshold=2) for register
#   2. x3_executeWrappedCouncil drives Alice vote + Bob vote + close
#   3. Repeat for mint
#   4. Assert on-chain wrapped balance/supply increased
#
# Required:
#   - target/release/x3-chain-node built without SKIP_WASM_BUILD
#   - jq installed
#   - Alice and Bob are both dev council members
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
NODE_BIN="${X3_NODE_BIN:-$ROOT/target/release/x3-chain-node}"
RPC_PORT="${X3_WRAPPED_COUNCIL_PORT:-19945}"
CHAIN_ID="${X3_WRAPPED_CHAIN_ID:-1337}"
TOKEN_ADDRESS="${X3_WRAPPED_TOKEN_ADDRESS:-0x0000000000000000000000000000000000000001}"
RECIPIENT="${X3_WRAPPED_RECIPIENT:-0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d}"
AMOUNT="${X3_WRAPPED_AMOUNT:-1000}"
MINT_NONCE="${X3_WRAPPED_NONCE:-42}"

if [[ ! -x "$NODE_BIN" ]]; then
  echo "node binary missing: $NODE_BIN (build with: SKIP_WASM_BUILD= cargo build --release -p x3-chain-node)" >&2
  exit 1
fi

command -v jq >/dev/null || { echo "jq is required" >&2; exit 1; }

STATE="$(mktemp -d)"
LOG="$STATE/node.log"
PID=""
cleanup() {
  if [[ -n "$PID" ]] && kill -0 "$PID" 2>/dev/null; then
    kill "$PID" 2>/dev/null || true
    wait "$PID" 2>/dev/null || true
  fi
  rm -rf "$STATE"
}
trap cleanup EXIT

echo "[1/4] Starting dev node on port $RPC_PORT"
X3_SUBMITTER_SEED=//Alice X3_APPROVER_SEED=//Bob \
  "$NODE_BIN" --dev --tmp --rpc-port "$RPC_PORT" --rpc-methods unsafe \
  --no-prometheus --no-telemetry --no-mdns --force-authoring >"$LOG" 2>&1 &
PID=$!

for _ in $(seq 1 45); do
  if curl -sf -X POST -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"system_health","params":[]}' \
    "http://127.0.0.1:$RPC_PORT" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

rpc() {
  local body="$1"
  curl -sf -X POST -H 'Content-Type: application/json' -d "$body" \
    "http://127.0.0.1:$RPC_PORT"
}

propose_and_execute() {
  local action="$1" extra="$2"
  local resp hash index exec_resp
  resp="$(rpc "{\"jsonrpc\":\"2.0\",\"id\":10,\"method\":\"x3_proposeWrappedCouncil\",\"params\":[{\"action\":\"$action\",\"chain_id\":$CHAIN_ID,\"token_address\":\"$TOKEN_ADDRESS\"$extra}]}")"
  hash="$(echo "$resp" | jq -r .result.proposal_hash)"
  index="$(echo "$resp" | jq -r .result.proposal_index)"
  [[ "$hash" != "null" && -n "$hash" ]] || { echo "propose $action failed: $resp" >&2; exit 1; }
  exec_resp="$(rpc "{\"jsonrpc\":\"2.0\",\"id\":11,\"method\":\"x3_executeWrappedCouncil\",\"params\":[{\"proposal_hash\":\"$hash\",\"proposal_index\":$index}]}")"
  [[ "$(echo "$exec_resp" | jq -r .result.status)" == "executed" ]] || {
    echo "execute $action failed: $exec_resp" >&2
    exit 1
  }
  echo "[ok] $action proposal $index executed"
}

echo "[2/4] Registering wrapped asset via two-member council"
propose_and_execute "register" ""

echo "[3/4] Minting wrapped asset via two-member council"
propose_and_execute "mint" ",\"recipient\":\"$RECIPIENT\",\"amount\":\"$AMOUNT\",\"nonce\":\"$MINT_NONCE\""

echo "[4/4] Verifying on-chain accounting"
ACCT="$(rpc "{\"jsonrpc\":\"2.0\",\"id\":12,\"method\":\"x3_getWrappedAccountingForToken\",\"params\":[\"$RECIPIENT\",$CHAIN_ID,\"$TOKEN_ADDRESS\"]}")"
BAL="$(echo "$ACCT" | jq -r .result.balance)"
SUP="$(echo "$ACCT" | jq -r .result.supply)"
[[ "$BAL" == "$AMOUNT" && "$SUP" == "$AMOUNT" ]] || {
  echo "accounting mismatch: expected $AMOUNT got balance=$BAL supply=$SUP" >&2
  exit 1
}
echo "[ok] wrapped balance/supply = $AMOUNT"
echo "PASS: two-member wrapped register + mint flow verified"
