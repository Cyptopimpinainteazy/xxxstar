#!/usr/bin/env bash
set -euo pipefail

# Testnet-only local 7-validator launcher.
# Seeds are exposed here by design for local testing. Do NOT use these for mainnet.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
NODE_BIN_DEFAULT="$ROOT_DIR/target/release/x3-chain-node"
CHAIN_SPEC_DEFAULT="$ROOT_DIR/deployment/chain-specs/x3-testnet-raw.json"
CHAIN_SPEC_PLAIN_DEFAULT="$ROOT_DIR/deployment/chain-specs/x3-testnet-plain.json"
BASE_DIR_DEFAULT="$HOME/.local/share/x3/testnet-local"
LOG_DIR_DEFAULT="$ROOT_DIR/logs/testnet"
SUBKEY_BIN_DEFAULT="${SUBKEY_BIN_DEFAULT:-/home/lojak/.cargo/bin/subkey}"

NODE_BIN="${NODE_BIN:-$NODE_BIN_DEFAULT}"
CHAIN_SPEC="${CHAIN_SPEC:-$CHAIN_SPEC_DEFAULT}"
CHAIN_SPEC_PLAIN="${CHAIN_SPEC_PLAIN:-$CHAIN_SPEC_PLAIN_DEFAULT}"
BASE_DIR="${BASE_DIR:-$BASE_DIR_DEFAULT}"
LOG_DIR="${LOG_DIR:-$LOG_DIR_DEFAULT}"
PID_DIR="${PID_DIR:-}"
CHAIN_SPEC_RUN="${CHAIN_SPEC_RUN:-}"
KEYSTORE_PASSWORD_FILE="${KEYSTORE_PASSWORD_FILE:-}"
COUNT="${COUNT:-7}"
LISTEN_IP="${LISTEN_IP:-127.0.0.1}"
PROMETHEUS="${PROMETHEUS:-0}"
NO_MDNS="${NO_MDNS:-1}"
NO_TELEMETRY="${NO_TELEMETRY:-1}"
DISABLE_LOG_COLOR="${DISABLE_LOG_COLOR:-1}"
NODE_NICE="${NODE_NICE:-}"
NODE_DB_CACHE_MIB="${NODE_DB_CACHE_MIB:-}"
SUBKEY_BIN="${SUBKEY_BIN:-$SUBKEY_BIN_DEFAULT}"

WIPE_BASE_DIR=0

usage() {
  cat <<EOF
Usage: $(basename "$0") [--wipe] [--base-dir PATH] [--chain-spec PATH] [--node-bin PATH] [--log-dir PATH]

Testnet-only local 7-validator launcher.

Options:
  --wipe              Stop existing nodes (via PID files) and wipe base dir before starting.
  --base-dir PATH     Override BASE_DIR (default: ${BASE_DIR_DEFAULT})
  --chain-spec PATH   Override CHAIN_SPEC (default: ${CHAIN_SPEC_DEFAULT})
  --node-bin PATH     Override NODE_BIN (default: ${NODE_BIN_DEFAULT})
  --log-dir PATH      Override LOG_DIR (default: ${LOG_DIR_DEFAULT})
  -h, --help          Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --wipe)
      WIPE_BASE_DIR=1
      shift
      ;;
    --base-dir)
      BASE_DIR="${2:-}"
      shift 2
      ;;
    --chain-spec)
      CHAIN_SPEC="${2:-}"
      shift 2
      ;;
    --node-bin)
      NODE_BIN="${2:-}"
      shift 2
      ;;
    --log-dir)
      LOG_DIR="${2:-}"
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

PID_DIR="${PID_DIR:-$BASE_DIR/pids}"
CHAIN_SPEC_RUN="${CHAIN_SPEC_RUN:-$BASE_DIR/chain-spec.json}"

stop_nodes() {
  if [[ ! -d "$PID_DIR" ]]; then
    return 0
  fi

  shopt -s nullglob
  local pids=("$PID_DIR"/node-*.pid)
  shopt -u nullglob
  if [[ ${#pids[@]} -eq 0 ]]; then
    return 0
  fi

  for pid_file in "${pids[@]}"; do
    local pid
    pid="$(cat "$pid_file" 2>/dev/null || true)"
    if [[ -n "$pid" ]]; then
      kill "$pid" 2>/dev/null || true
    fi
  done

  sleep 1
}

wipe_base_dir() {
  local dir="$1"
  if [[ -z "$dir" || "$dir" == "/" ]]; then
    echo "Refusing to wipe BASE_DIR='$dir'"
    exit 1
  fi
  rm -rf "$dir"
}

if [[ "$WIPE_BASE_DIR" -eq 1 ]]; then
  stop_nodes
  wipe_base_dir "$BASE_DIR"
fi

mkdir -p "$BASE_DIR" "$LOG_DIR" "$PID_DIR"

if [[ ! -x "$NODE_BIN" ]]; then
  echo "Node binary not found: $NODE_BIN"
  exit 1
fi

if ! command -v "$SUBKEY_BIN" >/dev/null 2>&1; then
  if command -v subkey >/dev/null 2>&1; then
    SUBKEY_BIN="$(command -v subkey)"
  else
    echo "subkey not found in PATH and SUBKEY_BIN is not executable: $SUBKEY_BIN"
    exit 1
  fi
fi

if [[ ! -f "$CHAIN_SPEC" ]]; then
  echo "Chain spec not found: $CHAIN_SPEC"
  exit 1
fi

ensure_raw_spec() {
  if [[ -s "$CHAIN_SPEC" ]]; then
    return 0
  fi

  if [[ ! -x "$NODE_BIN" ]]; then
    echo "Node binary not found for build-spec: $NODE_BIN"
    exit 1
  fi

  if [[ ! -f "$CHAIN_SPEC_PLAIN" ]]; then
    echo "Plain chain spec not found: $CHAIN_SPEC_PLAIN"
    exit 1
  fi

  echo "Raw chain spec is empty. Regenerating from $CHAIN_SPEC_PLAIN..."
  tmp_spec="${CHAIN_SPEC}.tmp"
  "$NODE_BIN" build-spec --chain "$CHAIN_SPEC_PLAIN" --raw --disable-log-color > "$tmp_spec" 2>/dev/null
  TMP_SPEC="$tmp_spec" CHAIN_SPEC="$CHAIN_SPEC" python - <<'PY'
import json
import os
from pathlib import Path

src = Path(os.environ["TMP_SPEC"])
dst = Path(os.environ["CHAIN_SPEC"])
text = src.read_text()
start = text.find("{")
end = text.rfind("}")
if start == -1 or end == -1 or end <= start:
    raise SystemExit("Failed to locate JSON object in build-spec output")
json_text = text[start:end+1]
json.loads(json_text)
dst.write_text(json_text)
print(f"Regenerated raw spec: {dst}")
PY
  rm -f "$tmp_spec"
}

ensure_raw_spec

# Sanitize chain spec to avoid conflicting bootnodes embedded in raw specs.
CHAIN_SPEC="${CHAIN_SPEC}" CHAIN_SPEC_RUN="${CHAIN_SPEC_RUN}" python - <<'PY'
import os
import re
from pathlib import Path

src = Path(os.environ["CHAIN_SPEC"])
dst = Path(os.environ["CHAIN_SPEC_RUN"])

text = src.read_text()
start = text.find("{")
if start == -1:
    raise SystemExit(f"Invalid chain spec (no JSON object found): {src}")
text = text[start:]

pattern = re.compile(r'("bootNodes"\s*:\s*\[).*?(\])', re.S)
text, count = pattern.subn(r'\1\2', text, count=1)
if count == 0:
    raise SystemExit("bootNodes key not found in chain spec")

dst.write_text(text)
print(f"Using sanitized chain spec: {dst}")
PY

DEV_SEEDS=(
  "//Alice"
  "//Bob"
  "//Charlie"
  "//Dave"
  "//Eve"
  "//Ferdie"
  "//One"
)

CHAIN_ID="$(CHAIN_SPEC_RUN="$CHAIN_SPEC_RUN" python - <<'PY'
import json
import os
from pathlib import Path

spec = json.loads(Path(os.environ["CHAIN_SPEC_RUN"]).read_text())
print(spec.get("id", ""))
PY
)"

if [[ -z "$CHAIN_ID" ]]; then
  echo "Failed to read chain id from ${CHAIN_SPEC_RUN}"
  exit 1
fi

if [[ "$COUNT" -lt 1 || "$COUNT" -gt 7 ]]; then
  echo "COUNT must be between 1 and 7 (got: ${COUNT})"
  exit 1
fi

if [[ "$COUNT" -lt 5 ]]; then
  echo "WARNING: COUNT<5 means GRANDPA finality will stall (7 authorities in genesis)."
  echo "         Anything waiting for finalized blocks (e.g. submit-remark.js) may hang."
fi

insert_keys() {
  local base_path="$1"
  local suri="$2"
  local keystore_dir="${base_path}/chains/${CHAIN_ID}/keystore"

  mkdir -p "$keystore_dir"

  local aura_pub
  local gran_pub
  aura_pub=$("$SUBKEY_BIN" inspect --scheme sr25519 "$suri" | awk '/Public key \(hex\):/ {print $4}')
  gran_pub=$("$SUBKEY_BIN" inspect --scheme ed25519 "$suri" | awk '/Public key \(hex\):/ {print $4}')

  if [[ -z "$aura_pub" || -z "$gran_pub" ]]; then
    echo "Failed to derive public keys for ${suri}"
    exit 1
  fi

  local aura_file="61757261${aura_pub#0x}"
  local gran_file="6772616e${gran_pub#0x}"

  SURI="$suri" OUT="$keystore_dir/$aura_file" python - <<'PY'
import json
import os
from pathlib import Path

path = Path(os.environ["OUT"])
path.write_text(json.dumps(os.environ["SURI"]))
path.chmod(0o600)
PY

  SURI="$suri" OUT="$keystore_dir/$gran_file" python - <<'PY'
import json
import os
from pathlib import Path

path = Path(os.environ["OUT"])
path.write_text(json.dumps(os.environ["SURI"]))
path.chmod(0o600)
PY
}

validate_keys() {
  local base_path="$1"
  local suri="$2"
  local keystore_dir="${base_path}/chains/${CHAIN_ID}/keystore"

  local aura_pub
  local gran_pub
  aura_pub=$("$SUBKEY_BIN" inspect --scheme sr25519 "$suri" | awk '/Public key \(hex\):/ {print $4}')
  gran_pub=$("$SUBKEY_BIN" inspect --scheme ed25519 "$suri" | awk '/Public key \(hex\):/ {print $4}')

  local aura_file="${keystore_dir}/61757261${aura_pub#0x}"
  local gran_file="${keystore_dir}/6772616e${gran_pub#0x}"

  if [[ ! -s "$aura_file" || ! -s "$gran_file" ]]; then
    echo "Missing keystore files for ${suri} in ${keystore_dir}"
    exit 1
  fi
}

wait_for_rpc() {
  local rpc_port="$1"
  for _ in $(seq 1 60); do
    if curl -s -H "Content-Type: application/json" \
      -d '{"jsonrpc":"2.0","id":1,"method":"system_health","params":[]}' \
      "http://127.0.0.1:${rpc_port}" | grep -q '"isSyncing"'; then
      return 0
    fi
    sleep 1
  done
  echo "RPC not ready on port ${rpc_port}"
  return 1
}

start_node() {
  local i="$1"
  local bootnode="${2:-}"

  local p2p_port=$((30333 + i - 1))
  local rpc_port=$((9944 + i - 1))
  local prom_port=$((9615 + i - 1))
  local base_path="${BASE_DIR}/node-${i}"
  local name="x3-testnet-node-$(printf '%02d' "$i")"
  local dev_seed="${DEV_SEEDS[$((i-1))]}"
  local log_file="${LOG_DIR}/node-${i}.log"

  mkdir -p "$base_path"

  local boot_args=()
  if [[ -n "$bootnode" ]]; then
    boot_args=(--bootnodes "$bootnode")
  fi

  insert_keys "$base_path" "$dev_seed"
  validate_keys "$base_path" "$dev_seed"

  local password_args=()
  if [[ -n "$KEYSTORE_PASSWORD_FILE" ]]; then
    password_args=(--password-filename "$KEYSTORE_PASSWORD_FILE")
  fi

  local log_args=()
  if [[ "$DISABLE_LOG_COLOR" == "1" ]]; then
    log_args+=(--disable-log-color)
  fi

  local net_args=(
    --listen-addr "/ip4/${LISTEN_IP}/tcp/${p2p_port}"
  )
  if [[ "$NO_MDNS" == "1" ]]; then
    net_args+=(--no-mdns)
  fi
  if [[ "$NO_TELEMETRY" == "1" ]]; then
    net_args+=(--no-telemetry)
  fi

  local prom_args=()
  if [[ "$PROMETHEUS" == "1" ]]; then
    prom_args+=(--prometheus-port "$prom_port")
  else
    prom_args+=(--no-prometheus)
  fi

  local db_args=()
  if [[ -n "$NODE_DB_CACHE_MIB" ]]; then
    db_args+=(--db-cache "$NODE_DB_CACHE_MIB")
  fi

  local nice_args=()
  if [[ -n "$NODE_NICE" ]]; then
    nice_args=(nice -n "$NODE_NICE")
  fi

  nohup "${nice_args[@]}" "$NODE_BIN" \
    --chain "$CHAIN_SPEC_RUN" \
    --base-path "$base_path" \
    --name "$name" \
    --rpc-port "$rpc_port" \
    --rpc-methods=Unsafe \
    --rpc-cors=all \
    "${log_args[@]}" \
    "${net_args[@]}" \
    "${prom_args[@]}" \
    "${db_args[@]}" \
    "${password_args[@]}" \
    --validator \
    --force-authoring \
    --allow-private-ip \
    "${boot_args[@]}" \
    > "$log_file" 2>&1 &

  echo $! > "${PID_DIR}/node-${i}.pid"
  echo "Started ${name} (p2p=${p2p_port}, rpc=${rpc_port}, prom=${prom_port})"

  wait_for_rpc "$rpc_port"
  echo "Node ${name} ready"
}

echo "Starting node 1 (bootnode)..."
start_node 1

peer_id=""
for _ in $(seq 1 60); do
  peer_id="$(curl -s -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":1,"method":"system_localPeerId","params":[]}' \
    "http://127.0.0.1:9944" | python -c 'import json,sys; print(json.load(sys.stdin).get("result",""))' 2>/dev/null || true)"
  if [[ -n "$peer_id" ]]; then
    break
  fi
  sleep 1
done

if [[ -z "$peer_id" ]]; then
  echo "Failed to detect node-1 peer ID via RPC (system_localPeerId)"
  exit 1
fi

BOOTNODE="/ip4/${LISTEN_IP}/tcp/30333/p2p/${peer_id}"
echo "Bootnode: ${BOOTNODE}"

for i in $(seq 2 "$COUNT"); do
  echo "Starting node ${i}..."
  start_node "$i" "$BOOTNODE"
done

echo "All ${COUNT} validators started."
echo "Logs: ${LOG_DIR}/node-*.log"
echo "PIDs: ${PID_DIR}/node-*.pid"
