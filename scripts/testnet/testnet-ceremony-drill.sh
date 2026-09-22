#!/usr/bin/env bash
# testnet-ceremony-drill.sh — record a launch, then verify it, then prove the
# verifier can fail.
#
# What a published testnet needs is not only that it starts: it needs a record of
# *what* was launched (the spec, the binary, the genesis hash, the authorities, the
# runtime version, the bootnode identities) that anyone can check a running network
# against. This drill produces that record from a real launch and then verifies it —
# and, because a verifier nobody has seen fail is not evidence, it tampers with a copy
# of the manifest and requires the verifier to reject it.
#
# Usage:
#   scripts/testnet/testnet-ceremony-drill.sh [--count N] [--keep]
# Env: NODE_BIN, BASE_DIR (default /tmp/x3-testnet-ceremony)
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
COUNT="${COUNT:-4}"
BASE_DIR="${BASE_DIR:-/tmp/x3-testnet-ceremony}"
LOG_DIR="$BASE_DIR/logs"
MANIFEST="$BASE_DIR/ceremony.json"
KEEP="${KEEP:-0}"

# Same binary discovery as the launcher.
NODE_BIN="${NODE_BIN:-}"
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
  echo "node binary not found; build it with cargo build -p x3-chain-node" >&2
  exit 1
}

RPC_BASE="${RPC_BASE:-9944}"
PORTS="$(seq -s, "$RPC_BASE" $((RPC_BASE + COUNT - 1)))"
SPEC="${CHAIN_SPEC:-$ROOT_DIR/deployment/chain-specs/fresh/generated/x3-testnet-plain.json}"

info() { printf '[ceremony-drill] %s\n' "$*"; }
fail() { printf '[ceremony-drill] FAIL: %s\n' "$*" >&2; exit 1; }
pass() { printf '[ceremony-drill] PASS: %s\n' "$*"; }

cleanup() {
  [[ "$KEEP" == "1" ]] && { info "KEEP=1 — leaving the network under $BASE_DIR"; return; }
  local f
  for f in "$BASE_DIR"/pids/node-*.pid; do
    [[ -f "$f" ]] || continue
    kill -9 "$(cat "$f")" 2>/dev/null || true
  done
  pkill -f -- "--base-path $BASE_DIR/node-" 2>/dev/null || true
}
trap cleanup EXIT

rm -rf "$BASE_DIR"
mkdir -p "$LOG_DIR"

info "building a ${COUNT}-authority Live spec"
X3_NODE_BIN="$NODE_BIN" python3 "$ROOT_DIR/scripts/testnet/build-x3-testnet-spec.py" "$COUNT" \
  >"$BASE_DIR.spec.log" 2>&1 || { tail -20 "$BASE_DIR.spec.log" >&2; fail "spec build failed"; }
[[ -f "$SPEC" ]] || fail "expected $SPEC after the build"
info "spec: $SPEC"

info "launching ${COUNT} validators through x3_testnet_up.sh"
COUNT="$COUNT" NODE_BIN="$NODE_BIN" CHAIN_SPEC="$SPEC" BASE_DIR="$BASE_DIR" LOG_DIR="$LOG_DIR" \
  SKIP_BUILD=1 bash "$ROOT_DIR/scripts/testnet/x3_testnet_up.sh" --skip-build \
  >"$BASE_DIR.launch.log" 2>&1 || { tail -20 "$BASE_DIR.launch.log" >&2; fail "launch failed"; }

# Wait for every validator to finalize a few blocks, so the manifest records a live
# network rather than a booting one.
deadline=$(( $(date +%s) + 300 ))
while :; do
  done_all=1
  for port in $(seq "$RPC_BASE" $((RPC_BASE + COUNT - 1))); do
    head=$(curl -s -m 5 -H 'Content-Type: application/json' \
      -d '{"jsonrpc":"2.0","id":1,"method":"chain_getFinalizedHead","params":[]}' \
      "http://127.0.0.1:$port" | python3 -c "import json,sys;print(json.load(sys.stdin).get('result',''))" 2>/dev/null || true)
    [[ -n "$head" ]] || { done_all=0; break; }
    num=$(curl -s -m 5 -H 'Content-Type: application/json' \
      -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"chain_getHeader\",\"params\":[\"$head\"]}" \
      "http://127.0.0.1:$port" | python3 -c "import json,sys;h=(json.load(sys.stdin).get('result') or {});print(int(h.get('number','0x0'),16) if h else 0)" 2>/dev/null || echo 0)
    [[ "${num:-0}" -ge 5 ]] || { done_all=0; break; }
  done
  [[ "$done_all" == "1" ]] && break
  [[ "$(date +%s)" -lt "$deadline" ]] || fail "the network did not finalize 5 blocks on every validator"
  sleep 3
done
pass "${COUNT} validators finalizing"

info "recording the ceremony manifest"
python3 "$ROOT_DIR/scripts/testnet/testnet-ceremony.py" record "$SPEC" \
  --node-bin "$NODE_BIN" --rpc "$PORTS" --out "$MANIFEST" || fail "could not record the manifest"
[[ -s "$MANIFEST" ]] || fail "manifest not written"
pass "manifest: $MANIFEST"

info "verifying the running network against the manifest"
if ! python3 "$ROOT_DIR/scripts/testnet/testnet-ceremony.py" verify "$MANIFEST" \
    --rpc "$PORTS" --node-bin "$NODE_BIN" --min-finalized 5 | sed 's/^/[ceremony-drill]   /'; then
  fail "the network does not match its own manifest"
fi
pass "every check passes against the launched network"

# Negative control: a verifier that has never failed proves nothing.
TAMPERED="$BASE_DIR/ceremony-tampered.json"
python3 - "$MANIFEST" "$TAMPERED" <<'PY'
import json, sys
manifest = json.load(open(sys.argv[1]))
genesis = manifest["genesis"]["hash"]
manifest["genesis"]["hash"] = ("0x" + "00" * 32) if genesis != "0x" + "00" * 32 else ("0x" + "11" * 32)
json.dump(manifest, open(sys.argv[2], "w"), indent=2)
PY
if python3 "$ROOT_DIR/scripts/testnet/testnet-ceremony.py" verify "$TAMPERED" \
    --rpc "$PORTS" --node-bin "$NODE_BIN" --min-finalized 5 >"$BASE_DIR/tampered-verify.log" 2>&1; then
  fail "a manifest with a wrong genesis hash was accepted"
fi
grep -q "genesis hash" "$BASE_DIR/tampered-verify.log" \
  || fail "the tampered manifest was rejected, but not for the genesis hash"
pass "a tampered manifest is rejected (wrong genesis hash)"

cleanup
# `pgrep` exits 1 when nothing matches; under `set -o pipefail` that would abort the
# script before it can report the success it is checking for.
left=$( { pgrep -f -- "--base-path $BASE_DIR/node-" 2>/dev/null || true; } | wc -l | tr -d '[:space:]')
[[ "$left" == "0" ]] || fail "cleanup left ${left} validator process(es) running"
pass "no validator processes left"

printf '\n[ceremony-drill] ALL PHASES PASSED: recorded, verified, and proven able to fail.\n'
