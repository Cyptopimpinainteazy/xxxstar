#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# make-fixture-live-spec.sh — build a Live spec (production or testnet) from
# fixture keys
#
# One source of truth for "a Live genesis that is real enough to boot", used by
# the gates that need one:
#
#   scripts/mainnet/production_genesis_gate.sh   (builds it, then boots it;
#                                                 X3_GENESIS_CHAIN picks production or testnet)
#   scripts/mainnet/validator_install_gate.sh    (builds it, then installs it)
#
# The fixture keys are constants and are not secrets: they exist only inside one
# run's temporary directory. The same three seeds drive `X3_DEV_SEED` when a gate
# boots a validator, which is what makes the genesis authorities and the node's
# session keys match.
#
# Usage: make-fixture-live-spec.sh <out-dir> [base-port] [chain-id]
#   chain-id: production (default) or testnet
#   writes <out-dir>/x3-<chain-id>-plain.json
#   writes <out-dir>/fixture.json   {spec, seeds, peers, bootnodes, authorities}
# Requires: a built x3-chain-node (X3_NODE_BIN or a target/ path), python3, curl.
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

OUT_DIR="${1:?usage: make-fixture-live-spec.sh <out-dir> [base-port] [chain-id]}"
BASE_PORT="${2:-${X3_PRODUCTION_GENESIS_BASE_PORT:-21100}}"
# Which Live chain to build. `production` and `testnet` are the two built-in Live
# ids; they take the same shape of input under different env prefixes and carry
# different names.
CHAIN_ID="${3:-${X3_FIXTURE_CHAIN:-production}}"
case "$CHAIN_ID" in
  production)
    ENV_PREFIX="X3_PRODUCTION"
    EXPECTED_NAME="X3 Chain Production"
    EXPECTED_ID="x3_chain_production"
    ;;
  testnet)
    ENV_PREFIX="X3_TESTNET"
    EXPECTED_NAME="X3 Chain Testnet"
    EXPECTED_ID="x3_chain_testnet"
    ;;
  *)
    echo "unknown chain id '$CHAIN_ID': expected production or testnet" >&2
    exit 2
    ;;
esac
PLAIN_NAME="$(printf '%s' "$CHAIN_ID" | tr '[:upper:]' '[:lower:]')"
mkdir -p "$OUT_DIR"

info() { printf '[fixture-spec] %s\n' "$*"; }
die() {
  printf '[fixture-spec] FAIL: %s\n' "$*" >&2
  exit 1
}

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
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
  || die "node binary not found; build it with cargo build -p x3-chain-node"
info "node: $NODE_BIN"

SEEDS=(
  0x0101010101010101010101010101010101010101010101010101010101010101
  0x0202020202020202020202020202020202020202020202020202020202020202
  0x0303030303030303030303030303030303030303030303030303030303030303
)

# `keys generate` writes the key to stdout and the banner to stderr.
keygen() { "$NODE_BIN" keys generate --key-type "$1" --seed "$2" --output "$3" 2>/dev/null; }

# A binary whose `keys generate` is still the placeholder prints advice instead
# of a key; without this the derivation dies inside Python with an unhelpful
# `non-hexadecimal number found in fromhex()`.
require_key_output() {
  local value="$1" what="$2" pattern="$3"
  if ! printf '%s' "$value" | grep -Eq "$pattern"; then
    die "$NODE_BIN did not return a $what (got '${value:0:60}').
    This is what a stale binary or a placeholder \`keys generate\` looks like.
    Rebuild it: cargo build --release -p x3-chain-node"
  fi
}

# libp2p peer id for an ed25519 node key. The derivation lives in one place now
# (scripts/mainnet/peer-id-from-ed25519-pubkey.py) because the testnet spec builder
# needs the same value to write `bootNodes`; the node's network identity is the
# ed25519 key built from the same 32 bytes, so this is the peer id it reports — and
# the boot gate asserts exactly that.
peer_id_for() {
  python3 "$ROOT/scripts/mainnet/peer-id-from-ed25519-pubkey.py" "$1"
}

info "deriving three authority keypairs from fixture seeds"
AUTHORITIES="["
ENDOWED="["
COUNCIL="["
TREASURY="["
PEERS=()
for i in 0 1 2; do
  seed="${SEEDS[$i]}"
  aura="$(keygen aura "$seed" ss58)" || die "keys generate (aura) failed"
  grandpa="$(keygen grandpa "$seed" ss58)" || die "keys generate (grandpa) failed"
  ed_hex="$(keygen grandpa "$seed" hex)" || die "keys generate (grandpa hex) failed"
  require_key_output "$aura" "SS58 address for aura" '^[1-9A-HJ-NP-Za-km-z]{47,48}$'
  require_key_output "$grandpa" "SS58 address for grandpa" '^[1-9A-HJ-NP-Za-km-z]{47,48}$'
  require_key_output "$ed_hex" "hex public key for grandpa" '^0x[0-9a-f]{64}$'

  PEERS+=("$(peer_id_for "$ed_hex")")

  [ "$i" -gt 0 ] && AUTHORITIES="$AUTHORITIES," && ENDOWED="$ENDOWED,"
  AUTHORITIES="$AUTHORITIES{\"aura\":\"$aura\",\"grandpa\":\"$grandpa\"}"
  ENDOWED="$ENDOWED\"$aura\""
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

BOOTNODES="/ip4/127.0.0.1/tcp/$(( BASE_PORT + 1 ))/p2p/${PEERS[0]},\
/ip4/127.0.0.1/tcp/$(( BASE_PORT + 2 ))/p2p/${PEERS[1]},\
/ip4/127.0.0.1/tcp/$(( BASE_PORT + 3 ))/p2p/${PEERS[2]}"

export "${ENV_PREFIX}_AUTHORITIES=$AUTHORITIES"
export "${ENV_PREFIX}_ENDOWED_ACCOUNTS=$ENDOWED"
export "${ENV_PREFIX}_COUNCIL_MEMBERS=$COUNCIL"
export "${ENV_PREFIX}_TREASURY_SIGNERS=$TREASURY"
export X3_EVM_ESCROW_ADDR="0x$(printf '11%.0s' $(seq 1 20))"
export X3_SVM_ESCROW_ADDR="0x$(printf '22%.0s' $(seq 1 32))"
export TESTNET_BOOTNODES="$BOOTNODES"

PLAIN="$OUT_DIR/x3-${PLAIN_NAME}-plain.json"
info "building the $CHAIN_ID spec"
"$NODE_BIN" build-spec --chain "$CHAIN_ID" --disable-log-color >"$PLAIN" \
  || die "build-spec --chain $CHAIN_ID failed"
[ -s "$PLAIN" ] || die "build-spec produced an empty file"

python3 - "$PLAIN" "$AUTHORITIES" "$BOOTNODES" "$EXPECTED_NAME" "$EXPECTED_ID" <<'PY' \
  || die "the generated spec is not a valid Live genesis"
import json
import os
import sys

path, authorities_json, bootnodes, expected_name, expected_id = sys.argv[1:6]
raw = open(path, encoding="utf-8").read()

# Parsing the whole file is the point: `build-spec` writes the spec to stdout, so
# a startup banner on stdout (the bug these gates exist to catch) makes this fail
# rather than producing a spec nobody can boot.
spec = json.loads(raw)
assert spec["name"] == expected_name, spec["name"]
assert spec["id"] == expected_id, spec["id"]
assert spec["chainType"] == "Live", spec["chainType"]

for entry in json.loads(authorities_json):
    for key in ("aura", "grandpa"):
        assert entry[key] in raw, f"authority {entry[key]} is missing from the genesis"

boot = spec.get("bootNodes") or []
expected = bootnodes.split(",")
assert boot == expected, f"bootNodes mismatch:\n  spec:     {boot}\n  expected: {expected}"

for forbidden in ("Alice", "Bob", "Charlie", "/Alice", "/Bob", "TestnetAlpha", "ValidatorAlpha"):
    assert forbidden not in raw, f"dev seed marker {forbidden!r} leaked into the {expected_name} genesis"

# A network a validator can join must not accept a cross-domain proof set whose
# underlying proof nothing verified: that is the parameter that decides whether a
# terminal refund can be released against a self-attested bundle.
settlement = (
    spec.get("genesis", {})
    .get("runtimeGenesis", {})
    .get("config", {})
    .get("x3SettlementEngine", {})
)
assert settlement.get("allowUnattestedCrossDomainProofs") is False, (
    "the Live genesis allows unattested cross-domain proof sets; "
    f"x3SettlementEngine = {settlement}"
)

print(f"[fixture-spec] spec ok: {os.path.getsize(path)} bytes, 3 authorities, {len(boot)} bootnodes")
PY

# Machine-readable handoff: the boot gate needs the seeds for X3_DEV_SEED and the
# peer ids for its bootnode assertion; the install gate needs the spec path.
python3 - "$OUT_DIR/fixture.json" "$PLAIN" "$BOOTNODES" "${SEEDS[0]}" "${SEEDS[1]}" "${SEEDS[2]}" \
  "${PEERS[0]}" "${PEERS[1]}" "${PEERS[2]}" "$AUTHORITIES" <<'PY'
import json
import sys

(out, spec, bootnodes, s0, s1, s2, p0, p1, p2, authorities) = sys.argv[1:11]
json.dump(
    {
        "spec": spec,
        "seeds": [s0, s1, s2],
        "peers": [p0, p1, p2],
        "bootnodes": bootnodes.split(","),
        "authorities": json.loads(authorities),
    },
    open(out, "w"),
    indent=2,
)
print(f"[fixture-spec] wrote {out}")
PY

info "done: $PLAIN"
