#!/usr/bin/env bash
# consensus-soak.sh — run the local network for a while and require it to stay honest.
#
# Everything proven about consensus so far was proven in minutes: finality, agreement,
# failure and recovery. A mainnet network also has to *last* — no stalls, no node
# dying quietly, no peers drifting away, no memory that only grows. This harness samples
# a running network on an interval and fails on the first of those it sees, then writes
# a report of what it observed.
#
# Usage:
#   scripts/testnet/consensus-soak.sh [--count N] [--minutes M] [--interval S] [--keep]
# Env: NODE_BIN, BASE_DIR (default /tmp/x3-soak), CHAIN_SPEC
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
COUNT="${COUNT:-4}"
MINUTES="${MINUTES:-10}"
INTERVAL="${INTERVAL:-15}"
BASE_DIR="${BASE_DIR:-/tmp/x3-soak}"
LOG_DIR="$BASE_DIR/logs"
REPORT="$BASE_DIR/soak-report.json"
KEEP="${KEEP:-0}"
# A stall is `finalized height unchanged` for longer than this. GRANDPA finalizes
# several blocks a minute on an idle chain, so a minute of nothing is a stall, not a
# slow round; the tolerance exists because samples are taken on an interval.
STALL_TOLERANCE_SECS="${STALL_TOLERANCE_SECS:-60}"
# Memory: the check is about **the node**, so the state cache is off unless the caller asks
# for it.
#
# `--trie-cache-size` (the state cache) defaults to 1 GiB in this SDK, and a node filling a
# configured cache is not leaking. Measured on 2026-09-22, 4 validators, 2 hours each way:
# growth 1,714–1,760 MiB (debug) and 1,736–1,748 MiB (release) with the default cache, against
# +227 MiB and *flat* in 15 minutes with `NODE_TRIE_CACHE_BYTES=0`. A 1 GiB bound therefore
# fails every normally-configured node and measures its cache, not its allocations — so the
# soak disables the cache by default and the bound below is about the pipeline.
#
# Set `NODE_TRIE_CACHE_BYTES` to a production value to measure the footprint instead: 1 GiB of
# state cache put a node at ~2.4 GiB RSS over two hours. That number belongs in an operator's
# memory budget, not in a leak detector.
NODE_TRIE_CACHE_BYTES="${NODE_TRIE_CACHE_BYTES:-0}"
export NODE_TRIE_CACHE_BYTES
# Memory is reported, and only a *large* growth fails. With the cache disabled, this is the
# node's own growth: ~230 MiB over 15 minutes in the measurement above.
MAX_RSS_GROWTH_MIB="${MAX_RSS_GROWTH_MIB:-1024}"
RPC_BASE="${RPC_BASE:-9944}"

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

while [[ $# -gt 0 ]]; do
  case "$1" in
    --count) COUNT="${2:-4}"; shift 2 ;;
    --minutes) MINUTES="${2:-10}"; shift 2 ;;
    --interval) INTERVAL="${2:-15}"; shift 2 ;;
    --keep) KEEP=1; shift ;;
    -h|--help) sed -n '2,14p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) echo "Unknown argument: $1" >&2; exit 2 ;;
  esac
done

SPEC="${CHAIN_SPEC:-$ROOT_DIR/deployment/chain-specs/fresh/generated/x3-testnet-plain.json}"
PORTS=$(seq "$RPC_BASE" $((RPC_BASE + COUNT - 1)))

info() { printf '[soak] %s\n' "$*"; }
fail() { printf '[soak] FAIL: %s\n' "$*" >&2; exit 1; }
pass() { printf '[soak] PASS: %s\n' "$*"; }

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

rpc() { # <port> <method> [params-json]
  curl -s -m 10 -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"$2\",\"params\":${3:-[]}}" \
    "http://127.0.0.1:$1"
}
number_of() { python3 -c "
import json,sys
try:
    d=json.load(sys.stdin)
except Exception:
    print(-1); raise SystemExit
h=d.get('result') or {}
print(int(h.get('number','0x0'),16) if isinstance(h,dict) and h else -1)
"; }
finalized_height() { # <port>
  local head
  head=$(rpc "$1" chain_getFinalizedHead | python3 -c "import json,sys;print(json.load(sys.stdin).get('result',''))" 2>/dev/null || true)
  [[ -n "$head" ]] || { echo -1; return; }
  rpc "$1" chain_getHeader "[\"$head\"]" | number_of
}
peers_of() { # <port>
  rpc "$1" system_health | python3 -c "import json,sys;print((json.load(sys.stdin).get('result') or {}).get('peers',-1))" 2>/dev/null || echo -1
}

rm -rf "$BASE_DIR"
mkdir -p "$LOG_DIR"

info "building a ${COUNT}-authority spec and booting ${COUNT} validators"
X3_NODE_BIN="$NODE_BIN" python3 "$ROOT_DIR/scripts/testnet/build-x3-testnet-spec.py" "$COUNT" \
  >"$BASE_DIR.spec.log" 2>&1 || { tail -20 "$BASE_DIR.spec.log" >&2; fail "spec build failed"; }
COUNT="$COUNT" NODE_BIN="$NODE_BIN" CHAIN_SPEC="$SPEC" BASE_DIR="$BASE_DIR" LOG_DIR="$LOG_DIR" \
  SKIP_BUILD=1 bash "$ROOT_DIR/scripts/testnet/x3_testnet_up.sh" --skip-build \
  >"$BASE_DIR.launch.log" 2>&1 || { tail -20 "$BASE_DIR.launch.log" >&2; fail "launch failed"; }

pids() { # pid per node, in RPC order
  local i
  for i in $(seq 1 "$COUNT"); do cat "$BASE_DIR/pids/node-$i.pid" 2>/dev/null || echo 0; done
}

SAMPLES_FILE="$BASE_DIR/samples.tsv"
printf 'elapsed_s' > "$SAMPLES_FILE"
for port in $PORTS; do printf '\theight_%s\tpeers_%s\trss_kib_%s' "$port" "$port" "$port" >> "$SAMPLES_FILE"; done
printf '\n' >> "$SAMPLES_FILE"

declare -A last_height last_change
for port in $PORTS; do last_height[$port]=-1; last_change[$port]=0; done
declare -A first_rss
start=$(date +%s)
deadline=$(( start + MINUTES * 60 ))
stall_fail=""

info "soaking for ${MINUTES} minute(s), sampling every ${INTERVAL}s (stall tolerance ${STALL_TOLERANCE_SECS}s)"
while :; do
  now=$(date +%s); elapsed=$(( now - start ))
  printf '%s' "$elapsed" >> "$SAMPLES_FILE"
  i=0
  for port in $PORTS; do
    i=$((i + 1))
    pid=$(sed -n "${i}p" <(pids))
    if [[ "${pid:-0}" == "0" ]] || ! kill -0 "$pid" 2>/dev/null; then
      # Nothing restarts a validator during a soak: a node that is gone is a finding,
      # not a sample.
      fail "node $i (rpc $port, pid ${pid:-unknown}) is not running after ${elapsed}s"
    fi
    h=$(finalized_height "$port")
    p=$(peers_of "$port")
    rss=$(ps -o rss= -p "$pid" 2>/dev/null | tr -d ' ' || echo -1)
    [[ -n "${rss:-}" ]] || rss=-1
    printf '\t%s\t%s\t%s' "$h" "$p" "$rss" >> "$SAMPLES_FILE"
    if [[ "${first_rss[$port]:-}" == "" && "${rss:--1}" != "-1" ]]; then first_rss[$port]="$rss"; fi
    if [[ "$h" -gt "${last_height[$port]}" ]]; then
      last_height[$port]="$h"; last_change[$port]="$now"
    elif [[ "${last_change[$port]}" != "0" ]] && [[ $(( now - last_change[$port] )) -gt "$STALL_TOLERANCE_SECS" ]]; then
      stall_fail="rpc $port has not finalized since ${last_change[$port]} (${h} at height)"
    fi
  done
  printf '\n' >> "$SAMPLES_FILE"
  [[ -n "$stall_fail" ]] && fail "$stall_fail"
  [[ "$now" -lt "$deadline" ]] || break
  sleep "$INTERVAL"
done

final_elapsed=$(( $(date +%s) - start ))
info "state cache: NODE_TRIE_CACHE_BYTES=${NODE_TRIE_CACHE_BYTES:-0} (0 = disabled; see the memory note above)"
info "soak window complete (${final_elapsed}s). Checking agreement at sampled heights"

min_height=999999999
for port in $PORTS; do
  h=$(finalized_height "$port")
  [[ "$h" -lt "$min_height" ]] && min_height="$h"
done
[[ "$min_height" -gt 10 ]] || fail "the network finalized only ${min_height} blocks in ${MINUTES} minute(s)"

agree_fail=""
for height in $((min_height / 4)) $((min_height / 2)) $((min_height * 3 / 4)); do
  first=""
  for port in $PORTS; do
    hash=$(rpc "$port" chain_getBlockHash "[$height]" | python3 -c "import json,sys;print(json.load(sys.stdin).get('result',''))" 2>/dev/null || true)
    [[ -n "$hash" && "$hash" != "null" ]] || { agree_fail="rpc $port has no hash for height ${height}"; break; }
    if [[ -z "$first" ]]; then first="$hash"; fi
    [[ "$hash" == "$first" ]] || { agree_fail="height ${height}: $port disagrees ($hash != $first)"; break; }
  done
  [[ -n "$agree_fail" ]] && break
done
[[ -z "$agree_fail" ]] || fail "$agree_fail"
pass "all ${COUNT} validators agree on the same chain at heights $((min_height / 4)), $((min_height / 2)), $((min_height * 3 / 4))"

# Per-port summary: height growth, longest observed stall, peers at the end, RSS growth.
if ! python3 - "$SAMPLES_FILE" "$REPORT" "$COUNT" "$RPC_BASE" "$MAX_RSS_GROWTH_MIB" "$MINUTES" <<'PY'
import json, sys
path, out, count, rpc_base, max_growth, minutes = (
    sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]),
    float(sys.argv[5]), float(sys.argv[6]),
)
rows = [line.rstrip("\n").split("\t") for line in open(path) if line.strip()]
header, samples = rows[0], [r for r in rows[1:] if len(r) == len(rows[0])]
summary = {"minutes": minutes, "samples": len(samples), "validators": []}
worst_growth = 0.0
for idx in range(count):
    port = rpc_base + idx
    heights = [int(s[1 + idx * 3]) for s in samples]
    peers = [int(s[2 + idx * 3]) for s in samples]
    rss = [int(s[3 + idx * 3]) for s in samples if int(s[3 + idx * 3]) > 0]
    growth = (max(rss) - min(rss)) / 1024.0 if len(rss) >= 2 else 0.0
    worst_growth = max(worst_growth, growth)
    summary["validators"].append({
        "rpc": port,
        "first_height": heights[0],
        "last_height": heights[-1],
        "height_growth": heights[-1] - heights[0],
        "min_peers": min(peers),
        "last_peers": peers[-1],
        "rss_kib_min": min(rss) if rss else 0,
        "rss_kib_max": max(rss) if rss else 0,
        "rss_growth_mib": round(growth, 1),
    })
summary["worst_rss_growth_mib"] = round(worst_growth, 1)
summary["max_rss_growth_mib_allowed"] = max_growth
json.dump(summary, open(out, "w"), indent=2)
print(f"[soak] report -> {out}")
for v in summary["validators"]:
    print(
        f"[soak]   rpc {v['rpc']}: height {v['first_height']} -> {v['last_height']} "
        f"(+{v['height_growth']}), peers {v['last_peers']} (min {v['min_peers']}), "
        f"rss growth {v['rss_growth_mib']} MiB"
    )
if worst_growth > max_growth:
    print(f"[soak] FAIL: a node grew {worst_growth:.1f} MiB, above the {max_growth} MiB bound")
    raise SystemExit(1)
if any(v["height_growth"] <= 0 for v in summary["validators"]):
    print("[soak] FAIL: a validator's finalized height did not advance")
    raise SystemExit(1)
PY
then
  fail "the soak report found a problem (see ${REPORT})"
fi

pass "${MINUTES} minute(s) with no stall beyond ${STALL_TOLERANCE_SECS}s, no node lost, agreement held"
pass "report: ${REPORT}"
cleanup
left=$( { pgrep -f -- "--base-path $BASE_DIR/node-" 2>/dev/null || true; } | wc -l | tr -d '[:space:]')
[[ "$left" == "0" ]] || fail "cleanup left ${left} validator process(es) running"

printf '\n[soak] PASSED: %s validators, %s minutes, one chain throughout.\n' "$COUNT" "$MINUTES"
