#!/usr/bin/env bash
# Power-loss drill: SIGKILL a node mid-work and prove that the chain it had
# already finalized is still there when it comes back.
#
# The audit's §14 asks what happens on power loss. Nothing tested it: the only
# restart script in the tree (`scripts/drills/node_restart_drill.sh`) sends
# SIGTERM — a graceful shutdown — and then starts a node on a *fresh* base path,
# so it never opens the database that was interrupted. That is a start-up test
# wearing a crash test's name.
#
# This one kills the process the way a power cut does, `SIGKILL`, while it is
# working, and then makes four claims checkable:
#
#   A. the *author*: whatever it had finalized before the kill is still in its
#      database afterwards, at the same height *and the same hash*;
#   B. the *importer*: a full node killed mid-import comes back on the same
#      database, catches up, and agrees with the author about a height they both
#      have — same block hash, same state root.
#
# A block that was not finalized may be re-authored or re-imported; that is
# correct behaviour, not data loss, and the drill does not pretend otherwise.
#
# Usage: bash scripts/mainnet/prove-crash-consistency.sh
#   X3_NODE_BIN   node binary (default: target/{release,debug}/x3-chain-node)
#   X3_KEEP_TMP   1 to keep the work directory and its logs
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
NODE_BIN="${X3_NODE_BIN:-}"
if [[ -z "$NODE_BIN" ]]; then
  for candidate in target/release/x3-chain-node target/debug/x3-chain-node; do
    if [[ -x "$REPO_ROOT/$candidate" ]]; then
      NODE_BIN="$REPO_ROOT/$candidate"
      break
    fi
  done
fi
if [[ -z "$NODE_BIN" || ! -x "$NODE_BIN" ]]; then
  echo "error: x3-chain-node not found; set X3_NODE_BIN or build it" >&2
  exit 2
fi

WORK="$(mktemp -d "${TMPDIR:-/tmp}/x3-crash-drill-XXXXXX")"
AUTHOR_BASE="$WORK/author"
IMPORTER_BASE="$WORK/importer"
AUTHOR_LOG="$WORK/author.log"
IMPORTER_LOG="$WORK/importer.log"
AUTHOR_PID=""
IMPORTER_PID=""

cleanup() {
  local exit_code=$?
  for pid in "$AUTHOR_PID" "$IMPORTER_PID"; do
    if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
      kill -TERM "$pid" 2>/dev/null || true
    fi
  done
  sleep 2
  for pid in "$AUTHOR_PID" "$IMPORTER_PID"; do
    if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
      kill -KILL "$pid" 2>/dev/null || true
    fi
  done
  if [[ "${X3_KEEP_TMP:-0}" == "1" ]]; then
    echo "work directory kept: $WORK"
  else
    find "$WORK" -depth -delete 2>/dev/null || true
  fi
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

rpc() { # <port> <method> <params-json>
  curl -sS -m 15 -X POST -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"$2\",\"params\":$3,\"id\":1}" \
    "http://127.0.0.1:$1"
}

field() { # <dotted-path>, JSON on stdin
  python3 -c 'import json, sys
value = json.load(sys.stdin)
for step in sys.argv[1].split("."):
    value = value[step]
print(value)' "$1"
}

height() { # <port>
  local hex
  hex="$(rpc "$1" chain_getHeader '[]' | field result.number 2>/dev/null || echo 0x0)"
  python3 -c 'import sys; print(int(sys.argv[1], 16))' "$hex"
}

wait_for_rpc() { # <port> <pid> <log> <seconds>
  local port="$1" pid="$2" log="$3" deadline="$4" waited=0
  while (( waited < deadline )); do
    if rpc "$port" chain_getBlockHash '[0]' 2>/dev/null | grep -q '"result"'; then
      return 0
    fi
    if ! kill -0 "$pid" 2>/dev/null; then
      echo "error: node exited during startup; log tail:" >&2
      tail -20 "$log" >&2
      return 1
    fi
    sleep 5
    waited=$(( waited + 5 ))
  done
  echo "error: RPC on $port did not come up in ${deadline}s; log tail:" >&2
  tail -20 "$log" >&2
  return 1
}

wait_for_height() { # <port> <pid> <target> <seconds>
  local port="$1" pid="$2" target="$3" deadline="$4" waited=0 current
  while (( waited < deadline )); do
    current="$(height "$port" 2>/dev/null || echo 0)"
    if (( current >= target )); then
      return 0
    fi
    if ! kill -0 "$pid" 2>/dev/null; then
      return 1
    fi
    sleep 3
    waited=$(( waited + 3 ))
  done
  return 1
}

check() { # <description> <0 for pass>
  if [[ "$2" == "0" ]]; then
    echo "PASS  $1"
  else
    echo "FAIL  $1"
    failures=$(( failures + 1 ))
  fi
}

same() { # <a> <b>  -> 0 when equal, case-insensitively
  [[ "${1,,}" == "${2,,}" ]] && echo 0 || echo 1
}

failures=0
AUTHOR_RPC="$(free_port)"
AUTHOR_P2P="$(free_port)"
AUTHOR_PROM="$(free_port)"
IMPORTER_RPC="$(free_port)"
IMPORTER_PROM="$(free_port)"

echo "node:  $NODE_BIN"
echo "work:  $WORK"
echo

# ── The author, and a second node that imports from it ──────────────────────
echo "[1/5] starting the author on :$AUTHOR_RPC ..."
"$NODE_BIN" --dev --base-path "$AUTHOR_BASE" --port "$AUTHOR_P2P" \
  --rpc-port "$AUTHOR_RPC" --prometheus-port "$AUTHOR_PROM" --log warn \
  > "$AUTHOR_LOG" 2>&1 &
AUTHOR_PID=$!
wait_for_rpc "$AUTHOR_RPC" "$AUTHOR_PID" "$AUTHOR_LOG" 420

AUTHOR_PEER="$(rpc "$AUTHOR_RPC" system_localPeerId '[]' | field result)"
echo "      author peer $AUTHOR_PEER"

echo "[2/5] starting a full node that imports the author's blocks ..."
"$NODE_BIN" --chain dev --base-path "$IMPORTER_BASE" --port "$(free_port)" \
  --bootnodes "/ip4/127.0.0.1/tcp/$AUTHOR_P2P/p2p/$AUTHOR_PEER" \
  --rpc-port "$IMPORTER_RPC" --prometheus-port "$IMPORTER_PROM" --log warn \
  > "$IMPORTER_LOG" 2>&1 &
IMPORTER_PID=$!
wait_for_rpc "$IMPORTER_RPC" "$IMPORTER_PID" "$IMPORTER_LOG" 420

# Let it import for a while, so the kill lands with imported (and not yet
# finalized) blocks in the database — the interesting state to interrupt.
wait_for_height "$IMPORTER_RPC" "$IMPORTER_PID" "$(( $(height "$AUTHOR_RPC") + 25 ))" 240 || true

AUTHOR_FINAL_HASH="$(rpc "$AUTHOR_RPC" chain_getFinalizedHead '[]' | field result)"
AUTHOR_FINAL_HEIGHT="$(rpc "$AUTHOR_RPC" chain_getHeader "[\"$AUTHOR_FINAL_HASH\"]" | field result.number)"
AUTHOR_FINAL_HEIGHT="$(( AUTHOR_FINAL_HEIGHT ))"
AUTHOR_FINAL_ROOT="$(rpc "$AUTHOR_RPC" chain_getHeader "[\"$AUTHOR_FINAL_HASH\"]" | field result.stateRoot)"
GENESIS_HASH="$(rpc "$AUTHOR_RPC" chain_getBlockHash '[0]' | field result)"

echo "      author best $(height "$AUTHOR_RPC"), finalized $AUTHOR_FINAL_HEIGHT"
echo "      importer best $(height "$IMPORTER_RPC")"

# ── The kill ────────────────────────────────────────────────────────────────
echo "[3/5] SIGKILL both nodes (no shutdown, exactly like a power cut) ..."
kill -KILL "$AUTHOR_PID"
kill -KILL "$IMPORTER_PID"
sleep 3
if kill -0 "$AUTHOR_PID" 2>/dev/null || kill -0 "$IMPORTER_PID" 2>/dev/null; then
  echo "error: a node survived SIGKILL" >&2
  exit 1
fi
AUTHOR_PID=""
IMPORTER_PID=""

# ── A: the author keeps everything it had finalized ─────────────────────────
echo "[4/5] restarting the author on its own database ..."
"$NODE_BIN" --dev --base-path "$AUTHOR_BASE" --port "$AUTHOR_P2P" \
  --rpc-port "$AUTHOR_RPC" --prometheus-port "$AUTHOR_PROM" --log warn \
  > "$AUTHOR_LOG.restart" 2>&1 &
AUTHOR_PID=$!
wait_for_rpc "$AUTHOR_RPC" "$AUTHOR_PID" "$AUTHOR_LOG.restart" 420

RESTARTED_GENESIS="$(rpc "$AUTHOR_RPC" chain_getBlockHash '[0]' | field result)"
RESTARTED_HASH="$(rpc "$AUTHOR_RPC" chain_getBlockHash "[$AUTHOR_FINAL_HEIGHT]" | field result)"
RESTARTED_ROOT="$(rpc "$AUTHOR_RPC" chain_getHeader "[\"$RESTARTED_HASH\"]" | field result.stateRoot)"

echo "      after restart: genesis $RESTARTED_GENESIS"
echo "      height $AUTHOR_FINAL_HEIGHT is $RESTARTED_HASH"
echo "      its state root is $RESTARTED_ROOT"
echo
check "the restarted author has the same genesis" \
  "$(same "$RESTARTED_GENESIS" "$GENESIS_HASH")"
check "the block finalized before the kill is still at its height" \
  "$(same "$RESTARTED_HASH" "$AUTHOR_FINAL_HASH")"
check "and its state root is unchanged" \
  "$(same "$RESTARTED_ROOT" "$AUTHOR_FINAL_ROOT")"

# ── B: the importer comes back, catches up, and agrees ──────────────────────
echo "[5/5] restarting the importer on its own database ..."
"$NODE_BIN" --chain dev --base-path "$IMPORTER_BASE" --port "$(free_port)" \
  --bootnodes "/ip4/127.0.0.1/tcp/$AUTHOR_P2P/p2p/$AUTHOR_PEER" \
  --rpc-port "$IMPORTER_RPC" --prometheus-port "$IMPORTER_PROM" --log warn \
  > "$IMPORTER_LOG.restart" 2>&1 &
IMPORTER_PID=$!
wait_for_rpc "$IMPORTER_RPC" "$IMPORTER_PID" "$IMPORTER_LOG.restart" 420

AUTHOR_HEIGHT_NOW="$(height "$AUTHOR_RPC")"
CAUGHT_UP=1
if wait_for_height "$IMPORTER_RPC" "$IMPORTER_PID" "$AUTHOR_HEIGHT_NOW" 300; then
  CAUGHT_UP=0
fi
IMPORTER_GENESIS="$(rpc "$IMPORTER_RPC" chain_getBlockHash '[0]' | field result)"

COMMON_HEIGHT="$AUTHOR_FINAL_HEIGHT"
AUTHOR_COMMON_HASH="$(rpc "$AUTHOR_RPC" chain_getBlockHash "[$COMMON_HEIGHT]" | field result)"
IMPORTER_COMMON_HASH="$(rpc "$IMPORTER_RPC" chain_getBlockHash "[$COMMON_HEIGHT]" | field result)"
AUTHOR_COMMON_ROOT="$(rpc "$AUTHOR_RPC" chain_getHeader "[\"$AUTHOR_COMMON_HASH\"]" | field result.stateRoot)"
IMPORTER_COMMON_ROOT="$(rpc "$IMPORTER_RPC" chain_getHeader "[\"$IMPORTER_COMMON_HASH\"]" | field result.stateRoot 2>/dev/null || echo unknown)"

echo "      author $AUTHOR_HEIGHT_NOW, importer $(height "$IMPORTER_RPC")"
echo "      height $COMMON_HEIGHT: author   $AUTHOR_COMMON_HASH"
echo "                            importer $IMPORTER_COMMON_HASH"
echo
check "the restarted importer has the same genesis" \
  "$(same "$IMPORTER_GENESIS" "$GENESIS_HASH")"
check "the importer caught up with the author after the kill" "$CAUGHT_UP"
check "both nodes agree on the block at height $COMMON_HEIGHT" \
  "$(same "$AUTHOR_COMMON_HASH" "$IMPORTER_COMMON_HASH")"
check "both nodes agree on its state root" \
  "$(same "$AUTHOR_COMMON_ROOT" "$IMPORTER_COMMON_ROOT")"

echo
if (( failures == 0 )); then
  echo "PASS — SIGKILL interrupted both nodes with blocks in flight; the author kept"
  echo "       every finalized block (height $AUTHOR_FINAL_HEIGHT, $AUTHOR_FINAL_HASH) and the"
  echo "       importer re-imported on its own database and agreed with it."
  exit 0
fi
echo "FAIL — $failures check(s) failed"
exit 1
