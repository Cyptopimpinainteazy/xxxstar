#!/usr/bin/env bash
# validator-failure-drill.sh — what the network does when validators die.
#
# The multi-validator row says "no sustained independent-validator evidence". A
# network that finalizes while everything is up proves liveness on the happy path
# and nothing about the properties mainnet depends on:
#
#   * with a **minority** down (2 of 7), finality must continue — that is liveness
#     under the failure a production network sees every day;
#   * with **more than a third** down (3 of 7 < 2/3), finality must **stop** — that
#     is the safety property. Authoring continues (Aura needs one author), and a
#     chain that keeps finalizing at less than a supermajority has no safety at all;
#   * when the dead validators come back, they must catch up and rejoin one chain.
#
# Usage:
#   scripts/testnet/validator-failure-drill.sh [--count N] [--kill K] [--keep]
# Env: NODE_BIN, CHAIN_SPEC, BASE_DIR, LOG_DIR (same meaning as the launcher).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
LAUNCHER="$ROOT_DIR/scripts/testnet/run-7-validators-local.sh"

COUNT="${COUNT:-7}"
# How many to kill first: the largest number that still leaves more than two thirds
# standing. Derived rather than fixed so `--count` alone cannot produce a drill whose
# first phase is out of range (the guard below refuses those, and a gate that always
# refuses is worse than no gate).
KILL_COUNT="${KILL_COUNT:-$(( (COUNT - 1) / 3 ))}"
[[ "$KILL_COUNT" -ge 1 ]] || KILL_COUNT=1
BASE_DIR="${BASE_DIR:-/tmp/x3-failure-drill}"
LOG_DIR="${LOG_DIR:-$BASE_DIR/logs}"
RPC_BASE="${RPC_BASE:-9944}"
KEEP="${KEEP:-0}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --count) COUNT="$2"; shift 2 ;;
    --kill) KILL_COUNT="$2"; shift 2 ;;
    --keep) KEEP=1; shift ;;
    -h|--help)
      sed -n '2,20p' "${BASH_SOURCE[0]}"
      exit 0
      ;;
    *) echo "Unknown argument: $1" >&2; exit 2 ;;
  esac
done

if [[ "$COUNT" -lt 4 ]]; then
  echo "COUNT must be at least 4 (a 2/3 supermajority needs three of four)" >&2
  exit 2
fi
if [[ "$KILL_COUNT" -lt 1 ]]; then
  echo "KILL_COUNT must be at least 1 — a drill that kills nothing proves nothing" >&2
  exit 2
fi
# start_node keeps numbering from 1, and 1 is the CLI bootnode, so the drill kills
# from the top and leaves it alone.
SURVIVORS=$((COUNT - KILL_COUNT))
# Phase 1 must leave a supermajority (that is the liveness claim) and phase 2 must
# break it (that is the safety claim). Anything else is a drill that proves half of
# what it says.
if [[ "$((SURVIVORS * 3))" -le "$((COUNT * 2))" ]]; then
  echo "Refusing: with ${KILL_COUNT} of ${COUNT} down, the ${SURVIVORS} survivors are already" >&2
  echo "below 2/3, so the first phase would test the stall instead of liveness. Use a" >&2
  echo "smaller --kill." >&2
  exit 2
fi
if [[ "$(( (SURVIVORS - 1) * 3 ))" -gt "$((COUNT * 2))" ]]; then
  echo "Refusing: one more failure would leave $((SURVIVORS - 1)) of ${COUNT} — still a" >&2
  echo "supermajority — so the stall phase would be testing nothing. Use a larger --kill." >&2
  exit 2
fi

info() { printf '[drill] %s\n' "$*"; }
fail() { printf '[drill] FAIL: %s\n' "$*" >&2; exit 1; }
pass() { printf '[drill] PASS: %s\n' "$*"; }

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
print(d.get('result', '') if isinstance(d.get('result'), str) else (d.get('result') or ''))
"
}

height() { # <port> — best block number
  local port="$1"
  curl -s -m 10 -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"chain_getHeader","params":[]}' \
    "http://127.0.0.1:${port}" | python3 -c "
import json, sys
h = (json.load(sys.stdin).get('result') or {})
print(int(h.get('number', '0x0'), 16) if h else -1)
"
}

finalized_height() { # <port> — number of the block this node has finalized
  local port="$1" head
  head="$(rpc "$port" chain_getFinalizedHead)"
  [[ -n "$head" ]] || { echo -1; return; }
  curl -s -m 10 -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"chain_getHeader\",\"params\":[\"${head}\"]}" \
    "http://127.0.0.1:${port}" | python3 -c "
import json, sys
h = (json.load(sys.stdin).get('result') or {})
print(int(h.get('number', '0x0'), 16) if h else -1)
"
}

block_hash() { # <port> <height>
  rpc "$1" chain_getBlockHash "[$2]"
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

# ── helpers for the phases ────────────────────────────────────────────────────
all_finalized_at_least() { # <height>
  local want="$1" p
  for p in $(seq "$RPC_BASE" $((RPC_BASE + COUNT - 1))); do
    [[ "$(finalized_height "$p")" -ge "$want" ]] || return 1
  done
  return 0
}

survivors_advance_past() { # <height> — every live node finalizes beyond it
  local want="$1" p
  for p in $(seq "$RPC_BASE" $((RPC_BASE + SURVIVORS - 1))); do
    [[ "$(finalized_height "$p")" -gt "$want" ]] || return 1
  done
  return 0
}

agreement_at() { # <height> <how-many-nodes> — every listed node reports one hash
  local h="$1" nodes="$2" first="" p got
  for p in $(seq "$RPC_BASE" $((RPC_BASE + nodes - 1))); do
    got="$(block_hash "$p" "$h")"
    [[ -n "$got" && "$got" != "null" ]] || return 1
    if [[ -z "$first" ]]; then first="$got"; fi
    [[ "$got" == "$first" ]] || return 1
  done
  echo "$first"
  return 0
}

head_advanced() { # <port> <since-height>
  local p="$1" since="$2"
  [[ "$(height "$p")" -gt "$since" ]]
}

[[ -x "$LAUNCHER" ]] || fail "launcher not found: $LAUNCHER"

# No spec given (or one that is not there)? Build one, so this runs from a clean
# checkout with no arguments — the launcher's default spec is the tracked raw Live
# fixture, which the node's loader refuses on purpose.
if [[ -z "${CHAIN_SPEC:-}" || ! -f "${CHAIN_SPEC}" ]]; then
  info "no usable CHAIN_SPEC; building a ${COUNT}-authority Live spec"
  BUILDER="$ROOT_DIR/scripts/testnet/build-x3-testnet-spec.py"
  [[ -f "$BUILDER" ]] || fail "spec builder not found: $BUILDER"
  X3_NODE_BIN="${NODE_BIN:-}" python3 "$BUILDER" "$COUNT" >"${BASE_DIR}.spec.log" 2>&1 || {
    tail -20 "${BASE_DIR}.spec.log" >&2 || true
    fail "could not build a ${COUNT}-authority spec (see ${BASE_DIR}.spec.log); a node built with SKIP_WASM_BUILD cannot build one"
  }
  CHAIN_SPEC="$ROOT_DIR/deployment/chain-specs/fresh/generated/x3-testnet-plain.json"
  [[ -f "$CHAIN_SPEC" ]] || fail "the builder reported success but ${CHAIN_SPEC} is missing"
  info "spec: ${CHAIN_SPEC}"
fi
export CHAIN_SPEC

# Stopping by pid file alone is not enough: a restart rewrites pid files, and a
# stale one kills the wrong process while the real validator keeps running (and
# holds the ports the next run needs). Kill by pid file and then sweep by base path.
kill_network() {
  local f
  for f in "${BASE_DIR}"/pids/node-*.pid; do
    [[ -f "$f" ]] || continue
    kill -9 "$(cat "$f")" 2>/dev/null || true
  done
  pkill -f -- "--base-path ${BASE_DIR}/node-" 2>/dev/null || true
  sleep 2
}

running_nodes() {
  # `pgrep` exits 1 when nothing matches, and `set -o pipefail` turns that into a
  # failed pipeline — which, inside `LEFT="$(running_nodes)"`, aborts the script
  # under `set -e` before it can print anything. Zero matches is the success case
  # here, so swallow pgrep's status.
  { pgrep -f -- "--base-path ${BASE_DIR}/node-" 2>/dev/null || true; } | wc -l | tr -d '[:space:]'
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
info "booting ${COUNT} validators (spec: ${CHAIN_SPEC:-launcher default})"
COUNT="$COUNT" BASE_DIR="$BASE_DIR" LOG_DIR="$LOG_DIR" \
  "${LAUNCHER}" >"${BASE_DIR}.launch.log" 2>&1 || {
    tail -20 "${BASE_DIR}.launch.log" >&2 || true
    fail "the launcher could not start ${COUNT} validators (see ${BASE_DIR}.launch.log)"
  }
pass "${COUNT} validators up"

# ── phase 0: everyone finalizes and agrees ───────────────────────────────────
wait_for "all ${COUNT} nodes to finalize 20 blocks" 240 all_finalized_at_least 20 \
  || fail "the network did not finalize 20 blocks with every validator up"
HEIGHT="$(finalized_height "$RPC_BASE")"
HASH="$(wait_for "agreement at height ${HEIGHT}" 120 agreement_at "$HEIGHT" "$COUNT" || true)"
[[ -n "$HASH" ]] || fail "nodes did not agree on a block hash at height ${HEIGHT}"
pass "all ${COUNT} finalizing; agreed at height ${HEIGHT} (${HASH:0:18}…)"

# ── phase 1: minority down — finality must continue ──────────────────────────
DOWN=()
for i in $(seq "$COUNT" -1 $((COUNT - KILL_COUNT + 1))); do DOWN+=("$i"); done
info "killing validator(s) ${DOWN[*]} (${KILL_COUNT} of ${COUNT}; ${SURVIVORS} remain)"
for i in "${DOWN[@]}"; do
  kill -9 "$(cat "${BASE_DIR}/pids/node-${i}.pid")" 2>/dev/null || true
done
BEFORE="$(finalized_height "$RPC_BASE")"
wait_for "survivors to finalize past ${BEFORE}" 180 survivors_advance_past "$BEFORE" \
  || fail "finality stopped with only ${KILL_COUNT} of ${COUNT} down — the survivors are a supermajority and must keep going"
AFTER="$(finalized_height "$RPC_BASE")"
HASH="$(wait_for "survivor agreement" 120 agreement_at "$((AFTER - 3))" "$SURVIVORS" || true)"
[[ -n "$HASH" ]] || fail "survivors did not agree on a common finalized block"
pass "with ${KILL_COUNT} of ${COUNT} down, finality continued (${BEFORE} → ${AFTER}) and the survivors agree (${HASH:0:18}…)"

# ── phase 2: beyond a third down — finality must stop ────────────────────────
one_more=$((COUNT - KILL_COUNT))
info "killing validator ${one_more} as well (${KILL_COUNT}+1 of ${COUNT} down; below 2/3)"
kill -9 "$(cat "${BASE_DIR}/pids/node-${one_more}.pid")" 2>/dev/null || true
STALL_FROM="$(finalized_height "$RPC_BASE")"
HEAD_FROM="$(height "$RPC_BASE")"
# GRANDPA gets a full window to try — the claim is that it *cannot* succeed, not
# that it is slow, so the height is sampled after the window rather than at the
# first quiet poll. Three extra blocks of slack covers votes already in flight.
STALL_WINDOW="${STALL_WINDOW:-60}"
info "waiting ${STALL_WINDOW}s to see whether finality can still advance without a supermajority"
sleep "$STALL_WINDOW"
STALL_TO="$(finalized_height "$RPC_BASE")"
HEAD_TO="$(height "$RPC_BASE")"
if [[ "$STALL_TO" -gt "$((STALL_FROM + 3))" ]]; then
  fail "finality advanced ${STALL_FROM} → ${STALL_TO} with only $((SURVIVORS - 1)) of ${COUNT} validators online — that is finality without a supermajority"
fi
[[ "$HEAD_TO" -gt "$HEAD_FROM" ]] \
  || fail "block authoring stopped too (${HEAD_FROM} → ${HEAD_TO}) — this looks like a stuck node, not a missing quorum"
pass "with $((KILL_COUNT + 1)) of ${COUNT} down, finality stopped at ${STALL_FROM} (still ${STALL_TO} after ${STALL_WINDOW}s) while authoring continued (head ${HEAD_FROM} → ${HEAD_TO})"

# ── phase 3: the dead come back — catch up and agree ─────────────────────────
info "restarting validator(s) ${DOWN[*]} ${one_more}"
for i in "${DOWN[@]}" "$one_more"; do
  COUNT="$COUNT" BASE_DIR="$BASE_DIR" LOG_DIR="$LOG_DIR" \
    "${LAUNCHER}" --only "$i" >>"${BASE_DIR}.launch.log" 2>&1 \
    || fail "validator ${i} did not restart (see ${BASE_DIR}.launch.log)"
done
pass "restarted; waiting for the network to finalize again"
TARGET="$((STALL_FROM + 5))"
wait_for "finality to resume past ${TARGET}" 300 all_finalized_at_least "$TARGET" \
  || fail "finality did not resume after the validators came back"
HEIGHT="$(finalized_height "$RPC_BASE")"
HASH="$(wait_for "post-restart agreement" 180 agreement_at "$((HEIGHT - 3))" "$COUNT" || true)"
[[ -n "$HASH" ]] || fail "the restarted validators did not rejoin one chain"
pass "all ${COUNT} finalizing again at ${HEIGHT}; one chain (${HASH:0:18}…)"

# ── phase 4: the drill has to leave nothing behind ───────────────────────────
# A leaked validator holds the RPC and p2p ports, so the *next* drill fails to bind
# and the failure looks like the network's fault rather than the harness's. This
# caught exactly that: node-1's pid file had been rewritten by a restart, so the
# pid-file kill missed it and the node kept port 9944.
kill_network
LEFT="$(running_nodes)"
[[ "$LEFT" == "0" ]] \
  || fail "cleanup left ${LEFT} validator process(es) running under ${BASE_DIR} (ports stay bound)"
pass "no validator processes left under ${BASE_DIR}"

printf '\n[drill] ALL PHASES PASSED: minority failure keeps finality, a missing supermajority stops it, recovery restores it.\n'
