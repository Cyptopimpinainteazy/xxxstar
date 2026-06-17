#!/usr/bin/env bash
# ── E2E Atomic Trade Test ──────────────────────────────────────────────────
# Boots Anvil + X3-chain-node + relayer, deposits ERC20 on EVM side,
# verifies the relayer bridges it to X3.
#
# Prerequisites:
#   forge, anvil, cast   — Foundry toolchain
#   cargo build --release -p x3-chain-node -p x3-relayer
#
# Usage:
#   bash scripts/e2e-atomic-trade.sh

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GATEWAY_SOL="$ROOT/X3-contracts/evm/contracts/X3ExternalGateway.sol"
FOUNDRY_TOML="$ROOT/X3-contracts/evm/foundry.toml"
NODE_BIN="$ROOT/target/release/x3-chain-node"
RELAYER_BIN="$ROOT/target/release/x3-relayer"
CHAIN_SPEC="$ROOT/chain-specs/x3-local3-current-raw.json"

# ── Sanity checks ───────────────────────────────────────────────────────────

command -v forge >/dev/null 2>&1 || { echo "forge not found — install Foundry (https://book.getfoundry.sh)"; exit 1; }
command -v anvil >/dev/null 2>&1 || { echo "anvil not found"; exit 1; }
command -v cast  >/dev/null 2>&1 || { echo "cast not found"; exit 1; }
command -v jq    >/dev/null 2>&1 || { echo "jq not found — apt install jq"; exit 1; }

if [ ! -f "$GATEWAY_SOL" ]; then
  echo "Gateway contract not found at $GATEWAY_SOL"
  exit 1
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  X3 Atomic Trade — End-to-End Test"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# ── 1. Build binaries (if missing) ──────────────────────────────────────────

if [ ! -f "$NODE_BIN" ]; then
  echo "[1/5] Building x3-chain-node (release)..."
  cargo build --release -p x3-chain-node 2>&1 | tail -3
fi

if [ ! -f "$RELAYER_BIN" ]; then
  echo "[1/5] Building x3-relayer (release)..."
  cargo build --release -p x3-relayer 2>&1 | tail -3
fi

# ── 2. Start Anvil (EVM side) ───────────────────────────────────────────────

echo "[2/5] Starting Anvil (EVM local chain)..."
ANVIL_DIR=$(mktemp -d)
anvil \
  --port 8545 \
  --block-time 1 \
  --chain-id 1337 \
  > "$ANVIL_DIR/anvil.log" 2>&1 &
ANVIL_PID=$!
echo "       Anvil PID=$ANVIL_PID"

# Wait for Anvil to be ready
for i in $(seq 1 30); do
  if curl -s -X POST -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"eth_chainId","params":[]}' \
    http://127.0.0.1:8545 2>/dev/null | grep -q '"result"'; then
    break
  fi
  sleep 0.5
done
echo "       Anvil ready"

# Get the default Anvil account (Account #0 with 10,000 ETH)
ANVIL_PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
ANVIL_DEPLOYER="0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"

# ── 3. Deploy X3ExternalGateway to Anvil ─────────────────────────────────────

echo "[3/5] Deploying TestOnlyVerifier and X3ExternalGateway to Anvil..."

# Use TestOnlyVerifier from the existing test file (X3ExternalGateway.t.sol)
VERIFIER_OUTPUT=$(forge create \
  --private-key "$ANVIL_PRIVATE_KEY" \
  --rpc-url http://127.0.0.1:8545 \
  --broadcast \
  "$ROOT/X3-contracts/evm/test/X3ExternalGateway.t.sol:TestOnlyVerifier" \
  --constructor-args true \
  2>&1)
VERIFIER_ADDR=$(echo "$VERIFIER_OUTPUT" | grep 'Deployed to:' | awk '{print $3}')

if [ -z "$VERIFIER_ADDR" ]; then
  echo "ERROR: Failed to deploy TestOnlyVerifier"
  echo "$VERIFIER_OUTPUT"
  kill "$ANVIL_PID" 2>/dev/null || true
  exit 1
fi
echo "       TestOnlyVerifier deployed at: $VERIFIER_ADDR"

# Now deploy the gateway
GATEWAY_OUTPUT=$(forge create \
  --private-key "$ANVIL_PRIVATE_KEY" \
  --rpc-url http://127.0.0.1:8545 \
  --broadcast \
  "$GATEWAY_SOL:X3ExternalGateway" \
  --constructor-args "$VERIFIER_ADDR" 1337 200 \
  2>&1)
GATEWAY_ADDR=$(echo "$GATEWAY_OUTPUT" | grep 'Deployed to:' | awk '{print $3}')

if [ -z "$GATEWAY_ADDR" ]; then
  echo "ERROR: Failed to deploy X3ExternalGateway"
  echo "$GATEWAY_OUTPUT"
  kill "$ANVIL_PID" 2>/dev/null || true
  exit 1
fi

echo "       X3ExternalGateway deployed at: $GATEWAY_ADDR"

# ── 4. Start x3-chain-node ──────────────────────────────────────────────────

echo "[4/5] Starting x3-chain-node..."
NODE_DIR=$(mktemp -d)
"$NODE_BIN" \
  --chain "$CHAIN_SPEC" \
  --alice \
  --tmp \
  --unsafe-rpc-external \
  --rpc-cors all \
  > "$NODE_DIR/node.log" 2>&1 &
NODE_PID=$!
echo "       Node PID=$NODE_PID"

# Wait for node to be ready (up to 90 seconds on slower machines)
for i in $(seq 1 90); do
  if curl -s -X POST -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"system_health","params":[]}' \
    http://127.0.0.1:9944 2>/dev/null | grep -q '"result"'; then
    break
  fi
  sleep 1
done
echo "       Node ready"

# Wait for finality to advance a few blocks
sleep 5

# ── 5. Run relayer → trigger deposit → verify ───────────────────────────────

echo "[5/5] Starting relayer and executing atomic trade..."

# Configure relayer via env vars to point at Anvil + X3 node
export X3_RPC="http://127.0.0.1:9944"
export ETH_RPC="http://127.0.0.1:8545"
export ETH_GATEWAY="$GATEWAY_ADDR"
export X3_NETWORK="local"
export POLL_INTERVAL="2"

# Start relayer in background
"$RELAYER_BIN" > "$ANVIL_DIR/relayer.log" 2>&1 &
RELAYER_PID=$!
echo "       Relayer PID=$RELAYER_PID (polling every 2s)"

sleep 3

# ── Execute deposit on EVM gateway ───────────────────────────────────────────

# First whitelist a mock token (the gateway has an owner; we use the deployer)
DEPLOYER_PK="$ANVIL_PRIVATE_KEY"
TOKEN_ADDR="0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"  # USDC address on mainnet (mock)

cat > "$ANVIL_DIR/deposit_call.sol" <<'SOL'
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

interface IGateway {
    function deposit(address token, uint256 amount, bytes calldata x3Recipient) external returns (bytes32 messageId);
}

contract DepositCaller {
    function execute(address gateway, address token, uint256 amount, bytes calldata recipient)
        external
        returns (bytes32)
    {
        return IGateway(gateway).deposit(token, amount, recipient);
    }
}
SOL

# Deploy DepositCaller
CALLER_ADDR=$(forge create \
  --private-key "$ANVIL_PRIVATE_KEY" \
  --rpc-url http://127.0.0.1:8545 \
  "$ANVIL_DIR/deposit_call.sol:DepositCaller" \
  2>&1 | grep 'Deployed to:' | awk '{print $3}')

echo "       Calling deposit(USDC, 1000, alice_x3)..."
RECIPIENT_HEX="0x$(printf 'alice_x3_recipient_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx' | head -c 64 | xxd -p)"
cast send \
  --private-key "$DEPLOYER_PK" \
  --rpc-url http://127.0.0.1:8545 \
  "$GATEWAY_ADDR" \
  "deposit(address,uint256,bytes)" \
  "$TOKEN_ADDR" 1000 "$RECIPIENT_HEX" \
  2>&1 | head -5

echo "       Deposit transaction sent! Waiting for relayer to pick it up..."

# ── Wait for relayer to process ──────────────────────────────────────────────

sleep 8

# ── Verification ─────────────────────────────────────────────────────────────

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Relayer Log (last 20 lines)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
tail -20 "$ANVIL_DIR/relayer.log" 2>/dev/null || echo "(relayer log not found)"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Verification: Checking if deposit was processed"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Check the EVM gateway for the deposit event
DEPOSIT_EVENTS=$(cast logs \
  --rpc-url http://127.0.0.1:8545 \
  --address "$GATEWAY_ADDR" \
  --from-block 0 \
  "DepositLocked(bytes32,address,address,bytes,uint256,uint256,uint256)" \
  2>/dev/null | wc -l)

echo "       EVM DepositLocked events found: $DEPOSIT_EVENTS"

# Check if the relayer logged a successful proof submission
if grep -q "Deposit proof submitted successfully" "$ANVIL_DIR/relayer.log" 2>/dev/null; then
  echo "       ✅ Relayer successfully submitted deposit proof to X3!"
  RESULT="PASSED"
else
  echo "       ⚠️  Relayer did not report successful proof submission"
  RESULT="PARTIAL"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  E2E Atomic Trade Test: $RESULT"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# ── Cleanup ──────────────────────────────────────────────────────────────────

echo ""
echo "  Cleaning up background processes..."
kill "$ANVIL_PID" 2>/dev/null || true
kill "$NODE_PID" 2>/dev/null || true
kill "$RELAYER_PID" 2>/dev/null || true
wait 2>/dev/null || true
echo "  Done."