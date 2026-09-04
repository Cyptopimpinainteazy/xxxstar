#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# x3_testnet_up.sh — Production testnet 7-validator launcher
#
# Launches 7 validator nodes using the testnet feature-flagged binary.
# Handles key injection, chain spec sanitization, and health checks.
#
# Usage:
#   ./scripts/testnet/x3_testnet_up.sh [--wipe] [--base-dir PATH] [--chain-spec PATH]
#       [--node-bin PATH] [--log-dir PATH] [--count N] [--features testnet]
#
# Environment:
#   NODE_BIN       Path to x3-chain-node binary (default: target/release/x3-chain-node)
#   CHAIN_SPEC     Path to raw chain spec JSON (default: deployment/chain-specs/x3-testnet-raw.json)
#   COUNT          Number of validators (default: 7, max: 7)
#   BUILD_FEATURES  Cargo features to build with (default: "testnet")
# ─────────────────────────────────────────────────────────────────────────────

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
NODE_BIN_DEFAULT="$ROOT_DIR/target/release/x3-chain-node"
CHAIN_SPEC_DEFAULT="$ROOT_DIR/deployment/chain-specs/x3-testnet-raw.json"
CHAIN_SPEC_PLAIN_DEFAULT="$ROOT_DIR/deployment/chain-specs/x3-testnet-plain.json"
BASE_DIR_DEFAULT="$HOME/.local/share/x3/testnet"
LOG_DIR_DEFAULT="$ROOT_DIR/logs/testnet"
SUBKEY_BIN_DEFAULT="${SUBKEY_BIN_DEFAULT:-$(command -v subkey || echo /home/lojak/.cargo/bin/subkey)}"

NODE_BIN="${NODE_BIN:-$NODE_BIN_DEFAULT}"
CHAIN_SPEC="${CHAIN_SPEC:-$CHAIN_SPEC_DEFAULT}"
CHAIN_SPEC_PLAIN="${CHAIN_SPEC_PLAIN:-$CHAIN_SPEC_PLAIN_DEFAULT}"
BASE_DIR="${BASE_DIR:-$BASE_DIR_DEFAULT}"
LOG_DIR="${LOG_DIR:-$LOG_DIR_DEFAULT}"
PID_DIR="${PID_DIR:-}"
CHAIN_SPEC_RUN="${CHAIN_SPEC_RUN:-}"
KEYSTORE_PASSWORD_FILE="${KEYSTORE_PASSWORD_FILE:-}"
COUNT="${COUNT:-7}"
LISTEN_IP="${LISTEN_IP:-0.0.0.0}"
PROMETHEUS="${PROMETHEUS:-1}"
NO_MDNS="${NO_MDNS:-1}"
NO_TELEMETRY="${NO_TELEMETRY:-0}"
DISABLE_LOG_COLOR="${DISABLE_LOG_COLOR:-1}"
NODE_NICE="${NODE_NICE:-}"
NODE_DB_CACHE_MIB="${NODE_DB_CACHE_MIB:-1024}"
SUBKEY_BIN="${SUBKEY_BIN:-$SUBKEY_BIN_DEFAULT}"
BUILD_FEATURES="${BUILD_FEATURES:-testnet}"
SKIP_BUILD="${SKIP_BUILD:-0}"

WIPE_BASE_DIR=0

usage() {
  cat <<EOF
Usage: $(basename "$0") [--wipe] [options]

Testnet 7-validator launcher with bridge-enabled testnet features.

Options:
  --wipe              Stop existing nodes and wipe base dir before starting.
  --base-dir PATH     Override BASE_DIR (default: ${BASE_DIR_DEFAULT})
  --chain-spec PATH   Override CHAIN_SPEC (default: ${CHAIN_SPEC_DEFAULT})
  --node-bin PATH     Override NODE_BIN (default: ${NODE_BIN_DEFAULT})
  --log-dir PATH      Override LOG_DIR (default: ${LOG_DIR_DEFAULT})
  --count N           Number of validators (default: 7, max: 7)
  --skip-build        Skip cargo build step
  -h, --help          Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --wipe) WIPE_BASE_DIR=1; shift ;;
    --base-dir) BASE_DIR="${2:-}"; shift 2 ;;
    --chain-spec) CHAIN_SPEC="${2:-}"; shift 2 ;;
    --node-bin) NODE_BIN="${2:-}"; shift 2 ;;
    --log-dir) LOG_DIR="${2:-}"; shift 2 ;;
    --count) COUNT="${2:-}"; shift 2 ;;
    --skip-build) SKIP_BUILD=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown argument: $1"; usage; exit 2 ;;
  esac
done

PID_DIR="${PID_DIR:-$BASE_DIR/pids}"
CHAIN_SPEC_RUN="${CHAIN_SPEC_RUN:-$BASE_DIR/chain-spec.json}"

# ── Build binary with testnet features ──────────────────────────────────────
if [[ "$SKIP_BUILD" == "0" ]]; then
  echo "[build] Building x3-chain-node with --features ${BUILD_FEATURES}..."
  cd "$ROOT_DIR"
  cargo build --release -p x3-chain-node --features "$BUILD_FEATURES"
  echo "[build] Build complete: ${NODE_BIN}"
fi

if [[ ! -x "$NODE_BIN" ]]; then
  echo "[error] Node binary not found: $NODE_BIN"
  echo "        Build with: cargo build --release -p x3-chain-node --features ${BUILD_FEATURES}"
  exit 1
fi

# Verify binary has testnet feature
if ! "$NODE_BIN" --version > /dev/null 2>&1; then
  echo "[error] Node binary is not executable or broken: $NODE_BIN"
  exit 1
fi

# ── Stop existing nodes ─────────────────────────────────────────────────────
stop_nodes() {
  if [[ ! -d "$PID_DIR" ]]; then return 0; fi
  shopt -s nullglob
  local pids=("$PID_DIR"/node-*.pid)
  shopt -u nullglob
  if [[ ${#pids[@]} -eq 0 ]]; then return 0; fi
  for pid_file in "${pids[@]}"; do
    local pid
    pid="$(cat "$pid_file" 2>/dev/null || true)"
    if [[ -n "$pid" ]]; then
      kill "$pid" 2>/dev/null || true
    fi
  done
  sleep 2
}

wipe_base_dir() {
  local dir="$1"
  if [[ -z "$dir" || "$dir" == "/" ]]; then
    echo "[error] Refusing to wipe BASE_DIR='$dir'"
    exit 1
  fi
  rm -rf "$dir"
}

if [[ "$WIPE_BASE_DIR" -eq 1 ]]; then
  stop_nodes
  wipe_base_dir "$BASE_DIR"
fi

mkdir -p "$BASE_DIR" "$LOG_DIR" "$PID_DIR"

# ── Subkey check ────────────────────────────────────────────────────────────
if ! command -v "$SUBKEY_BIN" >/dev/null 2>&1; then
  if command -v subkey >/dev/null 2>&1; then
    SUBKEY_BIN="$(command -v subkey)"
  else
    echo "[error] subkey not found. Install with: cargo install subkey"
    exit 1
  fi
fi

if [[ ! -f "$CHAIN_SPEC" ]]; then
  echo "[error] Chain spec not found: $CHAIN_SPEC"
  exit 1
fi

# ── Ensure raw chain spec ───────────────────────────────────────────────────
ensure_raw_spec() {
  if [[ -s "$CHAIN_SPEC" ]]; then return 0; fi
  if [[ ! -x "$NODE_BIN" ]]; then
    echo "[error] Node binary not found for build-spec: $NODE_BIN"
    exit 1
  fi
  if [[ ! -f "$CHAIN_SPEC_PLAIN" ]]; then
    echo "[error] Plain chain spec not found: $CHAIN_SPEC_PLAIN"
    exit 1
  fi
  echo "[spec] Raw chain spec is empty. Regenerating from $CHAIN_SPEC_PLAIN..."
  tmp_spec="${CHAIN_SPEC}.tmp"
  "$NODE_BIN" build-spec --chain "$CHAIN_SPEC_PLAIN" --raw --disable-log-color > "$tmp_spec" 2>/dev/null
  TMP_SPEC="$tmp_spec" CHAIN_SPEC="$CHAIN_SPEC" python3 - <<'PY'
import json, os
from pathlib import Path
src = Path(os.environ["TMP_SPEC"])
dst = Path(os.environ["CHAIN_SPEC"])
text = src.read_text()
start = text.find("{")
end = text.rfind("}")
if start == -1 or end == -1 or end <= start:
    raise SystemExit("Failed to locate JSON object in build-spec output")
json_text = text[start:end+1]
json.loads(json_text)  # validate
dst.write_text(json_text)
print(f"[spec] Regenerated raw spec: {dst}")
PY
  rm -f "$tmp_spec"
}

ensure_raw_spec

# ── Sanitize chain spec (remove embedded bootnodes) ─────────────────────────
CHAIN_SPEC="${CHAIN_SPEC}" CHAIN_SPEC_RUN="${CHAIN_SPEC_RUN}" python3 - <<'PY'
import os, re
from pathlib import Path
src = Path(os.environ["CHAIN_SPEC"])
dst = Path(os.environ["CHAIN_SPEC_RUN"])
text = src.read_text()
start = text.find("{")
if start == -1:
    raise SystemExit(f"Invalid chain spec: {src}")
text = text[start:]
pattern = re.compile(r'("bootNodes"\s*:\s*\[).*?(\])', re.S)
text, count = pattern.subn(r'\1\2', text, count=1)
if count == 0:
    raise SystemExit("bootNodes key not found in chain spec")
dst.write_text(text)
print(f"[spec] Using sanitized chain spec: {dst}")
PY

# ── Read chain ID ───────────────────────────────────────────────────────────
CHAIN_ID="$(CHAIN_SPEC_RUN="$CHAIN_SPEC_RUN" python3 - <<'PY'
import json, os
from pathlib import Path
spec = json.loads(Path(os.environ["CHAIN_SPEC_RUN"]).read_text())
print(spec.get("id", ""))
PY
)"

if [[ -z "$CHAIN_ID" ]]; then
  echo "[error] Failed to read chain id from ${CHAIN_SPEC_RUN}"
  exit 1
fi

if [[ "$COUNT" -lt 1 || "$COUNT" -gt 7 ]]; then
  echo "[error] COUNT must be between 1 and 7 (got: ${COUNT})"
  exit 1
fi

# ── Dev seeds (testnet only — DO NOT use for mainnet) ───────────────────────
DEV_SEEDS=(
  "//Alice"
  "//Bob"
  "//Charlie"
  "//Dave"
  "//Eve"
  "//Ferdie"
  "//One"
)

# ── Key injection ───────────────────────────────────────────────────────────
insert_keys() {
  local base_path="$1"
  local suri="$2"
  local keystore_dir="${base_path}/chains/${CHAIN_ID}/keystore"
  mkdir -p "$keystore_dir"

  local aura_pub gran_pub
  aura_pub=$("$SUBKEY_BIN" inspect --scheme sr25519 "$suri" | awk '/Public key \(hex\):/ {print $4}')
  gran_pub=$("$SUBKEY_BIN" inspect --scheme ed25519 "$suri" | awk '/Public key \(hex\):/ {print $4}')

  if [[ -z "$aura_pub" || -z "$gran_pub" ]]; then
    echo "[error] Failed to derive public keys for ${suri}"
    exit 1
  fi

  local aura_file="61757261${aura_pub#0x}"
  local gran_file="6772616e${gran_pub#0x}"

  SURI="$suri" OUT="$keystore_dir/$aura_file" python3 - <<'PY'
import json, os
from pathlib import Path
Path(os.environ["OUT"]).write_text(json.dumps(os.environ["SURI"]))
Path(os.environ["OUT"]).chmod(0o600)
PY

  SURI="$suri" OUT="$keystore_dir/$gran_file" python3 - <<'PY'
import json, os
from pathlib import Path
Path(os.environ["OUT"]).write_text(json.dumps(os.environ["SURI"]))
Path(os.environ["OUT"]).chmod(0o600)
PY
}

validate_keys() {
  local base_path="$1"
  local suri="$2"
  local keystore_dir="${base_path}/chains/${CHAIN_ID}/keystore"
  local aura_pub gran_pub
  aura_pub=$("$SUBKEY_BIN" inspect --scheme sr25519 "$suri" | awk '/Public key \(hex\):/ {print $4}')
  gran_pub=$("$SUBKEY_BIN" inspect --scheme ed25519 "$suri" | awk '/Public key \(hex\):/ {print $4}')
  local aura_file="${keystore_dir}/61757261${aura_pub#0x}"
  local gran_file="${keystore_dir}/6772616e${gran_pub#0x}"
  if [[ ! -s "$aura_file" || ! -s "$gran_file" ]]; then
    echo "[error] Missing keystore files for ${suri} in ${keystore_dir}"
    exit 1
  fi
}

wait_for_rpc() {
  local rpc_port="$1"
  for _ in $(seq 1 90); do
    if curl -s -H "Content-Type: application/json" \
      -d '{"jsonrpc":"2.0","id":1,"method":"system_health","params":[]}' \
      "http://127.0.0.1:${rpc_port}" | grep -q '"isSyncing"'; then
      return 0
    fi
    sleep 1
  done
  echo "[error] RPC not ready on port ${rpc_port}"
  return 1
}

# ── Start a single validator node ───────────────────────────────────────────
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
    prom_args+=(--prometheus-external)
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

  # Enable unsafe RPC for testnet (allows key injection, etc.)
  nohup "${nice_args[@]}" "$NODE_BIN" \
    --chain "$CHAIN_SPEC_RUN" \
    --base-path "$base_path" \
    --name "$name" \
    --rpc-port "$rpc_port" \
    --rpc-methods=Unsafe \
    --rpc-cors=all \
    --rpc-external \
    --unsafe-force-node-key-generation \
    "${log_args[@]}" \
    "${net_args[@]}" \
    "${prom_args[@]}" \
    "${db_args[@]}" \
    "${password_args[@]}" \
    --validator \
    --force-authoring \
    --allow-private-ip \
    --execution=native-else-wasm \
    "${boot_args[@]}" \
    > "$log_file" 2>&1 &

  echo $! > "${PID_DIR}/node-${i}.pid"
  echo "[node] Started ${name} (p2p=${p2p_port}, rpc=${rpc_port}, prom=${prom_port})"

  wait_for_rpc "$rpc_port"
  echo "[node] ${name} ready"
}

# ── Launch sequence ─────────────────────────────────────────────────────────
echo "=========================================="
echo " X3 Testnet Launcher (bridge-enabled)"
echo " Binary: ${NODE_BIN}"
echo " Chain:  ${CHAIN_SPEC}"
echo " Count:  ${COUNT} validators"
echo " Features: ${BUILD_FEATURES}"
echo "=========================================="

echo "[launch] Starting node 1 (bootnode)..."
start_node 1

# Get bootnode peer ID
peer_id=""
for _ in $(seq 1 60); do
  peer_id="$(curl -s -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":1,"method":"system_localPeerId","params":[]}' \
    "http://127.0.0.1:9944" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("result",""))' 2>/dev/null || true)"
  if [[ -n "$peer_id" ]]; then
    break
  fi
  sleep 1
done

if [[ -z "$peer_id" ]]; then
  echo "[error] Failed to detect node-1 peer ID via RPC"
  exit 1
fi

BOOTNODE="/ip4/${LISTEN_IP}/tcp/30333/p2p/${peer_id}"
echo "[launch] Bootnode: ${BOOTNODE}"

for i in $(seq 2 "$COUNT"); do
  echo "[launch] Starting node ${i}..."
  start_node "$i" "$BOOTNODE"
done

echo "=========================================="
echo " All ${COUNT} validators started."
echo " Logs: ${LOG_DIR}/node-*.log"
echo " PIDs: ${PID_DIR}/node-*.pid"
echo ""
echo " RPC endpoints:"
for i in $(seq 1 "$COUNT"); do
  rpc_port=$((9944 + i - 1))
  echo "   http://127.0.0.1:${rpc_port}"
done
echo ""
echo " To check status: ./scripts/testnet/status-7-validators.sh"
echo " To verify:       ./scripts/testnet/verify-testnet.sh"
echo " To stop:         ./scripts/testnet/x3_testnet_down.sh"
echo "=========================================="
