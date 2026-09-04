#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# deploy-explorer.sh — Deploy block explorer for X3 testnet
#
# Deploys Polkadot.js Apps (or a custom explorer) connected to the testnet
# RPC endpoints. Supports both Docker and static-site deployment modes.
#
# Usage:
#   ./scripts/testnet/deploy-explorer.sh [--mode docker|static] [--rpc-url URL]
#       [--domain explorer.testnet.x3chain.com] [--port 8080]
#
# Environment:
#   EXPLORER_MODE    docker|static (default: docker)
#   RPC_URL          Testnet RPC endpoint (default: http://127.0.0.1:9944)
#   EXPLORER_DOMAIN  Public domain for the explorer
#   EXPLORER_PORT    Local port to bind (default: 8080)
# ─────────────────────────────────────────────────────────────────────────────

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
EXPLORER_MODE="${EXPLORER_MODE:-docker}"
RPC_URL="${RPC_URL:-http://127.0.0.1:9944}"
EXPLORER_DOMAIN="${EXPLORER_DOMAIN:-explorer.testnet.x3chain.com}"
EXPLORER_PORT="${EXPLORER_PORT:-8080}"
COMPOSE_DIR="${ROOT_DIR}/scripts/testnet/compose"

usage() {
  cat <<EOF
Usage: $(basename "$0") [--mode docker|static] [--rpc-url URL] [--domain DOMAIN] [--port PORT]

Deploy block explorer for X3 testnet.

Options:
  --mode docker|static  Deployment mode (default: docker)
  --rpc-url URL         Testnet RPC endpoint (default: http://127.0.0.1:9944)
  --domain DOMAIN       Public domain (default: explorer.testnet.x3chain.com)
  --port PORT           Local port (default: 8080)
  -h, --help            Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --mode) EXPLORER_MODE="${2:-}"; shift 2 ;;
    --rpc-url) RPC_URL="${2:-}"; shift 2 ;;
    --domain) EXPLORER_DOMAIN="${2:-}"; shift 2 ;;
    --port) EXPLORER_PORT="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown argument: $1"; usage; exit 2 ;;
  esac
done

echo "=========================================="
echo " X3 Testnet Block Explorer Deploy"
echo " Mode:   ${EXPLORER_MODE}"
echo " RPC:    ${RPC_URL}"
echo " Domain: ${EXPLORER_DOMAIN}"
echo " Port:   ${EXPLORER_PORT}"
echo "=========================================="

case "$EXPLORER_MODE" in
  docker)
    echo "[deploy] Deploying explorer via Docker..."

    mkdir -p "$COMPOSE_DIR"

    # Write docker-compose for the explorer
    cat > "${COMPOSE_DIR}/docker-compose.explorer.yml" <<COMPOSE
version: '3.8'

services:
  explorer:
    image: polkadot-js/apps:latest
    container_name: x3-explorer
    ports:
      - "${EXPLORER_PORT}:80"
    environment:
      - REACT_APP_RPC_URL=${RPC_URL}
      - REACT_APP_CHAIN_NAME=X3 Testnet
      - REACT_APP_CHAIN_TYPE=substrate
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.explorer.rule=Host(\`${EXPLORER_DOMAIN}\`)"
      - "traefik.http.services.explorer.loadbalancer.server.port=80"
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:80/"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s
COMPOSE

    echo "[deploy] Starting explorer container..."
    docker compose -f "${COMPOSE_DIR}/docker-compose.explorer.yml" up -d

    echo "[deploy] Explorer deployed at http://localhost:${EXPLORER_PORT}"
    echo "        (configured to connect to ${RPC_URL})"
    ;;

  static)
    echo "[deploy] Deploying static explorer site..."

    BUILD_DIR="${ROOT_DIR}/scripts/testnet/explorer-build"
    mkdir -p "$BUILD_DIR"

    # Create a minimal explorer index.html that connects to the testnet
    cat > "${BUILD_DIR}/index.html" <<HTML
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>X3 Testnet Explorer</title>
  <style>
    body { font-family: monospace; margin: 2rem; background: #0d1117; color: #c9d1d9; }
    h1 { color: #58a6ff; }
    .status { padding: 1rem; background: #161b22; border-radius: 8px; margin: 1rem 0; }
    .status dt { color: #8b949e; font-size: 0.85rem; }
    .status dd { margin: 0 0 0.5rem 0; font-size: 1.1rem; }
    .error { color: #f85149; }
    .ok { color: #3fb950; }
  </style>
</head>
<body>
  <h1>🔭 X3 Testnet Explorer</h1>
  <p>RPC: <code>${RPC_URL}</code></p>
  <div id="status" class="status">
    <h3>Chain Status</h3>
    <dl id="chain-info">
      <dt>Connecting...</dt>
    </dl>
  </div>
  <script>
    const RPC_URL = "${RPC_URL}";
    async function fetchChainInfo() {
      const info = document.getElementById('chain-info');
      try {
        const [health, header, chain] = await Promise.all([
          rpcCall('system_health', []),
          rpcCall('chain_getHeader', []),
          rpcCall('system_chain', []),
        ]);
        info.innerHTML = \`
          <dt>Chain</dt><dd class="ok">\${chain.result}</dd>
          <dt>Best Block</dt><dd>\${parseInt(header.result.number, 16)}</dd>
          <dt>Peers</dt><dd>\${health.result.peers}</dd>
          <dt>Syncing</dt><dd>\${health.result.isSyncing ? 'Yes' : 'No'}</dd>
        \`;
      } catch (e) {
        info.innerHTML = \`<dt class="error">Error: \${e.message}</dt>\`;
      }
    }
    async function rpcCall(method, params) {
      const resp = await fetch(RPC_URL, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ jsonrpc: '2.0', id: 1, method, params }),
      });
      return resp.json();
    }
    fetchChainInfo();
    setInterval(fetchChainInfo, 6000);
  </script>
</body>
</html>
HTML

    echo "[deploy] Static explorer built at ${BUILD_DIR}"
    echo "        Serve with: python3 -m http.server ${EXPLORER_PORT} -d ${BUILD_DIR}"
    echo "        Or deploy to any static file server."
    ;;

  *)
    echo "[error] Unknown mode: ${EXPLORER_MODE}. Use 'docker' or 'static'."
    exit 1
    ;;
esac

echo "[deploy] Explorer deployment complete."
