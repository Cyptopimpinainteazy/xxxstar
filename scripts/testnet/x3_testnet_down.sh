#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# x3_testnet_down.sh — Stop testnet nodes and infrastructure services
#
# Stops all testnet validator nodes and optionally tears down
# infrastructure services (explorer, indexer, faucet, RPC gateway).
#
# Usage:
#   ./scripts/testnet/x3_testnet_down.sh [--all] [--base-dir PATH]
#
# Options:
#   --all       Also stop infrastructure services (explorer, indexer, faucet, RPC gateway)
#   --base-dir  Base directory for validator data (default: ~/.local/share/x3/testnet)
#   -h, --help  Show this help.
# ─────────────────────────────────────────────────────────────────────────────

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BASE_DIR="${BASE_DIR:-$HOME/.local/share/x3/testnet}"
PID_DIR="${BASE_DIR}/pids"
COMPOSE_DIR="${ROOT_DIR}/scripts/testnet/compose"
STOP_ALL=0

usage() {
  cat <<EOF
Usage: $(basename "$0") [--all] [--base-dir PATH]

Stop testnet nodes and optionally infrastructure services.

Options:
  --all       Also stop infrastructure services (Docker Compose stacks)
  --base-dir  Base directory for validator data (default: ${BASE_DIR})
  -h, --help  Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --all) STOP_ALL=1; shift ;;
    --base-dir) BASE_DIR="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown argument: $1"; usage; exit 2 ;;
  esac
done

echo "=========================================="
echo " X3 Testnet Shutdown"
echo " Base dir: ${BASE_DIR}"
echo "=========================================="

# ── Stop validator nodes ────────────────────────────────────────────────────
echo "[stop] Stopping validator nodes..."

if [[ -d "$PID_DIR" ]]; then
  shopt -s nullglob
  pids=("$PID_DIR"/node-*.pid)
  shopt -u nullglob

  if [[ ${#pids[@]} -gt 0 ]]; then
    for pid_file in "${pids[@]}"; do
      local pid
      pid="$(cat "$pid_file" 2>/dev/null || true)"
      if [[ -n "$pid" ]]; then
        node_name=$(basename "$pid_file" .pid)
        echo "  Stopping ${node_name} (pid: ${pid})..."
        kill "$pid" 2>/dev/null || true
      fi
    done
    sleep 2

    # Force kill any remaining
    for pid_file in "${pids[@]}"; do
      pid="$(cat "$pid_file" 2>/dev/null || true)"
      if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
        echo "  Force killing $(basename "$pid_file" .pid)..."
        kill -9 "$pid" 2>/dev/null || true
      fi
    done
  else
    echo "  No PID files found."
  fi
else
  echo "  No PID directory found at ${PID_DIR}"
fi

# ── Stop infrastructure services ────────────────────────────────────────────
if [[ "$STOP_ALL" == "1" ]]; then
  echo "[stop] Stopping infrastructure services..."

  for compose_file in "${COMPOSE_DIR}/docker-compose."*.yml; do
    if [[ -f "$compose_file" ]]; then
      service_name=$(basename "$compose_file" .yml | sed 's/docker-compose\.//')
      echo "  Stopping ${service_name}..."
      docker compose -f "$compose_file" down 2>/dev/null || true
    fi
  done
fi

# ── Cleanup ─────────────────────────────────────────────────────────────────
echo "[stop] Cleaning up PID files..."
rm -f "${PID_DIR}/node-".*.pid 2>/dev/null || true

echo "[stop] Testnet shutdown complete."
