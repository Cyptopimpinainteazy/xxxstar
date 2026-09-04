#!/usr/bin/env bash
# RC5 Internal Alpha 72h Stability Harness
#
# Runs long-duration internal stability checks on a local/private validator set.
# Supports fast shakedowns via DURATION_SECONDS override.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

OUT_DIR="${OUT_DIR:-$ROOT_DIR/reports/rc5}"
LOG_DIR="${LOG_DIR:-$ROOT_DIR/logs/rc5}"

BINARY="${BINARY:-$ROOT_DIR/target/release/x3-chain-node}"
RAW_SPEC="${RAW_SPEC:-$ROOT_DIR/chain-specs/x3-rc5-local3-raw.json}"

RC5_BASE_DIR="${RC5_BASE_DIR:-$ROOT_DIR/.rc5-runtime/rc5}"

ATTACH_EXISTING="${ATTACH_EXISTING:-0}"
VALIDATOR_COUNT="${VALIDATOR_COUNT:-3}"

ALICE_RPC="${ALICE_RPC:-http://127.0.0.1:9964}"
BOB_RPC="${BOB_RPC:-http://127.0.0.1:9965}"
CHARLIE_RPC="${CHARLIE_RPC:-http://127.0.0.1:9966}"

ALICE_WS="${ALICE_WS:-ws://127.0.0.1:9964}"

DURATION_SECONDS="${DURATION_SECONDS:-259200}"
SNAPSHOT_INTERVAL_SECONDS="${SNAPSHOT_INTERVAL_SECONDS:-60}"
SETTLEMENT_INTERVAL_CYCLES="${SETTLEMENT_INTERVAL_CYCLES:-10}"
RESTART_DRILL_CYCLE="${RESTART_DRILL_CYCLE:-3}"
RESTART_DRILL_TRIGGER_CYCLE="${RESTART_DRILL_TRIGGER_CYCLE:-$RESTART_DRILL_CYCLE}"
FINALITY_WAIT_SECONDS="${FINALITY_WAIT_SECONDS:-120}"
BLOCK_WAIT_SECONDS="${BLOCK_WAIT_SECONDS:-120}"
SYNC_WAIT_SECONDS="${SYNC_WAIT_SECONDS:-120}"

RUN_SETTLEMENT_SMOKE="${RUN_SETTLEMENT_SMOKE:-1}"
WASM_EXECUTION="${WASM_EXECUTION:-compiled}"

if (( DURATION_SECONDS < RESTART_DRILL_CYCLE * SNAPSHOT_INTERVAL_SECONDS )); then
  RESTART_DRILL_TRIGGER_CYCLE=1
fi

RC5_STRICT_ARTIFACTS="${RC5_STRICT_ARTIFACTS:-1}"
RC5_FORCE_CLEAN_BUILD="${RC5_FORCE_CLEAN_BUILD:-1}"
RC5_REGENERATE_CHAIN_SPEC="${RC5_REGENERATE_CHAIN_SPEC:-1}"
RC5_CHAIN_ALIAS="${RC5_CHAIN_ALIAS:-local3}"
RC5_PLAIN_SPEC="${RC5_PLAIN_SPEC:-$ROOT_DIR/chain-specs/x3-rc5-local3-plain.json}"
RC5_RAW_SPEC="${RC5_RAW_SPEC:-$ROOT_DIR/chain-specs/x3-rc5-local3-raw.json}"
RC5_ARTIFACT_REPORT="${RC5_ARTIFACT_REPORT:-$ROOT_DIR/reports/rc5/rc5_artifact_compatibility_report.md}"

JQ_BIN="${JQ_BIN:-$(command -v jq 2>/dev/null || true)}"
CURL_BIN="${CURL_BIN:-$(command -v curl 2>/dev/null || true)}"
CARGO_BIN="${CARGO_BIN:-$(command -v cargo 2>/dev/null || true)}"

HEALTH_FILE="$OUT_DIR/health_snapshots.jsonl"
FINALITY_FILE="$OUT_DIR/finality_snapshots.jsonl"
SETTLEMENT_FILE="$OUT_DIR/settlement_snapshots.jsonl"
INVARIANT_FILE="$OUT_DIR/invariant_snapshots.jsonl"
RESTART_FILE="$OUT_DIR/validator_restart_drill.json"
RESOURCE_FILE="$OUT_DIR/resource_usage.jsonl"
SUMMARY_FILE="$OUT_DIR/final_summary.json"
REPORT_FILE="$OUT_DIR/rc5_internal_alpha_72h_report.md"

ALICE_PID=""
BOB_PID=""
CHARLIE_PID=""
ALICE_BASE="$RC5_BASE_DIR/alice"
BOB_BASE="$RC5_BASE_DIR/bob"
CHARLIE_BASE="$RC5_BASE_DIR/charlie"

START_BLOCK=0
END_BLOCK=0
START_FINALIZED=""
END_FINALIZED=""
START_TS=0
END_TS=0
RESTART_DRILL_DONE=0
RESTART_DRILL_PASS=0
SETTLEMENT_FAILS=0
INVARIANT_FAILS=0
PANIC_FLAG=0
DB_CORRUPTION_FLAG=0
BLOCK_PROGRESS_PASS=0
FINALITY_PROGRESS_PASS=0
PEER_MIN_PASS=0
RUNTIME_VERSION_PASS=0
BRIDGES_DISABLED_PASS=0
UNEXPECTED_ERROR=""
HOST_BOOT_ID=""
RUN_BOOT_ID=""
RUN_ABORTED_BY_REBOOT=0

ARTIFACT_STATUS="PENDING"
ARTIFACT_NOTES=""
ARTIFACT_HEAD=""
ARTIFACT_BINARY_SHA=""
ARTIFACT_PLAIN_SHA=""
ARTIFACT_RAW_SHA=""
ARTIFACT_WASM_COUNT=0

now_iso() {
  date -u +%Y-%m-%dT%H:%M:%SZ
}

log() {
  printf '[RC5] %s\n' "$*"
}

capture_boot_id() {
  if [[ -f /proc/sys/kernel/random/boot_id ]]; then
    cat /proc/sys/kernel/random/boot_id 2>/dev/null || true
  else
    echo ""
  fi
}

check_host_reboot() {
  local current_boot_id
  current_boot_id="$(capture_boot_id)"
  if [[ -z "$RUN_BOOT_ID" ]]; then
    RUN_BOOT_ID="$current_boot_id"
  elif [[ -n "$current_boot_id" && "$current_boot_id" != "$RUN_BOOT_ID" ]]; then
    RUN_ABORTED_BY_REBOOT=1
    return 1
  fi
  return 0
}

binary_target_dir() {
  local bin_dir
  bin_dir="$(dirname "$BINARY")"
  dirname "$bin_dir"
}

capture_build_spec_json() {
  local out_file="$1"
  shift
  "$BINARY" build-spec "$@" \
    | awk 'BEGIN{emit=0} /^[[:space:]]*\{/ {emit=1} emit {print}' > "$out_file"

  if [[ ! -s "$out_file" ]]; then
    return 1
  fi
  if ! "$JQ_BIN" -e 'type == "object"' "$out_file" >/dev/null 2>&1; then
    return 1
  fi
  return 0
}

write_artifact_report() {
  local ts
  local wasm_file
  ts="$(now_iso)"
  wasm_file="$OUT_DIR/runtime_wasm_candidates.sha256"
  mkdir -p "$(dirname "$RC5_ARTIFACT_REPORT")"

  {
    echo "# RC5 Artifact Compatibility Report"
    echo
    echo "## Verdict"
    echo
    echo "$ARTIFACT_STATUS"
    echo
    echo "## Timestamp"
    echo
    echo "- generated_at_utc: $ts"
    echo
    echo "## Inputs"
    echo
    echo "- strict_mode: $RC5_STRICT_ARTIFACTS"
    echo "- force_clean_build: $RC5_FORCE_CLEAN_BUILD"
    echo "- regenerate_chain_spec: $RC5_REGENERATE_CHAIN_SPEC"
    echo "- chain_alias: $RC5_CHAIN_ALIAS"
    echo "- binary: $BINARY"
    echo "- plain_spec: $RC5_PLAIN_SPEC"
    echo "- raw_spec: $RC5_RAW_SPEC"
    echo
    echo "## Artifact Fingerprints"
    echo
    echo "- git_head: ${ARTIFACT_HEAD:-UNKNOWN}"
    echo "- node_binary_sha256: ${ARTIFACT_BINARY_SHA:-UNAVAILABLE}"
    echo "- plain_spec_sha256: ${ARTIFACT_PLAIN_SHA:-UNAVAILABLE}"
    echo "- raw_spec_sha256: ${ARTIFACT_RAW_SHA:-UNAVAILABLE}"
    echo "- runtime_wasm_candidates: $ARTIFACT_WASM_COUNT"
    echo
    echo "## Runtime WASM Candidate Hashes"
    echo
    if [[ -s "$wasm_file" ]]; then
      sed 's/^/- /' "$wasm_file"
    else
      echo "- none found"
    fi
    echo
    echo "## Notes"
    echo
    if [[ -n "$ARTIFACT_NOTES" ]]; then
      echo "- $ARTIFACT_NOTES"
    else
      echo "- none"
    fi
  } > "$RC5_ARTIFACT_REPORT"
}

artifact_preflight() {
  local target_dir
  local build_log
  local preflight_plain
  local wasm_file

  target_dir="$(binary_target_dir)"
  build_log="$LOG_DIR/rc5_artifact_build.log"
  preflight_plain="$OUT_DIR/rc5_buildspec_preflight_local3.json"
  wasm_file="$OUT_DIR/runtime_wasm_candidates.sha256"

  ARTIFACT_HEAD="$(git -C "$ROOT_DIR" rev-parse HEAD 2>/dev/null || true)"

  if [[ "$ATTACH_EXISTING" == "1" ]]; then
    ARTIFACT_STATUS="SKIPPED"
    ARTIFACT_NOTES="ATTACH_EXISTING=1; strict artifact preflight skipped"
    write_artifact_report
    return 0
  fi

  pkill -f "[x]3-chain-node" 2>/dev/null || true
  for _ in {1..20}; do
    if ! pgrep -f "[x]3-chain-node" >/dev/null 2>&1; then
      break
    fi
    sleep 0.5
  done
  rm -rf "$ALICE_BASE" "$BOB_BASE" "$CHARLIE_BASE"

  rm -f \
    "$ROOT_DIR/chain-specs/x3-rc5-local3-plain.json" \
    "$ROOT_DIR/chain-specs/x3-rc5-local3-raw.json" \
    "$ROOT_DIR/chain-specs/x3-local3-plain.json" \
    "$ROOT_DIR/chain-specs/x3-local3-raw.json"

  mkdir -p "$OUT_DIR" "$LOG_DIR" "$ROOT_DIR/chain-specs"
  : > "$wasm_file"

  if [[ "$RC5_FORCE_CLEAN_BUILD" == "1" ]]; then
    require_tool "$CARGO_BIN" cargo
    log "Running strict artifact build in $target_dir"
    if ! (
      cd "$ROOT_DIR"
      "$CARGO_BIN" clean -p x3-chain-runtime -p x3-chain-node --target-dir "$target_dir"
      "$CARGO_BIN" build --release -p x3-chain-node --target-dir "$target_dir"
    ) > "$build_log" 2>&1; then
      ARTIFACT_STATUS="FAIL"
      ARTIFACT_NOTES="clean build failed; see $build_log"
      write_artifact_report
      return 1
    fi
  elif [[ ! -x "$BINARY" ]]; then
    ARTIFACT_STATUS="FAIL"
    ARTIFACT_NOTES="node binary missing and RC5_FORCE_CLEAN_BUILD=0"
    write_artifact_report
    return 1
  fi

  if ! capture_build_spec_json "$preflight_plain" --chain "$RC5_CHAIN_ALIAS" 2> "$LOG_DIR/rc5_buildspec_preflight.log"; then
    ARTIFACT_STATUS="FAIL"
    ARTIFACT_NOTES="build-spec preflight failed; runtime cannot instantiate from current binary"
    write_artifact_report
    return 1
  fi

  if [[ "$RC5_REGENERATE_CHAIN_SPEC" == "1" ]]; then
    if ! capture_build_spec_json "$RC5_PLAIN_SPEC" --chain "$RC5_CHAIN_ALIAS"; then
      ARTIFACT_STATUS="FAIL"
      ARTIFACT_NOTES="failed to generate plain chain spec from current binary"
      write_artifact_report
      return 1
    fi
    if ! capture_build_spec_json "$RC5_RAW_SPEC" --chain "$RC5_PLAIN_SPEC" --raw; then
      ARTIFACT_STATUS="FAIL"
      ARTIFACT_NOTES="failed to generate raw chain spec from current binary"
      write_artifact_report
      return 1
    fi
    RAW_SPEC="$RC5_RAW_SPEC"
  fi

  if [[ ! -f "$RAW_SPEC" ]]; then
    ARTIFACT_STATUS="FAIL"
    ARTIFACT_NOTES="effective raw chain spec not found: $RAW_SPEC"
    write_artifact_report
    return 1
  fi

  ARTIFACT_BINARY_SHA="$(sha256sum "$BINARY" | awk '{print $1}' || true)"
  ARTIFACT_PLAIN_SHA="$(sha256sum "$RC5_PLAIN_SPEC" | awk '{print $1}' || true)"
  ARTIFACT_RAW_SHA="$(sha256sum "$RAW_SPEC" | awk '{print $1}' || true)"

  if find "$target_dir" -path '*wbuild*x3-chain-runtime*.wasm' -type f -print0 2>/dev/null | sort -z | xargs -0 sha256sum > "$wasm_file" 2>/dev/null; then
    ARTIFACT_WASM_COUNT="$(wc -l < "$wasm_file" | tr -d ' ')"
  else
    ARTIFACT_WASM_COUNT=0
    : > "$wasm_file"
  fi

  ARTIFACT_STATUS="PASS"
  ARTIFACT_NOTES="artifacts rebuilt/regenerated from one binary; runtime build-spec preflight passed"
  write_artifact_report
  return 0
}

require_tool() {
  local tool_path="$1"
  local label="$2"
  if [[ -z "$tool_path" || ! -x "$tool_path" ]]; then
    echo "ERROR: missing required tool: $label" >&2
    exit 1
  fi
}

rpc() {
  local url="$1"
  local method="$2"
  local params="${3:-[]}"
  "$CURL_BIN" -sf -m 10 "$url" -H 'Content-Type: application/json' \
    -d "{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"$method\",\"params\":$params}"
}

get_block_number() {
  local url="$1"
  local hex
  hex="$(rpc "$url" chain_getHeader '[]' | "$JQ_BIN" -r '.result.number // "0x0"' 2>/dev/null || true)"
  [[ "$hex" =~ ^0x[0-9a-fA-F]+$ ]] || { echo 0; return 0; }
  printf '%d\n' "$((16#${hex#0x}))"
}

get_finalized_hash() {
  local url="$1"
  rpc "$url" chain_getFinalizedHead '[]' | "$JQ_BIN" -r '.result // ""' 2>/dev/null || true
}

get_finalized_number() {
  local url="$1"
  local hash
  hash="$(get_finalized_hash "$url")"
  if [[ -z "$hash" ]]; then
    echo 0
    return 0
  fi
  local hex
  hex="$(rpc "$url" chain_getHeader "[\"$hash\"]" | "$JQ_BIN" -r '.result.number // "0x0"' 2>/dev/null || true)"
  [[ "$hex" =~ ^0x[0-9a-fA-F]+$ ]] || { echo 0; return 0; }
  printf '%d\n' "$((16#${hex#0x}))"
}

get_peer_count() {
  local url="$1"
  local peers
  peers="$(rpc "$url" system_health '[]' | "$JQ_BIN" -r '.result.peers // 0' 2>/dev/null || true)"
  [[ "$peers" =~ ^[0-9]+$ ]] || { echo 0; return 0; }
  echo "$peers"
}

get_runtime_version() {
  local url="$1"
  rpc "$url" state_getRuntimeVersion '[]' | "$JQ_BIN" -c '.result // {}'
}

get_runtime_spec_version() {
  local url="$1"
  local spec
  spec="$(rpc "$url" state_getRuntimeVersion '[]' | "$JQ_BIN" -r '.result.specVersion // 0' 2>/dev/null || true)"
  [[ "$spec" =~ ^[0-9]+$ ]] || { echo 0; return 0; }
  echo "$spec"
}

append_jsonl() {
  local file="$1"
  local payload="$2"
  printf '%s\n' "$payload" >> "$file"
}

wait_for_rpc() {
  local url="$1"
  local label="$2"
  local timeout="${3:-120}"
  local deadline=$(( $(date +%s) + timeout ))
  while [[ $(date +%s) -lt $deadline ]]; do
    if rpc "$url" system_health '[]' >/dev/null 2>&1; then
      log "$label RPC ready"
      return 0
    fi
    sleep 2
  done
  return 1
}

wait_for_block_progress() {
  local url="$1"
  local timeout="$2"
  local start cur
  start="$(get_block_number "$url")"
  local deadline=$(( $(date +%s) + timeout ))
  while [[ $(date +%s) -lt $deadline ]]; do
    cur="$(get_block_number "$url")"
    if (( cur > start )); then
      return 0
    fi
    sleep 2
  done
  return 1
}

wait_for_finality_progress() {
  local url="$1"
  local timeout="$2"
  local start cur
  start="$(get_finalized_hash "$url")"
  local deadline=$(( $(date +%s) + timeout ))
  while [[ $(date +%s) -lt $deadline ]]; do
    cur="$(get_finalized_hash "$url")"
    if [[ -n "$cur" && "$cur" != "$start" ]]; then
      return 0
    fi
    sleep 2
  done
  return 1
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

def state_call(url, method_name):
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


def bridge_enabled(router_raw):
  if router_raw in (None, 'null', '0x00', '0x0'):
    return False
  return router_raw in ('0x01', '0x1', True, 'true')

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
    'pending_supply': state_call(url, 'X3SupplyLedger_pending_supply'),
    'represented_supply': state_call(url, 'X3SupplyLedger_represented_supply'),
    'canonical_supply': state_call(url, 'X3SupplyLedger_canonical_supply'),
    'external_locked_supply': state_call(url, 'X3SupplyLedger_external_locked_supply'),
    'economic_halt_active': state_call(url, 'X3SupplyLedger_economic_halt_active'),
    'would_reject_transfer': state_call(url, 'X3SupplyLedger_would_reject_transfer'),
    'can_refund_while_halted': state_call(url, 'X3SupplyLedger_can_refund_while_halted'),
}

ledger_agg = aggregate_ledgers(url)
snapshot['ledger_aggregation'] = ledger_agg

for field in ('pending_supply', 'represented_supply', 'canonical_supply', 'external_locked_supply'):
    if snapshot[field].get('decoded') is None:
        snapshot[field]['decoded'] = ledger_agg.get(field)
        snapshot[field]['source'] = 'ledger_aggregation'
    else:
        snapshot[field]['source'] = 'state_call'

router_storage_key = '0x' + twox128('X3CrossVmRouter') + twox128('ExternalBridgesEnabled')
legacy_storage_key = '0x' + twox128('X3BridgeAdapters') + twox128('ExternalBridgesEnabled')
bridge_value = storage_get(url, router_storage_key)
legacy_bridge_value = storage_get(url, legacy_storage_key)
snapshot['external_bridges'] = {
    'storage_key': router_storage_key,
    'legacy_storage_key': legacy_storage_key,
    'raw': bridge_value,
    'legacy_raw': legacy_bridge_value,
    'enabled': bridge_enabled(bridge_value),
}

print(json.dumps(snapshot))
PY
}

cleanup() {
  if [[ "$ATTACH_EXISTING" == "1" ]]; then
    return 0
  fi
  for pid in "$ALICE_PID" "$BOB_PID" "$CHARLIE_PID"; do
    [[ -n "$pid" ]] && kill "$pid" 2>/dev/null || true
  done
  pkill -f "[x]3-chain-node.*\.rc5-runtime/rc5" 2>/dev/null || true
  pkill -f "[x]3-chain-node.*x3-rc5" 2>/dev/null || true
}
trap cleanup EXIT

start_alice() {
  "$BINARY" \
    --chain "$RAW_SPEC" \
    --alice \
    --node-key 0000000000000000000000000000000000000000000000000000000000000101 \
    --base-path "$ALICE_BASE" \
    --port 30633 \
    --rpc-port 9964 \
    --prometheus-port 9715 \
    --rpc-cors all \
    --rpc-methods unsafe \
    --wasm-execution "$WASM_EXECUTION" \
    --validator \
    --log info,runtime::x3=debug \
    > >(tee "$LOG_DIR/alice.log") 2>&1 &
  ALICE_PID=$!
}

start_bob() {
  local bootnode="$1"
  "$BINARY" \
    --chain "$RAW_SPEC" \
    --bob \
    --node-key 0000000000000000000000000000000000000000000000000000000000000102 \
    --base-path "$BOB_BASE" \
    --port 30634 \
    --rpc-port 9965 \
    --prometheus-port 9716 \
    --rpc-cors all \
    --rpc-methods unsafe \
    --wasm-execution "$WASM_EXECUTION" \
    --validator \
    --bootnodes "$bootnode" \
    --log info,runtime::x3=debug \
    > >(tee "$LOG_DIR/bob.log") 2>&1 &
  BOB_PID=$!
}

start_charlie() {
  local bootnode="$1"
  "$BINARY" \
    --chain "$RAW_SPEC" \
    --charlie \
    --node-key 0000000000000000000000000000000000000000000000000000000000000103 \
    --base-path "$CHARLIE_BASE" \
    --port 30635 \
    --rpc-port 9966 \
    --prometheus-port 9717 \
    --rpc-cors all \
    --rpc-methods unsafe \
    --wasm-execution "$WASM_EXECUTION" \
    --validator \
    --bootnodes "$bootnode" \
    --log info,runtime::x3=debug \
    > >(tee "$LOG_DIR/charlie.log") 2>&1 &
  CHARLIE_PID=$!
}

get_alice_bootnode() {
  local attempts=45
  for _ in $(seq 1 "$attempts"); do
    local id
    id="$(grep -m1 'Local node identity is:' "$LOG_DIR/alice.log" 2>/dev/null | awk '{print $NF}' || true)"
    if [[ -n "$id" ]]; then
      echo "/ip4/127.0.0.1/tcp/30633/p2p/$id"
      return 0
    fi
    sleep 2
  done
  return 1
}

snapshot_resource() {
  local ts="$1"
  local disk_used
  disk_used="$(df -Pk "$ROOT_DIR" | awk 'NR==2 {print $3}' 2>/dev/null || echo 0)"

  for pair in "alice:$ALICE_PID" "bob:$BOB_PID" "charlie:$CHARLIE_PID"; do
    local name="${pair%%:*}"
    local pid="${pair##*:}"
    local cpu="null"
    local mem_pct="null"
    local rss_kb="null"
    local alive=false
    if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
      alive=true
      cpu="$(ps -p "$pid" -o %cpu= 2>/dev/null | awk '{print $1+0}' || echo 0)"
      mem_pct="$(ps -p "$pid" -o %mem= 2>/dev/null | awk '{print $1+0}' || echo 0)"
      rss_kb="$(ps -p "$pid" -o rss= 2>/dev/null | awk '{print $1+0}' || echo 0)"
    fi
    append_jsonl "$RESOURCE_FILE" "$($JQ_BIN -cn \
      --arg ts "$ts" \
      --arg name "$name" \
      --argjson cpu "$cpu" \
      --argjson mem "$mem_pct" \
      --argjson rss "$rss_kb" \
      --argjson disk "$disk_used" \
      --argjson alive "$alive" \
      '{timestamp:$ts, validator:$name, alive:$alive, cpu_percent:$cpu, mem_percent:$mem, rss_kb:$rss, disk_used_kb:$disk}')"
  done
}

run_settlement_smoke() {
  local cycle="$1"
  local ts="$2"
  local smoke_log="$OUT_DIR/settlement_smoke_cycle_${cycle}.log"
  local result="PASS"
  local detail="test_x3_native_evm_svm_roundtrip_preserves_supply"

  if [[ "$RUN_SETTLEMENT_SMOKE" != "1" ]]; then
    result="NOT_RUN"
    detail="RUN_SETTLEMENT_SMOKE=$RUN_SETTLEMENT_SMOKE"
  elif [[ -z "$CARGO_BIN" || ! -x "$CARGO_BIN" ]]; then
    result="FAIL"
    detail="cargo not found"
  else
    if (
      cd "$ROOT_DIR"
      "$CARGO_BIN" test -p pallet-x3-cross-vm-router --lib test_x3_native_evm_svm_roundtrip_preserves_supply -- --nocapture
    ) > "$smoke_log" 2>&1; then
      result="PASS"
      detail="smoke test passed"
    else
      result="FAIL"
      detail="smoke test failed; see $smoke_log"
      SETTLEMENT_FAILS=$((SETTLEMENT_FAILS + 1))
    fi
  fi

  local snapshot
  snapshot="$(collect_supply_snapshot "$ALICE_RPC")"
  local represented_ok pending_ok external_locked_ok bridges_disabled
  represented_ok="$($JQ_BIN -r '((.represented_supply.decoded // null) == (.canonical_supply.decoded // null))' <<<"$snapshot")"
  pending_ok="$($JQ_BIN -r '((.pending_supply.decoded // -1) == 0)' <<<"$snapshot")"
  external_locked_ok="$($JQ_BIN -r '((.external_locked_supply.decoded // -1) == 0)' <<<"$snapshot")"
  bridges_disabled="$($JQ_BIN -r '(.external_bridges.enabled == true | not)' <<<"$snapshot")"

  if [[ "$represented_ok" != "true" || "$pending_ok" != "true" || "$external_locked_ok" != "true" || "$bridges_disabled" != "true" ]]; then
    INVARIANT_FAILS=$((INVARIANT_FAILS + 1))
  fi

  append_jsonl "$SETTLEMENT_FILE" "$($JQ_BIN -cn \
    --arg ts "$ts" \
    --argjson cycle "$cycle" \
    --arg result "$result" \
    --arg detail "$detail" \
    --arg represented "$represented_ok" \
    --arg pending "$pending_ok" \
    --arg extlocked "$external_locked_ok" \
    --arg bridges "$bridges_disabled" \
    '{timestamp:$ts, cycle:$cycle, routes:["X3Native->X3Evm","X3Evm->X3Svm","X3Svm->X3Native"], settlement_result:$result, detail:$detail, checks:{represented_equals_canonical:($represented=="true"), pending_zero:($pending=="true"), external_locked_zero:($extlocked=="true"), external_bridges_disabled:($bridges=="true")}}')"
}

run_restart_drill() {
  local ts="$1"
  local pre_block pre_fin post_stop_block post_stop_fin catchup_ok
  pre_block="$(get_block_number "$ALICE_RPC")"
  pre_fin="$(get_finalized_hash "$ALICE_RPC")"

  if [[ -n "$BOB_PID" ]]; then
    kill "$BOB_PID" 2>/dev/null || true
  fi
  sleep 12

  post_stop_block="$(get_block_number "$ALICE_RPC")"
  post_stop_fin="$(get_finalized_hash "$ALICE_RPC")"

  local bob_alive_during_stop=false
  if [[ -n "$BOB_PID" ]] && kill -0 "$BOB_PID" 2>/dev/null; then
    bob_alive_during_stop=true
  fi

  local bootnode
  bootnode="$(get_alice_bootnode || true)"
  if [[ -z "$bootnode" ]]; then
    RESTART_DRILL_PASS=0
    RESTART_DRILL_DONE=1
    write_json_restart "$ts" "$pre_block" "$pre_fin" "$post_stop_block" "$post_stop_fin" false "$bob_alive_during_stop" "missing bootnode for bob restart"
    return 0
  fi

  start_bob "$bootnode"

  if ! wait_for_rpc "$BOB_RPC" "Bob(restart)" 90; then
    RESTART_DRILL_PASS=0
    RESTART_DRILL_DONE=1
    write_json_restart "$ts" "$pre_block" "$pre_fin" "$post_stop_block" "$post_stop_fin" false "$bob_alive_during_stop" "bob rpc did not return after restart"
    return 0
  fi

  local deadline=$(( $(date +%s) + SYNC_WAIT_SECONDS ))
  catchup_ok=false
  while [[ $(date +%s) -lt $deadline ]]; do
    local bob_block alice_block
    bob_block="$(get_block_number "$BOB_RPC")"
    alice_block="$(get_block_number "$ALICE_RPC")"
    if (( bob_block >= alice_block - 2 )); then
      catchup_ok=true
      break
    fi
    sleep 3
  done

  if wait_for_block_progress "$ALICE_RPC" 30 && wait_for_finality_progress "$ALICE_RPC" 30 && [[ "$catchup_ok" == "true" ]]; then
    RESTART_DRILL_PASS=1
  else
    RESTART_DRILL_PASS=0
  fi
  RESTART_DRILL_DONE=1
  write_json_restart "$ts" "$pre_block" "$pre_fin" "$post_stop_block" "$post_stop_fin" "$catchup_ok" "$bob_alive_during_stop" ""
}

write_json_restart() {
  local ts="$1"
  local pre_block="$2"
  local pre_fin="$3"
  local post_stop_block="$4"
  local post_stop_fin="$5"
  local catchup_ok="$6"
  local bob_alive_during_stop="$7"
  local note="$8"
  "$JQ_BIN" -cn \
    --arg ts "$ts" \
    --argjson pre_block "$pre_block" \
    --arg pre_fin "$pre_fin" \
    --argjson post_stop_block "$post_stop_block" \
    --arg post_stop_fin "$post_stop_fin" \
    --argjson catchup_ok "$catchup_ok" \
    --argjson drill_pass "$RESTART_DRILL_PASS" \
    --argjson bob_alive_during_stop "$bob_alive_during_stop" \
    --arg note "$note" \
    '{timestamp:$ts, validator:"bob", pre_stop:{block:$pre_block, finalized:$pre_fin}, during_stop:{block:$post_stop_block, finalized:$post_stop_fin, bob_alive:$bob_alive_during_stop}, post_restart:{bob_catchup_ok:$catchup_ok}, drill_pass:$drill_pass, note:$note}' \
    > "$RESTART_FILE"
}

write_report() {
  local overall="$1"
  local short_status="$2"
  local run72_status="$3"
  local blockers_json="$4"
  local blockers
  blockers="$($JQ_BIN -r '.[]' <<<"$blockers_json" 2>/dev/null || true)"

  {
    echo "# RC5 Internal Alpha 72h Report"
    echo
    echo "## Verdict"
    echo
    echo "$overall"
    echo
    echo "## Status Gates"
    echo
    echo "- RC5_SHORT_SHAKEDOWN: $short_status"
    echo "- RC5_72H: $run72_status"
    echo
    echo "## Scope"
    echo
    echo "- validators: $VALIDATOR_COUNT"
    echo "- duration_seconds: $DURATION_SECONDS"
    echo "- snapshot_interval_seconds: $SNAPSHOT_INTERVAL_SECONDS"
    echo "- settlement_interval_cycles: $SETTLEMENT_INTERVAL_CYCLES"
    echo "- restart_drill_cycle: $RESTART_DRILL_CYCLE"
    echo "- external bridges: disabled (required)"
    echo
    echo "## Result Files"
    echo
    echo "- health_snapshots.jsonl"
    echo "- finality_snapshots.jsonl"
    echo "- settlement_snapshots.jsonl"
    echo "- invariant_snapshots.jsonl"
    echo "- validator_restart_drill.json"
    echo "- resource_usage.jsonl"
    echo "- final_summary.json"
    echo
    echo "## Blockers"
    echo
    if [[ -n "$blockers" ]]; then
      while IFS= read -r line; do
        [[ -n "$line" ]] && echo "- $line"
      done <<< "$blockers"
    else
      echo "None"
    fi
  } > "$REPORT_FILE"
}

main() {
  mkdir -p "$OUT_DIR" "$LOG_DIR"
  : > "$HEALTH_FILE"
  : > "$FINALITY_FILE"
  : > "$SETTLEMENT_FILE"
  : > "$INVARIANT_FILE"
  : > "$RESOURCE_FILE"

  require_tool "$JQ_BIN" jq
  require_tool "$CURL_BIN" curl

  if [[ "$RC5_STRICT_ARTIFACTS" == "1" ]]; then
    artifact_preflight
  fi

  # 1. Confirm RC4 PASS report exists.
  local rc4_report="$ROOT_DIR/reports/rc4/rc4_runtime_upgrade_rehearsal_report.md"
  if [[ ! -f "$rc4_report" ]]; then
    echo "ERROR: missing RC4 report: $rc4_report" >&2
    exit 1
  fi
  if ! python3 - "$rc4_report" <<'PY'
import sys
from pathlib import Path
text = Path(sys.argv[1]).read_text(encoding='utf-8')
lines = [line.strip() for line in text.splitlines()]
ok = False
for i, line in enumerate(lines):
    if line == '## Verdict':
        for j in range(i+1, min(i+8, len(lines))):
            if lines[j] == 'PASS':
                ok = True
                break
        break
print('PASS' if ok else 'FAIL')
sys.exit(0 if ok else 1)
PY
  then
    echo "ERROR: RC4 report does not declare PASS verdict" >&2
    exit 1
  fi

  # 2. Build release node or verify existing fresh binary.
  if [[ ! -x "$BINARY" ]]; then
    log "Release node missing; building $BINARY"
    require_tool "$CARGO_BIN" cargo
    (
      cd "$ROOT_DIR"
      "$CARGO_BIN" build --release -p x3-chain-node
    ) > "$LOG_DIR/release_build.log" 2>&1
  fi

  # 3. Boot validators (or attach existing).
  if [[ "$ATTACH_EXISTING" != "1" ]]; then
    rm -rf "$ALICE_BASE" "$BOB_BASE" "$CHARLIE_BASE"
    start_alice
    local bootnode
    bootnode="$(get_alice_bootnode || true)"
    if [[ -z "$bootnode" ]]; then
      echo "ERROR: failed to discover Alice bootnode identity" >&2
      exit 1
    fi
    start_bob "$bootnode"
    start_charlie "$bootnode"
  fi

  wait_for_rpc "$ALICE_RPC" Alice 120 || { echo "ERROR: Alice RPC unavailable" >&2; exit 1; }
  wait_for_rpc "$BOB_RPC" Bob 120 || { echo "ERROR: Bob RPC unavailable" >&2; exit 1; }
  wait_for_rpc "$CHARLIE_RPC" Charlie 120 || { echo "ERROR: Charlie RPC unavailable" >&2; exit 1; }

  # 4-6 peers, blocks, finality.
  local peers
  peers="$(get_peer_count "$ALICE_RPC")"
  if (( peers >= 2 )); then PEER_MIN_PASS=1; fi
  if wait_for_block_progress "$ALICE_RPC" "$BLOCK_WAIT_SECONDS"; then BLOCK_PROGRESS_PASS=1; fi
  if wait_for_finality_progress "$ALICE_RPC" "$FINALITY_WAIT_SECONDS"; then FINALITY_PROGRESS_PASS=1; fi

  # 7 runtime version >= 10.
  local spec
  spec="$(get_runtime_spec_version "$ALICE_RPC")"
  if (( spec >= 10 )); then RUNTIME_VERSION_PASS=1; fi

  # 8 external bridges disabled.
  local init_snapshot
  init_snapshot="$(collect_supply_snapshot "$ALICE_RPC")"
  if [[ "$($JQ_BIN -r '(.external_bridges.enabled == true | not)' <<<"$init_snapshot")" == "true" ]]; then
    BRIDGES_DISABLED_PASS=1
  fi

  START_BLOCK="$(get_block_number "$ALICE_RPC")"
  START_FINALIZED="$(get_finalized_hash "$ALICE_RPC")"
  START_TS="$(date +%s)"
  RUN_BOOT_ID="$(capture_boot_id)"

  local cycles=0
  local end_deadline=$(( START_TS + DURATION_SECONDS ))

  while [[ $(date +%s) -lt $end_deadline ]]; do
    if ! check_host_reboot; then
      log "ABORT: Host rebooted during run (boot ID mismatch). Marking run as invalid."
      break
    fi
    cycles=$((cycles + 1))
    local ts block fin_hash fin_num peer_count rtv specv uptime_sec

    ts="$(now_iso)"
    block="$(get_block_number "$ALICE_RPC")"
    fin_hash="$(get_finalized_hash "$ALICE_RPC")"
    fin_num="$(get_finalized_number "$ALICE_RPC")"
    peer_count="$(get_peer_count "$ALICE_RPC")"
    rtv="$(get_runtime_version "$ALICE_RPC")"
    specv="$($JQ_BIN -r '.specVersion // 0' <<<"$rtv")"
    uptime_sec=$(( $(date +%s) - START_TS ))

    append_jsonl "$HEALTH_FILE" "$($JQ_BIN -cn \
      --arg ts "$ts" --argjson block "$block" --argjson finalized "$fin_num" --arg fin_hash "$fin_hash" --argjson peers "$peer_count" --argjson spec "$specv" --argjson uptime "$uptime_sec" \
      '{timestamp:$ts, latest_block:$block, finalized_block:$finalized, finalized_hash:$fin_hash, peers:$peers, runtime_spec_version:$spec, uptime_seconds:$uptime}')"

    append_jsonl "$FINALITY_FILE" "$($JQ_BIN -cn \
      --arg ts "$ts" --argjson latest "$block" --argjson finalized "$fin_num" --arg hash "$fin_hash" \
      '{timestamp:$ts, latest_block:$latest, finalized_block:$finalized, finalized_hash:$hash}')"

    local snapshot rep_ok pend_ok ext_ok bridge_ok bridge_raw bridge_legacy_raw
    snapshot="$(collect_supply_snapshot "$ALICE_RPC")"
    rep_ok="$($JQ_BIN -r '((.represented_supply.decoded // null) == (.canonical_supply.decoded // null))' <<<"$snapshot")"
    pend_ok="$($JQ_BIN -r '((.pending_supply.decoded // -1) == 0)' <<<"$snapshot")"
    ext_ok="$($JQ_BIN -r '((.external_locked_supply.decoded // -1) == 0)' <<<"$snapshot")"
    bridge_ok="$($JQ_BIN -r '(.external_bridges.enabled == true | not)' <<<"$snapshot")"
    bridge_raw="$($JQ_BIN -r '(.external_bridges.raw // "null")' <<<"$snapshot")"
    bridge_legacy_raw="$($JQ_BIN -r '(.external_bridges.legacy_raw // "null")' <<<"$snapshot")"

    if [[ "$rep_ok" != "true" || "$pend_ok" != "true" || "$ext_ok" != "true" || "$bridge_ok" != "true" ]]; then
      INVARIANT_FAILS=$((INVARIANT_FAILS + 1))
    fi

    if (( peer_count >= 2 )); then
      PEER_MIN_PASS=1
    fi

    append_jsonl "$INVARIANT_FILE" "$($JQ_BIN -cn \
      --arg ts "$ts" --argjson cycle "$cycles" --argjson rep "$rep_ok" --argjson pend "$pend_ok" --argjson ext "$ext_ok" --argjson bridges "$bridge_ok" --arg bridge_raw "$bridge_raw" --arg bridge_legacy_raw "$bridge_legacy_raw" \
      '{timestamp:$ts, cycle:$cycle, checks:{represented_equals_canonical:$rep, pending_zero:$pend, external_locked_zero:$ext, external_bridges_disabled:$bridges}, external_bridges:{raw:$bridge_raw, legacy_raw:$bridge_legacy_raw}}')"

    snapshot_resource "$ts"

    if (( cycles % SETTLEMENT_INTERVAL_CYCLES == 0 )); then
      run_settlement_smoke "$cycles" "$ts"
    fi

    if (( RESTART_DRILL_DONE == 0 && cycles >= RESTART_DRILL_TRIGGER_CYCLE )); then
      run_restart_drill "$ts"
    fi

    local now_ts sleep_seconds
    now_ts=$(date +%s)
    sleep_seconds=$(( end_deadline - now_ts ))
    if (( sleep_seconds > SNAPSHOT_INTERVAL_SECONDS )); then
      sleep_seconds=$SNAPSHOT_INTERVAL_SECONDS
    fi
    if (( sleep_seconds > 0 )); then
      sleep "$sleep_seconds"
    fi
  done

  END_BLOCK="$(get_block_number "$ALICE_RPC")"
  END_FINALIZED="$(get_finalized_hash "$ALICE_RPC")"
  END_TS="$(date +%s)"

  if (( END_BLOCK > START_BLOCK )); then BLOCK_PROGRESS_PASS=1; fi
  if [[ -n "$START_FINALIZED" && -n "$END_FINALIZED" && "$START_FINALIZED" != "$END_FINALIZED" ]]; then FINALITY_PROGRESS_PASS=1; fi

  if grep -Eqi 'panic|thread .* panicked' "$LOG_DIR"/*.log 2>/dev/null; then
    PANIC_FLAG=1
  fi
  if grep -Eqi 'corrupt|corruption|rocksdb.*error|database.*corrupt' "$LOG_DIR"/*.log 2>/dev/null; then
    DB_CORRUPTION_FLAG=1
  fi

  local blockers='[]'
  local add_blocker
  add_blocker() {
    local msg="$1"
    blockers="$($JQ_BIN -cn --argjson arr "$blockers" --arg msg "$msg" '$arr + [$msg]')"
  }

  if (( RUN_ABORTED_BY_REBOOT )); then
    add_blocker 'HOST REBOOT: run aborted and invalidated for official proof'
  fi

  (( PANIC_FLAG == 0 )) || add_blocker 'panic loop detected in validator logs'
  (( DB_CORRUPTION_FLAG == 0 )) || add_blocker 'database corruption indicators detected in logs'
  (( BLOCK_PROGRESS_PASS == 1 )) || add_blocker 'blocks did not continue advancing'
  (( FINALITY_PROGRESS_PASS == 1 )) || add_blocker 'finality did not continue advancing'
  (( RESTART_DRILL_DONE == 1 && RESTART_DRILL_PASS == 1 )) || add_blocker 'validator restart drill failed'
  (( SETTLEMENT_FAILS == 0 )) || add_blocker 'settlement smoke had failures'
  (( INVARIANT_FAILS == 0 )) || add_blocker 'one or more invariant snapshots failed'
  (( BRIDGES_DISABLED_PASS == 1 )) || add_blocker 'external bridges not disabled'
  (( RUNTIME_VERSION_PASS == 1 )) || add_blocker 'runtime specVersion is below 10'
  (( PEER_MIN_PASS == 1 )) || add_blocker 'peer connectivity did not reach minimum threshold'

  local overall='FAIL'
  local short_status='FAIL'
  local run72_status='PENDING'
  local blockers_count
  blockers_count="$($JQ_BIN -r 'length' <<<"$blockers")"

  if (( DURATION_SECONDS < 259200 )); then
    run72_status='PENDING'
    if (( blockers_count == 0 )); then
      short_status='PASS'
      overall='RC5_SHORT_SHAKEDOWN: PASS (RC5_72H: PENDING)'
    else
      short_status='FAIL'
      overall='RC5_SHORT_SHAKEDOWN: FAIL (RC5_72H: PENDING)'
    fi
  else
    if (( blockers_count == 0 )); then
      short_status='PASS'
      run72_status='PASS'
      overall='PASS'
    else
      short_status='FAIL'
      run72_status='FAIL'
      overall='FAIL'
    fi
  fi

  BLOCKERS_JSON="$blockers" \
    START_TS_ISO="$(date -u -d "@$START_TS" +%Y-%m-%dT%H:%M:%SZ)" \
    END_TS_ISO="$(date -u -d "@$END_TS" +%Y-%m-%dT%H:%M:%SZ)" \
    SUMMARY_FILE="$SUMMARY_FILE" \
    OVERALL="$overall" \
    SHORT_STATUS="$short_status" \
    RUN72_STATUS="$run72_status" \
    DURATION_SECONDS="$DURATION_SECONDS" \
    START_BLOCK="$START_BLOCK" \
    END_BLOCK="$END_BLOCK" \
    START_FINALIZED="$START_FINALIZED" \
    END_FINALIZED="$END_FINALIZED" \
    RESTART_DRILL_DONE="$RESTART_DRILL_DONE" \
    RESTART_DRILL_PASS="$RESTART_DRILL_PASS" \
    SETTLEMENT_FAILS="$SETTLEMENT_FAILS" \
    INVARIANT_FAILS="$INVARIANT_FAILS" \
    PANIC_FLAG="$PANIC_FLAG" \
    DB_CORRUPTION_FLAG="$DB_CORRUPTION_FLAG" \
    BRIDGES_DISABLED_PASS="$BRIDGES_DISABLED_PASS" \
    RUN_BOOT_ID="$RUN_BOOT_ID" \
    CURRENT_BOOT_ID="$(capture_boot_id)" \
    RUN_ABORTED_BY_REBOOT="$RUN_ABORTED_BY_REBOOT" \
    python3 - <<'PY'
import json
import os

summary = {
    "verdict": os.environ["OVERALL"],
    "RC5_SHORT_SHAKEDOWN": os.environ["SHORT_STATUS"],
    "RC5_72H": os.environ["RUN72_STATUS"],
    "duration_seconds": int(os.environ["DURATION_SECONDS"]),
    "run_window": {
        "start": os.environ["START_TS_ISO"],
        "end": os.environ["END_TS_ISO"],
    },
    "liveness": {
        "start_block": int(os.environ["START_BLOCK"]),
        "end_block": int(os.environ["END_BLOCK"]),
        "start_finalized": os.environ["START_FINALIZED"],
        "end_finalized": os.environ["END_FINALIZED"],
    },
    "checks": {
        "restart_drill_done": int(os.environ["RESTART_DRILL_DONE"]),
        "restart_drill_pass": int(os.environ["RESTART_DRILL_PASS"]),
        "settlement_failures": int(os.environ["SETTLEMENT_FAILS"]),
        "invariant_failures": int(os.environ["INVARIANT_FAILS"]),
        "panic_flag": int(os.environ["PANIC_FLAG"]),
        "db_corruption_flag": int(os.environ["DB_CORRUPTION_FLAG"]),
        "bridges_disabled_initial": int(os.environ["BRIDGES_DISABLED_PASS"]),
    },
    "host_integrity": {
        "run_boot_id": os.environ.get("RUN_BOOT_ID", "unknown"),
        "current_boot_id": os.environ.get("CURRENT_BOOT_ID", "unknown"),
        "reboot_detected": int(os.environ.get("RUN_ABORTED_BY_REBOOT", "0")),
        "abort_reason": "host reboot during 72h window" if int(os.environ.get("RUN_ABORTED_BY_REBOOT", "0")) else None,
    },
    "blockers": json.loads(os.environ.get("BLOCKERS_JSON", "[]")),
}

with open(os.environ["SUMMARY_FILE"], "w", encoding="utf-8") as handle:
    json.dump(summary, handle, indent=2)
    handle.write("\n")
PY

  write_report "$overall" "$short_status" "$run72_status" "$blockers"

  log "RC5 complete. Summary: $SUMMARY_FILE"
  log "Report: $REPORT_FILE"
  if (( blockers_count != 0 )); then
    return 1
  fi
  return 0
}

main "$@"
