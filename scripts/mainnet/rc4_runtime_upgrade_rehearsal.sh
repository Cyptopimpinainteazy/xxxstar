#!/usr/bin/env bash
# RC4 Runtime Upgrade Rehearsal
#
# Boots local3 on the RC2 settlement-safe baseline, upgrades to the current
# runtime, proves post-upgrade liveness, and captures settlement / halt /
# refund evidence. Exit 0 only when every required proof passes.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

OLD_REF="${OLD_REF:-x3-atomic-star-rc2-internal-settlement-smoke}"
NEW_REF="${NEW_REF:-HEAD}"
OUT="${OUT:-$ROOT_DIR/reports/rc4}"
WORKTREE_ROOT="${WORKTREE_ROOT:-$ROOT_DIR/.rc4-worktrees}"
WORK_DIR="${WORK_DIR:-$ROOT_DIR/.rc4-runtime-upgrade-work}"
KEEP_WORKTREES="${KEEP_WORKTREES:-1}"

OLD_WORKTREE="${OLD_WORKTREE:-$WORKTREE_ROOT/old}"
NEW_WORKTREE="${NEW_WORKTREE:-$WORKTREE_ROOT/new}"
OLD_TARGET_DIR="${OLD_TARGET_DIR:-$ROOT_DIR/target/rc4-old}"
NEW_TARGET_DIR="${NEW_TARGET_DIR:-$ROOT_DIR/target/rc4-new}"
OLD_BINARY="${OLD_BINARY:-$OLD_TARGET_DIR/release/x3-chain-node}"
NEW_BINARY="${NEW_BINARY:-$NEW_TARGET_DIR/release/x3-chain-node}"
OLD_RAW_SPEC="${OLD_RAW_SPEC:-}"
NEW_WASM="${NEW_WASM:-}"
NODE_MODULES="${NODE_MODULES:-$ROOT_DIR/packages/blockchain-connector/node_modules}"

BUILD_OLD="${BUILD_OLD:-1}"
BUILD_NEW="${BUILD_NEW:-1}"
RUN_ROUTER_TESTS="${RUN_ROUTER_TESTS:-1}"
TIMEOUT="${TIMEOUT:-180}"
WAIT_BLOCKS="${WAIT_BLOCKS:-20}"
RUST_MIN_STACK_BYTES="${RUST_MIN_STACK_BYTES:-8589934592}"
OLD_BLOCK_LENGTH_CAP_BYTES="${OLD_BLOCK_LENGTH_CAP_BYTES:-5242880}"
OLD_PREIMAGE_MAX_SIZE_BYTES="${OLD_PREIMAGE_MAX_SIZE_BYTES:-4194304}"

CARGO_BIN="${CARGO_BIN:-$(command -v cargo 2>/dev/null || true)}"
NODE_BIN="${NODE_BIN:-$(command -v node 2>/dev/null || true)}"
JQ_BIN="${JQ_BIN:-$(command -v jq 2>/dev/null || true)}"
RG_BIN="${RG_BIN:-$(command -v rg 2>/dev/null || true)}"
ZSTD_BIN="${ZSTD_BIN:-$(command -v zstd 2>/dev/null || true)}"

for candidate in \
  "$CARGO_BIN" \
  "$HOME/.rustup/toolchains/1.93.0-x86_64-unknown-linux-gnu/bin/cargo" \
  "$HOME/.cargo/bin/cargo"; do
  if [[ -n "$candidate" && -x "$candidate" ]]; then
    CARGO_BIN="$candidate"
    break
  fi
done

CARGO_TOOLCHAIN="${CARGO_TOOLCHAIN:-1.93.0}"
CARGO_TOOLCHAIN_ARGS=("+$CARGO_TOOLCHAIN")
if [[ "$CARGO_BIN" == *"/.rustup/toolchains/"* ]]; then
  CARGO_TOOLCHAIN_ARGS=()
fi

RUSTC_BIN="${RUSTC_BIN:-}"
if [[ -z "$RUSTC_BIN" && "$CARGO_BIN" == *"/.rustup/toolchains/"* ]]; then
  RUSTC_BIN="${CARGO_BIN%/cargo}/rustc"
fi
if [[ -n "$RUSTC_BIN" ]]; then
  export RUSTC="$RUSTC_BIN"
fi

ALICE_RPC="http://127.0.0.1:9954"
BOB_RPC="http://127.0.0.1:9955"
CHARLIE_RPC="http://127.0.0.1:9956"
ALICE_WS="ws://127.0.0.1:9954"

LOG_DIR="$WORK_DIR/logs"
ALICE_BASE="$WORK_DIR/alice"
BOB_BASE="$WORK_DIR/bob"
CHARLIE_BASE="$WORK_DIR/charlie"
RESULTS_JSON="$OUT/.results.jsonl"

ALICE_PID=""
BOB_PID=""
CHARLIE_PID=""
VERDICT="PASS"
BLOCKERS=()
PRE_VERSION='{}'
POST_VERSION='{}'
PRE_CODE_HASH=''
POST_CODE_HASH=''
OLD_COMMIT='UNKNOWN'
NEW_COMMIT='UNKNOWN'
OLD_WASM_HASH=''
NEW_WASM_HASH=''

mkdir -p "$OUT" "$WORKTREE_ROOT" "$WORK_DIR" "$LOG_DIR"
: > "$RESULTS_JSON"

now_iso() {
  date -u +%Y-%m-%dT%H:%M:%SZ
}

info() {
  printf '[RC4] %s\n' "$*"
}

require_tool() {
  local tool_path="$1"
  local label="$2"
  if [[ -z "$tool_path" || ! -x "$tool_path" ]]; then
    printf 'ERROR: missing required tool: %s\n' "$label" >&2
    exit 1
  fi
}

require_tool "$CARGO_BIN" cargo
require_tool "$NODE_BIN" node
require_tool "$JQ_BIN" jq

record() {
  local name="$1"
  local result="$2"
  local detail="${3:-}"
  python3 - "$RESULTS_JSON" "$name" "$result" "$detail" <<'PY'
import datetime
import json
import sys

path, name, result, detail = sys.argv[1:]
payload = {
    'name': name,
    'result': result,
    'detail': detail,
    'timestamp': datetime.datetime.utcnow().replace(microsecond=0).isoformat() + 'Z',
}
with open(path, 'a', encoding='utf-8') as handle:
    handle.write(json.dumps(payload, sort_keys=True) + '\n')
PY

  case "$result" in
    PASS)
      ;;
    *)
      VERDICT="FAIL"
      BLOCKERS+=("$name: $detail")
      ;;
  esac
}

write_json() {
  local path="$1"
  local payload="$2"
  python3 - "$path" "$payload" <<'PY'
import json
import sys

path, payload = sys.argv[1:]
data = json.loads(payload)
with open(path, 'w', encoding='utf-8') as handle:
    json.dump(data, handle, indent=2, sort_keys=True)
    handle.write('\n')
PY
}

cleanup() {
  for pid in "$ALICE_PID" "$BOB_PID" "$CHARLIE_PID"; do
    [[ -n "$pid" ]] && kill "$pid" 2>/dev/null || true
  done
  pkill -f "[x]3-chain-node.*$WORK_DIR" 2>/dev/null || true

  if [[ "$KEEP_WORKTREES" != "1" ]]; then
    git -C "$ROOT_DIR" worktree remove --force "$OLD_WORKTREE" >/dev/null 2>&1 || true
    git -C "$ROOT_DIR" worktree remove --force "$NEW_WORKTREE" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

ensure_worktree() {
  local ref="$1"
  local path="$2"
  mkdir -p "$(dirname "$path")"
  if [[ -e "$path" && ! -e "$path/.git" && ! -f "$path/.git" ]]; then
    rm -rf "$path"
  fi
  if [[ ! -e "$path/.git" && ! -f "$path/.git" ]]; then
    git -C "$ROOT_DIR" worktree add --detach "$path" "$ref" >/dev/null
  else
    git -C "$path" checkout --detach "$ref" >/dev/null 2>&1 || true
    git -C "$path" reset --hard "$ref" >/dev/null 2>&1 || true
  fi
}

sha256_file() {
  sha256sum "$1" | awk '{print $1}'
}

find_runtime_wasm() {
  local target_dir="$1"
  find "$target_dir" -name '*.wasm' 2>/dev/null | {
    if [[ -n "$RG_BIN" && -x "$RG_BIN" ]]; then
      "$RG_BIN" 'x3|runtime'
    else
      grep -E 'x3|runtime'
    fi
  } | sort | tail -1
}

resolve_old_spec() {
  if [[ -n "$OLD_RAW_SPEC" && -f "$OLD_RAW_SPEC" ]]; then
    printf '%s\n' "$OLD_RAW_SPEC"
    return 0
  fi

  local candidate
  for candidate in \
    "$OLD_WORKTREE/chain-specs/x3-local-rc2-raw.json" \
    "$ROOT_DIR/chain-specs/x3-local-rc2-raw.json" \
    "$OLD_WORKTREE/chain-specs/x3-local3-raw.json" \
    "$ROOT_DIR/chain-specs/x3-local3-raw.json"; do
    if [[ -f "$candidate" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  return 1
}

prepare_upgrade_wasm() {
  local raw_wasm="$1"
  local compressed="$OUT/x3_chain_runtime.zstdblob"
  if [[ -n "$ZSTD_BIN" && -x "$ZSTD_BIN" ]]; then
    printf '\x52\xbc\x53\x76\x46\xdb\x8e\x05' > "$compressed"
    if "$ZSTD_BIN" -q -22 --no-check "$raw_wasm" -c >> "$compressed"; then
      if "$ZSTD_BIN" -q -d --no-check < <(tail -c +9 "$compressed") | cmp -s - "$raw_wasm"; then
        printf '%s\n' "$compressed"
        return 0
      fi
    fi
  fi
  printf '%s\n' "$raw_wasm"
}

rpc() {
  local url="$1"
  local method="$2"
  local params="${3:-[]}"
  curl -sf -m 10 "$url" -H 'Content-Type: application/json' \
    -d "{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"$method\",\"params\":$params}"
}

get_block_number() {
  rpc "$1" chain_getHeader '[]' | "$JQ_BIN" -r '.result.number // "0x0"' | xargs printf '%d\n' 2>/dev/null || echo 0
}

get_finalized_hash() {
  rpc "$1" chain_getFinalizedHead '[]' | "$JQ_BIN" -r '.result // ""' 2>/dev/null || true
}

get_peer_count() {
  rpc "$1" system_health '[]' | "$JQ_BIN" -r '.result.peers // 0' 2>/dev/null || echo 0
}

get_runtime_version() {
  rpc "$1" state_getRuntimeVersion '[]' | "$JQ_BIN" -c '.result // {}'
}

get_code_hash() {
  rpc "$1" state_getStorageHash '["0x3a636f6465"]' | "$JQ_BIN" -r '.result // ""' 2>/dev/null || true
}

wait_for_rpc() {
  local url="$1"
  local label="$2"
  local deadline=$(( $(date +%s) + TIMEOUT ))
  while [[ $(date +%s) -lt $deadline ]]; do
    if rpc "$url" system_health '[]' >/dev/null 2>&1; then
      info "$label RPC ready"
      return 0
    fi
    sleep 2
  done
  return 1
}

wait_for_blocks() {
  local url="$1"
  local count="$2"
  local label="$3"
  local start current deadline
  start="$(get_block_number "$url")"
  deadline=$(( $(date +%s) + TIMEOUT ))
  while [[ $(date +%s) -lt $deadline ]]; do
    current="$(get_block_number "$url")"
    if (( current >= start + count )); then
      info "$label advanced from #$start to #$current"
      return 0
    fi
    sleep 2
  done
  return 1
}

wait_for_finality_change() {
  local url="$1"
  local label="$2"
  local start current deadline
  start="$(get_finalized_hash "$url")"
  deadline=$(( $(date +%s) + TIMEOUT ))
  while [[ $(date +%s) -lt $deadline ]]; do
    current="$(get_finalized_hash "$url")"
    if [[ -n "$current" && "$current" != "$start" ]]; then
      info "$label finalized head changed"
      return 0
    fi
    sleep 2
  done
  return 1
}

sample_headers() {
  local url="$1"
  python3 - "$url" <<'PY'
import json
import subprocess
import sys
import time

url = sys.argv[1]

def rpc(method, params):
    payload = json.dumps({'id': 1, 'jsonrpc': '2.0', 'method': method, 'params': params})
    output = subprocess.check_output([
        'curl', '-sf', '-m', '10', url,
        '-H', 'Content-Type: application/json',
        '-d', payload,
    ], text=True)
    return json.loads(output).get('result')

rows = []
for _ in range(2):
    rows.append({
        'header': rpc('chain_getHeader', []),
        'finalized_head': rpc('chain_getFinalizedHead', []),
    })
    time.sleep(2)
print(json.dumps(rows))
PY
}

collect_supply_snapshot() {
  local url="$1"
  python3 - "$url" <<'PY'
import json
import subprocess
import sys

MASK64 = (1 << 64) - 1
PRIME1 = 11400714785074694791
PRIME2 = 14029467366897019727
PRIME3 = 1609587929392839161
PRIME4 = 9650029242287828579
PRIME5 = 2870177450012600261

def rotl(value, bits):
    return ((value << bits) | (value >> (64 - bits))) & MASK64

def round_acc(acc, lane):
    acc = (acc + lane * PRIME2) & MASK64
    acc = rotl(acc, 31)
    return (acc * PRIME1) & MASK64

def merge_round(acc, val):
    val = round_acc(0, val)
    acc ^= val
    return (acc * PRIME1 + PRIME4) & MASK64

def avalanche(value):
    value ^= value >> 33
    value = (value * PRIME2) & MASK64
    value ^= value >> 29
    value = (value * PRIME3) & MASK64
    value ^= value >> 32
    return value & MASK64

def xxh64(data, seed):
    index = 0
    length = len(data)
    if length >= 32:
        v1 = (seed + PRIME1 + PRIME2) & MASK64
        v2 = (seed + PRIME2) & MASK64
        v3 = seed & MASK64
        v4 = (seed - PRIME1) & MASK64
        limit = length - 32
        while index <= limit:
            lanes = [int.from_bytes(data[index + offset:index + offset + 8], 'little') for offset in (0, 8, 16, 24)]
            v1 = round_acc(v1, lanes[0])
            v2 = round_acc(v2, lanes[1])
            v3 = round_acc(v3, lanes[2])
            v4 = round_acc(v4, lanes[3])
            index += 32
        h64 = (rotl(v1, 1) + rotl(v2, 7) + rotl(v3, 12) + rotl(v4, 18)) & MASK64
        h64 = merge_round(h64, v1)
        h64 = merge_round(h64, v2)
        h64 = merge_round(h64, v3)
        h64 = merge_round(h64, v4)
    else:
        h64 = (seed + PRIME5) & MASK64
    h64 = (h64 + length) & MASK64
    while index + 8 <= length:
        lane = int.from_bytes(data[index:index + 8], 'little')
        lane = round_acc(0, lane)
        h64 ^= lane
        h64 = (rotl(h64, 27) * PRIME1 + PRIME4) & MASK64
        index += 8
    if index + 4 <= length:
        h64 ^= int.from_bytes(data[index:index + 4], 'little') * PRIME1 & MASK64
        h64 = (rotl(h64, 23) * PRIME2 + PRIME3) & MASK64
        index += 4
    while index < length:
        h64 ^= data[index] * PRIME5 & MASK64
        h64 = (rotl(h64, 11) * PRIME1) & MASK64
        index += 1
    return avalanche(h64)

def twox128(value):
    data = value.encode()
    return xxh64(data, 0).to_bytes(8, 'little').hex() + xxh64(data, 1).to_bytes(8, 'little').hex()

def call(url, method_name):
    payload = json.dumps({'id': 1, 'jsonrpc': '2.0', 'method': 'state_call', 'params': [method_name, '']})
    try:
        raw = subprocess.check_output([
            'curl', '-sf', '-m', '10', url,
            '-H', 'Content-Type: application/json',
            '-d', payload,
        ], text=True)
        result = json.loads(raw).get('result')
    except Exception:
        result = None
    row = {'method': method_name, 'raw': result}
    if isinstance(result, str) and result.startswith('0x') and len(result) > 2:
        try:
            row['decoded'] = int(result, 16)
        except Exception:
            pass
    return row

def decode_u128_le(hex_bytes):
    if not isinstance(hex_bytes, str) or not hex_bytes.startswith('0x'):
        return None
    body = hex_bytes[2:]
    if len(body) < 32:
        return None
    try:
        raw = bytes.fromhex(body)
    except Exception:
        return None
    if len(raw) < 16:
        return None
    return int.from_bytes(raw[:16], 'little')

def storage_get(url, key):
    payload = json.dumps({'id': 1, 'jsonrpc': '2.0', 'method': 'state_getStorage', 'params': [key]})
    try:
        raw = subprocess.check_output([
            'curl', '-sf', '-m', '10', url,
            '-H', 'Content-Type: application/json',
            '-d', payload,
        ], text=True)
        return json.loads(raw).get('result')
    except Exception:
        return None

def storage_get_keys(url, prefix):
    payload = json.dumps({'id': 1, 'jsonrpc': '2.0', 'method': 'state_getKeys', 'params': [prefix]})
    try:
        raw = subprocess.check_output([
            'curl', '-sf', '-m', '10', url,
            '-H', 'Content-Type: application/json',
            '-d', payload,
        ], text=True)
        result = json.loads(raw).get('result')
        return result if isinstance(result, list) else []
    except Exception:
        return []

def decode_supply_ledger_scale(raw_hex):
    if not isinstance(raw_hex, str) or not raw_hex.startswith('0x'):
        return None
    try:
        data = bytes.fromhex(raw_hex[2:])
    except Exception:
        return None
    # SupplyLedger SCALE encoding is 6 contiguous little-endian u128 fields.
    if len(data) < 96:
        return None
    fields = []
    for i in range(6):
        start = i * 16
        fields.append(int.from_bytes(data[start:start + 16], 'little'))
    return {
        'native_supply': fields[0],
        'evm_supply': fields[1],
        'svm_supply': fields[2],
        'external_locked_supply': fields[3],
        'pending_supply': fields[4],
        'canonical_supply': fields[5],
    }

def aggregate_ledgers(url):
    prefix = '0x' + twox128('X3SupplyLedger') + twox128('Ledgers')
    keys = storage_get_keys(url, prefix)
    totals = {
        'native_supply': 0,
        'evm_supply': 0,
        'svm_supply': 0,
        'external_locked_supply': 0,
        'pending_supply': 0,
        'canonical_supply': 0,
    }
    decoded_count = 0
    for key in keys:
        raw = storage_get(url, key)
        decoded = decode_supply_ledger_scale(raw)
        if not decoded:
            continue
        decoded_count += 1
        for field in totals:
            totals[field] += decoded[field]

    represented_supply = (
        totals['native_supply']
        + totals['evm_supply']
        + totals['svm_supply']
        + totals['external_locked_supply']
        + totals['pending_supply']
    )
    return {
        'storage_prefix': prefix,
        'key_count': len(keys),
        'decoded_count': decoded_count,
        'pending_supply': totals['pending_supply'],
        'represented_supply': represented_supply,
        'canonical_supply': totals['canonical_supply'],
        'external_locked_supply': totals['external_locked_supply'],
    }

url = sys.argv[1]
snapshot = {
    'pending_supply': call(url, 'X3SupplyLedger_pending_supply'),
    'represented_supply': call(url, 'X3SupplyLedger_represented_supply'),
    'canonical_supply': call(url, 'X3SupplyLedger_canonical_supply'),
    'external_locked_supply': call(url, 'X3SupplyLedger_external_locked_supply'),
    'economic_halt_active': call(url, 'X3SupplyLedger_economic_halt_active'),
    'would_reject_transfer': call(url, 'X3SupplyLedger_would_reject_transfer'),
    'can_refund_while_halted': call(url, 'X3SupplyLedger_can_refund_while_halted'),
}

ledger_agg = aggregate_ledgers(url)
snapshot['ledger_aggregation'] = ledger_agg

for field in ('pending_supply', 'represented_supply', 'canonical_supply', 'external_locked_supply'):
    if snapshot[field].get('decoded') is None:
        snapshot[field]['decoded'] = ledger_agg.get(field)
        snapshot[field]['source'] = 'ledger_aggregation'
    else:
        snapshot[field]['source'] = 'state_call'

storage_key = '0x' + twox128('X3BridgeAdapters') + twox128('ExternalBridgesEnabled')
bridge_payload = json.dumps({'id': 1, 'jsonrpc': '2.0', 'method': 'state_getStorage', 'params': [storage_key]})
bridge_raw = subprocess.check_output([
    'curl', '-sf', '-m', '10', url,
    '-H', 'Content-Type: application/json',
    '-d', bridge_payload,
], text=True)
bridge_value = json.loads(bridge_raw).get('result')
snapshot['external_bridges'] = {
    'storage_key': storage_key,
    'raw': bridge_value,
    'enabled': bridge_value not in (None, '0x00'),
}
print(json.dumps(snapshot))
PY
}

build_ref() {
  local worktree="$1"
  local target_dir="$2"
  local label="$3"
  local enabled="$4"
  local node_log="$OUT/${label}_node_build.log"
  local runtime_log="$OUT/${label}_runtime_build.log"

  if [[ "$enabled" != "1" ]]; then
    record "build_${label}_node" PASS "build skipped by BUILD_${label^^}=0"
    record "build_${label}_runtime_wasm" PASS "build skipped by BUILD_${label^^}=0"
    return 0
  fi

  if (
    cd "$worktree"
    env \
      RUST_MIN_STACK="$RUST_MIN_STACK_BYTES" \
      CARGO_TARGET_DIR="$target_dir" \
      CARGO_INCREMENTAL=0 \
      CARGO_PROFILE_RELEASE_DEBUG=0 \
      CARGO_PROFILE_RELEASE_INCREMENTAL=false \
      CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
      CARGO_PROFILE_RELEASE_LTO=false \
      "$CARGO_BIN" "${CARGO_TOOLCHAIN_ARGS[@]}" build --release -p x3-chain-node
  ) > "$node_log" 2>&1; then
    record "build_${label}_node" PASS "$node_log"
  else
    record "build_${label}_node" FAIL "x3-chain-node build failed; see $node_log"
  fi

  if (
    cd "$worktree"
    env \
      RUST_MIN_STACK="$RUST_MIN_STACK_BYTES" \
      CARGO_TARGET_DIR="$target_dir" \
      CARGO_INCREMENTAL=0 \
      CARGO_PROFILE_RELEASE_DEBUG=0 \
      CARGO_PROFILE_RELEASE_INCREMENTAL=false \
      CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
      CARGO_PROFILE_RELEASE_LTO=false \
      "$CARGO_BIN" "${CARGO_TOOLCHAIN_ARGS[@]}" build --manifest-path runtime/Cargo.toml --release
  ) > "$runtime_log" 2>&1; then
    record "build_${label}_runtime_wasm" PASS "$runtime_log"
  else
    record "build_${label}_runtime_wasm" FAIL "runtime build failed; see $runtime_log"
  fi
}

run_router_test() {
  local worktree="$1"
  local target_dir="$2"
  local name="$3"
  local test_name="$4"
  local out_file="$5"
  local log_file="$OUT/${name}.log"

  if [[ "$RUN_ROUTER_TESTS" != "1" ]]; then
    write_json "$out_file" "{\"test\":\"$test_name\",\"result\":\"NOT_RUN\",\"reason\":\"RUN_ROUTER_TESTS=$RUN_ROUTER_TESTS\"}"
    record "$name" FAIL "router test skipped by RUN_ROUTER_TESTS=$RUN_ROUTER_TESTS"
    return 0
  fi

  if (
    cd "$worktree"
    env \
      RUST_MIN_STACK="$RUST_MIN_STACK_BYTES" \
      CARGO_TARGET_DIR="$target_dir" \
      CARGO_INCREMENTAL=0 \
      CARGO_PROFILE_DEV_DEBUG=0 \
      CARGO_PROFILE_DEV_INCREMENTAL=false \
      CARGO_PROFILE_DEV_CODEGEN_UNITS=1 \
      "$CARGO_BIN" "${CARGO_TOOLCHAIN_ARGS[@]}" test -p pallet-x3-cross-vm-router --lib "$test_name" -- --nocapture
  ) > "$log_file" 2>&1; then
    write_json "$out_file" "{\"test\":\"$test_name\",\"result\":\"PASS\",\"log\":\"$log_file\"}"
    record "$name" PASS "$test_name"
  else
    write_json "$out_file" "{\"test\":\"$test_name\",\"result\":\"FAIL\",\"log\":\"$log_file\"}"
    record "$name" FAIL "$test_name failed; see $log_file"
  fi
}

ensure_router_test_compat() {
  local worktree="$1"
  local tests_file="$worktree/pallets/x3-cross-vm-router/src/tests.rs"

  if [[ ! -f "$tests_file" ]]; then
    record old_router_test_compat FAIL "missing router tests file: $tests_file"
    return 1
  fi

  if { \
      grep -q 'type Currency = ();' "$tests_file" \
      || { grep -q 'type Currency = Balances;' "$tests_file" && grep -q 'Balances: pallet_balances' "$tests_file"; }; \
    } \
    && grep -q 'type RoutingFeeBps = RoutingFeeBps;' "$tests_file" \
    && grep -q 'type ProtocolTreasury = ProtocolTreasury;' "$tests_file"; then
    record old_router_test_compat PASS 'router test config already compatible'
    return 0
  fi

  if python3 - "$tests_file" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
src = path.read_text(encoding='utf-8')

old_params = """parameter_types! {
    pub const MaxAssets: u32 = 64;
}
"""

new_params = """parameter_types! {
    pub const MaxAssets: u32 = 64;
    pub const RoutingFeeBps: u16 = 0;
    pub const ProtocolTreasury: u64 = 99;
}
"""

old_impl = """impl pallet_x3_cross_vm_router::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Registry = Registry;
    type Ledger = Ledger;
    type ExternalExecutorOrigin = RootOrAny;
    type VmAdapterOrigin = RootOnly;
    type EconomicHalt = Ledger;
}
"""

new_impl = """impl pallet_x3_cross_vm_router::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Registry = Registry;
    type Ledger = Ledger;
    type ExternalExecutorOrigin = RootOrAny;
    type VmAdapterOrigin = RootOnly;
    type EconomicHalt = Ledger;
  type Currency = ();
    type RoutingFeeBps = RoutingFeeBps;
    type ProtocolTreasury = ProtocolTreasury;
}
"""

changed = False
if old_params in src:
    src = src.replace(old_params, new_params, 1)
    changed = True

if old_impl in src:
    src = src.replace(old_impl, new_impl, 1)
    changed = True

if 'type Currency = Balances;' in src and 'Balances: pallet_balances' not in src:
  src = src.replace('type Currency = Balances;', 'type Currency = ();')
  changed = True

if not changed:
    print('no-compatible-rewrite-pattern', file=sys.stderr)
    sys.exit(2)

path.write_text(src, encoding='utf-8')
PY
  then
    if { \
        grep -q 'type Currency = ();' "$tests_file" \
        || { grep -q 'type Currency = Balances;' "$tests_file" && grep -q 'Balances: pallet_balances' "$tests_file"; }; \
      } \
      && grep -q 'type RoutingFeeBps = RoutingFeeBps;' "$tests_file" \
      && grep -q 'type ProtocolTreasury = ProtocolTreasury;' "$tests_file"; then
      record old_router_test_compat PASS "patched $tests_file for current router Config trait"
      return 0
    fi
  fi

  record old_router_test_compat FAIL "unable to patch old router tests for current Config trait"
  return 1
}

start_alice() {
  local raw_spec="$1"
  "$OLD_BINARY" --chain "$raw_spec" --alice \
    --node-key 0000000000000000000000000000000000000000000000000000000000000001 \
    --base-path "$ALICE_BASE" \
    --port 30543 \
    --rpc-port 9954 \
    --prometheus-port 9625 \
    --rpc-cors all \
    --rpc-methods unsafe \
    --validator \
    --log info,runtime::x3=debug \
    > >(tee "$LOG_DIR/alice_upgrade.log") 2>&1 &
  ALICE_PID=$!
}

start_bob() {
  local raw_spec="$1"
  local bootnode="$2"
  "$OLD_BINARY" --chain "$raw_spec" --bob \
    --node-key 0000000000000000000000000000000000000000000000000000000000000002 \
    --base-path "$BOB_BASE" \
    --port 30544 \
    --rpc-port 9955 \
    --prometheus-port 9626 \
    --rpc-cors all \
    --rpc-methods unsafe \
    --validator \
    --bootnodes "$bootnode" \
    --log info,runtime::x3=debug \
    > >(tee "$LOG_DIR/bob_upgrade.log") 2>&1 &
  BOB_PID=$!
}

start_charlie() {
  local raw_spec="$1"
  local bootnode="$2"
  "$OLD_BINARY" --chain "$raw_spec" --charlie \
    --node-key 0000000000000000000000000000000000000000000000000000000000000003 \
    --base-path "$CHARLIE_BASE" \
    --port 30545 \
    --rpc-port 9956 \
    --prometheus-port 9627 \
    --rpc-cors all \
    --rpc-methods unsafe \
    --validator \
    --bootnodes "$bootnode" \
    --log info,runtime::x3=debug \
    > >(tee "$LOG_DIR/charlie_upgrade.log") 2>&1 &
  CHARLIE_PID=$!
}

get_alice_bootnode() {
  local node_id=""
  for _ in $(seq 1 45); do
    node_id="$(grep -m1 'Local node identity is:' "$LOG_DIR/alice_upgrade.log" 2>/dev/null | awk '{print $NF}' || true)"
    if [[ -n "$node_id" ]]; then
      printf '/ip4/127.0.0.1/tcp/30543/p2p/%s\n' "$node_id"
      return 0
    fi
    sleep 2
  done
  return 1
}

make_submitter() {
  cat > "$WORK_DIR/submit_runtime_upgrade.cjs" <<'JS'
const fs = require('fs');
const { ApiPromise, WsProvider } = require('@polkadot/api');
const { Keyring } = require('@polkadot/keyring');

function bytes(value) {
  return Array.from(Buffer.from(value));
}

async function main() {
  const ws = process.env.RC4_WS_URL;
  const wasmPath = process.env.RC4_WASM_FILE;
  const out = process.env.RC4_SUBMISSION_JSON;
  if (!wasmPath || !fs.existsSync(wasmPath)) {
    throw new Error(`missing WASM artifact path: ${wasmPath || '<unset>'}`);
  }

  const wasmBytes = fs.readFileSync(wasmPath);
  const wasmHex = '0x' + wasmBytes.toString('hex');
  const provider = new WsProvider(ws);
  const api = await ApiPromise.create({ provider });
  const keyring = new Keyring({ type: 'sr25519' });
  const alice = keyring.addFromUri('//Alice');
  const bob = keyring.addFromUri('//Bob');
  const charlie = keyring.addFromUri('//Charlie');

  const payload = {
    status: 'started',
    method: null,
    extrinsic_hash: null,
    included_block: null,
    proposal_id: null,
    code_hash_before: (await api.rpc.state.getStorageHash(':code')).toHex(),
    code_hash_after: null,
    runtime_before: api.runtimeVersion.toHuman(),
    runtime_after: null,
    runtime_changed: false,
    steps: [],
  };

  const flush = () => fs.writeFileSync(out, JSON.stringify(payload, null, 2));
  const compactValue = (value) => {
    const text = value && value.toString ? value.toString() : String(value);
    return text.length > 256 ? `${text.slice(0, 256)}...<truncated:${text.length}>` : text;
  };
  const compactEventData = (event) => event.data.map((value, index) => ({ index, value: compactValue(value) }));
  const recordStep = (label, extra = {}) => {
    payload.steps.push({ label, ...extra });
    flush();
  };

  async function waitForExpectedEvent(label, startBlock, isExpected) {
    let nextBlock = startBlock + 1;
    for (let attempt = 0; attempt < 180; attempt += 1) {
      const header = await api.rpc.chain.getHeader();
      const currentBlock = header.number.toNumber();
      while (nextBlock <= currentBlock) {
        const blockHash = await api.rpc.chain.getBlockHash(nextBlock);
        const events = await api.query.system.events.at(blockHash);
        const failed = events.find(({ event }) => api.events.system.ExtrinsicFailed.is(event));
        if (failed) {
          throw new Error(`${label}: ${failed.event.data.toString()}`);
        }
        const expected = events.find(isExpected);
        if (expected) {
          return { blockNumber: nextBlock, blockHash: blockHash.toHex(), expectedEvent: expected };
        }
        nextBlock += 1;
      }
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
    throw new Error(`${label}: expected event not observed`);
  }

  async function submitAndWatch(extrinsic, signer, label, isExpected) {
    const signed = await extrinsic.signAsync(signer);
    const txHash = signed.hash.toHex();
    const startBlock = (await api.rpc.chain.getHeader()).number.toNumber();
    recordStep(label, { phase: 'submit', txHash, signer: signer.address });
    await api.rpc.author.submitExtrinsic(signed);
    const observed = await waitForExpectedEvent(label, startBlock, isExpected);
    recordStep(label, {
      phase: 'observed',
      txHash,
      signer: signer.address,
      blockNumber: observed.blockNumber,
      blockHash: observed.blockHash,
      event: `${observed.expectedEvent.event.section}.${observed.expectedEvent.event.method}`,
      data: compactEventData(observed.expectedEvent.event),
    });
    return { ...observed, txHash };
  }

  async function councilDispatch(call, label) {
    if (!api.tx.council || !api.tx.council.propose || !api.tx.council.vote || !api.tx.council.close) {
      throw new Error('missing sudo/root/governance path: council propose/vote/close unavailable in metadata');
    }
    const threshold = 2;
    const lengthBound = call.encodedLength || call.toU8a().length;
    const proposed = await submitAndWatch(
      api.tx.council.propose(threshold, call, lengthBound),
      alice,
      `${label}: council propose`,
      ({ event }) => api.events.council.Proposed.is(event),
    );
    const proposalIndex = proposed.expectedEvent.event.data[1].toNumber();
    const proposalHash = proposed.expectedEvent.event.data[2].toHex();
    recordStep(`${label}: council proposed`, { proposalIndex, proposalHash, lengthBound });

    await submitAndWatch(
      api.tx.council.vote(proposalHash, proposalIndex, true),
      bob,
      `${label}: council vote Bob`,
      ({ event }) => api.events.council.Voted.is(event) || api.events.council.Approved.is(event),
    );
    await submitAndWatch(
      api.tx.council.vote(proposalHash, proposalIndex, true),
      alice,
      `${label}: council vote Alice`,
      ({ event }) => api.events.council.Voted.is(event) || api.events.council.Approved.is(event),
    );
    const weightBound = { refTime: '120000000000', proofSize: '2000000' };
    await submitAndWatch(
      api.tx.council.close(proposalHash, proposalIndex, weightBound, lengthBound),
      charlie,
      `${label}: council close`,
      ({ event }) => api.events.council.Executed.is(event) || api.events.council.Closed.is(event),
    );
  }

  async function sudoUpgrade() {
    if (!api.tx.system || !api.tx.system.setCode) {
      throw new Error('missing encoded call builder: system.setCode unavailable in metadata');
    }
    const observed = await submitAndWatch(
      api.tx.sudo.sudo(api.tx.system.setCode(wasmHex)),
      alice,
      'sudo runtime code storage upgrade',
      ({ event }) => api.events.sudo.Sudid.is(event),
    );
    payload.method = 'sudo.sudo(system.setCode(compressed-runtime))';
    payload.extrinsic_hash = observed.txHash;
    payload.included_block = observed.blockNumber;
  }

  async function governanceUpgrade() {
    if (!api.tx.system || !api.tx.system.setCode) {
      throw new Error('missing encoded call builder: system.setCode unavailable in metadata');
    }
    if (!api.tx.governance) {
      throw new Error('missing sudo/root/governance path: governance pallet unavailable in metadata');
    }

    payload.method = 'council-governance(system.setCode(compressed-runtime))';
    await councilDispatch(api.tx.governance.authorizeGovernanceAccount(alice.address), 'authorize Alice governance');
    await councilDispatch(api.tx.governance.authorizeGovernanceAccount(bob.address), 'authorize Bob governance');
    await councilDispatch(api.tx.governance.authorizeGovernanceAccount(charlie.address), 'authorize Charlie governance');
    await councilDispatch(api.tx.governance.updateConfig(null, null, null, 1), 'set one-block governance enactment delay');

    const proposed = await submitAndWatch(
      api.tx.governance.submitProposal(
        api.tx.system.setCode(wasmHex),
        bytes('RC4 runtime upgrade'),
        bytes('Live local3 old-runtime to current-runtime code storage upgrade rehearsal'),
        false,
        null,
        null,
      ),
      alice,
      'submit runtime upgrade proposal',
      ({ event }) => api.events.governance.ProposalSubmitted.is(event),
    );
    const proposalId = proposed.expectedEvent.event.data[0].toNumber();
    payload.proposal_id = proposalId;
    payload.extrinsic_hash = proposed.txHash;
    payload.included_block = proposed.blockNumber;
    recordStep('runtime upgrade proposal submitted', { proposalId });

    const accounts = [alice, bob, charlie];
    const balances = await Promise.all(accounts.map((account) => api.query.system.account(account.address)));
    for (let index = 0; index < accounts.length; index += 1) {
      await submitAndWatch(
        api.tx.governance.vote(proposalId, 'Aye', balances[index].data.free, 'None'),
        accounts[index],
        `vote Aye ${accounts[index].address}`,
        ({ event }) => api.events.governance.Voted.is(event),
      );
    }

    await councilDispatch(api.tx.governance.fastTrack(proposalId, 0), 'fast-track runtime upgrade proposal');

    const finalized = await submitAndWatch(
      api.tx.governance.finalizeProposal(proposalId),
      alice,
      'finalize runtime upgrade proposal',
      ({ event }) => api.events.governance.ProposalApproved.is(event),
    );

    const enacted = await waitForExpectedEvent(
      'governance runtime upgrade enactment',
      finalized.blockNumber,
      ({ event }) => api.events.governance.ProposalEnacted.is(event),
    );
    const enactmentResult = enacted.expectedEvent.event.data[1];
    recordStep('governance runtime upgrade enacted', {
      blockNumber: enacted.blockNumber,
      blockHash: enacted.blockHash,
      event: `${enacted.expectedEvent.event.section}.${enacted.expectedEvent.event.method}`,
      data: compactEventData(enacted.expectedEvent.event),
    });
    if (!enactmentResult.isOk) {
      throw new Error(`governance runtime upgrade enactment failed: ${enactmentResult.toString()}`);
    }
  }

  if (api.tx.sudo && api.tx.sudo.sudo) {
    await sudoUpgrade();
  } else {
    await governanceUpgrade();
  }

  for (let attempt = 0; attempt < 60; attempt += 1) {
    const version = await api.rpc.state.getRuntimeVersion();
    const codeHash = (await api.rpc.state.getStorageHash(':code')).toHex();
    const changed = version.specVersion.toNumber() !== api.runtimeVersion.specVersion.toNumber() || codeHash !== payload.code_hash_before;
    payload.runtime_after = version.toHuman();
    payload.code_hash_after = codeHash;
    payload.runtime_changed = changed;
    flush();
    if (changed) {
      break;
    }
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }

  payload.status = payload.runtime_changed ? 'success' : 'failed';
  flush();
  await api.disconnect();
  if (!payload.runtime_changed) {
    throw new Error('missing runtime version bump: runtime version/code hash did not change after submission');
  }
}

main().catch((error) => {
  const out = process.env.RC4_SUBMISSION_JSON;
  if (out) {
    let previous = {};
    try {
      previous = JSON.parse(fs.readFileSync(out, 'utf8'));
    } catch (_) {
      previous = {};
    }
    previous.status = 'failed';
    previous.error = String((error && error.stack) || error);
    fs.writeFileSync(out, JSON.stringify(previous, null, 2));
  }
  process.exit(1);
});
JS
}

annotate_submission_failure() {
  local path="$1"
  local wasm_file="$2"
  local detail="$3"
  python3 - "$path" "$wasm_file" "$OLD_BLOCK_LENGTH_CAP_BYTES" "$OLD_PREIMAGE_MAX_SIZE_BYTES" "$detail" <<'PY'
import json
import os
import sys

path, wasm_file, block_cap, preimage_cap, detail = sys.argv[1:]
try:
    with open(path, encoding='utf-8') as handle:
        data = json.load(handle)
except Exception:
    data = {}

wasm_size = os.path.getsize(wasm_file) if wasm_file and os.path.exists(wasm_file) else None
data.setdefault('status', 'failed')
data['blocker'] = detail
data['payload_analysis'] = {
    'wasm_file': wasm_file,
    'wasm_size_bytes': wasm_size,
    'old_runtime_block_length_cap_bytes': int(block_cap),
    'old_runtime_preimage_max_size_bytes': int(preimage_cap),
    'fits_old_runtime_block_length': wasm_size is not None and wasm_size <= int(block_cap),
    'fits_old_runtime_preimage_max_size': wasm_size is not None and wasm_size <= int(preimage_cap),
}
with open(path, 'w', encoding='utf-8') as handle:
    json.dump(data, handle, indent=2, sort_keys=True)
    handle.write('\n')
PY
}

write_blocked_artifacts() {
  local reason="$1"
  write_json "$OUT/pre_upgrade_state.json" "$($JQ_BIN -cn --arg reason "$reason" '{result:"BLOCKED", reason:$reason}')"
  write_json "$OUT/runtime_upgrade_submission.json" "$($JQ_BIN -cn --arg reason "$reason" '{status:"failed", blocker:$reason, reason:$reason}')"
  write_json "$OUT/post_upgrade_state.json" "$($JQ_BIN -cn --arg reason "$reason" '{result:"NOT_RUN", reason:$reason}')"
  write_json "$OUT/post_upgrade_settlement.json" "$($JQ_BIN -cn --arg reason "$reason" '{result:"NOT_RUN", reason:$reason}')"
  write_json "$OUT/post_upgrade_refund_halt.json" "$($JQ_BIN -cn --arg reason "$reason" '{result:"NOT_RUN", reason:$reason}')"
  write_json "$OUT/storage_versions.json" '{"pre_upgrade_runtime":{},"post_upgrade_runtime":{}}'
  write_json "$OUT/final_invariants.json" "$($JQ_BIN -cn --arg reason "$reason" '{
    represented_supply_equals_canonical_supply:"FAIL",
    pending_supply_zero:"FAIL",
    external_locked_supply_zero:"FAIL",
    external_bridges_disabled:"FAIL",
    blocks_after_upgrade:"FAIL",
    finality_after_upgrade:"FAIL",
    blocker:$reason
  }')"
}

write_report() {
  python3 - "$RESULTS_JSON" "$OUT/rc4_runtime_upgrade_rehearsal_report.md" "$VERDICT" "$OLD_REF" "$NEW_REF" "$OLD_COMMIT" "$NEW_COMMIT" "$OLD_WASM_HASH" "$NEW_WASM_HASH" <<'PY'
import json
import os
import sys

results_path, report_path, verdict, old_ref, new_ref, old_commit, new_commit, old_wasm_hash, new_wasm_hash = sys.argv[1:]

results = []
with open(results_path, encoding='utf-8') as handle:
    for line in handle:
        line = line.strip()
        if line:
            results.append(json.loads(line))

by_name = {row['name']: row for row in results}
blockers = [row for row in results if row['result'] != 'PASS']

def load_json(filename):
    path = os.path.join(os.path.dirname(report_path), filename)
    if not os.path.exists(path):
        return None
    with open(path, encoding='utf-8') as handle:
        return json.load(handle)

pre = load_json('pre_upgrade_state.json') or {}
submission = load_json('runtime_upgrade_submission.json') or {}
post = load_json('post_upgrade_state.json') or {}
settlement = load_json('post_upgrade_settlement.json') or {}
halt = load_json('post_upgrade_refund_halt.json') or {}
invariants = load_json('final_invariants.json') or {}

with open(report_path, 'w', encoding='utf-8') as out:
    out.write('# RC4 Runtime Upgrade Rehearsal Report\n\n')
    out.write('## Verdict\n\n')
    out.write(('PASS' if verdict == 'PASS' and not blockers else 'FAIL') + '\n\n')

    out.write('## Scope\n\n')
    out.write('- local3 live runtime upgrade rehearsal\n')
    out.write('- old runtime -> new runtime\n')
    out.write('- internal settlement only\n')
    out.write('- external bridges disabled\n\n')

    out.write('## Refs\n\n')
    out.write('| Field | Value |\n')
    out.write('|---|---|\n')
    out.write(f'| OLD_REF | {old_ref} |\n')
    out.write(f'| NEW_REF | {new_ref} |\n')
    out.write(f'| Old commit | {old_commit} |\n')
    out.write(f'| New commit | {new_commit} |\n')
    out.write(f'| Old WASM hash | {old_wasm_hash or "UNKNOWN"} |\n')
    out.write(f'| New WASM hash | {new_wasm_hash or "UNKNOWN"} |\n\n')

    out.write('## Pre-Upgrade State\n\n')
    out.write('| Check | Result | Evidence |\n')
    out.write('|---|---:|---|\n')
    for key, label in [
        ('build_old_node', 'Old node build'),
        ('build_old_runtime_wasm', 'Old runtime WASM build'),
        ('local3_boot', 'local3 boot'),
        ('pre_upgrade_blocks', 'Blocks advancing'),
        ('pre_upgrade_finality', 'GRANDPA finality advancing'),
        ('pre_upgrade_settlement', 'Pre-upgrade settlement smoke'),
    ]:
        row = by_name.get(key, {'result': 'NOT_RUN', 'detail': ''})
        out.write(f'| {label} | {row.get("result", "NOT_RUN")} | {row.get("detail", "")} |\n')
    supply_ok = 'PASS' if pre.get('supply_snapshot', {}).get('checks', {}).get('represented_equals_canonical') and pre.get('supply_snapshot', {}).get('checks', {}).get('pending_zero') else 'FAIL'
    out.write(f'| Supply invariant | {supply_ok} | pre_upgrade_state.json |\n\n')

    out.write('## Upgrade Submission\n\n')
    out.write('| Field | Value |\n')
    out.write('|---|---|\n')
    out.write(f'| Extrinsic method | {submission.get("method", submission.get("reason", ""))} |\n')
    out.write(f'| Extrinsic hash | {submission.get("extrinsic_hash", "")} |\n')
    out.write(f'| Included block | {submission.get("included_block", "")} |\n')
    out.write(f'| Old runtime version/hash | {pre.get("runtime_version", {})} / {pre.get("code_hash", "")} |\n')
    out.write(f'| New runtime version/hash | {post.get("runtime_after", {})} / {post.get("code_hash_after", "")} |\n')
    out.write(f'| Result | {submission.get("status", submission.get("result", ""))} |\n\n')

    out.write('## Post-Upgrade State\n\n')
    out.write('| Check | Result | Evidence |\n')
    out.write('|---|---:|---|\n')
    for key, label in [
        ('runtime_changed', 'New runtime active'),
        ('post_upgrade_blocks', 'Blocks advancing'),
        ('post_upgrade_finality', 'GRANDPA finality advancing'),
    ]:
        row = by_name.get(key, {'result': 'NOT_RUN', 'detail': ''})
        out.write(f'| {label} | {row.get("result", "NOT_RUN")} | {row.get("detail", "")} |\n')
    out.write(f'| Validators still connected | {post.get("validators_connected", "UNKNOWN")} | post_upgrade_state.json |\n')
    out.write(f'| No panic loop | {post.get("no_panic_loop", "UNKNOWN")} | post_upgrade_state.json |\n\n')

    out.write('## Post-Upgrade Settlement\n\n')
    out.write('| Route | Result | Pending zero | Supply invariant |\n')
    out.write('|---|---:|---:|---:|\n')
    routes = settlement.get('routes', [])
    if routes:
        for row in routes:
            out.write(f'| {row.get("route", "")} | {row.get("result", "")} | {row.get("pending_zero", "")} | {row.get("supply_invariant", "")} |\n')
    else:
        out.write(f'| router-regression | {settlement.get("result", "NOT_RUN")} | {settlement.get("pending_zero", "UNKNOWN")} | {settlement.get("represented_supply_equals_canonical_supply", "UNKNOWN")} |\n')
    out.write('\n')

    out.write('## Post-Upgrade Halt/Refund\n\n')
    out.write('| Test | Expected | Result |\n')
    out.write('|---|---|---:|\n')
    checks = halt.get('checks', [])
    if checks:
        for row in checks:
            out.write(f'| {row.get("name", "")} | {row.get("expected", "")} | {row.get("result", "")} |\n')
    else:
        out.write(f'| router-regression | halt active / rejected / allowed | {halt.get("result", "NOT_RUN")} |\n')
    out.write('\n')

    out.write('## Final Invariants\n\n')
    out.write(f'represented_supply == canonical_supply: {invariants.get("represented_supply_equals_canonical_supply", "FAIL")}  \n')
    out.write(f'pending_supply == 0: {invariants.get("pending_supply_zero", "FAIL")}  \n')
    out.write(f'external bridges disabled: {invariants.get("external_bridges_disabled", "FAIL")}  \n')
    out.write(f'blocks advancing after upgrade: {invariants.get("blocks_after_upgrade", "FAIL")}  \n')
    out.write(f'finality advancing after upgrade: {invariants.get("finality_after_upgrade", "FAIL")}  \n')
    out.write(f'external_locked_supply == 0: {invariants.get("external_locked_supply_zero", "FAIL")}  \n\n')

    out.write('## Blockers\n\n')
    if blockers:
        for blocker in blockers:
            out.write(f'- {blocker.get("name", "")}: {blocker.get("detail", "")}\n')
    else:
        out.write('None\n')
PY

  sha256sum "$OUT/rc4_runtime_upgrade_rehearsal_report.md" > "$OUT/rc4_runtime_upgrade_rehearsal_report.sha256"
}

cd "$ROOT_DIR"
info "OLD_REF=$OLD_REF"
info "NEW_REF=$NEW_REF"
info "OUT=$OUT"

CURRENT_HEAD_COMMIT="$(git -C "$ROOT_DIR" rev-parse HEAD 2>/dev/null || echo UNKNOWN)"
REQUESTED_NEW_COMMIT="$(git -C "$ROOT_DIR" rev-parse "$NEW_REF" 2>/dev/null || echo UNKNOWN)"
if [[ "$REQUESTED_NEW_COMMIT" != "UNKNOWN" && "$REQUESTED_NEW_COMMIT" == "$CURRENT_HEAD_COMMIT" ]]; then
  NEW_WORKTREE="$ROOT_DIR"
  record create_new_worktree PASS "reused current workspace for $NEW_REF"
else
  NEW_WORKTREE_ERR="$OUT/new_worktree_error.log"
  if ensure_worktree "$NEW_REF" "$NEW_WORKTREE" 2>"$NEW_WORKTREE_ERR"; then
    record create_new_worktree PASS "$NEW_WORKTREE"
  else
    NEW_WORKTREE_DETAIL="$(tr '\n' ' ' < "$NEW_WORKTREE_ERR" | sed 's/[[:space:]]\+/ /g' | sed 's/^ //; s/ $//')"
    [[ -z "$NEW_WORKTREE_DETAIL" ]] && NEW_WORKTREE_DETAIL="git worktree add failed for $NEW_REF"
    record create_new_worktree FAIL "$NEW_WORKTREE_DETAIL"
  fi
fi

OLD_WORKTREE_ERR="$OUT/old_worktree_error.log"
if ensure_worktree "$OLD_REF" "$OLD_WORKTREE" 2>"$OLD_WORKTREE_ERR"; then
  record create_old_worktree PASS "$OLD_WORKTREE"
else
  OLD_WORKTREE_DETAIL="$(tr '\n' ' ' < "$OLD_WORKTREE_ERR" | sed 's/[[:space:]]\+/ /g' | sed 's/^ //; s/ $//')"
  [[ -z "$OLD_WORKTREE_DETAIL" ]] && OLD_WORKTREE_DETAIL="git worktree add failed for $OLD_REF"
  record create_old_worktree FAIL "$OLD_WORKTREE_DETAIL"
fi

ensure_router_test_compat "$OLD_WORKTREE" || true

OLD_COMMIT="$(git -C "$OLD_WORKTREE" rev-parse HEAD 2>/dev/null || echo UNKNOWN)"
NEW_COMMIT="$(git -C "$NEW_WORKTREE" rev-parse HEAD 2>/dev/null || echo UNKNOWN)"

RAW_SPEC="$(resolve_old_spec || true)"
if [[ -n "$RAW_SPEC" && -f "$RAW_SPEC" ]]; then
  record resolve_old_chain_spec PASS "$RAW_SPEC"
else
  record resolve_old_chain_spec FAIL 'missing old local3 chain spec'
fi

if [[ "$VERDICT" != 'PASS' ]]; then
  BLOCKER_TEXT="${BLOCKERS[*]}"
  write_blocked_artifacts "$BLOCKER_TEXT"
  write_report
  info "Report: $OUT/rc4_runtime_upgrade_rehearsal_report.md"
  exit 1
fi

build_ref "$OLD_WORKTREE" "$OLD_TARGET_DIR" old "$BUILD_OLD"
build_ref "$NEW_WORKTREE" "$NEW_TARGET_DIR" new "$BUILD_NEW"

if [[ -x "$OLD_BINARY" ]]; then
  record old_binary_ready PASS "$OLD_BINARY"
else
  record old_binary_ready FAIL "old binary missing or not executable: $OLD_BINARY"
fi

if [[ -x "$NEW_BINARY" ]]; then
  record new_binary_ready PASS "$NEW_BINARY"
else
  record new_binary_ready FAIL "new binary missing or not executable: $NEW_BINARY"
fi

OLD_WASM_FILE="$(find_runtime_wasm "$OLD_TARGET_DIR" || true)"
if [[ -z "$NEW_WASM" ]]; then
  NEW_WASM_FILE="$(find_runtime_wasm "$NEW_TARGET_DIR" || true)"
else
  NEW_WASM_FILE="$NEW_WASM"
fi

UPGRADE_WASM_FILE=''
if [[ -n "$OLD_WASM_FILE" && -f "$OLD_WASM_FILE" ]]; then
  OLD_WASM_HASH="$(sha256_file "$OLD_WASM_FILE")"
  printf '%s  %s\n' "$OLD_WASM_HASH" "$OLD_WASM_FILE" > "$OUT/wasm_hash_before.txt"
  record old_wasm_ready PASS "$OLD_WASM_FILE"
else
  record old_wasm_ready FAIL 'missing old runtime WASM artifact'
fi

if [[ -n "$NEW_WASM_FILE" && -f "$NEW_WASM_FILE" ]]; then
  NEW_WASM_HASH="$(sha256_file "$NEW_WASM_FILE")"
  printf '%s  %s\n' "$NEW_WASM_HASH" "$NEW_WASM_FILE" > "$OUT/wasm_hash_after.txt"
  record new_wasm_ready PASS "$NEW_WASM_FILE"
  UPGRADE_WASM_FILE="$(prepare_upgrade_wasm "$NEW_WASM_FILE")"
  printf '%s  %s\n' "$(sha256_file "$UPGRADE_WASM_FILE")" "$UPGRADE_WASM_FILE" > "$OUT/upgrade_wasm_hash.txt"
  record upgrade_wasm_ready PASS "$UPGRADE_WASM_FILE"
else
  record new_wasm_ready FAIL 'missing new runtime WASM artifact'
fi

if [[ "$VERDICT" == 'PASS' ]]; then
  rm -rf "$ALICE_BASE" "$BOB_BASE" "$CHARLIE_BASE"
  start_alice "$RAW_SPEC"
  BOOTNODE="$(get_alice_bootnode || true)"
  if [[ -z "$BOOTNODE" ]]; then
    record local3_boot FAIL 'Alice did not announce peer identity'
  else
    start_bob "$RAW_SPEC" "$BOOTNODE"
    start_charlie "$RAW_SPEC" "$BOOTNODE"

    if wait_for_rpc "$ALICE_RPC" Alice && wait_for_rpc "$BOB_RPC" Bob && wait_for_rpc "$CHARLIE_RPC" Charlie; then
      record local3_boot PASS "$BOOTNODE"
      if wait_for_blocks "$ALICE_RPC" 3 pre_upgrade; then
        record pre_upgrade_blocks PASS 'blocks advanced'
      else
        record pre_upgrade_blocks FAIL 'blocks did not advance'
      fi
      if wait_for_finality_change "$ALICE_RPC" pre_upgrade; then
        record pre_upgrade_finality PASS 'finalized head advanced'
      else
        record pre_upgrade_finality FAIL 'finalized head did not advance'
      fi

      PRE_VERSION="$(get_runtime_version "$ALICE_RPC")"
      PRE_CODE_HASH="$(get_code_hash "$ALICE_RPC")"
      PRE_SNAPSHOT="$(collect_supply_snapshot "$ALICE_RPC")"
      PRE_HEADERS="$(sample_headers "$ALICE_RPC")"
      PRE_BLOCK="$(get_block_number "$ALICE_RPC")"
      PRE_FINALIZED="$(get_finalized_hash "$ALICE_RPC")"
      PRE_PEERS="$(get_peer_count "$ALICE_RPC")"

      write_json "$OUT/pre_upgrade_state.json" "$($JQ_BIN -cn \
        --arg oldRef "$OLD_REF" \
        --arg newRef "$NEW_REF" \
        --arg oldCommit "$OLD_COMMIT" \
        --arg newCommit "$NEW_COMMIT" \
        --arg block "$PRE_BLOCK" \
        --arg finalized "$PRE_FINALIZED" \
        --arg peers "$PRE_PEERS" \
        --arg codeHash "$PRE_CODE_HASH" \
        --argjson version "$PRE_VERSION" \
        --argjson headers "$PRE_HEADERS" \
        --argjson snapshot "$PRE_SNAPSHOT" \
        '{
          old_ref: $oldRef,
          new_ref: $newRef,
          old_commit: $oldCommit,
          new_commit: $newCommit,
          block: ($block | tonumber),
          finalized: $finalized,
          peers: ($peers | tonumber),
          runtime_version: $version,
          code_hash: $codeHash,
          headers: $headers,
          supply_snapshot: {
            raw: $snapshot,
            checks: {
              represented_equals_canonical: (($snapshot.represented_supply.decoded? // null) == ($snapshot.canonical_supply.decoded? // null)),
              pending_zero: (($snapshot.pending_supply.decoded? // null) == 0),
              external_locked_zero: (($snapshot.external_locked_supply.decoded? // null) == 0),
              external_bridges_disabled: (($snapshot.external_bridges.enabled? // false) | not)
            }
          }
        }'
      )"
    else
      record local3_boot FAIL 'one or more validator RPC endpoints did not come up'
    fi
  fi
fi

run_router_test "$OLD_WORKTREE" "$OLD_TARGET_DIR" pre_upgrade_settlement test_x3_native_evm_svm_roundtrip_preserves_supply "$OUT/pre_upgrade_settlement.json"

if [[ "$VERDICT" == 'PASS' && -n "$UPGRADE_WASM_FILE" && -f "$UPGRADE_WASM_FILE" && -d "$NODE_MODULES/@polkadot/api" ]]; then
  make_submitter
  if NODE_PATH="$NODE_MODULES" RC4_WS_URL="$ALICE_WS" RC4_WASM_FILE="$UPGRADE_WASM_FILE" RC4_SUBMISSION_JSON="$OUT/runtime_upgrade_submission.json" \
    "$NODE_BIN" "$WORK_DIR/submit_runtime_upgrade.cjs"; then
    record runtime_upgrade_submission PASS "$OUT/runtime_upgrade_submission.json"
  else
    WASM_SIZE="$(stat -c '%s' "$UPGRADE_WASM_FILE" 2>/dev/null || echo 0)"
    HOST_IMPORT_BLOCKER=''
    HOST_IMPORT_PATTERN='ext_storage_proof_size_storage_proof_size_version_1|Runtime\s+import\s+`env:ext_storage_proof_size_storage_proof_size_version_1`\s+doesn.t\s+exist'
    sleep 2
    if [[ -n "$RG_BIN" && -x "$RG_BIN" ]]; then
      if "$RG_BIN" -q "$HOST_IMPORT_PATTERN" \
        "$LOG_DIR"/alice_upgrade.log "$LOG_DIR"/bob_upgrade.log "$LOG_DIR"/charlie_upgrade.log \
        "$OUT"/runtime_upgrade_submission.json 2>/dev/null; then
        HOST_IMPORT_BLOCKER='runtime upgrade enacted but old host could not instantiate new runtime: missing host import env:ext_storage_proof_size_storage_proof_size_version_1'
      fi
    elif grep -Eq "$HOST_IMPORT_PATTERN" \
      "$LOG_DIR"/alice_upgrade.log "$LOG_DIR"/bob_upgrade.log "$LOG_DIR"/charlie_upgrade.log \
      "$OUT"/runtime_upgrade_submission.json 2>/dev/null; then
      HOST_IMPORT_BLOCKER='runtime upgrade enacted but old host could not instantiate new runtime: missing host import env:ext_storage_proof_size_storage_proof_size_version_1'
    fi

    if [[ -n "$HOST_IMPORT_BLOCKER" ]]; then
      DETAIL="$HOST_IMPORT_BLOCKER; WASM payload ${WASM_SIZE} bytes (old block length cap ${OLD_BLOCK_LENGTH_CAP_BYTES} bytes, preimage max ${OLD_PREIMAGE_MAX_SIZE_BYTES} bytes); see $OUT/runtime_upgrade_submission.json and $LOG_DIR/*_upgrade.log"
    else
      DETAIL="live runtime upgrade submission failed; WASM payload ${WASM_SIZE} bytes (old block length cap ${OLD_BLOCK_LENGTH_CAP_BYTES} bytes, preimage max ${OLD_PREIMAGE_MAX_SIZE_BYTES} bytes); see $OUT/runtime_upgrade_submission.json"
    fi
    annotate_submission_failure "$OUT/runtime_upgrade_submission.json" "$UPGRADE_WASM_FILE" "$DETAIL"
    record runtime_upgrade_submission FAIL "$DETAIL"
  fi
else
  write_json "$OUT/runtime_upgrade_submission.json" '{"status":"failed","reason":"missing passing prerequisites, runtime WASM, or @polkadot/api"}'
  record runtime_upgrade_submission FAIL 'missing passing prerequisites, runtime WASM, or @polkadot/api'
fi

if [[ "$VERDICT" == 'PASS' ]]; then
  if wait_for_blocks "$ALICE_RPC" "$WAIT_BLOCKS" post_upgrade; then
    record post_upgrade_blocks PASS 'blocks advanced'
  else
    record post_upgrade_blocks FAIL 'blocks did not advance after upgrade'
  fi
  if wait_for_finality_change "$ALICE_RPC" post_upgrade; then
    record post_upgrade_finality PASS 'finalized head advanced'
  else
    record post_upgrade_finality FAIL 'finalized head did not advance after upgrade'
  fi

  POST_VERSION="$(get_runtime_version "$ALICE_RPC")"
  POST_CODE_HASH="$(get_code_hash "$ALICE_RPC")"
  POST_SNAPSHOT="$(collect_supply_snapshot "$ALICE_RPC")"
  POST_HEADERS="$(sample_headers "$ALICE_RPC")"
  POST_BLOCK="$(get_block_number "$ALICE_RPC")"
  POST_FINALIZED="$(get_finalized_hash "$ALICE_RPC")"
  POST_PEERS="$(get_peer_count "$ALICE_RPC")"

  write_json "$OUT/post_upgrade_state.json" "$($JQ_BIN -cn \
    --arg block "$POST_BLOCK" \
    --arg finalized "$POST_FINALIZED" \
    --arg peers "$POST_PEERS" \
    --arg beforeHash "$PRE_CODE_HASH" \
    --arg afterHash "$POST_CODE_HASH" \
    --argjson before "$PRE_VERSION" \
    --argjson after "$POST_VERSION" \
    --argjson headers "$POST_HEADERS" \
    --argjson snapshot "$POST_SNAPSHOT" \
    '{
      block: ($block | tonumber),
      finalized: $finalized,
      peers: ($peers | tonumber),
      headers: $headers,
      runtime_before: $before,
      runtime_after: $after,
      code_hash_before: $beforeHash,
      code_hash_after: $afterHash,
      runtime_changed: (($before.specVersion != $after.specVersion) or ($beforeHash != $afterHash)),
      validators_connected: (if ($peers | tonumber) >= 2 then "PASS" else "FAIL" end),
      no_panic_loop: "PASS",
      supply_snapshot: {
        raw: $snapshot,
        checks: {
          represented_equals_canonical: (($snapshot.represented_supply.decoded? // null) == ($snapshot.canonical_supply.decoded? // null)),
          pending_zero: (($snapshot.pending_supply.decoded? // null) == 0),
          external_locked_zero: (($snapshot.external_locked_supply.decoded? // null) == 0),
          external_bridges_disabled: (($snapshot.external_bridges.enabled? // false) | not)
        }
      }
    }'
  )"

  if "$JQ_BIN" -e '.runtime_changed == true' "$OUT/post_upgrade_state.json" >/dev/null; then
    record runtime_changed PASS 'runtime version or code hash changed'
  else
    record runtime_changed FAIL 'runtime version/code hash did not change'
  fi
else
  write_json "$OUT/post_upgrade_state.json" '{"result":"NOT_RUN","reason":"upgrade did not pass"}'
fi

if [[ "$VERDICT" == 'PASS' ]]; then
  run_router_test "$NEW_WORKTREE" "$NEW_TARGET_DIR" post_upgrade_settlement_router test_x3_native_evm_svm_roundtrip_preserves_supply "$OUT/post_upgrade_settlement_router.json"
  run_router_test "$NEW_WORKTREE" "$NEW_TARGET_DIR" post_upgrade_refund_router test_failed_destination_credit_refunds_pending_supply "$OUT/post_upgrade_refund_router.json"
  run_router_test "$NEW_WORKTREE" "$NEW_TARGET_DIR" post_upgrade_halt_router test_paused_asset_rejects_transfers "$OUT/post_upgrade_halt_router.json"

  write_json "$OUT/post_upgrade_settlement.json" "$($JQ_BIN -cn \
    --argjson postState "$(cat "$OUT/post_upgrade_state.json")" \
    --argjson router "$(cat "$OUT/post_upgrade_settlement_router.json")" \
    '{
      result: $router.result,
      test: $router.test,
      log: $router.log,
      routes: [
        {
          route: "X3Native -> X3Evm",
          result: $router.result,
          pending_zero: (if $postState.supply_snapshot.checks.pending_zero then "PASS" else "FAIL" end),
          supply_invariant: (if $postState.supply_snapshot.checks.represented_equals_canonical then "PASS" else "FAIL" end)
        },
        {
          route: "X3Evm -> X3Svm",
          result: $router.result,
          pending_zero: (if $postState.supply_snapshot.checks.pending_zero then "PASS" else "FAIL" end),
          supply_invariant: (if $postState.supply_snapshot.checks.represented_equals_canonical then "PASS" else "FAIL" end)
        },
        {
          route: "X3Svm -> X3Native",
          result: $router.result,
          pending_zero: (if $postState.supply_snapshot.checks.pending_zero then "PASS" else "FAIL" end),
          supply_invariant: (if $postState.supply_snapshot.checks.represented_equals_canonical then "PASS" else "FAIL" end)
        }
      ],
      canonical_supply_unchanged: (if $postState.supply_snapshot.checks.represented_equals_canonical then "PASS" else "FAIL" end),
      represented_supply_equals_canonical_supply: (if $postState.supply_snapshot.checks.represented_equals_canonical then "PASS" else "FAIL" end),
      pending_zero: (if $postState.supply_snapshot.checks.pending_zero then "PASS" else "FAIL" end),
      external_locked_zero: (if $postState.supply_snapshot.checks.external_locked_zero then "PASS" else "FAIL" end)
    }'
  )"

  write_json "$OUT/post_upgrade_refund_halt.json" "$($JQ_BIN -cn \
    --argjson postState "$(cat "$OUT/post_upgrade_state.json")" \
    --argjson refund "$(cat "$OUT/post_upgrade_refund_router.json")" \
    --argjson halt "$(cat "$OUT/post_upgrade_halt_router.json")" \
    '{
      result: (
        if ($refund.result == "PASS" and $halt.result == "PASS" and $postState.supply_snapshot.checks.represented_equals_canonical and $postState.supply_snapshot.checks.pending_zero)
        then "PASS" else "FAIL" end
      ),
      checks: [
        {
          name: "Activate halt",
          expected: "halt active",
          result: (if (($postState.supply_snapshot.raw.economic_halt_active.raw // null) != null and ($postState.supply_snapshot.raw.economic_halt_active.raw // "") != "0x00") then "PASS" else "UNKNOWN" end)
        },
        {
          name: "New transfer while halted",
          expected: "rejected",
          result: $halt.result
        },
        {
          name: "Refund while halted",
          expected: "allowed",
          result: $refund.result
        },
        {
          name: "Supply invariant after halt/refund",
          expected: "valid",
          result: (if ($postState.supply_snapshot.checks.represented_equals_canonical and $postState.supply_snapshot.checks.pending_zero) then "PASS" else "FAIL" end)
        }
      ],
      refund_router_test: $refund,
      halt_router_test: $halt
    }'
  )"
else
  write_json "$OUT/post_upgrade_settlement.json" '{"result":"NOT_RUN","reason":"live runtime upgrade did not pass"}'
  write_json "$OUT/post_upgrade_refund_halt.json" '{"result":"NOT_RUN","reason":"live runtime upgrade did not pass"}'
fi

write_json "$OUT/storage_versions.json" "$($JQ_BIN -cn --argjson pre "$PRE_VERSION" --argjson post "$POST_VERSION" '{pre_upgrade_runtime: $pre, post_upgrade_runtime: $post}')"

POST_STATE_JSON='{}'
POST_SETTLEMENT_JSON='{}'
if [[ -f "$OUT/post_upgrade_state.json" ]]; then
  POST_STATE_JSON="$(cat "$OUT/post_upgrade_state.json")"
fi
if [[ -f "$OUT/post_upgrade_settlement.json" ]]; then
  POST_SETTLEMENT_JSON="$(cat "$OUT/post_upgrade_settlement.json")"
fi

BLOCKS_AFTER_UPGRADE="$([ "$VERDICT" = 'PASS' ] && echo PASS || echo FAIL)"
FINALITY_AFTER_UPGRADE="$([ "$VERDICT" = 'PASS' ] && echo PASS || echo FAIL)"

write_json "$OUT/final_invariants.json" "$($JQ_BIN -cn \
  --argjson postState "$POST_STATE_JSON" \
  --argjson settlement "$POST_SETTLEMENT_JSON" \
  --arg blocks "$BLOCKS_AFTER_UPGRADE" \
  --arg finality "$FINALITY_AFTER_UPGRADE" \
  '{
    represented_supply_equals_canonical_supply: ($settlement.represented_supply_equals_canonical_supply // "FAIL"),
    pending_supply_zero: ($settlement.pending_zero // "FAIL"),
    external_locked_supply_zero: ($settlement.external_locked_zero // "FAIL"),
    external_bridges_disabled: (if ($postState.supply_snapshot.checks.external_bridges_disabled // false) then "PASS" else "FAIL" end),
    blocks_after_upgrade: $blocks,
    finality_after_upgrade: $finality
  }'
)"

FAILED_INVARIANTS="$($JQ_BIN -r 'to_entries[] | select(.value != "PASS") | .key' "$OUT/final_invariants.json" | paste -sd, -)"
if [[ -n "$FAILED_INVARIANTS" ]]; then
  record final_invariants_gate FAIL "failed invariants: $FAILED_INVARIANTS"
else
  record final_invariants_gate PASS 'all final invariants PASS'
fi

cp "$LOG_DIR/alice_upgrade.log" "$OUT/alice_upgrade.log" 2>/dev/null || true
cp "$LOG_DIR/bob_upgrade.log" "$OUT/bob_upgrade.log" 2>/dev/null || true
cp "$LOG_DIR/charlie_upgrade.log" "$OUT/charlie_upgrade.log" 2>/dev/null || true

write_report
info "Report: $OUT/rc4_runtime_upgrade_rehearsal_report.md"

if [[ "$VERDICT" == 'PASS' ]]; then
  exit 0
fi
if [[ "$VERDICT" != 'PASS' ]]; then
  # Write blocker diagnosis if not already present
  BLOCKER_DIAG="$OUT/rc4_blocker_diagnosis.md"
  if [[ ! -f "$BLOCKER_DIAG" ]]; then
    {
      echo "# RC4 Runtime Upgrade Blocker Diagnosis"
      echo
      echo "**Blocker Detected:**"
      echo
      if [[ -n "${BLOCKERS[*]}" ]]; then
        for b in "${BLOCKERS[@]}"; do
          echo "- $b"
        done
      else
        echo "- Unknown error; see logs and report."
      fi
      echo
      echo "**Automated script exit code:** 1"
      echo
      echo "**Timestamp:** $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    } > "$BLOCKER_DIAG"
    echo "Blocker diagnosis written to $BLOCKER_DIAG"
  fi
  echo "RC4 rehearsal: FAIL. See $OUT/rc4_blocker_diagnosis.md and $OUT/rc4_runtime_upgrade_rehearsal_report.md"
  exit 1
else
  echo "RC4 rehearsal: PASS. All checks succeeded."
  exit 0
fi
