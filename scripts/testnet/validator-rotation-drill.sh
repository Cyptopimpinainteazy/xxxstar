#!/usr/bin/env bash
# validator-rotation-drill.sh — prove the operator-driven key rotation path.
#
# The on-chain `pallet_x3_custody` registry is the single source of truth for who
# is a registered validator and when their next rotation is due. This drill runs
# the `x3-chain-node validator rotate` operator command against a live node and
# proves two things:
#
#   * an unregistered account is **refused** (rotation is never guessed);
#   * a registered account produces a signed `session.set_keys` extrinsic and the
#     reported next due block is the current block plus the rotation period.
#
# Operator-driven only: the drill never rotates unattended, and it never submits
# unless `--submit` is passed.
#
# Usage:
#   scripts/testnet/validator-rotation-drill.sh [--rpc-url URL] [--suri SURI] [--submit]
# Env: NODE_BIN, RPC_URL, BASE_DIR, LOG_DIR (same meaning as the launcher).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
NODE_BIN="${NODE_BIN:-$ROOT_DIR/target/debug/x3-chain-node}"
RPC_URL="${RPC_URL:-http://127.0.0.1:9944}"
SURI="${SURI:-//Alice}"
AURA_SEED="${AURA_SEED:-//aura-drill}"
GRANDPA_SEED="${GRANDPA_SEED:-//grandpa-drill}"
SUBMIT="${SUBMIT:-0}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --rpc-url) RPC_URL="$2"; shift 2 ;;
    --suri) SURI="$2"; shift 2 ;;
    --aura-seed) AURA_SEED="$2"; shift 2 ;;
    --grandpa-seed) GRANDPA_SEED="$2"; shift 2 ;;
    --submit) SUBMIT=1; shift ;;
    -h|--help)
      sed -n '2,24p' "${BASH_SOURCE[0]}"
      exit 0
      ;;
    *) echo "Unknown argument: $1" >&2; exit 2 ;;
  esac
done

info() { printf '[drill] %s\n' "$*"; }
fail() { printf '[drill] FAIL: %s\n' "$*" >&2; exit 1; }
pass() { printf '[drill] PASS: %s\n' "$*"; }

[[ -x "$NODE_BIN" ]] || fail "node binary not found at $NODE_BIN (build it first: cargo build -p x3-chain-node)"

if ! curl -s -m 5 -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","id":1,"method":"system_chain","params":[]}' "$RPC_URL" >/dev/null; then
  fail "no node reachable at $RPC_URL — start one first (e.g. x3-chain-node --dev)"
fi

# 1. An unregistered account must be refused, not silently rotated.
info "checking unregistered-account refusal"
if "$NODE_BIN" validator rotate \
    --rpc-url "$RPC_URL" --suri "//UnregisteredDrill" \
    --aura-seed "$AURA_SEED" --grandpa-seed "$GRANDPA_SEED" >/tmp/x3-rot-unreg.out 2>&1; then
  fail "unregistered account was not refused"
fi
grep -qi "not a registered validator" /tmp/x3-rot-unreg.out \
  || fail "refusal message did not name the unregistered condition"
pass "unregistered account refused"

# 2. A registered account produces a signed set_keys extrinsic and reports the
#    next due block as current + KeyRotationPeriod.
info "building set_keys for registered account $SURI"
OUT="$("$NODE_BIN" validator rotate \
  --rpc-url "$RPC_URL" --suri "$SURI" \
  --aura-seed "$AURA_SEED" --grandpa-seed "$GRANDPA_SEED" 2>&1)" || {
  printf '%s\n' "$OUT" >&2
  fail "validator rotate failed for $SURI (is the account registered on-chain?)"
}
echo "$OUT" | grep -q "session.set_keys extrinsic" \
  || fail "rotate output did not contain a session.set_keys extrinsic"
echo "$OUT" | grep -q "next due:" \
  || fail "rotate output did not report the next due block"
pass "registered account produced a signed set_keys extrinsic and next due block"

if [[ "$SUBMIT" -eq 1 ]]; then
  info "submitting set_keys"
  "$NODE_BIN" validator rotate \
    --rpc-url "$RPC_URL" --suri "$SURI" \
    --aura-seed "$AURA_SEED" --grandpa-seed "$GRANDPA_SEED" --submit \
    || fail "submission failed"
  pass "session.set_keys submitted"
else
  info "dry-run only (pass --submit to send the extrinsic)"
fi

info "validator rotation drill complete"
