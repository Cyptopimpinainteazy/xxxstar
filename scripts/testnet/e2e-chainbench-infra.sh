#!/usr/bin/env bash
set -euo pipefail

HOST="${CHAINBENCH_HOST:-127.0.0.1}"
PORT="${CHAINBENCH_PORT:-7788}"
CHAIN_DB_URL="${CHAINBENCH_CHAIN_DB_URL:-http://127.0.0.1:7070}"
TPS_URL="${CHAINBENCH_TPS_URL:-http://127.0.0.1:3010}"
DEFAULT_CHAIN_ID="${CHAINBENCH_DEFAULT_CHAIN_ID:-eth}"

banner() { echo -e "\n== $* =="; }
json() { python3 -c 'import json,sys; print(json.dumps(json.loads(sys.stdin.read()), indent=2))'; }

banner "Chainbench Health"
curl -s "http://${HOST}:${PORT}/health" | json

banner "Infra Status (Chainbench)"
curl -s "http://${HOST}:${PORT}/api/infra/status" | json

banner "Chain DB Health"
curl -s "${CHAIN_DB_URL}/health" | json

banner "Chain DB RPC Stats"
curl -s "${CHAIN_DB_URL}/api/rpc/stats" | json

banner "Blockchain TPS Health"
curl -s "${TPS_URL}/health" | json

banner "dRPC Status (Chainbench)"
curl -s "http://${HOST}:${PORT}/api/drpc/status" | json

banner "RPC Bench (default chain)"
curl -s -X POST "http://${HOST}:${PORT}/api/rpc/bench" \
  -H 'Content-Type: application/json' \
  -d "{\"chain_id\":\"${DEFAULT_CHAIN_ID}\"}" | json

banner "Connector Health"
curl -s "http://${HOST}:${PORT}/api/connectors/health" | json

banner "GPU Route Benchmark (smoke)"
curl -s -X POST "http://${HOST}:${PORT}/api/gpu-route/benchmark" \
  -H 'Content-Type: application/json' \
  -d '{"iterations":5,"method":"system_health"}' | json

if [ "${CHAINBENCH_E2E_CREATE_CONNECTOR:-0}" = "1" ]; then
  banner "Create Connector (E2E)"
  curl -s -X POST "http://${HOST}:${PORT}/api/onboarding/chains" \
    -H 'Content-Type: application/json' \
    -H "X-Admin-Key: ${CHAINBENCH_ADMIN_KEY:-x3-admin-local}" \
    -d '{"chain":"Ethereum Mainnet","rpc":"https://eth.llamarpc.com","cred":"e2e-demo-key","notes":"e2e test"}' | json
fi
