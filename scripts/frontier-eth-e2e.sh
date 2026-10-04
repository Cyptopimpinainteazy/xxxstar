#!/usr/bin/env bash
# Boot a `--features frontier` dev node and send real signed Ethereum transactions to it.
#
# tests/wallet-integration/live/frontier-eth-e2e.ts signs with ethers and checks, against the live
# chain, that eth_sendRawTransaction:
#   - includes a signed transfer: sender nonce advances, recipient's native balance is credited,
#     and the returned hash is keccak(raw)
#   - refuses a tampered transaction, a wrong chain id, garbage bytes, the old caller-named
#     payload, and a replay — none of which may move a nonce or a balance
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
NODE_PID=""

cleanup() {
  if [ -n "$NODE_PID" ] && kill -0 "$NODE_PID" 2>/dev/null; then
    kill "$NODE_PID" 2>/dev/null || true
    for _ in $(seq 1 20); do
      kill -0 "$NODE_PID" 2>/dev/null || break
      sleep 0.5
    done
    kill -9 "$NODE_PID" 2>/dev/null || true
  fi
  rm -f "$NODE_LOG"
}
trap cleanup EXIT

info() { printf '[frontier-eth-e2e] %s\n' "$*"; }
fail() { printf '[frontier-eth-e2e] FAIL: %s\n' "$*" >&2; tail -40 "$NODE_LOG" >&2 || true; exit 1; }

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
for _ in $(seq 1 120); do
  if curl -s -m 5 -H 'Content-Type: application/json' \
      -d '{"jsonrpc":"2.0","method":"chain_getHeader","params":[],"id":1}' "$RPC_URL" | grep -q '"number"'; then
    ready=1; break
  fi
  kill -0 "$NODE_PID" 2>/dev/null || fail "the node exited before it answered RPC"
  sleep 1
done
[ "$ready" = 1 ] || fail "the node did not answer RPC within 120s"

info "running the ethers checks"
( cd "$ROOT/tests/wallet-integration" && \
  X3_RPC="$RPC_URL" npx ts-node --transpile-only --compiler-options '{"module":"commonjs"}' \
    live/frontier-eth-e2e.ts ) || fail "ethers checks failed"
info "PASS"
