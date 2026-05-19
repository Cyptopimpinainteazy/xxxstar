#!/usr/bin/env bash
set -euo pipefail

BASE_RPC_PORT="${BASE_RPC_PORT:-9944}"
COUNT="${COUNT:-7}"
MIN_PEERS="${MIN_PEERS:-3}"
PEER_TIMEOUT_SEC="${PEER_TIMEOUT_SEC:-120}"
CHECK_PEERS="${CHECK_PEERS:-0}"
CHECK_FINALITY="${CHECK_FINALITY:-0}"
FINALITY_WINDOW_SEC="${FINALITY_WINDOW_SEC:-12}"
FINALITY_TIMEOUT_SEC="${FINALITY_TIMEOUT_SEC:-120}"
TARGET_BLOCK_TIME_SEC="${TARGET_BLOCK_TIME_SEC:-6}"
MAX_FINALITY_LAG_MULT="${MAX_FINALITY_LAG_MULT:-2}"
# Absolute cap on acceptable lag between best and finalized (in blocks).
# Set to 0 to disable lag checks and only require finalized head to advance.
MAX_FINALITY_LAG_BLOCKS="${MAX_FINALITY_LAG_BLOCKS:-0}"

usage() {
  cat <<EOF
Usage: $(basename "$0") [--check-peers] [--check-finality] [--min-peers N] [--timeout-sec N]

Options:
  --check-peers            Fail if any node has < min peers within timeout.
  --check-finality         Fail if finalized blocks do not progress within window.
  --min-peers N            Minimum peer count (default: ${MIN_PEERS}).
  --timeout-sec N          Peer check timeout in seconds (default: ${PEER_TIMEOUT_SEC}).
  --finality-window-sec N  Seconds between finality samples (default: ${FINALITY_WINDOW_SEC}).
  --finality-timeout-sec N Seconds to wait for finality progress (default: ${FINALITY_TIMEOUT_SEC}).
  --target-block-time-sec N Target block time in seconds (default: ${TARGET_BLOCK_TIME_SEC}).
  --max-finality-lag-mult N Max finalized lag in blocks as multiple of target block time (default: ${MAX_FINALITY_LAG_MULT}).
  --max-finality-lag-blocks N Absolute max finalized lag in blocks; 0 disables lag check (default: ${MAX_FINALITY_LAG_BLOCKS}).
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --check-peers)
      CHECK_PEERS=1
      shift
      ;;
    --check-finality)
      CHECK_FINALITY=1
      shift
      ;;
    --min-peers)
      MIN_PEERS="${2:-}"
      shift 2
      ;;
    --timeout-sec)
      PEER_TIMEOUT_SEC="${2:-}"
      shift 2
      ;;
    --finality-window-sec)
      FINALITY_WINDOW_SEC="${2:-}"
      shift 2
      ;;
    --finality-timeout-sec)
      FINALITY_TIMEOUT_SEC="${2:-}"
      shift 2
      ;;
    --target-block-time-sec)
      TARGET_BLOCK_TIME_SEC="${2:-}"
      shift 2
      ;;
    --max-finality-lag-mult)
      MAX_FINALITY_LAG_MULT="${2:-}"
      shift 2
      ;;
    --max-finality-lag-blocks)
      MAX_FINALITY_LAG_BLOCKS="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1"
      usage
      exit 2
      ;;
  esac
done

printf "%-5s %-7s %-9s %-6s %-12s %s\n" "NODE" "RPC" "SYNCING" "PEERS" "BLOCK" "HASH"

for i in $(seq 0 $((COUNT - 1))); do
  port=$((BASE_RPC_PORT + i))
  node=$((i + 1))
  url="http://127.0.0.1:${port}"

  health=$(curl -s --max-time 1 -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":1,"method":"system_health","params":[]}' \
    "$url" || true)

  if [[ -z "$health" ]]; then
    printf "%-5s %-7s %-9s %-6s %-12s %s\n" "$node" "$port" "DOWN" "-" "-" "-"
    continue
  fi

  header=$(curl -s --max-time 1 -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":1,"method":"chain_getHeader","params":[]}' \
    "$url" || true)

  hash=$(curl -s --max-time 1 -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":1,"method":"chain_getBlockHash","params":[]}' \
    "$url" || true)

  NODE="$node" PORT="$port" HEALTH="$health" HEADER="$header" HASH="$hash" python - <<'PY'
import json
import os
import sys

health_raw = os.environ["HEALTH"]
header_raw = os.environ.get("HEADER", "")
hash_raw = os.environ.get("HASH", "")

def parse_json(raw: str):
    try:
        return json.loads(raw)
    except Exception:
        return None

try:
    health = json.loads(health_raw)
except Exception:
    if os.environ.get("DEBUG") == "1":
        print("DEBUG health:", health_raw)
    print("DOWN")
    sys.exit(0)

if "result" not in health:
    if os.environ.get("DEBUG") == "1":
        print("DEBUG health:", health_raw)
    print("DOWN")
    sys.exit(0)

is_syncing = health["result"].get("isSyncing", True)
peers = health["result"].get("peers", 0)

block = health["result"].get("bestBlock", None)
if block is None:
    header = parse_json(header_raw)
    if header and "result" in header and header["result"]:
        num = header["result"].get("number")
        try:
            if isinstance(num, str) and num.startswith("0x"):
                block = int(num, 16)
            else:
                block = int(num)
        except Exception:
            block = "-"
    else:
        block = "-"

hash_ = "-"
block_hash = parse_json(hash_raw)
if block_hash and "result" in block_hash and block_hash["result"]:
    hash_ = block_hash["result"]

node = os.environ.get("NODE")
port = os.environ.get("PORT")

print(f"{node:<5} {port:<7} {str(is_syncing):<9} {peers:<6} {block:<12} {hash_}")
PY
done

jsonrpc() {
  local url="$1"
  local method="$2"
  local params="${3:-[]}"
  curl -s --max-time 1 -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"${method}\",\"params\":${params}}" \
    "$url" || true
}

peer_check() {
  local start_ts
  start_ts="$(date +%s)"
  while true; do
    local ok=1
    for i in $(seq 0 $((COUNT - 1))); do
      local port=$((BASE_RPC_PORT + i))
      local url="http://127.0.0.1:${port}"
      local health
      health="$(jsonrpc "$url" "system_health")"
      if [[ -z "$health" ]]; then
        ok=0
        continue
      fi
      MIN_PEERS="$MIN_PEERS" HEALTH="$health" python - <<'PY'
import json
import os
import sys
try:
    health = json.loads(os.environ["HEALTH"])
    peers = int(health.get("result", {}).get("peers", 0))
except Exception:
    peers = 0
min_peers = int(os.environ["MIN_PEERS"])
sys.exit(0 if peers >= min_peers else 2)
PY
      if [[ $? -ne 0 ]]; then
        ok=0
      fi
    done
    if [[ "$ok" -eq 1 ]]; then
      return 0
    fi
    local now
    now="$(date +%s)"
    if (( now - start_ts >= PEER_TIMEOUT_SEC )); then
      return 1
    fi
    sleep 2
  done
}

finality_check() {
  local max_lag_blocks
  max_lag_blocks="$MAX_FINALITY_LAG_BLOCKS"

  local start_ts
  start_ts="$(date +%s)"

  while true; do
    local sample1=()
    local sample2=()
    for i in $(seq 0 $((COUNT - 1))); do
      local port=$((BASE_RPC_PORT + i))
      local url="http://127.0.0.1:${port}"
      local finalized_hash
      finalized_hash="$(jsonrpc "$url" "chain_getFinalizedHead")"
      local header
      header="$(jsonrpc "$url" "chain_getHeader")"
      sample1+=("${finalized_hash}|||${header}")
    done

    sleep "$FINALITY_WINDOW_SEC"

    for i in $(seq 0 $((COUNT - 1))); do
      local port=$((BASE_RPC_PORT + i))
      local url="http://127.0.0.1:${port}"
      local finalized_hash
      finalized_hash="$(jsonrpc "$url" "chain_getFinalizedHead")"
      local header
      header="$(jsonrpc "$url" "chain_getHeader")"
      sample2+=("${finalized_hash}|||${header}")
    done

    if SAMPLE1="${sample1[*]}" SAMPLE2="${sample2[*]}" MAX_LAG="$max_lag_blocks" BASE_RPC_PORT="$BASE_RPC_PORT" python - <<'PY'
import json
import os
import sys

def parse_num(header_json: str) -> int:
    try:
        obj = json.loads(header_json)
        num = obj.get("result", {}).get("number")
        if isinstance(num, str) and num.startswith("0x"):
            return int(num, 16)
        return int(num)
    except Exception:
        return -1

def parse_finalized(finalized_json: str) -> str:
    try:
        obj = json.loads(finalized_json)
        return obj.get("result", "")
    except Exception:
        return ""

def finalized_number(rpc_url: str, finalized_hash: str) -> int:
    if not finalized_hash:
        return -1
    try:
        import urllib.request
        req = json.dumps({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "chain_getHeader",
            "params": [finalized_hash],
        }).encode()
        request = urllib.request.Request(
            rpc_url,
            data=req,
            headers={"Content-Type": "application/json"},
        )
        resp = urllib.request.urlopen(request, timeout=1)
        obj = json.loads(resp.read().decode())
        num = obj.get("result", {}).get("number")
        if isinstance(num, str) and num.startswith("0x"):
            return int(num, 16)
        return int(num)
    except Exception:
        return -1

sample1 = os.environ["SAMPLE1"].split()
sample2 = os.environ["SAMPLE2"].split()
max_lag = int(os.environ["MAX_LAG"])

if len(sample1) != len(sample2):
    sys.exit(2)

ok = True
for idx, (row1, row2) in enumerate(zip(sample1, sample2)):
    try:
        f1_raw, h1_raw = row1.split("|||", 1)
        f2_raw, h2_raw = row2.split("|||", 1)
    except ValueError:
        ok = False
        continue
    best_2 = parse_num(h2_raw)
    finalized_hash_1 = parse_finalized(f1_raw)
    finalized_hash_2 = parse_finalized(f2_raw)
    if best_2 < 0 or not finalized_hash_2:
        ok = False
        continue
    base_port = int(os.environ.get("BASE_RPC_PORT", "9944"))
    port = base_port + idx
    rpc_url = f"http://127.0.0.1:{port}"
    fin_1 = finalized_number(rpc_url, finalized_hash_1)
    fin_2 = finalized_number(rpc_url, finalized_hash_2)
    if fin_1 < 0 or fin_2 < 0:
        ok = False
        continue
    if fin_2 <= fin_1:
        ok = False
        continue
    if max_lag > 0:
        lag = best_2 - fin_2
        if lag > max_lag:
            ok = False

sys.exit(0 if ok else 3)
PY
    then
      return 0
    fi

    local now
    now="$(date +%s)"
    if (( now - start_ts >= FINALITY_TIMEOUT_SEC )); then
      return 1
    fi
  done
}

if [[ "$CHECK_PEERS" == "1" ]]; then
  if peer_check; then
    echo "Peer check OK (>=${MIN_PEERS} peers within ${PEER_TIMEOUT_SEC}s)"
  else
    echo "Peer check FAILED (>=${MIN_PEERS} peers within ${PEER_TIMEOUT_SEC}s)"
    exit 1
  fi
fi

if [[ "$CHECK_FINALITY" == "1" ]]; then
  if finality_check; then
    if [[ "${MAX_FINALITY_LAG_BLOCKS}" == "0" ]]; then
      echo "Finality check OK (progress within ${FINALITY_WINDOW_SEC}s, lag check disabled)"
    else
      echo "Finality check OK (progress within ${FINALITY_WINDOW_SEC}s, lag <= ${MAX_FINALITY_LAG_BLOCKS} blocks)"
    fi
  else
    echo "Finality check FAILED"
    exit 1
  fi
fi
