#!/usr/bin/env bash
# Does x3-state-snapshot agree with the chain about what the state hashes to?
#
# This is the check that makes snapshot verification mean anything. Everything
# else in the snapshot tool proves transport integrity — the chunks are the ones
# the manifest committed to, the manifest is the one the caller expected, the
# state root recomputes from the bytes. All of that is circular unless the
# recomputed root is the same function the chain uses.
#
# The procedure is deliberately two-sided:
#
#   1. ask a *running node* for the state root it put in a block header
#      (`chain_getHeader`), which is the chain's own value;
#   2. ask the same node binary to dump the genesis state it holds
#      (`build-spec --chain <id> --raw`), and rebuild the root from that state
#      with `x3-state-snapshot root`.
#
# Any disagreement means the tool and the chain compute different roots, and no
# snapshot verified by the tool can be trusted.
#
# Usage:
#   verify-snapshot-root-against-chain.sh <rpc-url> [chain-id]
#     X3_NODE_BIN     node binary (default: target/{release,debug}/x3-chain-node)
#     X3_SNAPSHOT_BIN x3-state-snapshot binary (default: target/{release,debug}/…)
#     X3_STATE_VERSION  trie layout to assume, 0 or 1 (default: 1)
set -euo pipefail

RPC_URL="${1:-}"
CHAIN_ID="${2:-dev}"

if [[ -z "$RPC_URL" ]]; then
  echo "usage: $0 <rpc-url> [chain-id]" >&2
  exit 2
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

resolve_bin() { # <override-var-value> <name>
  local override="$1" name="$2"
  if [[ -n "$override" ]]; then
    printf '%s' "$override"
    return 0
  fi
  local candidate
  for candidate in "target/release/$name" "target/debug/$name"; do
    if [[ -x "$REPO_ROOT/$candidate" ]]; then
      printf '%s' "$REPO_ROOT/$candidate"
      return 0
    fi
  done
  return 1
}

if ! NODE_BIN="$(resolve_bin "${X3_NODE_BIN:-}" x3-chain-node)"; then
  echo "error: x3-chain-node not found; set X3_NODE_BIN or build it" >&2
  exit 2
fi
if ! SNAPSHOT_BIN="$(resolve_bin "${X3_SNAPSHOT_BIN:-}" x3-state-snapshot)"; then
  echo "error: x3-state-snapshot not found; set X3_SNAPSHOT_BIN or build it" >&2
  exit 2
fi

rpc() { # <method> <params-json>
  curl -sS -m 10 -X POST -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"$1\",\"params\":$2,\"id\":1}" "$RPC_URL"
}

GENESIS_HASH="$(rpc chain_getBlockHash '[0]' | python3 -c 'import sys,json; print(json.load(sys.stdin)["result"])')"
if [[ -z "$GENESIS_HASH" || "$GENESIS_HASH" == "null" ]]; then
  echo "error: could not read the genesis hash from $RPC_URL" >&2
  exit 2
fi

CHAIN_ROOT="$(rpc chain_getHeader "[\"$GENESIS_HASH\"]" \
  | python3 -c 'import sys,json; print(json.load(sys.stdin)["result"]["stateRoot"])')"
SPEC_VERSION="$(rpc state_getRuntimeVersion '[]' \
  | python3 -c 'import sys,json; print(json.load(sys.stdin)["result"]["specVersion"])')"

SPEC_FILE="$(mktemp "${TMPDIR:-/tmp}/x3-rawgenesis-XXXXXX.json")"
trap 'find "$SPEC_FILE" -maxdepth 0 -delete 2>/dev/null || true' EXIT

echo "chain:     $CHAIN_ID (spec_version $SPEC_VERSION) at $RPC_URL"
echo "genesis:   $GENESIS_HASH"
echo "root (chain header): $CHAIN_ROOT"
echo "dumping raw genesis state ..."
"$NODE_BIN" build-spec --chain "$CHAIN_ID" --raw --disable-log-color > "$SPEC_FILE" 2>/dev/null

DERIVED_ROOT="$("$SNAPSHOT_BIN" root --from-raw-spec "$SPEC_FILE" \
  --state-version "${X3_STATE_VERSION:-1}")"
echo "root (recomputed):   $DERIVED_ROOT"

if [[ "${DERIVED_ROOT,,}" == "${CHAIN_ROOT,,}" ]]; then
  echo ""
  echo "MATCH — the snapshot tool reproduces the chain's own state root"
  exit 0
fi

echo ""
echo "MISMATCH — the snapshot tool and the chain disagree about the state root."
echo "Every snapshot this tool verifies inherits that disagreement; do not use it"
echo "for state sync until this is resolved."
exit 1
