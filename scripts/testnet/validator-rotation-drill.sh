#!/usr/bin/env bash
# validator-rotation-drill.sh — the on-chain validator-key rotation path.
#
# This is the operator path the validator-key row was missing. It boots a
# three-validator dev network, proves that the node refuses `Session::set_keys`
# for an account with no active `X3Custody::ValidatorKeyRegistry` entry, inserts
# fresh Aura/GRANDPA keys, submits `Session::set_keys` for the registered
# //Alice validator, and verifies `Session::NextKeys` now carries the new keys.
#
# The registry entries are seeded from the chain spec's authority set by
# `x3_chain_genesis`, so this drill is testing the real source of truth rather
# than registering an account through a test-only backdoor.
#
# Usage:
#   scripts/testnet/validator-rotation-drill.sh [--keep]
# Env: NODE_BIN, CHAIN_SPEC, BASE_DIR, LOG_DIR, RPC_BASE.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
LAUNCHER="$ROOT_DIR/scripts/testnet/run-7-validators-local.sh"

COUNT="${COUNT:-3}"
BASE_DIR="${BASE_DIR:-/tmp/x3-rotation-drill}"
LOG_DIR="${LOG_DIR:-$BASE_DIR/logs}"
RPC_BASE="${RPC_BASE:-9944}"
KEEP="${KEEP:-0}"
CHAIN_SPEC="${CHAIN_SPEC:-}"
KEYS_DIR="${KEYS_DIR:-$ROOT_DIR/deployment/chain-specs/fresh/generated/validator-keys}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --keep) KEEP=1; shift ;;
    -h|--help)
      sed -n '2,24p' "${BASH_SOURCE[0]}"
      exit 0
      ;;
    *) echo "Unknown argument: $1" >&2; exit 2 ;;
  esac
done

if [[ "$COUNT" -ne 3 ]]; then
  echo "This drill pins COUNT=3 so the session-key rotation exercises a" >&2
  echo "supermajority network without becoming a long soak. COUNT is not" >&2
  echo "configurable here." >&2
  exit 2
fi

info() { printf '[rotation-drill] %s\n' "$*"; }
fail() { printf '[rotation-drill] FAIL: %s\n' "$*" >&2; exit 1; }
pass() { printf '[rotation-drill] PASS: %s\n' "$*"; }

rpc() { # <port> <method> [json-params]
  local port="$1" method="$2" params="${3:-[]}"
  curl -s -m 10 -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"${method}\",\"params\":${params}}" \
    "http://127.0.0.1:${port}" | python3 -c "
import json, sys
try:
    d = json.load(sys.stdin)
except Exception:
    print(''); raise SystemExit
result = d.get('result', '')
if isinstance(result, str):
    print(result)
elif result is None:
    print('null')
else:
    print(result)
"
}

finalized_height() { # <port>
  local port="$1" head
  head="$(rpc "$port" chain_getFinalizedHead)"
  [[ -n "$head" && "$head" != "null" ]] || { echo -1; return; }
  curl -s -m 10 -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"chain_getHeader\",\"params\":[\"${head}\"]}" \
    "http://127.0.0.1:${port}" | python3 -c "
import json, sys
h = (json.load(sys.stdin).get('result') or {})
print(int(h.get('number', '0x0'), 16) if h else -1)
"
}

wait_for() { # <description> <timeout-seconds> <predicate...>
  local what="$1" timeout="$2"; shift 2
  local deadline=$(( $(date +%s) + timeout ))
  while :; do
    if "$@"; then return 0; fi
    [[ "$(date +%s)" -lt "$deadline" ]] || return 1
    sleep 3
  done
}

all_finalized_at_least() { # <height>
  local want="$1" p
  for p in $(seq "$RPC_BASE" $((RPC_BASE + COUNT - 1))); do
    [[ "$(finalized_height "$p")" -ge "$want" ]] || return 1
  done
  return 0
}

find_node_bin() {
  local candidate
  for candidate in \
    "${CARGO_TARGET_DIR:-$ROOT_DIR/target}/release/x3-chain-node" \
    "${CARGO_TARGET_DIR:-$ROOT_DIR/target}/debug/x3-chain-node" \
    "$ROOT_DIR/target/release/x3-chain-node" \
    "$ROOT_DIR/target/debug/x3-chain-node"; do
    [[ -x "$candidate" ]] && { NODE_BIN="$candidate"; return; }
  done
  echo "node binary not found; build it with cargo build -p x3-chain-node" >&2
  exit 1
}
NODE_BIN="${NODE_BIN:-}"
[[ -z "$NODE_BIN" ]] && find_node_bin
[[ -x "$NODE_BIN" ]] || fail "node binary not executable: $NODE_BIN"

# Build a fresh spec unless the caller supplied one. This is the same path the
# failure drill uses: the generated spec is produced by the node's own chain-spec
# code, which now seeds an active ValidatorKeyRegistry entry for every authority.
if [[ -z "$CHAIN_SPEC" || ! -f "$CHAIN_SPEC" ]]; then
  info "no usable CHAIN_SPEC; building a 3-authority Live spec"
  BUILDER="$ROOT_DIR/scripts/testnet/build-x3-testnet-spec.py"
  [[ -f "$BUILDER" ]] || fail "spec builder not found: $BUILDER"
  X3_NODE_BIN="$NODE_BIN" python3 "$BUILDER" 3 >"${BASE_DIR}.spec.log" 2>&1 || {
    tail -30 "${BASE_DIR}.spec.log" >&2 || true
    fail "could not build a 3-authority spec (see ${BASE_DIR}.spec.log)"
  }
  CHAIN_SPEC="$ROOT_DIR/deployment/chain-specs/fresh/generated/x3-testnet-plain.json"
  [[ -f "$CHAIN_SPEC" ]] || fail "the builder reported success but ${CHAIN_SPEC} is missing"
fi
[[ -f "$KEYS_DIR/validator-1.suri" ]] \
  || fail "validator seed file not found at $KEYS_DIR/validator-1.suri"

CHAIN_ID="$(python3 -c "import json;print(json.load(open('$CHAIN_SPEC'))['id'])")"
KSTORE="$BASE_DIR/node-1/chains/$CHAIN_ID/keystore"

kill_network() {
  local f
  for f in "${BASE_DIR}"/pids/node-*.pid; do
    [[ -f "$f" ]] || continue
    kill -9 "$(cat "$f")" 2>/dev/null || true
  done
  pkill -f -- "--base-path ${BASE_DIR}/node-" 2>/dev/null || true
  sleep 2
}

cleanup() {
  if [[ "$KEEP" == "1" ]]; then
    info "KEEP=1 — leaving the network running under ${BASE_DIR}"
    return
  fi
  kill_network
}
trap cleanup EXIT

rm -rf "$BASE_DIR"
mkdir -p "$LOG_DIR"
info "booting ${COUNT} validators from ${CHAIN_SPEC}"
COUNT="$COUNT" CHAIN_SPEC="$CHAIN_SPEC" KEYS_DIR="$KEYS_DIR" BASE_DIR="$BASE_DIR" LOG_DIR="$LOG_DIR" \
  NO_MDNS=1 DISABLE_LOG_COLOR=1 \
  RPC_READY_TIMEOUT_SECS="${RPC_READY_TIMEOUT_SECS:-120}" \
  "${LAUNCHER}" >"${BASE_DIR}.launch.log" 2>&1 || {
    tail -30 "${BASE_DIR}.launch.log" >&2 || true
    fail "the launcher could not start ${COUNT} validators"
  }
pass "three validators up"

wait_for "all validators to finalize 20 blocks" 240 all_finalized_at_least 20 \
  || fail "the network did not finalize 20 blocks"
pass "network finalized 20 blocks"

SEED="$(sed -n 's/^seed=//p' "$KEYS_DIR/validator-1.suri" | head -1)"
[[ -n "$SEED" ]] || fail "validator-1.suri has no seed= line"
ACCOUNT_SS58="$("$NODE_BIN" keys generate --key-type aura --seed "$SEED" --output ss58)"
UNREGISTERED_SS58="$("$NODE_BIN" keys generate --key-type aura --seed //Dave --output ss58)"

REGISTRY_OUT="$("$NODE_BIN" keys show-registry --account "$ACCOUNT_SS58" --rpc-url "http://127.0.0.1:$RPC_BASE")"
grep -q 'active: true' <<<"$REGISTRY_OUT" \
  || fail "genesis did not seed an active ValidatorKeyRegistry entry for validator-1"
pass "validator-1 has an active on-chain validator-key registry entry"

AURA_JSON="$("$NODE_BIN" keys generate --key-type aura --output json)"
GRAN_JSON="$("$NODE_BIN" keys generate --key-type grandpa --output json)"
AURA_SECRET="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["secretSeed"])' <<<"$AURA_JSON")"
GRAN_SECRET="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["secretSeed"])' <<<"$GRAN_JSON")"

mkdir -p "$KSTORE"
AURA_SS58="$("$NODE_BIN" keys insert --key-type aura --seed "$AURA_SECRET" --keystore-path "$KSTORE")"
GRAN_SS58="$("$NODE_BIN" keys insert --key-type grandpa --seed "$GRAN_SECRET" --keystore-path "$KSTORE")"
pass "fresh session keys inserted into validator-1 keystore"

if "$NODE_BIN" keys set-session \
  --account "$UNREGISTERED_SS58" \
  --aura-key "$AURA_SS58" \
  --grandpa-key "$GRAN_SS58" \
  --signer "$SEED" \
  --rpc-url "http://127.0.0.1:$RPC_BASE" >"${BASE_DIR}.refused.log" 2>&1; then
  fail "keys set-session accepted an account with no active validator-key registry entry"
fi
grep -q 'no active entry in X3Custody::ValidatorKeyRegistry' "${BASE_DIR}.refused.log" \
  || fail "the unregistered-account refusal did not report the registry reason"
pass "unregistered validator is refused"

SET_OUT="$("$NODE_BIN" keys set-session \
  --account "$ACCOUNT_SS58" \
  --aura-key "$AURA_SS58" \
  --grandpa-key "$GRAN_SS58" \
  --signer "$SEED" \
  --rpc-url "http://127.0.0.1:$RPC_BASE")"
grep -q 'submitted session.setKeys' <<<"$SET_OUT" \
  || fail "keys set-session did not report a submitted transaction"

SHOWN="$("$NODE_BIN" keys show-session --account "$ACCOUNT_SS58" --rpc-url "http://127.0.0.1:$RPC_BASE")"
grep -q "aura:    ${AURA_SS58}" <<<"$SHOWN" \
  || fail "Session::NextKeys aura does not match the inserted key"
grep -q "grandpa: ${GRAN_SS58}" <<<"$SHOWN" \
  || fail "Session::NextKeys grandpa does not match the inserted key"
pass "Session::NextKeys now carries the new validator session keys"

info "rotation drill complete"
