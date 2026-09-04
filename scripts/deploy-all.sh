#!/usr/bin/env bash
set -euo pipefail

# Deploy contracts to all 103 chains.
# Configure required values through environment variables before running.
# Example:
#   export TREASURY_WALLET=0x...
#   export UNIVERSAL_ADAPTER=0x...
#   export STAKING_TOKEN_ADDRESS=0x...
#   export DEV_WALLET=0x...
#   export DAO_WALLET=0x...
#   export LP_WALLET=0x...
#   export X3_ADDRESS=0x...
#   export RPC_ENDPOINT_FOR_CHAIN_1=https://...

CHAINS=($(seq 1 103))

TREASURY="${TREASURY_WALLET:-}"
ADAPTER="${UNIVERSAL_ADAPTER:-}"
STAKING_TOKEN="${STAKING_TOKEN_ADDRESS:-}"
DEV_WALLET="${DEV_WALLET:-}"
DAO_WALLET="${DAO_WALLET:-}"
LP_WALLET="${LP_WALLET:-}"
X3_ADDRESS="${X3_ADDRESS:-}"

if [[ -z "$TREASURY" || -z "$STAKING_TOKEN" || -z "$DEV_WALLET" || -z "$DAO_WALLET" || -z "$LP_WALLET" || -z "$X3_ADDRESS" ]]; then
  echo "Missing required environment variables."
  echo "Required: TREASURY_WALLET, STAKING_TOKEN_ADDRESS, DEV_WALLET, DAO_WALLET, LP_WALLET, X3_ADDRESS"
  exit 1
fi

for CHAIN in "${CHAINS[@]}"; do
  echo "Deploying to chain $CHAIN..."

  rpc_var="RPC_ENDPOINT_FOR_CHAIN_${CHAIN}"
  RPC_URL="${!rpc_var:-}"
  if [[ -z "$RPC_URL" ]]; then
    echo "Skipping chain $CHAIN: missing $rpc_var"
    continue
  fi

  forge create --rpc-url "$RPC_URL" --constructor-args "$TREASURY" contracts/AtlasSphereX3.sol:AtlasSphereX3
  forge create --rpc-url "$RPC_URL" --constructor-args "$TREASURY" "$ADAPTER" "$CHAIN" contracts/WrappedX3.sol:WrappedX3
  forge create --rpc-url "$RPC_URL" adapters/UniversalAdapter.sol:UniversalAdapter
  forge create --rpc-url "$RPC_URL" --constructor-args "$ADAPTER" "$TREASURY" bridges/AtomicBridge.sol:AtomicBridge
  forge create --rpc-url "$RPC_URL" --constructor-args "$STAKING_TOKEN" "$TREASURY" staking/StakingPool.sol:StakingPool
  forge create --rpc-url "$RPC_URL" --constructor-args "$DEV_WALLET" "$DAO_WALLET" "$LP_WALLET" treasury/Treasury.sol:Treasury
  forge create --rpc-url "$RPC_URL" --constructor-args "$X3_ADDRESS" "$TREASURY" governance/CrossChainGovernance.sol:CrossChainGovernance

  echo "Chain $CHAIN deployment complete."
done
