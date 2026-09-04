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
EVM_DIR="$ROOT/X3-contracts/evm"
GATEWAY_SOL="$ROOT/X3-contracts/evm/contracts/X3ExternalGateway.sol"
FOUNDRY_TOML="$ROOT/X3-contracts/evm/foundry.toml"
NODE_BIN="$ROOT/target/release/x3-chain-node"
RELAYER_BIN="$ROOT/target/release/x3-relayer"
CHAIN_SPEC="${X3_CHAIN_SPEC:-dev}"

ANVIL_PID=""
NODE_PID=""
RELAYER_PID=""

cleanup() {
  kill "${ANVIL_PID:-}" 2>/dev/null || true
  kill "${NODE_PID:-}" 2>/dev/null || true
  kill "${RELAYER_PID:-}" 2>/dev/null || true
}
trap cleanup EXIT

rpc() {
  local method="$1"
  local params="${2:-[]}"
  curl -s -X POST -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"$method\",\"params\":$params}" \
    http://127.0.0.1:9944
}

canonical_balance() {
  local account_hex="$1"
  rpc "x3_getCanonicalBalance" "[\"$account_hex\",0]" | jq -r '.result.balance // empty'
}

wrapped_accounting_for_token() {
  local account_hex="$1"
  local chain_id="$2"
  local token_addr="$3"
  rpc "x3_getWrappedAccountingForToken" "[\"$account_hex\",$chain_id,\"$token_addr\"]" | jq -c '.result // empty'
}

chain_number() {
  rpc "chain_getHeader" "[]" | jq -r '.result.number // "0x0"'
}

pending_extrinsic_count() {
  rpc "author_pendingExtrinsics" "[]" | jq -r '(.result // []) | length'
}

kernel_bridge_state() {
  rpc "x3_getKernelBridgeState" "[]" | jq -c '.result // {}'
}

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

if [ "${X3_E2E_SKIP_BUILD:-0}" != "1" ] || [ ! -f "$NODE_BIN" ] || [ ! -f "$RELAYER_BIN" ]; then
  echo "[1/5] Building x3-chain-node + x3-relayer (release)..."
  cargo build --release -p x3-chain-node -p x3-relayer 2>&1 | tail -20
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
if ! VERIFIER_OUTPUT=$(cd "$EVM_DIR" && forge create \
  --private-key "$ANVIL_PRIVATE_KEY" \
  --rpc-url http://127.0.0.1:8545 \
  --broadcast \
  "test/X3ExternalGateway.t.sol:TestOnlyVerifier" \
  --constructor-args true \
  2>&1); then
  echo "ERROR: Failed to deploy TestOnlyVerifier"
  echo "$VERIFIER_OUTPUT"
  exit 1
fi
VERIFIER_ADDR=$(echo "$VERIFIER_OUTPUT" | grep 'Deployed to:' | awk '{print $3}')

if [ -z "$VERIFIER_ADDR" ]; then
  echo "ERROR: Failed to deploy TestOnlyVerifier"
  echo "$VERIFIER_OUTPUT"
  exit 1
fi
echo "       TestOnlyVerifier deployed at: $VERIFIER_ADDR"

# Now deploy the gateway
if ! GATEWAY_OUTPUT=$(cd "$EVM_DIR" && forge create \
  --private-key "$ANVIL_PRIVATE_KEY" \
  --rpc-url http://127.0.0.1:8545 \
  --broadcast \
  "contracts/X3ExternalGateway.sol:X3ExternalGateway" \
  --constructor-args "$VERIFIER_ADDR" 1337 42 1 \
  2>&1); then
  echo "ERROR: Failed to deploy X3ExternalGateway"
  echo "$GATEWAY_OUTPUT"
  exit 1
fi
GATEWAY_ADDR=$(echo "$GATEWAY_OUTPUT" | grep 'Deployed to:' | awk '{print $3}')

if [ -z "$GATEWAY_ADDR" ]; then
  echo "ERROR: Failed to deploy X3ExternalGateway"
  echo "$GATEWAY_OUTPUT"
  exit 1
fi

echo "       X3ExternalGateway deployed at: $GATEWAY_ADDR"

# ── 4. Start x3-chain-node ──────────────────────────────────────────────────

echo "[4/5] Starting x3-chain-node..."
NODE_DIR=$(mktemp -d)
export X3_SUBMITTER_SEED="${X3_SUBMITTER_SEED:-//Alice}"
"$NODE_BIN" \
  --chain "$CHAIN_SPEC" \
  --alice \
  --tmp \
  --node-key 000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f \
  --unsafe-rpc-external \
  --rpc-cors all \
  --rpc-port 9944 \
  > "$NODE_DIR/node.log" 2>&1 &
NODE_PID=$!
echo "       Node PID=$NODE_PID"

# Wait for node to be ready (up to 90 seconds on slower machines)
NODE_READY=0
for i in $(seq 1 90); do
  if curl -s -X POST -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"system_health","params":[]}' \
    http://127.0.0.1:9944 2>/dev/null | grep -q '"result"'; then
    NODE_READY=1
    break
  fi
  if ! kill -0 "$NODE_PID" 2>/dev/null; then
    break
  fi
  sleep 1
done
if [ "$NODE_READY" -ne 1 ]; then
  echo "ERROR: x3-chain-node did not expose RPC on 127.0.0.1:9944"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "  Node Log (last 80 lines)"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  tail -80 "$NODE_DIR/node.log" 2>/dev/null || true
  exit 1
fi
echo "       Node ready"
echo "       X3 block height at startup: $(chain_number)"

# Wait for finality to advance a few blocks
sleep 5
ALICE_ACCOUNT_HEX="0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"
INITIAL_X3_BALANCE="$(canonical_balance "$ALICE_ACCOUNT_HEX")"
if [ -z "$INITIAL_X3_BALANCE" ]; then
  echo "ERROR: Unable to read initial X3 canonical ledger balance for Alice"
  tail -80 "$NODE_DIR/node.log" 2>/dev/null || true
  exit 1
fi
echo "       Initial Alice X3 canonical ledger balance: $INITIAL_X3_BALANCE"
echo "       X3 block height before deposit: $(chain_number)"
echo "       Kernel bridge state: $(kernel_bridge_state)"

# ── 5. Run relayer → trigger deposit → verify ───────────────────────────────

echo "[5/5] Starting relayer and executing atomic trade..."

# Configure relayer via env vars to point at Anvil + X3 node
export X3_RPC="http://127.0.0.1:9944"
export ETH_RPC="http://127.0.0.1:8545"
export ETH_GATEWAY="$GATEWAY_ADDR"
export X3_NETWORK="local"
export POLL_INTERVAL="2"

cat > "$ANVIL_DIR/relayer-local.yaml" <<EOF
x3:
  rpc_url: "http://127.0.0.1:9944"
  relayer_account: "local-e2e"
  relayer_seed_phrase: "local-e2e"

evm_chains:
  - name: "Anvil Local"
    chain_id: 1337
    x3_domain_id: 42
    rpc_endpoint: "http://127.0.0.1:8545"
    state_root_contract: "$GATEWAY_ADDR"
    finality_threshold: 1
    block_poll_interval_ms: 1000
    max_concurrent_requests: 1

svm_clusters: []

submission:
  batch_size: 1
  timeout_secs: 2
  max_retries: 1
  retry_backoff_ms: 1000

governance:
  poll_interval_secs: 2
  enable_graceful_shutdown: true

logging:
  level: "info"
  format: "default"
EOF
export X3_RELAYER_CONFIG="$ANVIL_DIR/relayer-local.yaml"

# Start relayer in background
"$RELAYER_BIN" > "$ANVIL_DIR/relayer.log" 2>&1 &
RELAYER_PID=$!
echo "       Relayer PID=$RELAYER_PID (polling every 2s)"

sleep 3

# ── Execute deposit on EVM gateway ───────────────────────────────────────────

DEPLOYER_PK="$ANVIL_PRIVATE_KEY"
TOKEN_OUTPUT=$(cd "$EVM_DIR" && forge create \
  --private-key "$DEPLOYER_PK" \
  --rpc-url http://127.0.0.1:8545 \
  --broadcast \
  "test/X3ExternalGateway.t.sol:MockERC20" \
  2>&1)
TOKEN_ADDR=$(echo "$TOKEN_OUTPUT" | grep 'Deployed to:' | awk '{print $3}')

if [ -z "$TOKEN_ADDR" ]; then
  echo "ERROR: Failed to deploy MockERC20"
  echo "$TOKEN_OUTPUT"
  exit 1
fi
echo "       MockERC20 deployed at: $TOKEN_ADDR"

INITIAL_WRAPPED_ACCOUNTING="$(wrapped_accounting_for_token "$ALICE_ACCOUNT_HEX" 1337 "$TOKEN_ADDR")"
INITIAL_WRAPPED_BALANCE="$(echo "$INITIAL_WRAPPED_ACCOUNTING" | jq -r '.balance // empty')"
INITIAL_WRAPPED_SUPPLY="$(echo "$INITIAL_WRAPPED_ACCOUNTING" | jq -r '.supply // empty')"
INITIAL_TOTAL_WRAPPED_SUPPLY="$(echo "$INITIAL_WRAPPED_ACCOUNTING" | jq -r '.total_supply // empty')"
if [ -z "$INITIAL_WRAPPED_BALANCE" ] || [ -z "$INITIAL_WRAPPED_SUPPLY" ] || [ -z "$INITIAL_TOTAL_WRAPPED_SUPPLY" ]; then
  echo "ERROR: Unable to read initial wrapped accounting for Alice/token"
  tail -80 "$NODE_DIR/node.log" 2>/dev/null || true
  exit 1
fi
echo "       Initial Alice wrapped balance: $INITIAL_WRAPPED_BALANCE"
echo "       Initial wrapped token supply: $INITIAL_WRAPPED_SUPPLY"
echo "       Initial total wrapped supply: $INITIAL_TOTAL_WRAPPED_SUPPLY"

cast send \
  --private-key "$DEPLOYER_PK" \
  --rpc-url http://127.0.0.1:8545 \
  "$TOKEN_ADDR" \
  "mint(address,uint256)" \
  "$ANVIL_DEPLOYER" 1000000000000000000000 >/dev/null

cast send \
  --private-key "$DEPLOYER_PK" \
  --rpc-url http://127.0.0.1:8545 \
  "$GATEWAY_ADDR" \
  "setSupportedToken(address,bool,uint256,uint256)" \
  "$TOKEN_ADDR" true 1000000000000000000000 1000000000000000000000 >/dev/null

cast send \
  --private-key "$DEPLOYER_PK" \
  --rpc-url http://127.0.0.1:8545 \
  "$TOKEN_ADDR" \
  "approve(address,uint256)" \
  "$GATEWAY_ADDR" 1000 >/dev/null

echo "       Calling depositToX3(MockERC20, 1000, Alice X3 account)..."
RECIPIENT_HEX="$ALICE_ACCOUNT_HEX"
cast send \
  --private-key "$DEPLOYER_PK" \
  --rpc-url http://127.0.0.1:8545 \
  "$GATEWAY_ADDR" \
  "depositToX3(address,bytes,uint256,uint256)" \
  "$TOKEN_ADDR" "$RECIPIENT_HEX" 1000 0 \
  2>&1 | head -5

echo "       Deposit transaction sent! Waiting for relayer to pick it up..."

# ── Wait for relayer to process ──────────────────────────────────────────────

sleep 8

FINAL_X3_BALANCE=""
FINAL_WRAPPED_ACCOUNTING=""
FINAL_WRAPPED_BALANCE=""
FINAL_WRAPPED_SUPPLY=""
FINAL_TOTAL_WRAPPED_SUPPLY=""
for i in $(seq 1 30); do
  FINAL_X3_BALANCE="$(canonical_balance "$ALICE_ACCOUNT_HEX" || true)"
  FINAL_WRAPPED_ACCOUNTING="$(wrapped_accounting_for_token "$ALICE_ACCOUNT_HEX" 1337 "$TOKEN_ADDR" || true)"
  FINAL_WRAPPED_BALANCE="$(echo "$FINAL_WRAPPED_ACCOUNTING" | jq -r '.balance // empty')"
  FINAL_WRAPPED_SUPPLY="$(echo "$FINAL_WRAPPED_ACCOUNTING" | jq -r '.supply // empty')"
  FINAL_TOTAL_WRAPPED_SUPPLY="$(echo "$FINAL_WRAPPED_ACCOUNTING" | jq -r '.total_supply // empty')"
  if [ -n "$FINAL_X3_BALANCE" ] \
    && [ -n "$FINAL_WRAPPED_BALANCE" ] \
    && [ -n "$FINAL_WRAPPED_SUPPLY" ] \
    && [ -n "$FINAL_TOTAL_WRAPPED_SUPPLY" ] \
    && [ "$FINAL_X3_BALANCE" -ge "$((INITIAL_X3_BALANCE + 1000))" ] \
    && [ "$FINAL_WRAPPED_BALANCE" -ge "$((INITIAL_WRAPPED_BALANCE + 1000))" ] \
    && [ "$FINAL_WRAPPED_SUPPLY" -ge "$((INITIAL_WRAPPED_SUPPLY + 1000))" ] \
    && [ "$FINAL_TOTAL_WRAPPED_SUPPLY" -ge "$((INITIAL_TOTAL_WRAPPED_SUPPLY + 1000))" ]; then
    break
  fi
  sleep 1
done

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
  2>/dev/null | grep -c 'blockHash' || true)

echo "       EVM DepositLocked events found: $DEPOSIT_EVENTS"
echo "       Final Alice X3 canonical ledger balance: ${FINAL_X3_BALANCE:-unavailable}"
echo "       Final Alice wrapped balance: ${FINAL_WRAPPED_BALANCE:-unavailable}"
echo "       Final wrapped token supply: ${FINAL_WRAPPED_SUPPLY:-unavailable}"
echo "       Final total wrapped supply: ${FINAL_TOTAL_WRAPPED_SUPPLY:-unavailable}"
echo "       Wrapped accounting: ${FINAL_WRAPPED_ACCOUNTING:-unavailable}"
echo "       X3 block height after relay: $(chain_number)"
echo "       Pending X3 extrinsics after relay: $(pending_extrinsic_count)"
echo "       Kernel bridge state after relay: $(kernel_bridge_state)"

# Check if the relayer logged a successful proof submission
if grep -q "Deposit proof submitted successfully" "$ANVIL_DIR/relayer.log" 2>/dev/null; then
  echo "       ✅ Relayer successfully submitted deposit proof to X3!"
else
  echo "       ⚠️  Relayer did not report successful proof submission"
fi

if [ "$DEPOSIT_EVENTS" -gt 0 ] \
  && grep -q "Deposit proof submitted successfully" "$ANVIL_DIR/relayer.log" 2>/dev/null \
  && [ -n "$FINAL_X3_BALANCE" ] \
  && [ -n "$FINAL_WRAPPED_BALANCE" ] \
  && [ -n "$FINAL_WRAPPED_SUPPLY" ] \
  && [ -n "$FINAL_TOTAL_WRAPPED_SUPPLY" ] \
  && [ "$FINAL_X3_BALANCE" -ge "$((INITIAL_X3_BALANCE + 1000))" ] \
  && [ "$FINAL_WRAPPED_BALANCE" -ge "$((INITIAL_WRAPPED_BALANCE + 1000))" ] \
  && [ "$FINAL_WRAPPED_SUPPLY" -ge "$((INITIAL_WRAPPED_SUPPLY + 1000))" ] \
  && [ "$FINAL_TOTAL_WRAPPED_SUPPLY" -ge "$((INITIAL_TOTAL_WRAPPED_SUPPLY + 1000))" ]; then
  echo "       ✅ X3 canonical ledger balance increased by at least 1000 after relay"
  echo "       ✅ X3 wrapped balance and supply increased by at least 1000 after relay"
  RESULT="PASSED"
else
  echo "       ⚠️  X3 canonical ledger or wrapped accounting did not increase after relay"
  RESULT="PARTIAL"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  E2E Atomic Trade Test: $RESULT"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ "$RESULT" != "PASSED" ]; then
  echo ""
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "  Node Log (last 120 lines)"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  tail -120 "$NODE_DIR/node.log" 2>/dev/null || true
fi

# ── Cleanup ──────────────────────────────────────────────────────────────────

echo ""
echo "  Cleaning up background processes..."
cleanup
wait 2>/dev/null || true
echo "  Done."

if [ "$RESULT" != "PASSED" ]; then
  exit 1
fi
