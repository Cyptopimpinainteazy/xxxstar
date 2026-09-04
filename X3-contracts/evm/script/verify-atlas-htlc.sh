#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# verify-atlas-htlc.sh — Verify deployed AtlasHTLC contract on EVM explorer
#
# Usage:
#   ATLAS_HTLC_ADDRESS=0x... ETHERSCAN_API_KEY=... \
#     bash script/verify-atlas-htlc.sh [chain_id] [explorer]
#
# Environment variables:
#   ATLAS_HTLC_ADDRESS   — Required. Address of the deployed AtlasHTLC.
#   ETHERSCAN_API_KEY    — Required for Etherscan verification.
#   VERIFIER_URL         — Custom Blockscout verifier URL (omit for Etherscan).
#
# Examples:
#   # Verify on Sepolia (Etherscan)
#   ATLAS_HTLC_ADDRESS=0x... ETHERSCAN_API_KEY=... \
#     bash script/verify-atlas-htlc.sh 11155111 etherscan
#
#   # Verify on Holesky (Etherscan)
#   ATLAS_HTLC_ADDRESS=0x... ETHERSCAN_API_KEY=... \
#     bash script/verify-atlas-htlc.sh 17000 etherscan
#
#   # Verify on Blockscout (e.g., local/testnet)
#   ATLAS_HTLC_ADDRESS=0x... VERIFIER_URL=https://blockscout.example.com/api/ \
#     bash script/verify-atlas-htlc.sh 31337 blockscout
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

CHAIN_ID="${1:-11155111}"          # Sepolia default
EXPLORER="${2:-etherscan}"         # etherscan | blockscout

ATLAS_ADDR="${ATLAS_HTLC_ADDRESS:-}"
API_KEY="${ETHERSCAN_API_KEY:-}"

if [[ -z "$ATLAS_ADDR" ]]; then
    echo "❌ ATLAS_HTLC_ADDRESS is required"
    echo "   Usage: ATLAS_HTLC_ADDRESS=0x... bash script/verify-atlas-htlc.sh [chain_id] [explorer]"
    exit 1
fi

echo "=== AtlasHTLC Verification ==="
echo "   Contract:  $ATLAS_ADDR"
echo "   Chain ID:  $CHAIN_ID"
echo "   Explorer:  $EXPLORER"
echo ""

# AtlasHTLC has no constructor arguments, so --constructor-args is omitted.
if [[ "$EXPLORER" == "blockscout" ]]; then
    BLOCKSCOUT_URL="${VERIFIER_URL:-}"
    if [[ -z "$BLOCKSCOUT_URL" ]]; then
        echo "❌ VERIFIER_URL required for blockscout verifier"
        exit 1
    fi
    forge verify-contract \
        --watch \
        --chain "$CHAIN_ID" \
        --verifier blockscout \
        --verifier-url "$BLOCKSCOUT_URL" \
        "$ATLAS_ADDR" \
        "contracts/AtlasHTLC.sol:AtlasHTLC"
else
    if [[ -z "$API_KEY" ]]; then
        echo "❌ ETHERSCAN_API_KEY required for etherscan verifier"
        exit 1
    fi
    forge verify-contract \
        --watch \
        --chain "$CHAIN_ID" \
        --verifier etherscan \
        --etherscan-api-key "$API_KEY" \
        "$ATLAS_ADDR" \
        "contracts/AtlasHTLC.sol:AtlasHTLC"
fi

echo ""
echo "✅ AtlasHTLC verification submitted successfully!"
echo "   Check pending status with:"
echo "   forge verify-check --chain $CHAIN_ID --explorer $EXPLORER \"$ATLAS_ADDR\""
