#!/usr/bin/env bash
# Boot the node on a dev chain and prove it runs.
#
# Everything else in the gate set checks code: it compiles, its tests pass, the
# runtime upgrades. None of it starts a chain. This does — the node authors
# blocks, finality advances, and the operator CLI answers against it over RPC —
# which is the one claim a blockchain release cannot take on faith.
#
#   ./scripts/local-node-smoke.sh
#   X3_NODE_BIN=/path/to/x3-chain-node ./scripts/local-node-smoke.sh
#
# Needs the node binary; builds it (release) when it is not there. No anvil, no
# solana, no network: `--dev` is self-contained.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
RPC_PORT="${X3_SMOKE_RPC_PORT:-$(( 20000 + RANDOM % 10000 ))}"
P2P_PORT="${X3_SMOKE_P2P_PORT:-$(( 31000 + RANDOM % 9000 ))}"
PROMETHEUS_PORT="${X3_SMOKE_PROMETHEUS_PORT:-$(( 40000 + RANDOM % 9000 ))}"
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

info() { printf '[node-smoke] %s\n' "$*"; }
fail() { printf '[node-smoke] FAIL: %s\n' "$*" >&2; tail -25 "$NODE_LOG" >&2 || true; exit 1; }

# ── the binary ───────────────────────────────────────────────────────────────
NODE_BIN="${X3_NODE_BIN:-}"
if [ -z "$NODE_BIN" ]; then
  for candidate in "$TARGET_DIR/release/x3-chain-node" "$ROOT/target/release/x3-chain-node"; do
    [ -x "$candidate" ] && NODE_BIN="$candidate" && break
  done
fi
if [ -z "$NODE_BIN" ]; then
  info "no node binary found; building it (cargo build --release -p x3-chain-node)"
  ( cd "$ROOT" && cargo build --release -p x3-chain-node ) || fail "could not build the node"
  NODE_BIN="$TARGET_DIR/release/x3-chain-node"
fi
[ -x "$NODE_BIN" ] || fail "node binary is not executable: $NODE_BIN"
info "node: $NODE_BIN"

# ── rpc helper ───────────────────────────────────────────────────────────────
rpc() {
  local method="$1" params="${2:-[]}"
  curl -s -m 5 -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"$method\",\"params\":$params,\"id\":1}" \
    "$RPC_URL"
}

# ── boot ─────────────────────────────────────────────────────────────────────
# Every port is pinned: the defaults (9944 RPC, 30333 p2p, 9615 Prometheus)
# collide with any other node on the box, and the run failed with "Address
# already in use" before it ever got to the checks.
info "booting a dev chain on $RPC_URL (p2p $P2P_PORT, prometheus $PROMETHEUS_PORT)"
"$NODE_BIN" --dev --tmp \
  --rpc-port "$RPC_PORT" --port "$P2P_PORT" --prometheus-port "$PROMETHEUS_PORT" \
  >"$NODE_LOG" 2>&1 &
NODE_PID=$!

info "waiting for the RPC endpoint"
ready=0
for _ in $(seq 1 90); do
  if rpc chain_getHeader | grep -q '"number"'; then ready=1; break; fi
  kill -0 "$NODE_PID" 2>/dev/null || fail "the node exited before it answered RPC"
  sleep 1
done
[ "$ready" = 1 ] || fail "the node did not answer chain_getHeader within 90s"

number_of() {  # number_of <params-json> — the header's number, in decimal
  local hex
  hex="$(rpc chain_getHeader "$1" | sed -n 's/.*"number":"\(0x[0-9a-f]*\)".*/\1/p' | head -1)"
  [ -n "$hex" ] || return 1
  printf '%d' "$((hex))"
}

first="$(number_of '[]')"
[ -n "$first" ] || fail "could not read the best block number"
info "best block: $first"

info "waiting for the chain to advance and to finalize"
sleep 12
second="$(number_of '[]')"
finalized_hash="$(rpc chain_getFinalizedHead | sed -n 's/.*"result":"\(0x[0-9a-f]*\)".*/\1/p' | head -1)"
[ -n "$finalized_hash" ] || fail "chain_getFinalizedHead returned nothing"
finalized="$(number_of "[\"$finalized_hash\"]")"
info "best block: $second, finalized: $finalized"

[ "$second" -gt "$first" ] || fail "the chain did not author a block in 12s ($first -> $second)"
[ "$finalized" -ge 1 ] || fail "nothing has been finalized (finalized block: $finalized)"
[ "$finalized" -le "$second" ] || fail "finalized ($finalized) is ahead of best ($second)"

# ── the operator CLI, over the same RPC ──────────────────────────────────────
info "asking the operator CLI for the authority set"
authorities="$("$NODE_BIN" inspect authorities --rpc-url "$RPC_URL" 2>&1 || true)"
echo "$authorities" | grep -q "Total: 1 authorities" \
  || fail "inspect authorities did not report the dev chain's single authority:\n$authorities"

info "asking the operator CLI for asset metadata"
asset="$("$NODE_BIN" inspect asset --asset-id 0 --rpc-url "$RPC_URL" 2>&1 || true)"
echo "$asset" | grep -q "Symbol:   X3" \
  || fail "inspect asset did not report the native asset:\n$asset"

info "PASS — chain authored blocks $first -> $second, finalized at $finalized, CLI answered"
