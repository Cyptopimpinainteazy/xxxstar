#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# x3_testnet_up.sh — bring up the local testnet
#
# This entry point used to launch the network itself, and could not: it required
# `subkey` to derive session keys (`subkey` is not installed on the build boxes and is
# not part of this repository), defaulted to `deployment/chain-specs/x3-testnet-raw.json`
# — a storage-raw *Live* spec the node's loader refuses — and started nodes with
# `--unsafe-force-node-key-generation`, so the peer ids a spec's bootNodes name could
# never be stable across a restart.
#
# `scripts/testnet/run-7-validators-local.sh` does all of it correctly today: keys are
# inserted through the node itself, its preflight refuses to start unless every session
# key is an authority in the spec *and* every node's peer id is one of its bootNodes,
# per-validator `--node-key` files keep identities stable, and `--only <i>` restarts a
# single node. So this keeps its CLI and delegates, instead of being a third copy of
# the launcher to keep in sync.
#
# Usage:
#   ./scripts/testnet/x3_testnet_up.sh [--wipe] [--base-dir PATH] [--chain-spec PATH]
#       [--node-bin PATH] [--log-dir PATH] [--count N] [--keys-dir PATH] [--skip-build]
#
# Environment:
#   NODE_BIN       Path to x3-chain-node (default: target/{release,debug}/x3-chain-node)
#   CHAIN_SPEC     Path to a *plain* Live chain spec (default: the generated one; a raw
#                  spec is refused on purpose, and one is built for you if none exists)
#   COUNT          Number of validators (default: 7)
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
LAUNCHER="$ROOT_DIR/scripts/testnet/run-7-validators-local.sh"
BUILDER="$ROOT_DIR/scripts/testnet/build-x3-testnet-spec.py"
GENERATED_SPEC="$ROOT_DIR/deployment/chain-specs/fresh/generated/x3-testnet-plain.json"

NODE_BIN="${NODE_BIN:-}"
CHAIN_SPEC="${CHAIN_SPEC:-}"
BASE_DIR="${BASE_DIR:-$HOME/.local/share/x3/testnet}"
LOG_DIR="${LOG_DIR:-$ROOT_DIR/logs/testnet}"
KEYS_DIR="${KEYS_DIR:-}"
COUNT="${COUNT:-7}"
SKIP_BUILD="${SKIP_BUILD:-0}"
BUILD_FEATURES="${BUILD_FEATURES:-testnet}"
WIPE_BASE_DIR=0

usage() {
  cat <<EOF
Usage: $(basename "$0") [--wipe] [options]

Local testnet launcher (delegates to scripts/testnet/run-7-validators-local.sh).

Options:
  --wipe              Stop existing nodes and wipe the base dir before starting.
  --base-dir PATH     Override BASE_DIR (default: ${BASE_DIR})
  --chain-spec PATH   Override CHAIN_SPEC (plain Live spec; must exist or be buildable)
  --node-bin PATH     Override NODE_BIN
  --log-dir PATH      Override LOG_DIR (default: ${LOG_DIR})
  --count N           Number of validators (default: ${COUNT})
  --keys-dir PATH     Per-validator seeds + node keys (default: the generated ones)
  --skip-build        Do not run cargo build first (accepted; the build is a release
                      build of the same binary the launcher would use)
  -h, --help          Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --wipe) WIPE_BASE_DIR=1; shift ;;
    --base-dir) BASE_DIR="${2:-}"; shift 2 ;;
    --chain-spec) CHAIN_SPEC="${2:-}"; shift 2 ;;
    --node-bin) NODE_BIN="${2:-}"; shift 2 ;;
    --log-dir) LOG_DIR="${2:-}"; shift 2 ;;
    --count) COUNT="${2:-}"; shift 2 ;;
    --keys-dir) KEYS_DIR="${2:-}"; shift 2 ;;
    --skip-build) SKIP_BUILD=1; shift ;;
    --features) BUILD_FEATURES="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown argument: $1" >&2; usage; exit 2 ;;
  esac
done

[[ -x "$LAUNCHER" ]] || { echo "[error] launcher not found: $LAUNCHER" >&2; exit 1; }

if [[ -z "$NODE_BIN" ]]; then
  for candidate in \
    "${CARGO_TARGET_DIR:-$ROOT_DIR/target}/release/x3-chain-node" \
    "${CARGO_TARGET_DIR:-$ROOT_DIR/target}/debug/x3-chain-node" \
    "$ROOT_DIR/target/release/x3-chain-node" \
    "$ROOT_DIR/target/debug/x3-chain-node"; do
    [[ -x "$candidate" ]] && NODE_BIN="$candidate" && break
  done
fi
[[ -n "$NODE_BIN" && -x "$NODE_BIN" ]] || {
  echo "[error] node binary not found. Build one that can also build a chain spec:" >&2
  echo "        cargo build -p x3-chain-node   # without SKIP_WASM_BUILD" >&2
  exit 1
}
echo "[node] $NODE_BIN"

if [[ "$SKIP_BUILD" == "0" ]]; then
  echo "[build] cargo build --release -p x3-chain-node --features ${BUILD_FEATURES} (--skip-build to reuse the binary above)"
  ( cd "$ROOT_DIR" && cargo build --release -p x3-chain-node --features "$BUILD_FEATURES" ) || {
    echo "[error] build failed; re-run with --skip-build to use the existing binary" >&2
    exit 1
  }
fi

# A Live spec in *raw* form is refused by the node's loader — that is what this script
# used to hand it by default. Insist on the plain form.
case "${CHAIN_SPEC:-}" in
  *.raw.json|*-raw.json)
    echo "[spec] $CHAIN_SPEC is a raw Live spec; the node refuses to load one." >&2
    echo "       Build a plain one instead: python3 $BUILDER $COUNT" >&2
    exit 1
    ;;
esac
if [[ -z "$CHAIN_SPEC" || ! -f "$CHAIN_SPEC" ]]; then
  echo "[spec] no plain spec at '${CHAIN_SPEC:-<unset>}'; building one for ${COUNT} authorities"
  X3_NODE_BIN="$NODE_BIN" python3 "$BUILDER" "$COUNT" || {
    echo "[error] could not build a spec (see the builder output above)" >&2
    exit 1
  }
  CHAIN_SPEC="$GENERATED_SPEC"
  [[ -f "$CHAIN_SPEC" ]] || { echo "[error] $CHAIN_SPEC missing after the build" >&2; exit 1; }
fi
echo "[spec] $CHAIN_SPEC"

args=( --chain-spec "$CHAIN_SPEC" --node-bin "$NODE_BIN" --base-dir "$BASE_DIR" --log-dir "$LOG_DIR" )
[[ "$WIPE_BASE_DIR" == "1" ]] && args+=( --wipe )
[[ -n "$KEYS_DIR" ]] && args+=( --keys-dir "$KEYS_DIR" )

echo "[delegate] COUNT=${COUNT} run-7-validators-local.sh ${args[*]}"
COUNT="$COUNT" exec bash "$LAUNCHER" "${args[@]}"
