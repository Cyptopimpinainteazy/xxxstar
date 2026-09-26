#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# snapshot-live-restore-proof.sh — prove a snapshot restores the *same chain*.
#
# The repository already ships the snapshot mechanism (`scripts/snapshot-restore.sh`
# for a base-path archive, and `crates/x3-state-snapshot` for the
# content-addressed format). What it did not ship is a proof that a restore is
# *load-bearing*: the old `test-restore` action round-trips a text file through a
# tarball, which passes just as well for an empty database.
#
# This script boots a real dev chain, lets it finalize, and then:
#
#   1. reads the anchor **from the running chain** (finalized height H, the
#      canonical block hash at H, the state root of that block, and the full set
#      of storage entries at H) — every one of those is read over RPC while the
#      node is still producing blocks;
#   2. stops the node (the archive tooling refuses to snapshot a live DB, and a
#      plain tar of a live RocksDB is not a consistent snapshot) and takes the
#      base-path archive with `scripts/snapshot-restore.sh backup`;
#   3. restores it into a fresh base path with
#      `scripts/snapshot-restore.sh restore`, boots a node on the restored DB,
#      and requires H, the hash at H, the state root at H, and every storage
#      entry at H (all keys *and* all values) to come back identical;
#   4. boots a **control** node on an empty base path, lets it reach height H,
#      and requires its hash at H to differ — i.e. the exact assertion that
#      passed for the restored DB fails for an empty one.
#
# Step 3 is what the public-testnet checklist means by "snapshot/restore
# demonstrated"; step 4 is what stops step 3 from being a check that an empty
# database also passes.
#
# Usage:
#   bash scripts/snapshot-live-restore-proof.sh
#
# Environment:
#   X3_NODE_BIN                       node binary (default target/{release,debug})
#   X3_SNAPSHOT_PROOF_MIN_FINALIZED   height the source must reach (default 4)
#   X3_SNAPSHOT_PROOF_WAIT_SECS       per-wait timeout (default 240)
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SNAPSHOT_TOOL="$ROOT/scripts/snapshot-restore.sh"
MIN_FINALIZED="${X3_SNAPSHOT_PROOF_MIN_FINALIZED:-4}"
WAIT_SECS="${X3_SNAPSHOT_PROOF_WAIT_SECS:-240}"

say() { printf '[snapshot-proof] %s\n' "$*"; }
die() {
  printf '[snapshot-proof] FAIL: %s\n' "$*" >&2
  for log in "$WORK"/*.log; do
    [ -f "$log" ] || continue
    printf '[snapshot-proof] --- %s (tail) ---\n' "$(basename "$log")" >&2
    tail -20 "$log" >&2 || true
  done
  exit 1
}

# ── the node binary ──────────────────────────────────────────────────────────
NODE_BIN="${X3_NODE_BIN:-}"
if [ -z "$NODE_BIN" ]; then
  for candidate in "$ROOT/target/release/x3-chain-node" "$ROOT/target/debug/x3-chain-node"; do
    if [ -x "$candidate" ]; then NODE_BIN="$candidate"; break; fi
  done
fi
[ -n "$NODE_BIN" ] && [ -x "$NODE_BIN" ] || {
  echo "[snapshot-proof] node binary not found; build it with" >&2
  echo "                 cargo build --release -p x3-chain-node" >&2
  exit 2
}
say "node:       $NODE_BIN"
say "snapshot:   $SNAPSHOT_TOOL"

WORK="$(mktemp -d)"
NODE_PIDS=()
cleanup() {
  for pid in "${NODE_PIDS[@]:-}"; do kill "$pid" 2>/dev/null || true; done
  sleep 1
  for pid in "${NODE_PIDS[@]:-}"; do kill -9 "$pid" 2>/dev/null || true; done
  rm -rf "$WORK"
}
trap cleanup EXIT

SRC="$WORK/source/node"
DST="$WORK/restored/node"
EMPTY="$WORK/control/node"
mkdir -p "$WORK/source" "$WORK/restored" "$WORK/control" "$WORK/snapshots"

# A private, per-run port block, so this gate never fights another chain gate.
BASE_PORT="$(python3 - <<'PY'
import random, socket
while True:
    base = random.randint(41000, 55000)
    socks = []
    try:
        # Three nodes, each needing a contiguous rpc/peer-port pair.
        for i in range(9):
            s = socket.socket()
            s.bind(("127.0.0.1", base + i))
            socks.append(s)
        print(base)
        break
    except OSError:
        pass
    finally:
        for s in socks:
            s.close()
PY
)"
say "ports:      base $BASE_PORT"

# ── JSON-RPC helpers ─────────────────────────────────────────────────────────
rpc() { # rpc <url> <method> <params-json>
  curl -s -m 10 -H 'Content-Type: application/json' \
    --data-binary "{\"jsonrpc\":\"2.0\",\"method\":\"$2\",\"params\":$3,\"id\":1}" "$1"
}

# Every reader below prints one value per line and never conflates "absent" with
# "the read failed": a failed RPC exits non-zero, an absent value prints "".
finalized_hash() { # finalized_hash <url>
  rpc "$1" chain_getFinalizedHead '[]' | python3 -c '
import json, sys
doc = json.load(sys.stdin)
if "error" in doc:
    sys.exit("chain_getFinalizedHead: " + str(doc["error"]))
print(doc.get("result") or "")
'
}

number_for_hash() { # number_for_hash <url> <hash>
  rpc "$1" chain_getHeader "[\"$2\"]" | python3 -c '
import json, sys
doc = json.load(sys.stdin)
if "error" in doc:
    sys.exit("chain_getHeader: " + str(doc["error"]))
header = doc.get("result")
print(0 if header is None else int(header["number"], 16))
'
}

hash_at() { # hash_at <url> <height>
  rpc "$1" chain_getBlockHash "[$2]" | python3 -c '
import json, sys
doc = json.load(sys.stdin)
if "error" in doc:
    sys.exit("chain_getBlockHash: " + str(doc["error"]))
print(doc.get("result") or "")
'
}

state_root_at() { # state_root_at <url> <hash>
  rpc "$1" chain_getHeader "[\"$2\"]" | python3 -c '
import json, sys
doc = json.load(sys.stdin)
header = doc.get("result")
print("" if header is None else header["stateRoot"])
'
}

# Every storage entry at a block, as "<key>\t<value>" lines, sorted by key.
#
# The keys are paged; a page that repeats ends the walk rather than looping. Each
# value is then read at the same block, so the output is a complete, comparable
# snapshot of application state — not a sample.
state_entries() { # state_entries <url> <hash>
  python3 - "$1" "$2" <<'PY'
import json, sys, urllib.request

url, at = sys.argv[1], sys.argv[2]

def call(method, params):
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}).encode()
    req = urllib.request.Request(url, data=body, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=30) as resp:
        doc = json.load(resp)
    if "error" in doc:
        raise SystemExit(f"{method}: {doc['error']}")
    return doc["result"]

seen = set()
start = None
for _ in range(10_000):
    page = call("state_getKeysPaged", ["", 1000, start, at])
    if not page:
        break
    new = [k for k in page if k not in seen]
    if not new:
        break
    seen.update(new)
    if len(page) < 1000:
        break
    start = page[-1]
else:
    raise SystemExit("state_getKeysPaged did not terminate")

for key in sorted(seen):
    value = call("state_getStorage", [key, at])
    print(f"{key}\t{value or ''}")
PY
}

runtime_spec_version() { # runtime_spec_version <url>
  rpc "$1" state_getRuntimeVersion '[]' | python3 -c '
import json, sys
doc = json.load(sys.stdin)
doc = doc.get("result") or {}
print(doc.get("specVersion", "?"))
'
}

# ── node lifecycle ───────────────────────────────────────────────────────────
start_node() { # start_node <name> <base-path> <rpc-port> [extra chain args...]
  local name="$1" base="$2" port="$3"
  shift 3
  # `--no-prometheus`: three nodes run in this proof at once, and the exporter
  # is on by default at a single fixed port (9615), so the second node to start
  # would die on "Address already in use". Metrics are not what this gate proves.
  "$NODE_BIN" --dev --base-path "$base" --rpc-port "$port" --port "$((port + 1))" \
    --rpc-methods unsafe --no-telemetry --no-mdns --no-prometheus "$@" \
    >"$WORK/$name.log" 2>&1 &
  NODE_PIDS+=("$!")
}

wait_rpc() { # wait_rpc <url>
  local url="$1"
  for _ in $(seq 1 "$WAIT_SECS"); do
    if rpc "$url" system_health '[]' | grep -q '"peers"'; then return 0; fi
    sleep 1
  done
  return 1
}

# Wait until the node has finalized at least <min> blocks. Genesis is finalized
# immediately, so "a finalized head exists" is not evidence of a running chain.
wait_finalized() { # wait_finalized <url> <min> -> prints "<number> <hash>"
  local url="$1" min="$2" hash number
  for _ in $(seq 1 "$WAIT_SECS"); do
    hash="$(finalized_hash "$url" || true)"
    if [ -n "$hash" ]; then
      number="$(number_for_hash "$url" "$hash" || true)"
      if [ -n "$number" ] && [ "$number" -ge "$min" ]; then
        printf '%s %s\n' "$number" "$hash"
        return 0
      fi
    fi
    sleep 1
  done
  return 1
}

stop_nodes() {
  for pid in "${NODE_PIDS[@]:-}"; do kill "$pid" 2>/dev/null || true; done
  for pid in "${NODE_PIDS[@]:-}"; do wait "$pid" 2>/dev/null || true; done
  NODE_PIDS=()
  sleep 2
}

# ── 1. boot the source chain and read the anchor from the running node ───────
# Each node owns a 3-port slice: rpc at +0, libp2p at +1, one spare.
SRC_RPC=$(( BASE_PORT ))
say "booting source chain at $SRC (rpc $SRC_RPC)"
start_node source "$SRC" "$SRC_RPC"
SRC_URL="http://127.0.0.1:$SRC_RPC"
wait_rpc "$SRC_URL" || die "source node never answered RPC"

ANCHOR="$(wait_finalized "$SRC_URL" "$MIN_FINALIZED")" \
  || die "source chain never finalized to height $MIN_FINALIZED"
HEIGHT="${ANCHOR%% *}"
HEAD_HASH="${ANCHOR##* }"
[ -n "$HEIGHT" ] && [ -n "$HEAD_HASH" ] || die "could not parse the finalized anchor"
STATE_ROOT="$(state_root_at "$SRC_URL" "$HEAD_HASH")"
[ -n "$STATE_ROOT" ] || die "could not read the state root at $HEAD_HASH"
SPEC_VERSION="$(runtime_spec_version "$SRC_URL")"

state_entries "$SRC_URL" "$HEAD_HASH" >"$WORK/state.src"
KEY_COUNT="$(wc -l <"$WORK/state.src" | tr -d ' ')"
[ "$KEY_COUNT" -gt 0 ] || die "the running chain exposed no state at $HEAD_HASH"
# The most substantial entry, shown only so the run is legible; the proof is the
# whole-file comparison below, not this line.
read -r SAMPLE_KEY SAMPLE_VALUE <<<"$(python3 - "$WORK/state.src" <<'PY'
import sys
best = ("", "")
for line in open(sys.argv[1]):
    line = line.rstrip("\n")
    if not line:
        continue
    key, value = line.split("\t", 1)
    if len(value) > len(best[1]):
        best = (key, value)
print(best[0], best[1])
PY
)"

say "running chain anchor: height=$HEIGHT hash=$HEAD_HASH"
say "                     state_root=$STATE_ROOT spec_version=$SPEC_VERSION"
say "                     keys=$KEY_COUNT sample_key=$SAMPLE_KEY"
say "                     sample_value=${SAMPLE_VALUE:0:66}$([ ${#SAMPLE_VALUE} -gt 66 ] && echo '…')"

# ── 2. stop the node and snapshot the base path it wrote ─────────────────────
say "stopping the source node (archive tooling refuses a live DB; a plain tar"
say "of a running RocksDB would not be a consistent snapshot)"
stop_nodes

say "backing up with scripts/snapshot-restore.sh backup"
X3_SNAPSHOT_DIR="$WORK/snapshots" bash "$SNAPSHOT_TOOL" backup "$SRC" \
  || die "backup failed"
TAR="$(ls -t "$WORK"/snapshots/*.tar.gz 2>/dev/null | head -1 || true)"
[ -n "$TAR" ] && [ -f "$TAR" ] || die "backup produced no archive"
say "archive:    $TAR ($(du -h "$TAR" | cut -f1))"

# ── 3. restore into a fresh base path and boot a node on it ──────────────────
say "restoring into $DST"
X3_SNAPSHOT_DIR="$WORK/snapshots" bash "$SNAPSHOT_TOOL" restore "$TAR" "$DST" \
  || die "restore failed"
[ -f "$DST/chains/x3_chain_dev/db/full" ] || [ -d "$DST/chains/x3_chain_dev" ] \
  || die "restore did not lay down a chain database under $DST"

DST_RPC=$(( BASE_PORT + 3 ))
start_node restored "$DST" "$DST_RPC"
DST_URL="http://127.0.0.1:$DST_RPC"
wait_rpc "$DST_URL" || die "restored node never answered RPC"
say "restored node is up; verifying its finalized history against the source"

wait_finalized "$DST_URL" "$HEIGHT" >/dev/null \
  || die "restored chain never reached the source height $HEIGHT"

RESTORED_HASH="$(hash_at "$DST_URL" "$HEIGHT")"
[ "$RESTORED_HASH" = "$HEAD_HASH" ] \
  || die "restored canonical hash at $HEIGHT is $RESTORED_HASH, not $HEAD_HASH"

RESTORED_ROOT="$(state_root_at "$DST_URL" "$RESTORED_HASH")"
[ "$RESTORED_ROOT" = "$STATE_ROOT" ] \
  || die "restored state root at $HEIGHT is $RESTORED_ROOT, not $STATE_ROOT"

state_entries "$DST_URL" "$HEAD_HASH" >"$WORK/state.dst"
if ! diff -u "$WORK/state.src" "$WORK/state.dst" >"$WORK/state.diff"; then
  head -20 "$WORK/state.diff" >&2
  die "the restored chain's state at $HEIGHT differs from the source's"
fi

say "restored chain matches: height>=$HEIGHT hash=$RESTORED_HASH"
say "                        state_root=$RESTORED_ROOT keys=$KEY_COUNT"

# ── 4. the control: an empty DB must fail the same check ─────────────────────
say "control: booting an empty chain and letting it reach height $HEIGHT"
CTL_RPC=$(( BASE_PORT + 6 ))
start_node control "$EMPTY" "$CTL_RPC"
CTL_URL="http://127.0.0.1:$CTL_RPC"
wait_rpc "$CTL_URL" || die "control node never answered RPC"
wait_finalized "$CTL_URL" "$HEIGHT" >/dev/null \
  || die "control chain never reached height $HEIGHT"
CTL_HASH="$(hash_at "$CTL_URL" "$HEIGHT")"
[ -n "$CTL_HASH" ] || die "control chain reports no hash at its own height $HEIGHT"
if [ "$CTL_HASH" = "$HEAD_HASH" ]; then
  die "an empty chain reproduced the source hash at $HEIGHT — the check is not load-bearing"
fi
say "control hash at $HEIGHT = $CTL_HASH (differs from the source)"

echo
echo "[snapshot-proof] PASS — the restored chain is the source chain:"
printf '  %-26s %s\n' "finalized height" "$HEIGHT"
printf '  %-26s %s\n' "canonical hash at height" "$HEAD_HASH"
printf '  %-26s %s\n' "state root at height" "$STATE_ROOT"
printf '  %-26s %s\n' "state entries at height" "$KEY_COUNT"
printf '  %-26s %s\n' "sample value at height" "${SAMPLE_VALUE:0:66}$([ ${#SAMPLE_VALUE} -gt 66 ] && echo '…')"
printf '  %-26s %s\n' "control (empty DB) hash" "$CTL_HASH  (differs → the check is load-bearing)"
