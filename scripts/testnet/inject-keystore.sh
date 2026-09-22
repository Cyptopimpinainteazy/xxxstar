#!/usr/bin/env bash
# inject-keystore.sh <base-path> <validator-index> [chain-id]
# Injects aura(sr25519)+grandpa(ed25519) session keys for fresh validator $index
# into <base-path>/chains/<chain-id>/keystore using the stored SURI.
#
# The keystore is written by the node itself (`keys insert`), which is the same code
# path that reads it at startup: it derives the public key, names the file
# `<key-type-hex><public-key-hex>` and stores the secret. This script used to shell
# out to `subkey` and hand-write those files — `subkey` is not installed on the build
# boxes and is not part of this repository, so the documented fresh-validator path
# could not run at all.
#
# Measured 2026-09-22: keys injected this way DO drive Aura authoring — a node
# started with `--validator --force-authoring`, no `X3_DEV_SEED`, and only these
# keystore files authored from the first slot. (`run-fresh-validators.sh` still
# carries an older note claiming the opposite; the dev-seed path is a convenience,
# not a requirement.)
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BASE="${1:?base-path}"; IDX="${2:?validator-index}"
# The spec builder writes to `fresh/generated/` (ignored by git); the tracked
# `fresh/x3-testnet-plain.json` is a shared fixture. Prefer the generated one.
SPEC="${X3_SPEC:-}"
if [[ -z "$SPEC" ]]; then
  for candidate in \
    "$ROOT/deployment/chain-specs/fresh/generated/x3-testnet-plain.json" \
    "$ROOT/deployment/chain-specs/fresh/x3-testnet-plain.json"; do
    [[ -f "$candidate" ]] && SPEC="$candidate" && break
  done
fi
[[ -n "$SPEC" && -f "$SPEC" ]] || { echo "no chain spec found; set X3_SPEC" >&2; exit 1; }
CHAIN_ID="${3:-$(python3 -c "import json;print(json.load(open('$SPEC'))['id'])")}"
SURI_FILE="$ROOT/deployment/chain-specs/fresh/validator-keys/validator-$IDX.suri"
[[ -s "$SURI_FILE" ]] || {
  echo "no seed file for validator $IDX at $SURI_FILE" >&2
  echo "build one with: python3 scripts/testnet/build-x3-testnet-spec.py <count>" >&2
  exit 1
}

NODE_BIN="${NODE_BIN:-}"
if [[ -z "$NODE_BIN" ]]; then
  for candidate in \
    "${CARGO_TARGET_DIR:-$ROOT/target}/release/x3-chain-node" \
    "${CARGO_TARGET_DIR:-$ROOT/target}/debug/x3-chain-node" \
    "$ROOT/target/release/x3-chain-node" \
    "$ROOT/target/debug/x3-chain-node"; do
    [[ -x "$candidate" ]] && NODE_BIN="$candidate" && break
  done
fi
[[ -n "$NODE_BIN" && -x "$NODE_BIN" ]] || {
  echo "node binary not found; build it with cargo build -p x3-chain-node" >&2
  exit 1
}

kdir="$BASE/chains/$CHAIN_ID/keystore"; mkdir -p "$kdir"
AURA="$(grep '^aura=' "$SURI_FILE" | cut -d= -f2)"
GRAN="$(grep '^grandpa=' "$SURI_FILE" | cut -d= -f2)"
# `seed=` is the builder's field name; fall back to it so either format works.
if [[ -z "$AURA" ]]; then AURA="$(grep '^seed=' "$SURI_FILE" | cut -d= -f2)"; fi
if [[ -z "$GRAN" ]]; then GRAN="$AURA"; fi
[[ -n "$AURA" && -n "$GRAN" ]] || { echo "$SURI_FILE has no aura/grandpa/seed line" >&2; exit 1; }

apub="$("$NODE_BIN" keys insert --key-type aura --seed "$AURA" --keystore-path "$kdir")"
gpub="$("$NODE_BIN" keys insert --key-type grandpa --seed "$GRAN" --keystore-path "$kdir")"
echo "injected validator-$IDX session keys into $kdir"
echo "  aura:    $apub"
echo "  grandpa: $gpub"
