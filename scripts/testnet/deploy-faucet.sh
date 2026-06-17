#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# deploy-faucet.sh — Deploy testnet faucet for X3 testnet
#
# Deploys a simple faucet service that dispenses testnet tokens to users
# who complete a captcha or provide a valid address. Supports both a
# custom faucet server and a Docker-based deployment.
#
# Usage:
#   ./scripts/testnet/deploy-faucet.sh [--rpc-url URL] [--suri SURI]
#       [--port 3000] [--domain faucet.testnet.x3chain.com]
#       [--drip-amount AMOUNT] [--drip-interval SECONDS]
#
# Environment:
#   FAUCET_RPC_URL     Testnet RPC endpoint (default: http://127.0.0.1:9944)
#   FAUCET_SURI        Secret URI for the faucet account (default: //Faucet)
#   FAUCET_PORT        HTTP port (default: 3000)
#   FAUCET_DRIP_AMOUNT Amount to drip per request (default: 1000000000000 = 1 X3)
#   FAUCET_DRIP_INTERVAL Min seconds between drips per address (default: 86400 = 24h)
#   FAUCET_DOMAIN      Public domain
# ─────────────────────────────────────────────────────────────────────────────

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
COMPOSE_DIR="${ROOT_DIR}/scripts/testnet/compose"
FAUCET_RPC_URL="${FAUCET_RPC_URL:-http://127.0.0.1:9944}"
FAUCET_SURI="${FAUCET_SURI:-//Faucet}"
FAUCET_PORT="${FAUCET_PORT:-3000}"
FAUCET_DRIP_AMOUNT="${FAUCET_DRIP_AMOUNT:-1000000000000}"
FAUCET_DRIP_INTERVAL="${FAUCET_DRIP_INTERVAL:-86400}"
FAUCET_DOMAIN="${FAUCET_DOMAIN:-faucet.testnet.x3chain.com}"

usage() {
  cat <<EOF
Usage: $(basename "$0") [--rpc-url URL] [--suri SURI] [--port PORT]
       [--drip-amount AMOUNT] [--drip-interval SECONDS] [--domain DOMAIN]

Deploy testnet faucet for X3 testnet.

Options:
  --rpc-url URL         Testnet RPC endpoint (default: ${FAUCET_RPC_URL})
  --suri SURI           Secret URI for faucet account (default: //Faucet)
  --port PORT           HTTP port (default: ${FAUCET_PORT})
  --drip-amount AMOUNT  Amount per drip (default: ${FAUCET_DRIP_AMOUNT})
  --drip-interval SEC   Min seconds between drips (default: ${FAUCET_DRIP_INTERVAL})
  --domain DOMAIN       Public domain (default: ${FAUCET_DOMAIN})
  -h, --help            Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --rpc-url) FAUCET_RPC_URL="${2:-}"; shift 2 ;;
    --suri) FAUCET_SURI="${2:-}"; shift 2 ;;
    --port) FAUCET_PORT="${2:-}"; shift 2 ;;
    --drip-amount) FAUCET_DRIP_AMOUNT="${2:-}"; shift 2 ;;
    --drip-interval) FAUCET_DRIP_INTERVAL="${2:-}"; shift 2 ;;
    --domain) FAUCET_DOMAIN="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown argument: $1"; usage; exit 2 ;;
  esac
done

echo "=========================================="
echo " X3 Testnet Faucet Deploy"
echo " RPC:    ${FAUCET_RPC_URL}"
echo " Port:   ${FAUCET_PORT}"
echo " Drip:   ${FAUCET_DRIP_AMOUNT} (every ${FAUCET_DRIP_INTERVAL}s per address)"
echo " Domain: ${FAUCET_DOMAIN}"
echo "=========================================="

mkdir -p "$COMPOSE_DIR"

# Write the faucet server source
FAUCET_SRC_DIR="${ROOT_DIR}/scripts/testnet/faucet-server"
mkdir -p "$FAUCET_SRC_DIR"

cat > "${FAUCET_SRC_DIR}/package.json" <<JSON
{
  "name": "x3-testnet-faucet",
  "version": "1.0.0",
  "description": "X3 Testnet Faucet Server",
  "main": "server.js",
  "scripts": {
    "start": "node server.js"
  },
  "dependencies": {
    "express": "^4.18.2",
    "@polkadot/api": "^10.9.1",
    "@polkadot/keyring": "^12.3.2",
    "cors": "^2.8.5",
    "express-rate-limit": "^7.1.4",
    "helmet": "^7.1.0"
  }
}
JSON

cat > "${FAUCET_SRC_DIR}/server.js" <<'JAVASCRIPT'
const express = require('express');
const cors = require('cors');
const helmet = require('helmet');
const rateLimit = require('express-rate-limit');
const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');

const app = express();
const PORT = process.env.FAUCET_PORT || 3000;
const RPC_URL = process.env.FAUCET_RPC_URL || 'ws://127.0.0.1:9944';
const FAUCET_SURI = process.env.FAUCET_SURI || '//Faucet';
const DRIP_AMOUNT = BigInt(process.env.FAUCET_DRIP_AMOUNT || '1000000000000');
const DRIP_INTERVAL_MS = parseInt(process.env.FAUCET_DRIP_INTERVAL || '86400000');

// Track last drip per address
const dripTracker = new Map();

// Middleware
app.use(helmet());
app.use(cors());
app.use(express.json());

// Global rate limit: 10 requests per minute per IP
const globalLimiter = rateLimit({
  windowMs: 60 * 1000,
  max: 10,
  message: { error: 'Too many requests, please try again later.' },
});
app.use(globalLimiter);

// Validate address format
function isValidAddress(address) {
  return /^[a-km-zA-HJ-NP-Z1-9]{47,48}$/.test(address);
}

// Connect to chain
let api;
async function connect() {
  try {
    const provider = new WsProvider(RPC_URL);
    api = await ApiPromise.create({ provider });
    console.log(`[faucet] Connected to ${RPC_URL}`);

    const keyring = new Keyring({ type: 'sr25519' });
    const faucetAccount = keyring.addFromUri(FAUCET_SURI);
    console.log(`[faucet] Faucet account: ${faucetAccount.address}`);

    const balance = await api.query.system.account(faucetAccount.address);
    console.log(`[faucet] Faucet balance: ${balance.data.free}`);

    return { api, faucetAccount };
  } catch (err) {
    console.error(`[faucet] Connection failed: ${err.message}`);
    process.exit(1);
  }
}

// Health endpoint
app.get('/health', (req, res) => {
  res.json({
    status: 'ok',
    chain: api ? api.genesisHash.toHex() : null,
    faucetAddress: global.faucetAccount?.address,
  });
});

// Drip endpoint
app.post('/drip', async (req, res) => {
  try {
    const { address } = req.body;

    if (!address || !isValidAddress(address)) {
      return res.status(400).json({ error: 'Invalid substrate address' });
    }

    // Check drip interval
    const lastDrip = dripTracker.get(address);
    const now = Date.now();
    if (lastDrip && (now - lastDrip) < DRIP_INTERVAL_MS) {
      const remaining = Math.ceil((DRIP_INTERVAL_MS - (now - lastDrip)) / 1000);
      return res.status(429).json({
        error: `Please wait ${remaining} seconds before requesting again.`,
      });
    }

    if (!api || !global.faucetAccount) {
      return res.status(503).json({ error: 'Faucet not connected to chain' });
    }

    // Check faucet balance
    const { data: balance } = await api.query.system.account(global.faucetAccount.address);
    if (balance.free.lt(DRIP_AMOUNT)) {
      return res.status(503).json({ error: 'Faucet is empty, please refill.' });
    }

    // Send transfer
    const transfer = api.tx.balances.transferAllowDeath(address, DRIP_AMOUNT.toString());
    const hash = await transfer.signAndSend(global.faucetAccount);

    dripTracker.set(address, now);

    console.log(`[faucet] Dripped ${DRIP_AMOUNT} to ${address} (tx: ${hash})`);

    res.json({
      success: true,
      amount: DRIP_AMOUNT.toString(),
      to: address,
      txHash: hash.toHex(),
    });
  } catch (err) {
    console.error(`[faucet] Error: ${err.message}`);
    res.status(500).json({ error: 'Internal server error' });
  }
});

// Info endpoint
app.get('/info', (req, res) => {
  res.json({
    network: 'X3 Testnet',
    dripAmount: DRIP_AMOUNT.toString(),
    dripInterval: `${DRIP_INTERVAL_MS / 1000}s`,
    faucetAddress: global.faucetAccount?.address,
  });
});

// Start server
async function start() {
  const connection = await connect();
  global.api = connection.api;
  global.faucetAccount = connection.faucetAccount;

  app.listen(PORT, '0.0.0.0', () => {
    console.log(`[faucet] Server running on port ${PORT}`);
    console.log(`[faucet] Drip amount: ${DRIP_AMOUNT}`);
    console.log(`[faucet] Drip interval: ${DRIP_INTERVAL_MS / 1000}s`);
  });
}

start().catch(console.error);
JAVASCRIPT

cat > "${FAUCET_SRC_DIR}/Dockerfile" <<DOCKERFILE
FROM node:20-alpine
WORKDIR /app
COPY package.json ./
RUN npm install --production
COPY server.js ./
EXPOSE 3000
CMD ["node", "server.js"]
DOCKERFILE

# Write docker-compose for the faucet
cat > "${COMPOSE_DIR}/docker-compose.faucet.yml" <<COMPOSE
version: '3.8'

services:
  faucet:
    build:
      context: ${FAUCET_SRC_DIR}
      dockerfile: Dockerfile
    container_name: x3-faucet
    ports:
      - "${FAUCET_PORT}:3000"
    environment:
      FAUCET_PORT: "3000"
      FAUCET_RPC_URL: "${FAUCET_RPC_URL}"
      FAUCET_SURI: "${FAUCET_SURI}"
      FAUCET_DRIP_AMOUNT: "${FAUCET_DRIP_AMOUNT}"
      FAUCET_DRIP_INTERVAL: "$((FAUCET_DRIP_INTERVAL * 1000))"
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.faucet.rule=Host(\`${FAUCET_DOMAIN}\`)"
      - "traefik.http.services.faucet.loadbalancer.server.port=3000"
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 15s
COMPOSE

echo "[deploy] Building faucet Docker image..."
docker compose -f "${COMPOSE_DIR}/docker-compose.faucet.yml" build

echo "[deploy] Starting faucet..."
docker compose -f "${COMPOSE_DIR}/docker-compose.faucet.yml" up -d

echo "[deploy] Faucet deployment complete."
echo "        Endpoint: http://localhost:${FAUCET_PORT}"
echo "        POST /drip  {\"address\": \"...\"}"
echo "        GET  /health"
echo "        GET  /info"
echo ""
echo " To check logs: docker compose -f ${COMPOSE_DIR}/docker-compose.faucet.yml logs -f"
echo " To stop:       docker compose -f ${COMPOSE_DIR}/docker-compose.faucet.yml down"
