#!/usr/bin/env bash
# Boot the three-validator local chain and check the validators agree.
#
# `scripts/local-node-smoke.sh` proves one node authors and finalizes on its own.
# A blockchain release needs more than that: independent validators have to
# reach consensus and agree on the same finalized history. This starts the
# node's built-in `local3` chain (Alice, Bob, Charlie), connects them, and then
# asks each node for the canonical block hash at the lowest height any of them
# has finalized. If they disagree, one of them is on a different chain.
#
#   ./scripts/local-network-smoke.sh
#   X3_NODE_BIN=/path/to/x3-chain-node ./scripts/local-network-smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
BASE_DIR="$(mktemp -d)"
LOG_DIR="$(mktemp -d)"
BASE_PORT="${X3_SMOKE_NETWORK_BASE_PORT:-$(( 21000 + RANDOM % 8000 ))}"
NODE_PIDS=()

cleanup() {
  for pid in "${NODE_PIDS[@]:-}"; do
    kill "$pid" 2>/dev/null || true
  done
  sleep 1
  for pid in "${NODE_PIDS[@]:-}"; do
    kill -9 "$pid" 2>/dev/null || true
  done
  rm -rf "$BASE_DIR" "$LOG_DIR"
}
trap cleanup EXIT

info() { printf '[net-smoke] %s\n' "$*"; }
fail() {
  printf '[net-smoke] FAIL: %s\n' "$*" >&2
  for log in "$LOG_DIR"/*.log; do
    [ -f "$log" ] || continue
    printf '[net-smoke] --- %s (tail) ---\n' "$(basename "$log")" >&2
    tail -12 "$log" >&2 || true
  done
  exit 1
}

# ── the binary ───────────────────────────────────────────────────────────────
NODE_BIN="${X3_NODE_BIN:-}"
if [ -z "$NODE_BIN" ]; then
  for candidate in "$TARGET_DIR/release/x3-chain-node" "$ROOT/target/release/x3-chain-node"; do
    [ -x "$candidate" ] && NODE_BIN="$candidate" && break
  done
fi
[ -n "$NODE_BIN" ] && [ -x "$NODE_BIN" ] || fail "node binary not found; build it with cargo build --release -p x3-chain-node"
info "node: $NODE_BIN"

rpc() {  # rpc <port> <method> [params]
  curl -s -m 5 -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"$2\",\"params\":${3:-[]},\"id\":1}" \
    "http://127.0.0.1:$1"
}

json_field() { sed -n "s/.*\"$1\":\"\([^\"]*\)\".*/\1/p" | head -1; }

node_key_for() {  # a deterministic 32-byte libp2p key per validator
  printf '%064x' "$1"
}

start_node() {  # start_node <name> <key-seed> <rpc-port> <p2p-port> <prom-port> [extra args...]
  local name="$1" key_seed="$2" rpc_port="$3" p2p_port="$4" prom_port="$5"; shift 5
  info "starting $name (rpc $rpc_port, p2p $p2p_port)"
  # No `--tmp` here: each validator gets its own base path under a scratch
  # directory, and cargo-clap refuses `--tmp` alongside `--base-path`. The
  # `--alice`/`--bob`/`--charlie` shortcuts add *session* keys only — without an
  # explicit `--node-key` the node fails with
  # `NetworkKeyNotFound(.../network/secret_ed25519)`.
  "$NODE_BIN" --chain local3 --base-path "$BASE_DIR/$name" \
    --rpc-port "$rpc_port" --port "$p2p_port" --prometheus-port "$prom_port" \
    --no-mdns --node-key "$(node_key_for "$key_seed")" "$@" >"$LOG_DIR/$name.log" 2>&1 &
  NODE_PIDS+=("$!")
}

wait_for_rpc() {  # wait_for_rpc <name> <rpc-port> <seconds>
  local name="$1" port="$2" tries="${3:-90}"
  for _ in $(seq 1 "$tries"); do
    if rpc "$port" chain_getHeader | grep -q '"number"'; then
      info "$name answered RPC"
      return 0
    fi
    sleep 1
  done
  fail "$name never answered chain_getHeader"
}

hex_to_dec() { printf '%d' "$(( $1 ))"; }

best_number() {  # best_number <rpc-port>
  local hex
  hex="$(rpc "$1" chain_getHeader | json_field number)"
  [ -n "$hex" ] || return 1
  hex_to_dec "$hex"
}

finalized_number() {  # finalized_number <rpc-port>
  local hash
  hash="$(rpc "$1" chain_getFinalizedHead | json_field result)"
  [ -n "$hash" ] || return 1
  local hex
  hex="$(rpc "$1" chain_getHeader "[\"$hash\"]" | json_field number)"
  [ -n "$hex" ] || return 1
  hex_to_dec "$hex"
}

# ── start the validators ─────────────────────────────────────────────────────
A_RPC=$(( BASE_PORT + 1 )); A_P2P=$(( BASE_PORT + 2 )); A_PROM=$(( BASE_PORT + 3 ))
B_RPC=$(( BASE_PORT + 11 )); B_P2P=$(( BASE_PORT + 12 )); B_PROM=$(( BASE_PORT + 13 ))
C_RPC=$(( BASE_PORT + 21 )); C_P2P=$(( BASE_PORT + 22 )); C_PROM=$(( BASE_PORT + 23 ))

start_node alice 1 "$A_RPC" "$A_P2P" "$A_PROM" --alice
wait_for_rpc alice "$A_RPC"

alice_peer_id="$(rpc "$A_RPC" system_localPeerId | json_field result)"
[ -n "$alice_peer_id" ] || fail "could not read Alice's peer id"
alice_bootnode="/ip4/127.0.0.1/tcp/$A_P2P/p2p/$alice_peer_id"
info "Alice is $alice_peer_id"

start_node bob 2 "$B_RPC" "$B_P2P" "$B_PROM" --bob --bootnodes "$alice_bootnode"
wait_for_rpc bob "$B_RPC"

bob_peer_id="$(rpc "$B_RPC" system_localPeerId | json_field result)"
bob_bootnode="/ip4/127.0.0.1/tcp/$B_P2P/p2p/$bob_peer_id"
start_node charlie 3 "$C_RPC" "$C_P2P" "$C_PROM" --charlie \
  --bootnodes "$alice_bootnode" "$bob_bootnode"
wait_for_rpc charlie "$C_RPC"

# ── they have to find each other ─────────────────────────────────────────────
info "waiting for the three validators to connect"
connected=0
for _ in $(seq 1 60); do
  peers_a="$(rpc "$A_RPC" system_health | sed -n 's/.*"peers":\([0-9]*\).*/\1/p' | head -1)"
  peers_b="$(rpc "$B_RPC" system_health | sed -n 's/.*"peers":\([0-9]*\).*/\1/p' | head -1)"
  peers_c="$(rpc "$C_RPC" system_health | sed -n 's/.*"peers":\([0-9]*\).*/\1/p' | head -1)"
  if [ "${peers_a:-0}" -ge 2 ] && [ "${peers_b:-0}" -ge 2 ] && [ "${peers_c:-0}" -ge 2 ]; then
    connected=1
    break
  fi
  sleep 1
done
[ "$connected" = 1 ] || fail "the validators did not connect (peers: alice=${peers_a:-?} bob=${peers_b:-?} charlie=${peers_c:-?})"
info "all three are connected (peers: $peers_a/$peers_b/$peers_c)"

# ── they have to agree on finalized history ──────────────────────────────────
info "waiting for finality"
lowest=0
for _ in $(seq 1 60); do
  fa="$(finalized_number "$A_RPC" || echo 0)"
  fb="$(finalized_number "$B_RPC" || echo 0)"
  fc="$(finalized_number "$C_RPC" || echo 0)"
  lowest=$(( fa < fb ? (fa < fc ? fa : fc) : (fb < fc ? fb : fc) ))
  if [ "$lowest" -ge 3 ]; then break; fi
  sleep 1
done
[ "$lowest" -ge 3 ] || fail "finality did not advance (alice=$fa bob=$fb charlie=$fc)"
info "all three finalized at least block $lowest (alice=$fa bob=$fb charlie=$fc)"

best_a="$(best_number "$A_RPC")"
best_b="$(best_number "$B_RPC")"
best_c="$(best_number "$C_RPC")"
[ "$best_a" -gt "$lowest" ] || fail "Alice's best block ($best_a) is not past the finalized height ($lowest)"
info "best blocks: alice=$best_a bob=$best_b charlie=$best_c"

hash_a="$(rpc "$A_RPC" chain_getBlockHash "[\"$lowest\"]" | json_field result)"
hash_b="$(rpc "$B_RPC" chain_getBlockHash "[\"$lowest\"]" | json_field result)"
hash_c="$(rpc "$C_RPC" chain_getBlockHash "[\"$lowest\"]" | json_field result)"
if [ -z "$hash_a" ] || [ -z "$hash_b" ] || [ -z "$hash_c" ]; then
  # Heights are decimal in the JSON-RPC, but a few clients want hex.
  hex_height="$(printf '0x%x' "$lowest")"
  hash_a="$(rpc "$A_RPC" chain_getBlockHash "[\"$hex_height\"]" | json_field result)"
  hash_b="$(rpc "$B_RPC" chain_getBlockHash "[\"$hex_height\"]" | json_field result)"
  hash_c="$(rpc "$C_RPC" chain_getBlockHash "[\"$hex_height\"]" | json_field result)"
fi
[ -n "$hash_a" ] || fail "could not read the block hash at height $lowest"

if [ "$hash_a" != "$hash_b" ] || [ "$hash_a" != "$hash_c" ]; then
  fail "the validators disagree at finalized height $lowest:
    alice:   $hash_a
    bob:     $hash_b
    charlie: $hash_c"
fi

info "PASS — 3 validators, finalized height $lowest, all agree on $hash_a"
