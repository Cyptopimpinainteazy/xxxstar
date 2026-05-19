#!/usr/bin/env bash
set -euo pipefail

# Simple smoke script for E2E: waits for services and runs the relayer tests
BITCOIN_RPC_URL=${BITCOIN_RPC_URL:-http://user:pass@localhost:18332}
ETHEREUM_RPC_URL=${ETHEREUM_RPC_URL:-http://localhost:8545}

echo "Using BITCOIN_RPC_URL=$BITCOIN_RPC_URL"

# wait for bitcoind RPC
for i in {1..30}; do
  if curl -s $BITCOIN_RPC_URL >/dev/null 2>&1; then
    echo "bitcoind RPC ready"; break
  fi
  echo "waiting for bitcoind RPC..."; sleep 2
done

# wait for eth RPC
for i in {1..30}; do
  if curl -s $ETHEREUM_RPC_URL >/dev/null 2>&1; then
    echo "ethereum RPC ready"; break
  fi
  echo "waiting for ethereum RPC..."; sleep 2
done

# Run relayer tests (sequential runner handles TS issues)
cd scripts/relayer
npm test --if-present
