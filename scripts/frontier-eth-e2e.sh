#!/usr/bin/env bash
# Boot a `--features frontier` dev node and send real signed Ethereum transactions to it.
#
# tests/wallet-integration/live/frontier-eth-e2e.ts signs with ethers and checks, against the live
# chain, that eth_sendRawTransaction:
#   - includes a signed transfer: sender nonce advances, recipient's native balance is credited,
#     and the returned hash is keccak(raw)
#   - refuses a tampered transaction, a wrong chain id, garbage bytes, the old caller-named
#     payload, and a replay — none of which may move a nonce or a balance
#   - serves receipts, transactions, logs and block fields from the node's Ethereum index
# Then a second, non-authoring node (archive state) syncs the chain from the first after it has
# grown well past those transactions, and must serve identical receipts, transactions and logs:
# the index's catch-up path, since a major sync announces no imports.
# Exits nonzero on any failure. A node built without `frontier` fails here ("EVM disabled").
#
# Usage: bash scripts/frontier-eth-e2e.sh
# Env:   X3_NODE_BIN  node binary (default: target/debug/x3-chain-node, built with frontier if absent)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RPC_PORT="$(( 21000 + RANDOM % 8000 ))"
P2P_PORT="$(( 31000 + RANDOM % 8000 ))"
PROMETHEUS_PORT="$(( 41000 + RANDOM % 8000 ))"
RPC_URL="http://127.0.0.1:$RPC_PORT"
NODE_LOG="$(mktemp)"
SYNC_LOG="$(mktemp)"
SYNC_FILE="$(mktemp)"
NODE_PID=""
SYNC_PID=""

stop() {
  local pid="$1"
  if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
    kill "$pid" 2>/dev/null || true
    for _ in $(seq 1 20); do
      kill -0 "$pid" 2>/dev/null || break
      sleep 0.5
    done
    kill -9 "$pid" 2>/dev/null || true
  fi
}
cleanup() {
  stop "$SYNC_PID"
  stop "$NODE_PID"
  rm -f "$NODE_LOG" "$SYNC_LOG" "$SYNC_FILE"
}
trap cleanup EXIT

info() { printf '[frontier-eth-e2e] %s\n' "$*"; }
fail() { printf '[frontier-eth-e2e] FAIL: %s\n' "$*" >&2; tail -n 40 "$NODE_LOG" "$SYNC_LOG" >&2 || true; exit 1; }
rpc_at() {  # rpc_at URL METHOD [PARAMS]
  curl -s -m 5 -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"$2\",\"params\":${3:-[]},\"id\":1}" "$1"
}
best_of() { printf '%d' "$(rpc_at "$1" chain_getHeader | sed -n 's/.*"number":"\(0x[0-9a-f]*\)".*/\1/p')"; }

NODE_BIN="${X3_NODE_BIN:-$ROOT/target/debug/x3-chain-node}"
if [ ! -x "$NODE_BIN" ]; then
  info "building the node with frontier"
  ( cd "$ROOT" && cargo build -p x3-chain-node --features frontier --locked ) || fail "could not build the node"
fi
info "node: $NODE_BIN"

info "booting a dev chain on $RPC_URL"
"$NODE_BIN" --dev --tmp \
  --rpc-port "$RPC_PORT" --port "$P2P_PORT" --prometheus-port "$PROMETHEUS_PORT" \
  >"$NODE_LOG" 2>&1 &
NODE_PID=$!

ready=0
for _ in $(seq 1 300); do
  if curl -s -m 5 -H 'Content-Type: application/json' \
      -d '{"jsonrpc":"2.0","method":"chain_getHeader","params":[],"id":1}' "$RPC_URL" | grep -q '"number"'; then
    ready=1; break
  fi
  kill -0 "$NODE_PID" 2>/dev/null || fail "the node exited before it answered RPC"
  sleep 1
done
[ "$ready" = 1 ] || fail "the node did not answer RPC within 300s"

info "running the ethers checks"
run_ts() {
  ( cd "$ROOT/tests/wallet-integration" && \
    npx ts-node --transpile-only --compiler-options '{"module":"commonjs"}' live/frontier-eth-e2e.ts )
}
X3_RPC="$RPC_URL" X3_SYNC_FILE="$SYNC_FILE" run_ts || fail "ethers checks failed"

# ── a second node syncs the chain and must serve the same answers ───────────────
tx_block="$(sed -n 's/.*"block":\([0-9]*\).*/\1/p' "$SYNC_FILE")"
target=$(( tx_block + 400 ))
info "growing the chain to #$target so the second node's catch-up is a major sync"
for _ in $(seq 1 600); do
  [ "$(best_of "$RPC_URL")" -ge "$target" ] && break
  sleep 1
done
[ "$(best_of "$RPC_URL")" -ge "$target" ] || fail "chain did not reach #$target"

peer="$(rpc_at "$RPC_URL" system_localPeerId | sed -n 's/.*"result":"\([^"]*\)".*/\1/p')"
[ -n "$peer" ] || fail "no peer id from the first node"
SYNC_RPC_URL="http://127.0.0.1:$(( RPC_PORT + 1 ))"
info "booting a syncing node on $SYNC_RPC_URL (archive state)"
"$NODE_BIN" --chain dev --tmp --state-pruning archive --no-mdns \
  --bootnodes "/ip4/127.0.0.1/tcp/$P2P_PORT/p2p/$peer" \
  --rpc-port "$(( RPC_PORT + 1 ))" --port "$(( P2P_PORT + 1 ))" --prometheus-port "$(( PROMETHEUS_PORT + 1 ))" \
  >"$SYNC_LOG" 2>&1 &
SYNC_PID=$!
synced=0
for _ in $(seq 1 300); do
  kill -0 "$SYNC_PID" 2>/dev/null || fail "the syncing node exited"
  if [ "$(best_of "$SYNC_RPC_URL" 2>/dev/null || echo 0)" -ge "$target" ]; then synced=1; break; fi
  sleep 1
done
[ "$synced" = 1 ] || fail "the second node did not sync to #$target"
X3_MODE=verify-synced X3_RPC="$SYNC_RPC_URL" X3_SOURCE_RPC="$RPC_URL" X3_SYNC_FILE="$SYNC_FILE" run_ts \
  || fail "synced node answers differ"
info "PASS"
