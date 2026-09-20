#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# production_genesis_gate.sh — prove the mainnet genesis path is real
#
# `production_config()` builds the Live (mainnet) chain spec and refuses to run
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
#   2. build the production spec from the X3_PRODUCTION_* env
#   3. assert the artifact: parses as JSON, Live, id `x3_chain_production`,
#      every authority we supplied is present, the three bootnodes are exactly
#      the peer ids derived from the fixture node keys, and no dev seed leaked in
#   4. boot the three validators on that spec and require finalized height >= 3
#      with all three agreeing on the canonical hash — a spec that parses but
#      cannot start a chain fails here
#
# Usage: bash scripts/mainnet/production_genesis_gate.sh
#        X3_NODE_BIN=/path/to/x3-chain-node bash scripts/mainnet/production_genesis_gate.sh
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
BASE_DIR="$(mktemp -d)"
BASE_PORT="${X3_PRODUCTION_GENESIS_BASE_PORT:-21100}"
NODE_PIDS=()

# Fixture seeds. Deliberately not dev aliases (production_config refuses those)
# and deliberately constant so the gate is reproducible. They are not secrets:
# they exist only inside this run's temporary directory.
SEED_1=0x0101010101010101010101010101010101010101010101010101010101010101
SEED_2=0x0202020202020202020202020202020202020202020202020202020202020202
SEED_3=0x0303030303030303030303030303030303030303030303030303030303030303
SEEDS=("$SEED_1" "$SEED_2" "$SEED_3")

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

# ── the binary ───────────────────────────────────────────────────────────────
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
[ -n "$NODE_BIN" ] && [ -x "$NODE_BIN" ] || fail "node binary not found; build it with cargo build -p x3-chain-node"
info "node: $NODE_BIN"

# `keys generate` writes the public key to stdout and the banner/advice to
# stderr, so `--output ss58` is pipeable.
keygen() { "$NODE_BIN" keys generate --key-type "$1" --seed "$2" --output "$3" 2>/dev/null; }

# libp2p peer id for an ed25519 node key: base58btc(0x00 0x24 || protobuf(ed25519 pub)).
# The node's network identity is the ed25519 key built from the same 32 bytes,
# so this is the peer id the node will report — and check 4 asserts that.
peer_id_for() {
  local pub_hex="$1"
  python3 - "$pub_hex" <<'PY'
import sys
ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
def b58(data: bytes) -> str:
    n = int.from_bytes(data, "big")
    out = ""
    while n:
        n, r = divmod(n, 58)
        out = ALPHABET[r] + out
    pad = 0
    for byte in data:
        if byte == 0:
            pad += 1
        else:
            break
    return "1" * pad + out
pub = bytes.fromhex(sys.argv[1].removeprefix("0x"))
assert len(pub) == 32, "ed25519 public key must be 32 bytes"
print(b58(bytes([0x00, 0x24, 0x08, 0x01, 0x12, 0x20]) + pub))
PY
}

info "deriving three authority keypairs from fixture seeds"
AUTHORITIES="["
ENDOWED="["
COUNCIL="["
TREASURY="["
PEER_IDS=()
NODE_KEYS=()
for i in 0 1 2; do
  seed="${SEEDS[$i]}"
  aura="$(keygen aura "$seed" ss58)" || fail "keys generate (aura) failed"
  grandpa="$(keygen grandpa "$seed" ss58)" || fail "keys generate (grandpa) failed"
  ed_hex="$(keygen grandpa "$seed" hex)" || fail "keys generate (grandpa hex) failed"
  [ -n "$aura" ] && [ -n "$grandpa" ] || fail "keys generate produced an empty key"

  PEER_IDS+=("$(peer_id_for "$ed_hex")")
  NODE_KEYS+=("${seed#0x}")

  [ "$i" -gt 0 ] && AUTHORITIES="$AUTHORITIES," && ENDOWED="$ENDOWED,"
  AUTHORITIES="$AUTHORITIES{\"aura\":\"$aura\",\"grandpa\":\"$grandpa\"}"
  ENDOWED="$ENDOWED\"$aura\""
  # A live council needs at least two members; the treasury needs at least one.
  if [ "$i" -lt 2 ]; then
    [ "$i" -gt 0 ] && COUNCIL="$COUNCIL,"
    COUNCIL="$COUNCIL\"$aura\""
  fi
  [ "$i" -gt 0 ] && TREASURY="$TREASURY,"
  TREASURY="$TREASURY\"$aura\""
done
AUTHORITIES="$AUTHORITIES]"
ENDOWED="$ENDOWED]"
COUNCIL="$COUNCIL]"
TREASURY="$TREASURY]"

A_P2P=$(( BASE_PORT + 1 ))   # 21101
B_P2P=$(( BASE_PORT + 2 ))   # 21102
C_P2P=$(( BASE_PORT + 3 ))   # 21103
A_RPC=$(( BASE_PORT + 101 ))
B_RPC=$(( BASE_PORT + 102 ))
C_RPC=$(( BASE_PORT + 103 ))

BOOTNODES="/ip4/127.0.0.1/tcp/$A_P2P/p2p/${PEER_IDS[0]},/ip4/127.0.0.1/tcp/$B_P2P/p2p/${PEER_IDS[1]},/ip4/127.0.0.1/tcp/$C_P2P/p2p/${PEER_IDS[2]}"
info "bootnodes: $BOOTNODES"

# ── 2. build the production spec ─────────────────────────────────────────────
export X3_PRODUCTION_AUTHORITIES="$AUTHORITIES"
export X3_PRODUCTION_ENDOWED_ACCOUNTS="$ENDOWED"
export X3_PRODUCTION_COUNCIL_MEMBERS="$COUNCIL"
export X3_PRODUCTION_TREASURY_SIGNERS="$TREASURY"
export X3_EVM_ESCROW_ADDR="0x$(printf '11%.0s' $(seq 1 20))"
export X3_SVM_ESCROW_ADDR="0x$(printf '22%.0s' $(seq 1 32))"
export TESTNET_BOOTNODES="$BOOTNODES"

PLAIN="$BASE_DIR/x3-production-plain.json"
info "building the production spec"
"$NODE_BIN" build-spec --chain production --disable-log-color >"$PLAIN" \
  || fail "build-spec --chain production failed"
[ -s "$PLAIN" ] || fail "build-spec produced an empty file"

# ── 3. assert the artifact ───────────────────────────────────────────────────
info "asserting the generated spec"
python3 - "$PLAIN" "$AUTHORITIES" "$BOOTNODES" <<'PY' || fail "the generated spec is not a valid mainnet genesis"
import json
import os
import sys

path, authorities_json, bootnodes = sys.argv[1], sys.argv[2], sys.argv[3]
raw = open(path, encoding="utf-8").read()

# Parsing the whole file is the point: `build-spec` writes the spec to stdout,
# so a startup banner on stdout (the bug this gate exists to catch) makes this
# json.loads fail rather than producing a spec nobody can boot.
spec = json.loads(raw)

assert spec["name"] == "X3 Chain Production", spec["name"]
assert spec["id"] == "x3_chain_production", spec["id"]
assert spec["chainType"] == "Live", spec["chainType"]

authorities = json.loads(authorities_json)
for entry in authorities:
    for key in ("aura", "grandpa"):
        assert entry[key] in raw, f"authority {entry[key]} is missing from the genesis"

boot = spec.get("bootNodes") or []
expected = bootnodes.split(",")
assert boot == expected, f"bootNodes mismatch:\n  spec:     {boot}\n  expected: {expected}"

for forbidden in ("Alice", "Bob", "Charlie", "/Alice", "/Bob", "TestnetAlpha", "ValidatorAlpha"):
    assert forbidden not in raw, f"dev seed marker {forbidden!r} leaked into the production genesis"

print(f"[genesis-gate] spec ok: {os.path.getsize(path)} bytes, {len(authorities)} authorities, {len(boot)} bootnodes")
PY

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
[ "$lowest" -ge 3 ] || fail "the production genesis did not reach finality (a=$fa b=$fb c=$fc)"
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

info "PASS — production genesis builds, boots, finalizes and agrees at height $lowest ($hash_a)"
