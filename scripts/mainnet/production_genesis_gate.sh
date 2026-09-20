#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# production_genesis_gate.sh — prove a Live genesis path is real
#
# `production_config()` / `testnet_config()` build the Live chain specs and refuse to run
# without X3_PRODUCTION_AUTHORITIES, the endowed/council/treasury accounts and
# non-zero escrow addresses. `scripts/mainnet/generate_mainnet_chain_spec.sh`
# called it with no env at all, so the documented mainnet genesis could never be
# produced; and the release gate only grepped `chain_spec.rs` for the string
# "production_config" — nothing ever loaded, let alone booted, a production spec.
#
# This gate does the whole path, with fixture keys and no secrets:
#
#   1. derive three authority keypairs with the node's own `keys generate`
#      (deterministic seeds: this is a fixture, and it is the same command an
#      operator uses with their own seeds)
#   2. build the spec from the X3_PRODUCTION_* / X3_TESTNET_* env
#   3. assert the artifact: parses as JSON, Live, the expected id,
#      every authority we supplied is present, the three bootnodes are exactly
#      the peer ids derived from the fixture node keys, and no dev seed leaked in
#   4. boot the three validators on that spec and require finalized height >= 3
#      with all three agreeing on the canonical hash — a spec that parses but
#      cannot start a chain fails here
#
# Usage: bash scripts/mainnet/production_genesis_gate.sh                 # production
#        X3_GENESIS_CHAIN=testnet bash scripts/mainnet/production_genesis_gate.sh
#        X3_NODE_BIN=/path/to/x3-chain-node bash scripts/mainnet/production_genesis_gate.sh
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
BASE_DIR="$(mktemp -d)"
BASE_PORT="${X3_PRODUCTION_GENESIS_BASE_PORT:-21100}"
# Which built-in Live chain to prove: production (mainnet) or testnet.
GENESIS_CHAIN="${X3_GENESIS_CHAIN:-production}"
NODE_PIDS=()

cleanup() {
  for pid in "${NODE_PIDS[@]:-}"; do
    kill "$pid" 2>/dev/null || true
  done
  sleep 1
  for pid in "${NODE_PIDS[@]:-}"; do
    kill -9 "$pid" 2>/dev/null || true
  done
  rm -rf "$BASE_DIR"
}
trap cleanup EXIT

info() { printf '[genesis-gate] %s\n' "$*"; }
fail() {
  printf '[genesis-gate] FAIL: %s\n' "$*" >&2
  for log in "$BASE_DIR"/*.log; do
    [ -f "$log" ] || continue
    printf '[genesis-gate] --- %s (tail) ---\n' "$(basename "$log")" >&2
    tail -15 "$log" >&2 || true
  done
  exit 1
}

# ── 1. the binary and the genesis ────────────────────────────────────────────
NODE_BIN="${X3_NODE_BIN:-}"
if [ -z "$NODE_BIN" ]; then
  for candidate in \
    "$TARGET_DIR/release/x3-chain-node" \
    "$TARGET_DIR/debug/x3-chain-node" \
    "$ROOT/target/release/x3-chain-node" \
    "$ROOT/target/debug/x3-chain-node"; do
    if [ -x "$candidate" ]; then NODE_BIN="$candidate"; break; fi
  done
fi
[ -n "$NODE_BIN" ] && [ -x "$NODE_BIN" ] \
  || fail "node binary not found; build it with cargo build -p x3-chain-node"
info "node: $NODE_BIN"

# Building the spec (and asserting it is a Live, mainnet-shaped genesis) lives in
# one place, shared with scripts/mainnet/validator_install_gate.sh.
X3_NODE_BIN="$NODE_BIN" bash "$ROOT/scripts/mainnet/make-fixture-live-spec.sh" \
  "$BASE_DIR" "$BASE_PORT" "$GENESIS_CHAIN" || fail "could not build the fixture $GENESIS_CHAIN genesis"

FIXTURE="$BASE_DIR/fixture.json"
PLAIN="$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['spec'])" "$FIXTURE")"
BOOTNODES="$(python3 -c "import json,sys; print(','.join(json.load(open(sys.argv[1]))['bootnodes']))" "$FIXTURE")"
PEER_IDS=($(python3 -c "import json,sys; print(' '.join(json.load(open(sys.argv[1]))['peers']))" "$FIXTURE"))
NODE_KEYS=($(python3 -c "import json,sys; print(' '.join(s.removeprefix('0x') for s in json.load(open(sys.argv[1]))['seeds']))" "$FIXTURE"))
SEEDS=($(python3 -c "import json,sys; print(' '.join(json.load(open(sys.argv[1]))['seeds']))" "$FIXTURE"))
info "bootnodes: $BOOTNODES"

A_RPC=$(( BASE_PORT + 101 ))
B_RPC=$(( BASE_PORT + 102 ))
C_RPC=$(( BASE_PORT + 103 ))
A_P2P=$(( BASE_PORT + 1 ))
B_P2P=$(( BASE_PORT + 2 ))
C_P2P=$(( BASE_PORT + 3 ))

# ── 4. boot the network on the generated genesis ─────────────────────────────
rpc() {
  curl -s -m 5 -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"$2\",\"params\":${3:-[]},\"id\":1}" \
    "http://127.0.0.1:$1"
}
json_field() { sed -n "s/.*\"$1\":\"\([^\"]*\)\".*/\1/p" | head -1; }
hex_to_dec() { printf '%d' "$(( $1 ))"; }

start_validator() {  # start_validator <name> <index> <rpc> <p2p> [extra args...]
  local name="$1" index="$2" rpc_port="$3" p2p_port="$4"; shift 4
  info "starting $name (rpc $rpc_port, p2p $p2p_port)"
  # X3_DEV_SEED per process: the node inserts that seed's Aura (sr25519) and
  # GRANDPA (ed25519) keys, which are exactly the authorities this fixture put
  # in the genesis for validator <index>.
  X3_DEV_SEED="${SEEDS[$index]}" "$NODE_BIN" \
    --chain "$PLAIN" --base-path "$BASE_DIR/$name" \
    --validator --force-authoring \
    --node-key "${NODE_KEYS[$index]}" \
    --port "$p2p_port" --rpc-port "$rpc_port" --prometheus-port "$(( p2p_port + 200 ))" \
    --no-mdns --no-telemetry --no-prometheus \
    "$@" >"$BASE_DIR/$name.log" 2>&1 &
  NODE_PIDS+=("$!")
}

wait_for_rpc() {  # wait_for_rpc <name> <rpc-port> <tries>
  local name="$1" port="$2" tries="${3:-180}"
  for _ in $(seq 1 "$tries"); do
    if rpc "$port" chain_getHeader | grep -q '"number"'; then
      info "$name answered RPC"
      return 0
    fi
    sleep 1
  done
  fail "$name never answered chain_getHeader (a spec that cannot boot is a launch blocker)"
}

finalized_number() {  # finalized_number <rpc-port>
  local hash hex
  hash="$(rpc "$1" chain_getFinalizedHead | json_field result)"
  [ -n "$hash" ] || return 1
  hex="$(rpc "$1" chain_getHeader "[\"$hash\"]" | json_field number)"
  [ -n "$hex" ] || return 1
  hex_to_dec "$hex"
}

start_validator alpha 0 "$A_RPC" "$A_P2P"
wait_for_rpc alpha "$A_RPC"

# The spec's bootnode entry for this validator must be the node we just started:
# that is what makes the generated bootNodes real rather than decorative.
alpha_peer="$(rpc "$A_RPC" system_localPeerId | json_field result)"
[ "$alpha_peer" = "${PEER_IDS[0]}" ] \
  || fail "the spec's bootnode for validator 1 is ${PEER_IDS[0]} but the node reports $alpha_peer"
info "validator 1 peer id matches the spec's bootnode entry"

# No --bootnodes here on purpose: the nodes must find each other through the
# bootNodes the generated spec carries.
start_validator beta 1 "$B_RPC" "$B_P2P"
start_validator gamma 2 "$C_RPC" "$C_P2P"
wait_for_rpc beta "$B_RPC"
wait_for_rpc gamma "$C_RPC"

info "waiting for the three validators to connect"
connected=0
for _ in $(seq 1 90); do
  pa="$(rpc "$A_RPC" system_health | sed -n 's/.*"peers":\([0-9]*\).*/\1/p' | head -1)"
  pb="$(rpc "$B_RPC" system_health | sed -n 's/.*"peers":\([0-9]*\).*/\1/p' | head -1)"
  pc="$(rpc "$C_RPC" system_health | sed -n 's/.*"peers":\([0-9]*\).*/\1/p' | head -1)"
  if [ "${pa:-0}" -ge 2 ] && [ "${pb:-0}" -ge 2 ] && [ "${pc:-0}" -ge 2 ]; then
    connected=1
    break
  fi
  sleep 1
done
[ "$connected" = 1 ] || fail "the validators did not connect (peers: a=${pa:-?} b=${pb:-?} c=${pc:-?})"
info "all three connected (peers: $pa/$pb/$pc)"

info "waiting for finality on the generated genesis"
lowest=0
for _ in $(seq 1 120); do
  fa="$(finalized_number "$A_RPC" || echo 0)"
  fb="$(finalized_number "$B_RPC" || echo 0)"
  fc="$(finalized_number "$C_RPC" || echo 0)"
  lowest=$(( fa < fb ? (fa < fc ? fa : fc) : (fb < fc ? fb : fc) ))
  if [ "$lowest" -ge 3 ]; then break; fi
  sleep 1
done
[ "$lowest" -ge 3 ] || fail "the $GENESIS_CHAIN genesis did not reach finality (a=$fa b=$fb c=$fc)"
info "all three finalized at least block $lowest (a=$fa b=$fb c=$fc)"

hex_height="$(printf '0x%x' "$lowest")"
hash_a="$(rpc "$A_RPC" chain_getBlockHash "[\"$hex_height\"]" | json_field result)"
hash_b="$(rpc "$B_RPC" chain_getBlockHash "[\"$hex_height\"]" | json_field result)"
hash_c="$(rpc "$C_RPC" chain_getBlockHash "[\"$hex_height\"]" | json_field result)"
[ -n "$hash_a" ] || fail "could not read the block hash at height $lowest"
if [ "$hash_a" != "$hash_b" ] || [ "$hash_a" != "$hash_c" ]; then
  fail "the validators disagree at finalized height $lowest:
    alpha: $hash_a
    beta:  $hash_b
    gamma: $hash_c"
fi

info "PASS — $GENESIS_CHAIN genesis builds, boots, finalizes and agrees at height $lowest ($hash_a)"
