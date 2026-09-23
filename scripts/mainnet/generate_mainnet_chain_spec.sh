#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# generate_mainnet_chain_spec.sh — build the mainnet (Live) genesis
#
# `production_config()` is env-gated on purpose: it refuses to build a Live
# genesis without the real authority set, the endowed/council/treasury accounts
# and non-zero escrow addresses, and it refuses any seed that names a dev
# account. This script used to call `build-spec --chain production` with no env
# at all, so it could only ever fail — and with `> file` it left two empty files
# behind first.
#
# Endpoints for each variable are produced by the node's own CLI:
#
#   x3-chain-node keys generate --key-type aura    --seed <SURI>   # sr25519
#   x3-chain-node keys generate --key-type grandpa --seed <SURI>   # ed25519
#
# `X3_PRODUCTION_AUTHORITIES` is a JSON array of `{"aura": <ss58>,
# "grandpa": <ss58>}` built from those two commands, one entry per validator.
# Generate the keys offline on an air-gapped machine and never commit the seeds.
#
# `X3_PRODUCTION_ATOMIC_GATEWAYS` and `X3_PRODUCTION_SETTLEMENT_GATEWAYS` are the
# accounts the custody registry authorizes for the atomic kernel's privileged
# origins. They are required and must be the operator's own accounts: the spec
# builder refuses the published development seeds (`//x3-atomic-gateway`,
# `//x3-settlement-gateway`), which used to be the runtime's origins themselves.
#
# To prove the mechanism without real keys:
#   bash scripts/mainnet/production_genesis_gate.sh
#
# Usage:
#   X3_PRODUCTION_AUTHORITIES='[...]' \
#   X3_PRODUCTION_ENDOWED_ACCOUNTS='[...]' \
#   X3_PRODUCTION_COUNCIL_MEMBERS='[...]' \
#   X3_PRODUCTION_TREASURY_SIGNERS='[...]' \
#   X3_PRODUCTION_ATOMIC_GATEWAYS='["<ss58>"]' \
#   X3_PRODUCTION_SETTLEMENT_GATEWAYS='["<ss58>"]' \
#   X3_EVM_ESCROW_ADDR=0x... X3_SVM_ESCROW_ADDR=0x... \
#   TESTNET_BOOTNODES='/ip4/.../p2p/12D3Koo...' \
#   bash scripts/mainnet/generate_mainnet_chain_spec.sh
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="${X3_MAINNET_SPEC_OUT_DIR:-$ROOT_DIR/chain-specs}"
mkdir -p "$OUT_DIR"

cd "$ROOT_DIR"

REQUIRED_VARS=(
  X3_PRODUCTION_AUTHORITIES
  X3_PRODUCTION_ENDOWED_ACCOUNTS
  X3_PRODUCTION_COUNCIL_MEMBERS
  X3_PRODUCTION_TREASURY_SIGNERS
  X3_PRODUCTION_ATOMIC_GATEWAYS
  X3_PRODUCTION_SETTLEMENT_GATEWAYS
  X3_EVM_ESCROW_ADDR
  X3_SVM_ESCROW_ADDR
)

missing=()
for var in "${REQUIRED_VARS[@]}"; do
  [ -n "${!var:-}" ] || missing+=("$var")
done
if [ "${#missing[@]}" -gt 0 ]; then
  cat >&2 <<EOF
refusing to build a mainnet spec: ${#missing[@]} required variable(s) unset
  ${missing[*]}

production_config() will not invent an authority set, an endowed account or an
escrow address, and neither will this script. See the header of this file for
the exact shapes, and run scripts/mainnet/production_genesis_gate.sh to exercise
the same path with fixture keys.
EOF
  exit 1
fi

if [ -z "${TESTNET_BOOTNODES:-}" ] && [ ! -f "$ROOT_DIR/deployment/keys/bootnode-info.txt" ]; then
  echo "refusing to build a mainnet spec with no bootnodes:" >&2
  echo "  set TESTNET_BOOTNODES to a comma-separated /ip4/.../p2p/<peerid> list," >&2
  echo "  or provide deployment/keys/bootnode-info.txt" >&2
  exit 1
fi

# Release build by default. `X3_NODE_BIN` is an explicit operator choice (the
# gate uses it to test the mechanism without a 10-minute release build); it is
# printed so a ceremony transcript always records which binary produced the
# genesis.
if [ -z "${X3_NODE_BIN:-}" ]; then
  cargo build --release -p x3-chain-node
fi

NODE_BIN="${X3_NODE_BIN:-$ROOT_DIR/target/release/x3-chain-node}"
[ -x "$NODE_BIN" ] || { echo "node binary not found at $NODE_BIN" >&2; exit 1; }
echo "[spec] using node binary: $NODE_BIN"

PLAIN="$OUT_DIR/x3-mainnet-plain.json"
RAW="$OUT_DIR/x3-mainnet-raw.json"

# `build-spec` writes the spec to stdout, so it is written to a temporary file
# first: a failure must not leave a truncated or empty spec where the launch
# ceremony expects a genesis.
tmp_plain="$(mktemp "$OUT_DIR/.x3-mainnet-plain.XXXXXX.json")"
tmp_raw="$(mktemp "$OUT_DIR/.x3-mainnet-raw.XXXXXX.json")"
trap 'rm -f "$tmp_plain" "$tmp_raw"' EXIT

"$NODE_BIN" build-spec --chain production --disable-log-color >"$tmp_plain"
"$NODE_BIN" build-spec --chain "$tmp_plain" --raw --disable-log-color >"$tmp_raw"

# Both artifacts have to be parseable JSON with the mainnet identity, or the
# only thing this script produced is a file with the right name.
python3 - "$tmp_plain" "$tmp_raw" <<'PY'
import json
import sys

for path in sys.argv[1:]:
    with open(path, encoding="utf-8") as handle:
        spec = json.load(handle)
    assert spec["id"] == "x3_chain_production", f"{path}: id is {spec['id']!r}"
    assert spec["chainType"] == "Live", f"{path}: chainType is {spec['chainType']!r}"
    assert spec.get("bootNodes"), f"{path}: no bootnodes"
    print(f"[spec] {path}: {spec['name']} ({spec['chainType']})")
PY

mv "$tmp_plain" "$PLAIN"
mv "$tmp_raw" "$RAW"

echo "generated: $PLAIN"
echo "generated: $RAW"
echo
echo "record these hashes in the genesis ceremony (launch-gates/GENESIS_CEREMONY_CHECKLIST.md):"
sha256sum "$PLAIN" "$RAW"
echo
echo "next: bash scripts/mainnet/genesis_lint.sh"
