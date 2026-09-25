#!/usr/bin/env bash
# Disk-full drill: put an authority on a real filesystem, take the free space
# away under it, and record what it does.
#
# The audit's §15 asks what happens when the volume fills. The node has a
# disk-space guard with a startup gate and a watchdog, and the watchdog's own log
# line says "an authority must not … keep authoring writes it cannot finish" —
# but nothing had ever verified that it *stops*, and the known-answer is that it
# does not: the watchdog only logs. A guard that reports a condition it cannot
# act on is worth knowing about before a validator finds out.
#
# The drill runs on a size-limited tmpfs created inside a private mount namespace
# (`unshare -Urm`), so it needs no root and cannot touch the host's disks. That
# makes the free-space threshold and the ENOSPC real rather than simulated.
#
# It separates two kinds of statement, because they are not the same thing and
# conflating them is how a gap becomes a "pass":
#
#   * checks (PASS/FAIL, they decide the exit code) — what the node is supposed
#     to do: refuse an authority below the floor, allow a full node, report
#     Critical, and be restartable afterwards;
#   * measurements (printed, not scored) — what it actually does when the floor
#     is crossed and when the volume runs out. These are the audit's "does the
#     guard stop authoring?" and "what happens at ENOSPC?" questions, and the
#     honest answer is recorded either way.
#
# Usage: bash scripts/mainnet/prove-disk-full-authoring.sh
#   X3_NODE_BIN       node binary (default: target/{release,debug}/x3-chain-node)
#   X3_DRILL_FS_MB    size of the tmpfs (default 256)
#   X3_DRILL_FLOOR_MB guard floor, the X3_MIN_FREE_DISK_BYTES value (default 96)
#   X3_KEEP_TMP       1 to keep logs
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

resolve_node() {
  if [[ -n "${X3_NODE_BIN:-}" ]]; then
    printf '%s' "$X3_NODE_BIN"
    return 0
  fi
  local candidate
  for candidate in target/release/x3-chain-node target/debug/x3-chain-node; do
    if [[ -x "$REPO_ROOT/$candidate" ]]; then
      printf '%s' "$REPO_ROOT/$candidate"
      return 0
    fi
  done
  return 1
}

if ! NODE_BIN="$(resolve_node)"; then
  echo "error: x3-chain-node not found; set X3_NODE_BIN or build it" >&2
  exit 2
fi

# Everything below runs inside a private mount namespace, so the tmpfs — and any
# ENOSPC it produces — is invisible to the rest of the machine.
if [[ "${X3_DISK_DRILL_INNER:-0}" != "1" ]]; then
  if ! unshare -Urm true 2>/dev/null; then
    echo "SKIP: this host cannot create user+mount namespaces (unshare -Urm), so a"
    echo "      size-limited filesystem cannot be made without root. The disk-full"
    echo "      drill stays unexecuted here; the guard's logic is unit-tested"
    echo "      (node/src/disk_guard.rs) but its behaviour has not been observed."
    exit 0
  fi
  exec unshare -Urm --propagation private \
    env X3_DISK_DRILL_INNER=1 X3_NODE_BIN="$NODE_BIN" bash "$0" "$@"
fi

FS_MB="${X3_DRILL_FS_MB:-256}"
FLOOR_MB="${X3_DRILL_FLOOR_MB:-96}"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/x3-disk-drill-XXXXXX")"
FS="$WORK/fs"
mkdir -p "$FS"
if ! mount -t tmpfs -o "size=${FS_MB}m" tmpfs "$FS" 2>/dev/null; then
  echo "SKIP: could not mount a tmpfs inside the namespace" >&2
  exit 0
fi

NODE_PID=""
cleanup() {
  local exit_code=$?
  if [[ -n "$NODE_PID" ]] && kill -0 "$NODE_PID" 2>/dev/null; then
    kill -TERM "$NODE_PID" 2>/dev/null || true
    sleep 2
    kill -KILL "$NODE_PID" 2>/dev/null || true
  fi
  if [[ "${X3_KEEP_TMP:-0}" == "1" ]]; then
    echo "work directory kept (inside the namespace, it disappears when this exits): $WORK"
    sleep 30
  fi
  umount "$FS" 2>/dev/null || true
  find "$WORK" -depth -delete 2>/dev/null || true
  exit "$exit_code"
}
trap cleanup EXIT INT TERM

free_port() {
  python3 -c 'import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()'
}

rpc() { # <port> <method> <params>
  curl -sS -m 15 -X POST -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"$2\",\"params\":$3,\"id\":1}" \
    "http://127.0.0.1:$1"
}

free_mb() {
  df -Pm "$FS" | awk 'NR==2 {print $4}'
}

height() { # <port>
  local hex
  hex="$(rpc "$1" chain_getHeader '[]' 2>/dev/null \
    | python3 -c 'import json,sys;print(json.load(sys.stdin)["result"]["number"])' 2>/dev/null || echo 0x0)"
  python3 -c 'import sys; print(int(sys.argv[1], 16))' "$hex"
}

wait_for_rpc() { # <port> <pid> <log> <seconds>
  local port="$1" pid="$2" log="$3" deadline="$4" waited=0
  while (( waited < deadline )); do
    if rpc "$port" chain_getBlockHash '[0]' 2>/dev/null | grep -q '"result"'; then
      return 0
    fi
    if ! kill -0 "$pid" 2>/dev/null; then
      return 1
    fi
    sleep 5
    waited=$(( waited + 5 ))
  done
  return 1
}

failures=0
check() { # <description> <0 for pass>
  if [[ "$2" == "0" ]]; then
    echo "PASS  $1"
  else
    echo "FAIL  $1"
    failures=$(( failures + 1 ))
  fi
}

RPC_PORT="$(free_port)"
GATE_PORT="$(free_port)"
FULL_PORT="$(free_port)"
AUTHORITY_BASE="$FS/authority"
GATE_BASE="$FS/gate"
FULL_BASE="$FS/full"
AUTHORITY_LOG="$WORK/authority.log"

echo "node:       $NODE_BIN"
echo "filesystem: ${FS_MB} MB tmpfs in a private mount namespace"
echo "guard floor: ${FLOOR_MB} MB (X3_MIN_FREE_DISK_BYTES)"
echo "free at start: $(free_mb) MB"
echo

# ── 1. the startup gate ─────────────────────────────────────────────────────
echo "[1/5] does the startup gate refuse an authority below the floor?"
gate_output="$(
  X3_MIN_FREE_DISK_BYTES=$(( FLOOR_MB * 1024 * 1024 * 20 )) \
    timeout 120 "$NODE_BIN" --dev --base-path "$GATE_BASE" --rpc-port "$GATE_PORT" \
      --log warn 2>&1 || true
)" || true
echo "      $(echo "$gate_output" | grep -i 'refus' | head -1)"
if echo "$gate_output" | grep -q "Refusing to start an authority"; then
  check "an authority below the floor is refused at startup" 0
else
  check "an authority below the floor is refused at startup" 1
fi

# ── 2. a full node is not blocked by the same gate ──────────────────────────
echo "[2/5] is a full node (non-authority) allowed to run under the same floor?"
X3_MIN_FREE_DISK_BYTES=$(( FLOOR_MB * 1024 * 1024 * 20 )) \
  "$NODE_BIN" --chain dev --base-path "$FULL_BASE" --rpc-port "$FULL_PORT" \
    --log warn > "$WORK/full.log" 2>&1 &
FULL_PID=$!
if wait_for_rpc "$FULL_PORT" "$FULL_PID" "$WORK/full.log" 240; then
  check "a full node starts under the same floor (the gate is authority-only)" 0
else
  check "a full node starts under the same floor (the gate is authority-only)" 1
  tail -5 "$WORK/full.log"
fi
kill -TERM "$FULL_PID" 2>/dev/null || true
sleep 3
kill -KILL "$FULL_PID" 2>/dev/null || true
find "$FULL_BASE" -depth -delete 2>/dev/null || true

# ── 3. the node under pressure ──────────────────────────────────────────────
echo "[3/5] starting an authority on the ${FS_MB} MB filesystem ..."
X3_MIN_FREE_DISK_BYTES=$(( FLOOR_MB * 1024 * 1024 )) X3_DISK_PROBE_SECS=2 \
  "$NODE_BIN" --dev --base-path "$AUTHORITY_BASE" --rpc-port "$RPC_PORT" \
    --log warn > "$AUTHORITY_LOG" 2>&1 &
NODE_PID=$!
if ! wait_for_rpc "$RPC_PORT" "$NODE_PID" "$AUTHORITY_LOG" 420; then
  echo "error: the authority did not start; log tail:" >&2
  tail -20 "$AUTHORITY_LOG" >&2
  exit 1
fi

HEIGHT_BEFORE="$(height "$RPC_PORT")"
FREE_BEFORE="$(free_mb)"
echo "      block $HEIGHT_BEFORE, free ${FREE_BEFORE} MB"

# Take free space down to half the floor: below Critical, but not yet zero.
TARGET_FREE_MB=$(( FLOOR_MB / 2 ))
FILL_MB=$(( FREE_BEFORE - TARGET_FREE_MB ))
echo "[4/5] filling ${FILL_MB} MB to put free space under the ${FLOOR_MB} MB floor ..."
if (( FILL_MB > 0 )); then
  dd if=/dev/zero of="$FS/filler" bs=1M count="$FILL_MB" status=none || true
fi
sleep 12

HEIGHT_CRITICAL_1="$(height "$RPC_PORT")"
sleep 15
HEIGHT_CRITICAL_2="$(height "$RPC_PORT")"
FREE_NOW="$(free_mb)"

echo "      free now ${FREE_NOW} MB; block $HEIGHT_CRITICAL_1 -> $HEIGHT_CRITICAL_2"
if grep -q "at or below the" "$AUTHORITY_LOG"; then
  check "the watchdog reports Critical below the floor" 0
else
  check "the watchdog reports Critical below the floor" 1
fi

# The guard's own log line says an authority "must not … keep authoring writes it
# cannot finish". Report and control are different things, so this is checked.
if (( HEIGHT_CRITICAL_2 > HEIGHT_CRITICAL_1 )); then
  KEPT_AUTHORING=1
  check "authoring stops while free space is below the floor" 1
else
  KEPT_AUTHORING=0
  check "authoring stops while free space is below the floor" 0
fi
if kill -0 "$NODE_PID" 2>/dev/null && rpc "$RPC_PORT" chain_getBlockHash '[0]' | grep -q '"result"'; then
  check "a paused node keeps running and answering RPC" 0
  PAUSED_AND_ALIVE=1
else
  check "a paused node keeps running and answering RPC" 1
  PAUSED_AND_ALIVE=0
fi
if grep -q "authoring paused" "$AUTHORITY_LOG"; then
  check "the pause says why, in the log" 0
else
  check "the pause says why, in the log" 1
fi

# ── 5. true ENOSPC, and recovery ────────────────────────────────────────────
echo "[5/5] filling the filesystem to ENOSPC ..."
dd if=/dev/zero of="$FS/filler2" bs=1M count="$(( FS_MB ))" status=none || true
echo "      free now $(free_mb) MB"

sleep 10
HEIGHT_AT_ENOSPC="$(height "$RPC_PORT")"
sleep 20
HEIGHT_AT_ENOSPC_2="$(height "$RPC_PORT")"
ALIVE_AT_ENOSPC=1
if kill -0 "$NODE_PID" 2>/dev/null; then
  ALIVE_AT_ENOSPC=0
fi
echo "      block $HEIGHT_AT_ENOSPC -> $HEIGHT_AT_ENOSPC_2 ($(grep -ciE 'error|panic' "$AUTHORITY_LOG") error/panic lines so far)"

echo "      MEASURED alive after ENOSPC: $(( 1 - ALIVE_AT_ENOSPC ))"

echo "      freeing the space back ..."
find "$FS" -maxdepth 1 -name 'filler*' -delete 2>/dev/null || true
sleep 20
HEIGHT_AFTER_FREE="$(height "$RPC_PORT")"
ALIVE_AFTER=1
if kill -0 "$NODE_PID" 2>/dev/null; then
  ALIVE_AFTER=0
fi
echo "      free now $(free_mb) MB; block $HEIGHT_AFTER_FREE"

if (( ALIVE_AFTER == 0 )); then
  echo "      MEASURED the node survived ENOSPC and space returning"
  if (( HEIGHT_AFTER_FREE > HEIGHT_AT_ENOSPC_2 )); then
    check "it resumed producing blocks without a restart" 0
  else
    check "it resumed producing blocks without a restart" 1
  fi
else
  # It died. The operator's real question is then "is this recoverable, or is the
  # database ruined?" — so restart it on the same base path with space available
  # and see whether the chain comes back.
  echo "      the node died at ENOSPC; restarting it on the same database ..."
  X3_MIN_FREE_DISK_BYTES=$(( FLOOR_MB * 1024 * 1024 )) X3_DISK_PROBE_SECS=2 \
    "$NODE_BIN" --dev --base-path "$AUTHORITY_BASE" --rpc-port "$RPC_PORT" \
      --log warn >> "$AUTHORITY_LOG" 2>&1 &
  NODE_PID=$!
  if wait_for_rpc "$RPC_PORT" "$NODE_PID" "$AUTHORITY_LOG" 300; then
    check "a node that died of ENOSPC restarts on the same database" 0
    RESTART_HEIGHT="$(height "$RPC_PORT")"
    sleep 15
    RESTART_HEIGHT_LATER="$(height "$RPC_PORT")"
    echo "      after restart: block $RESTART_HEIGHT -> $RESTART_HEIGHT_LATER"
    if (( RESTART_HEIGHT_LATER > RESTART_HEIGHT )); then
      check "it produces blocks again after the restart" 0
    else
      check "it produces blocks again after the restart" 1
    fi
  else
    check "a node that died of ENOSPC restarts on the same database" 1
  fi
fi

# Whatever else happened, the log has to be kept: this is the evidence for a
# failure mode nobody can reproduce on demand once the volume is fixed.
LOG_COPY="${X3_DRILL_LOG_OUT:-$REPO_ROOT/.goal-logs/disk-full-drill.log}"
mkdir -p "$(dirname "$LOG_COPY")" 2>/dev/null || true
cp "$AUTHORITY_LOG" "$LOG_COPY" 2>/dev/null || true

echo
echo "the disk-space guard is the only protection here, and it is advisory:"
echo "  watchdog reported Critical: $(grep -c 'at or below the' "$AUTHORITY_LOG") time(s)"
echo "  MEASURED authoring continued while Critical: $KEPT_AUTHORING (blocks $HEIGHT_CRITICAL_1 -> $HEIGHT_CRITICAL_2 with free space below the floor)"
echo "  SCORED   authoring paused and node still serving: $PAUSED_AND_ALIVE (log: $(grep -c 'authoring paused' "$AUTHORITY_LOG") pause line(s))"
echo "  MEASURED node alive after ENOSPC: $(( 1 - ALIVE_AT_ENOSPC )) (blocks went to $HEIGHT_AT_ENOSPC_2)"
echo "  blocks: before $HEIGHT_BEFORE, at ENOSPC $HEIGHT_AT_ENOSPC_2, after space returned $HEIGHT_AFTER_FREE"
echo "  errors/panics in the log: $(grep -ciE 'panic|No space left|disk full' "$AUTHORITY_LOG")"
grep -iE 'panic|No space left on device|Corruption|database' "$AUTHORITY_LOG" | tail -4 | sed 's/^/    /'
echo "  guard log lines:"
grep -i "disk-space guard" "$AUTHORITY_LOG" | sed 's/^/    /' | head -5
echo

if (( failures == 0 )); then
  echo "PASS — an authority below the floor is refused at startup; at the floor authoring"
  echo "       pauses while the node keeps serving and says why; and a node that died at"
  echo "       true ENOSPC restarts on the same database and produces blocks again."
  exit 0
fi
echo "FAIL — $failures check(s) failed"
exit 1
