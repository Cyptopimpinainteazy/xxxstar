#!/usr/bin/env bash
set -euo pipefail

ROOT="/home/lojak/Desktop/X3_ATOMIC_STAR"
RPC_HOST="127.0.0.1"
RPC_PORT="9944"
RPC_URL="http://${RPC_HOST}:${RPC_PORT}"
LIVE_LOG="${ROOT}/reports/rc2/live_node_for_e2e.log"
STARTED_NODE=0
NODE_PID=""

mkdir -p "${ROOT}/reports/rc2"

is_node_up() {
  curl -sS -m 2 -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"system_health","params":[]}' \
    "${RPC_URL}" >/dev/null 2>&1
}

wait_for_node() {
  local timeout_secs="${1:-60}"
  local start
  start=$(date +%s)

  while true; do
    if is_node_up; then
      return 0
    fi
    local now
    now=$(date +%s)
    if (( now - start >= timeout_secs )); then
      return 1
    fi
    sleep 1
  done
}

start_live_node_if_needed() {
  if is_node_up; then
    echo "[rc2_mock_and_live_gate] Reusing already running node at ${RPC_URL}"
    return 0
  fi

  echo "[rc2_mock_and_live_gate] Starting real local node for live E2E lane"
  (
    cd "${ROOT}"
    ALLOW_MOCK=false CHAIN=dev RPC_PORT=${RPC_PORT} WS_PORT=9945 bash scripts/start-x3-chain.sh
  ) >"${LIVE_LOG}" 2>&1 &

  NODE_PID="$!"
  STARTED_NODE=1

  if ! wait_for_node 90; then
    echo "[rc2_mock_and_live_gate] Node failed to start within timeout; see ${LIVE_LOG}" >&2
    return 1
  fi
}

cleanup() {
  if [[ "${STARTED_NODE}" == "1" && -n "${NODE_PID}" ]]; then
    echo "[rc2_mock_and_live_gate] Stopping node started by gate (pid=${NODE_PID})"
    kill "${NODE_PID}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

cd "${ROOT}"

echo "[rc2_mock_and_live_gate] Running mock/internal suite"
cargo test --manifest-path tests/e2e/Cargo.toml --test mainnet_rc1 -- --nocapture

echo "[rc2_mock_and_live_gate] Ensuring live node and running strict live suite"
start_live_node_if_needed
X3_E2E_REQUIRE_NODE=1 cargo test --manifest-path tests/e2e/Cargo.toml --test live_internal_mainnet_e2e -- --nocapture

echo "[rc2_mock_and_live_gate] PASS: mock suite and live suite both passed"
