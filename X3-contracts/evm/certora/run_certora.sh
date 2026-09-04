#!/bin/bash
set -e

EVM_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CERTORA_DIR="$EVM_DIR/certora"

echo "=== X3 Certora Prover Verification ==="
echo "Requires: CERTORAKEY environment variable"
echo ""

if [ -z "$CERTORAKEY" ]; then
    echo "ERROR: CERTORAKEY not set. Get a license at https://www.certora.com"
    echo "Then run: export CERTORAKEY=<your-key>"
    echo ""
    echo "Spec files created at:"
    echo "  $CERTORA_DIR/specs/GatewayRules.spec"
    echo "  $CERTORA_DIR/specs/TreasuryRules.spec"
    echo "  $CERTORA_DIR/specs/StakingPoolRules.spec"
    exit 1
fi

echo "--- Verifying X3ExternalGateway ---"
certoraRun "$EVM_DIR/contracts/X3ExternalGateway.sol:X3ExternalGateway" \
    --verify X3ExternalGateway:"$CERTORA_DIR/specs/GatewayRules.spec" \
    --solc solc0.8.24 \
    --optimistic_loop \
    --msg "X3 Gateway Formal Verification"

echo "--- Verifying Treasury ---"
certoraRun "$EVM_DIR/contracts/treasury/Treasury.sol:Treasury" \
    --verify Treasury:"$CERTORA_DIR/specs/TreasuryRules.spec" \
    --solc solc0.8.24 \
    --msg "X3 Treasury Formal Verification"

echo "--- Verifying StakingPool ---"
certoraRun "$EVM_DIR/contracts/staking/StakingPool.sol:StakingPool" \
    --verify StakingPool:"$CERTORA_DIR/specs/StakingPoolRules.spec" \
    --solc solc0.8.24 \
    --msg "X3 StakingPool Formal Verification"

echo "=== All Certora verifications submitted ==="
