#!/usr/bin/env bash
# Prove that a snapshot taken from a running chain can be put back into a node.
#
# `verify-snapshot-root-against-chain.sh` answers "does the tool agree with the
# chain about what the state hashes to?". This script answers the next question,
# the one the storage audit actually asked: can the state be *rebuilt*? A format
# that can be verified and never restored is a backup that has never been tested.
#
# The procedure, all against one machine:
#
#   1. start a chain, let it produce blocks, and read the best block's number,
#      hash and state root from the node's own header (`chain_getHeader`) — the
#      chain's value, not the tool's;
#   2. stop the node (the database is locked while it runs) and dump that block's
#      state with the node's own `export-state`;
#   3. `root` the dumped state and require it to equal the header's root: the tool
#      and the chain agree at a real, non-genesis block;
#   4. `build` a snapshot anchored to that block;
#   5. `restore` the snapshot into a raw chain spec;
#   6. boot a *second* node from the restored spec and read the state root out of
#      its genesis header. The node recomputed that root from the restored spec
#      alone, so equality means the snapshot rebuilt the chain's state;
#   7. tamper with one chunk and require the restore to refuse it.
#
# What this does not claim: a node booted from restored state is not joined to the
# source chain. Its genesis header is number 0 while the state it holds is block
# N's, so the runtime refuses to author (frame-system: "Block number must be
# strictly increasing") — the same state/header split that makes this a state
# transport rather than a chain fork. A validator joining at height N needs the
# state installed behind that header (state/warp sync), which is client work and
# is tracked separately.
#
# Usage:
#   verify-snapshot-restore.sh
#     X3_NODE_BIN      node binary (default: target/{release,debug}/x3-chain-node)
#     X3_SNAPSHOT_BIN  x3-state-snapshot binary (default: target/…/x3-state-snapshot)
#     X3_STATE_VERSION trie layout to assume, 0 or 1 (default: 1)
#     X3_KEEP_TMP      set to 1 to keep the work directory and its logs
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
STATE_VERSION="${X3_STATE_VERSION:-1}"

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

WORK="$(mktemp -d "${TMPDIR:-/tmp}/x3-restore-check-XXXXXX")"
SOURCE_BASE="$WORK/source-chain"
RESTORED_BASE="$WORK/restored-chain"
EXPORTED_SPEC="$WORK/exported-state.json"
RESTORED_SPEC="$WORK/restored-state.json"
SNAPSHOT_DIR="$WORK/snapshot"
TAMPERED_DIR="$WORK/tampered"

SOURCE_PID=""
RESTORED_PID=""

cleanup() {
  local exit_code=$?
  for pid in "$SOURCE_PID" "$RESTORED_PID"; do
    if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
      kill -TERM "$pid" 2>/dev/null || true
    fi
  done
  sleep 2
  for pid in "$SOURCE_PID" "$RESTORED_PID"; do
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
  python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
}

rpc() { # <port> <method> <params-json>
  curl -sS -m 15 -X POST -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"$2\",\"params\":$3,\"id\":1}" \
    "http://127.0.0.1:$1"
}

json_field() { # <field-path>  (reads stdin)
  python3 -c "
import json, sys
value = json.load(sys.stdin)
for step in sys.argv[1].split('.'):
    value = value[step]
print(value)
" "$1"
}

wait_for_rpc() { # <port> <log> <seconds>
  local port="$1" log="$2" deadline="$3" waited=0
  while (( waited < deadline )); do
    if rpc "$port" chain_getBlockHash '[0]' 2>/dev/null | grep -q '"result"'; then
      return 0
    fi
    if ! kill -0 "$SOURCE_PID" 2>/dev/null && [[ "$port" == "$SOURCE_PORT" ]]; then
      echo "error: the source node exited during startup; log:" >&2
      tail -20 "$log" >&2
      return 1
    fi
    sleep 5
    waited=$(( waited + 5 ))
  done
  echo "error: RPC on port $port did not come up in ${deadline}s; log:" >&2
  tail -20 "$log" >&2
  return 1
}

SOURCE_PORT="$(free_port)"
SOURCE_PROM="$(free_port)"
RESTORED_PORT="$(free_port)"
RESTORED_PROM="$(free_port)"

echo "binaries:"
echo "  node:     $NODE_BIN"
echo "  snapshot: $SNAPSHOT_BIN"
echo "work:     $WORK"
echo

echo "[1/7] starting a source chain (rpc :$SOURCE_PORT) ..."
"$NODE_BIN" --dev --base-path "$SOURCE_BASE" \
  --rpc-port "$SOURCE_PORT" --prometheus-port "$SOURCE_PROM" \
  --log warn > "$WORK/source-node.log" 2>&1 &
SOURCE_PID=$!
wait_for_rpc "$SOURCE_PORT" "$WORK/source-node.log" 420

# Let it get past genesis so the snapshot is of a real, non-empty block rather
# than of the genesis state a raw spec already carries.
echo "      waiting for the chain to produce blocks ..."
for _ in $(seq 1 60); do
  HEAD_NUMBER_HEX="$(rpc "$SOURCE_PORT" chain_getHeader '[]' | json_field result.number 2>/dev/null || echo 0x0)"
  if [[ "$(( HEAD_NUMBER_HEX ))" -ge 2 ]]; then
    break
  fi
  sleep 5
done

HEAD_HASH="$(rpc "$SOURCE_PORT" chain_getBlockHash '[null]' | json_field result)"
HEAD_NUMBER_HEX="$(rpc "$SOURCE_PORT" chain_getHeader "[\"$HEAD_HASH\"]" | json_field result.number)"
CHAIN_ROOT="$(rpc "$SOURCE_PORT" chain_getHeader "[\"$HEAD_HASH\"]" | json_field result.stateRoot)"
SPEC_VERSION="$(rpc "$SOURCE_PORT" state_getRuntimeVersion '[]' | json_field result.specVersion)"
HEAD_NUMBER="$(( HEAD_NUMBER_HEX ))"

echo "      block $HEAD_NUMBER ($HEAD_HASH)"
echo "      chain root $CHAIN_ROOT (spec_version $SPEC_VERSION)"

echo "[2/7] stopping the source node and exporting state at that block ..."
kill -TERM "$SOURCE_PID"
for _ in $(seq 1 60); do
  kill -0 "$SOURCE_PID" 2>/dev/null || break
  sleep 2
done
if kill -0 "$SOURCE_PID" 2>/dev/null; then
  echo "error: the source node did not stop; cannot open its database" >&2
  exit 1
fi
SOURCE_PID=""

if ! "$NODE_BIN" export-state --dev --base-path "$SOURCE_BASE" "$HEAD_HASH" \
    > "$EXPORTED_SPEC" 2> "$WORK/export-state.log"; then
  echo "error: export-state failed; log:" >&2
  tail -20 "$WORK/export-state.log" >&2
  exit 1
fi
EXPORTED_ENTRIES="$(python3 -c "
import json, sys
spec = json.load(open(sys.argv[1]))
print(len(spec['genesis']['raw']['top']))
" "$EXPORTED_SPEC")"
echo "      exported $EXPORTED_ENTRIES storage entries"

echo "[3/7] does the tool reproduce the chain's root at block $HEAD_NUMBER?"
DERIVED_ROOT="$("$SNAPSHOT_BIN" root --from-raw-spec "$EXPORTED_SPEC" --state-version "$STATE_VERSION")"
echo "      chain      $CHAIN_ROOT"
echo "      recomputed $DERIVED_ROOT"
if [[ "${DERIVED_ROOT,,}" != "${CHAIN_ROOT,,}" ]]; then
  echo "MISMATCH: the tool and the chain disagree about the state root at block $HEAD_NUMBER" >&2
  exit 1
fi

echo "[4/7] building an anchored snapshot ..."
"$SNAPSHOT_BIN" build --from-raw-spec "$EXPORTED_SPEC" --out "$SNAPSHOT_DIR" \
  --chain-id x3_chain_dev \
  --block-number "$HEAD_NUMBER" \
  --block-hash "$HEAD_HASH" \
  --state-root "$CHAIN_ROOT" \
  --runtime-version "$SPEC_VERSION" \
  --state-version "$STATE_VERSION" \
  --finality-proof 0x00 \
  > "$WORK/build.log"
cat "$WORK/build.log"

echo "[5/7] restoring the snapshot into a chain spec ..."
"$SNAPSHOT_BIN" restore --manifest "$SNAPSHOT_DIR/manifest.json" --chunks "$SNAPSHOT_DIR" \
  --out "$RESTORED_SPEC" --from-spec "$EXPORTED_SPEC" \
  --name "X3 Chain Development (state restored at block $HEAD_NUMBER)" \
  --id "x3_chain_dev_restored_$HEAD_NUMBER" \
  --chain-id x3_chain_dev \
  --block-hash "$HEAD_HASH" \
  --state-root "$CHAIN_ROOT" \
  --runtime-version "$SPEC_VERSION" \
  --state-version "$STATE_VERSION" \
  > "$WORK/restore.log"
cat "$WORK/restore.log"

echo "[6/7] booting a node from the restored spec (rpc :$RESTORED_PORT) ..."
"$NODE_BIN" --dev --chain "$RESTORED_SPEC" --base-path "$RESTORED_BASE" \
  --rpc-port "$RESTORED_PORT" --prometheus-port "$RESTORED_PROM" \
  --log warn > "$WORK/restored-node.log" 2>&1 &
RESTORED_PID=$!

for _ in $(seq 1 84); do
  if rpc "$RESTORED_PORT" chain_getBlockHash '[0]' 2>/dev/null | grep -q '"result"'; then
    break
  fi
  if ! kill -0 "$RESTORED_PID" 2>/dev/null; then
    echo "error: the restored node exited; log:" >&2
    tail -20 "$WORK/restored-node.log" >&2
    exit 1
  fi
  sleep 5
done

RESTORED_GENESIS="$(rpc "$RESTORED_PORT" chain_getBlockHash '[0]' | json_field result)"
RESTORED_ROOT="$(rpc "$RESTORED_PORT" chain_getHeader "[\"$RESTORED_GENESIS\"]" | json_field result.stateRoot)"
RESTORED_SPEC_VERSION="$(rpc "$RESTORED_PORT" state_getRuntimeVersion '[]' | json_field result.specVersion)"

echo "      restored genesis $RESTORED_GENESIS"
echo "      node-recomputed root $RESTORED_ROOT"
echo "      runtime spec_version $RESTORED_SPEC_VERSION (from the restored :code)"
if [[ "${RESTORED_ROOT,,}" != "${CHAIN_ROOT,,}" ]]; then
  echo "MISMATCH: the node booted from the restored spec computed state root" >&2
  echo "  $RESTORED_ROOT" >&2
  echo "which is not block $HEAD_NUMBER's root $CHAIN_ROOT" >&2
  exit 1
fi
if [[ "$RESTORED_SPEC_VERSION" != "$SPEC_VERSION" ]]; then
  echo "MISMATCH: the restored chain runs spec_version $RESTORED_SPEC_VERSION, the source ran $SPEC_VERSION" >&2
  exit 1
fi

echo "[7/7] tampering with a chunk must be refused ..."
mkdir -p "$TAMPERED_DIR"
cp "$SNAPSHOT_DIR"/*.chunk "$TAMPERED_DIR/"
python3 - "$TAMPERED_DIR/0.chunk" <<'PY'
import pathlib, sys
path = pathlib.Path(sys.argv[1])
data = bytearray(path.read_bytes())
data[len(data) // 2] ^= 0xff
path.write_bytes(data)
PY

if "$SNAPSHOT_BIN" restore --manifest "$SNAPSHOT_DIR/manifest.json" --chunks "$TAMPERED_DIR" \
    --out "$WORK/from-tampered.json" --from-spec "$EXPORTED_SPEC" \
    --chain-id x3_chain_dev --block-hash "$HEAD_HASH" --state-root "$CHAIN_ROOT" \
    --runtime-version "$SPEC_VERSION" --state-version "$STATE_VERSION" \
    > "$WORK/tampered.log" 2>&1; then
  echo "FAIL: a chunk with a flipped byte was restored instead of refused" >&2
  cat "$WORK/tampered.log" >&2
  exit 1
fi
if [[ -e "$WORK/from-tampered.json" ]]; then
  echo "FAIL: the refused restore still wrote a chain spec" >&2
  exit 1
fi
echo "      refused as expected: $(tail -1 "$WORK/tampered.log")"

echo
echo "PASS — a snapshot of block $HEAD_NUMBER restored into a chain spec, and a node"
echo "       booted from that spec recomputed the chain's own state root:"
echo "         $CHAIN_ROOT"
echo "       ($EXPORTED_ENTRIES entries, $EXPORTED_SPEC)"
echo
echo "Not claimed: the restored node is not joined to the source chain. Its genesis"
echo "header is number 0 while its state is block $HEAD_NUMBER's, so frame-system"
echo "refuses to author on it (\"Block number must be strictly increasing\"). Joining"
echo "at a height is state/warp sync and is tracked separately."
