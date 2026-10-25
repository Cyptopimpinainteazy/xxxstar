#!/usr/bin/env bash
set -euo pipefail

BASE_RPC_PORT="${BASE_RPC_PORT:-9944}"
COUNT="${COUNT:-7}"
MIN_PEERS="${MIN_PEERS:-3}"
PEER_TIMEOUT_SEC="${PEER_TIMEOUT_SEC:-120}"
CHECK_PEERS="${CHECK_PEERS:-0}"
CHECK_FINALITY="${CHECK_FINALITY:-0}"
FINALITY_WINDOW_SEC="${FINALITY_WINDOW_SEC:-12}"
TARGET_BLOCK_TIME_SEC="${TARGET_BLOCK_TIME_SEC:-6}"
MAX_FINALITY_LAG_MULT="${MAX_FINALITY_LAG_MULT:-2}"

usage() {
  cat <<EOF
Usage: $(basename "$0") [--check-peers] [--check-finality] [--min-peers N] [--timeout-sec N]

Options:
  --check-peers            Fail if any node has < min peers within timeout.
  --check-finality         Fail if finalized blocks do not progress within window.
  --min-peers N            Minimum peer count (default: ${MIN_PEERS}).
  --timeout-sec N          Peer check timeout in seconds (default: ${PEER_TIMEOUT_SEC}).
  --finality-window-sec N  Seconds between finality samples (default: ${FINALITY_WINDOW_SEC}).
  --target-block-time-sec N Target block time in seconds (default: ${TARGET_BLOCK_TIME_SEC}).
  --max-finality-lag-mult N Max finalized lag in blocks as multiple of target block time (default: ${MAX_FINALITY_LAG_MULT}).
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
    --target-block-time-sec)
      TARGET_BLOCK_TIME_SEC="${2:-}"
      shift 2
      ;;
    --max-finality-lag-mult)
      MAX_FINALITY_LAG_MULT="${2:-}"
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

  NODE="$node" PORT="$port" HEALTH="$health" HEADER="$header" HASH="$hash" python3 - <<'PY'
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
      MIN_PEERS="$MIN_PEERS" HEALTH="$health" python3 - <<'PY'
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
  COUNT="$COUNT" FINALITY_WINDOW_SEC="$FINALITY_WINDOW_SEC" \
    MAX_LAG="$MAX_FINALITY_LAG_MULT" BASE_RPC_PORT="$BASE_RPC_PORT" python3 - <<'PY'
import json
import math
import os
import re
import sys
import time
import urllib.request


def rpc(port, method, params=None):
    request = urllib.request.Request(
        f"http://127.0.0.1:{port}",
        data=json.dumps({"jsonrpc": "2.0", "id": 1, "method": method,
                         "params": params or []}).encode(),
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(request, timeout=2) as response:
        obj = json.load(response)
    if (not isinstance(obj, dict) or obj.get("jsonrpc") != "2.0"
            or type(obj.get("id")) is not int or obj["id"] != 1
            or "error" in obj or "result" not in obj):
        raise ValueError(f"RPC {port} {method}: invalid or failed response")
    return obj["result"]


def block_hash(port, method, params=None):
    value = rpc(port, method, params)
    if not isinstance(value, str) or not re.fullmatch(r"0x[0-9a-fA-F]{64}", value):
        raise ValueError(f"RPC {port} {method}: invalid block hash")
    return value.lower()


def number(port, params=None):
    header = rpc(port, "chain_getHeader", params)
    value = header.get("number") if isinstance(header, dict) else None
    if not isinstance(value, str) or not re.fullmatch(r"0x[0-9a-fA-F]+", value):
        raise ValueError(f"RPC {port}: invalid header number")
    return int(value, 16)


def sample(ports):
    rows = []
    for port in ports:
        genesis = block_hash(port, "chain_getBlockHash", [0])
        finalized = block_hash(port, "chain_getFinalizedHead")
        height = number(port, [finalized])
        best = number(port)
        if height <= 0 or height > best:
            raise ValueError(f"RPC {port}: finalized height {height} inconsistent with best {best}")
        if block_hash(port, "chain_getBlockHash", [height]) != finalized:
            raise ValueError(f"RPC {port}: finalized head is not canonical at height {height}")
        rows.append((genesis, finalized, height, best))
    return rows


try:
    count = int(os.environ["COUNT"])
    base = int(os.environ["BASE_RPC_PORT"])
    window = float(os.environ["FINALITY_WINDOW_SEC"])
    max_lag = int(os.environ["MAX_LAG"])
    if count < 1 or base < 1 or base + count - 1 > 65535 or not math.isfinite(window) or window < 0 or max_lag < 0:
        raise ValueError("invalid finality configuration")
    ports = list(range(base, base + count))
    before = sample(ports)
    time.sleep(window)
    after = sample(ports)
    if len({row[0] for row in before + after}) != 1:
        raise ValueError("validators do not share a stable genesis hash")
    common_height = min(row[2] for row in after)
    common_hashes = set()
    for port, previous, current in zip(ports, before, after):
        if current[2] <= previous[2]:
            raise ValueError(f"RPC {port}: finalized height did not progress")
        if current[3] - current[2] > max_lag:
            raise ValueError(f"RPC {port}: finality lag exceeds {max_lag} blocks")
        # Progress must extend the previously finalized block, not replace it.
        if block_hash(port, "chain_getBlockHash", [previous[2]]) != previous[1]:
            raise ValueError(f"RPC {port}: previously finalized block changed")
        common_hashes.add(block_hash(port, "chain_getBlockHash", [common_height]))
    if len(common_hashes) != 1:
        raise ValueError(f"validators disagree at finalized height {common_height}")
    print(f"Shared finalized block {common_height}: {next(iter(common_hashes))}")
except (ValueError, OSError, TypeError, KeyError) as error:
    print(f"Finality evidence rejected: {error}", file=sys.stderr)
    sys.exit(3)
PY
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
    echo "Finality check OK (progress within ${FINALITY_WINDOW_SEC}s, lag <= ${MAX_FINALITY_LAG_MULT} blocks)"
  else
    echo "Finality check FAILED"
    exit 1
  fi
fi
