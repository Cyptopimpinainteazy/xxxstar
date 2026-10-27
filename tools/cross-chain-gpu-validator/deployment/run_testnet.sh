#!/bin/bash
set -euo pipefail

SOLANA_VALIDATOR_BIN="${SOLANA_VALIDATOR_BIN:-solana-test-validator}"
GETH_BIN="${GETH_BIN:-geth}"
LOG_DIR="${LOG_DIR:-./logs}"

mkdir -p "${LOG_DIR}"

if ! command -v "${SOLANA_VALIDATOR_BIN}" >/dev/null 2>&1; then
  echo "Missing solana-test-validator. Install Solana CLI." >&2
  exit 1
fi

if ! command -v "${GETH_BIN}" >/dev/null 2>&1; then
  echo "Missing geth. Install go-ethereum." >&2
  exit 1
fi

echo "Starting Solana test validator..."
"${SOLANA_VALIDATOR_BIN}" --reset \
  --ledger "${LOG_DIR}/solana-ledger" \
  --log "${LOG_DIR}/solana-validator.log" &

SOLANA_PID=$!

sleep 2

echo "Starting Ethereum devnet (geth)..."
"${GETH_BIN}" --dev \
  --http --http.addr 127.0.0.1 --http.port 8545 \
  --http.api eth,net,web3,personal \
  --datadir "${LOG_DIR}/geth" \
  > "${LOG_DIR}/geth.log" 2>&1 &

GETH_PID=$!

echo "Solana PID: ${SOLANA_PID}"
echo "Geth PID: ${GETH_PID}"
echo "Logs: ${LOG_DIR}"

wait "${SOLANA_PID}" "${GETH_PID}"
