#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# verify-evm-contracts.sh — Verify deployed X3 EVM contracts on target explorer
#
# Usage:
#   # Verify Verifier:
#   VERIFIER_ADDRESS=0x... ETHERSCAN_API_KEY=... \
#     bash X3-contracts/evm/script/verify-gateway.sh verifier [chain_id] [explorer]
#
#   # Verify Gateway:
#   GATEWAY_ADDRESS=0x... VERIFIER_ADDRESS=0x... ETHERSCAN_API_KEY=... \
#     bash X3-contracts/evm/script/verify-gateway.sh gateway [chain_id] [explorer]
#
# Environment variables:
#   For Verifier: VERIFIER_ADDRESS, ETHERSCAN_API_KEY
#   For Gateway: GATEWAY_ADDRESS, VERIFIER_ADDRESS, X3_CHAIN_ID (200),
#                MIN_X3_CONFIRMATIONS (12), ETHERSCAN_API_KEY
#   VERIFIER_URL         — Custom Blockscout verifier URL (omit for Etherscan)
#   VALIDATOR_PUBKEYS     — Comma-separated pubkeys (for verifier constructor args)
#   QUORUM_THRESHOLD      — Quorum threshold (for verifier constructor args)
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

CONTRACT_TYPE="${1:-gateway}"
CHAIN_ID="${2:-11155111}"          # Sepolia default
EXPLORER="${3:-etherscan}"         # etherscan | blockscout

API_KEY="${ETHERSCAN_API_KEY:-}"

verify_with_forge() {
    local contract_addr="$1"
    local constructor_args="$2"
    local contract_path="$3"

    echo "   Contract:  $contract_addr"
    echo "   Chain ID:  $CHAIN_ID"
    echo "   Explorer:  $EXPLORER"
    echo "   Constructor args: $constructor_args"
    echo ""

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
            --constructor-args "$constructor_args" \
            "$contract_addr" \
            "$contract_path"
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
            --constructor-args "$constructor_args" \
            "$contract_addr" \
            "$contract_path"
    fi
}

if [[ "$CONTRACT_TYPE" == "verifier" ]]; then
    # ── Verify EvmReceiptVerifier ─────────────────────────────────────────
    VERIFIER_ADDR="${VERIFIER_ADDRESS:-}"
    PUBKEYS="${VALIDATOR_PUBKEYS:-}"
    QUORUM="${QUORUM_THRESHOLD:-0}"

    if [[ -z "$VERIFIER_ADDR" ]]; then
        echo "❌ VERIFIER_ADDRESS is required for verifier verification"
        exit 1
    fi
    if [[ -z "$PUBKEYS" ]]; then
        echo "❌ VALIDATOR_PUBKEYS is required for verifier constructor args"
        exit 1
    fi

    echo "=== Verifying EvmReceiptVerifier ==="

    # Parse comma-separated pubkeys into a Foundry cast array: [<pk0>,<pk1>,...]
    IFS=',' read -ra PK_ARRAY <<< "$PUBKEYS"
    PUBKEY_LIST=""
    for pk in "${PK_ARRAY[@]}"; do
        pk=$(echo "$pk" | xargs)
        if [[ -n "$PUBKEY_LIST" ]]; then
            PUBKEY_LIST="$PUBKEY_LIST,"
        fi
        PUBKEY_LIST="${PUBKEY_LIST}$pk"
    done

    # Default quorum: ceil(2/3 * count)
    if [[ "$QUORUM" -eq 0 ]]; then
        QUORUM=$(( (${#PK_ARRAY[@]} * 2) / 3 + 1 ))
    fi

    CONSTRUCTOR_ARGS=$(cast abi-encode \
        "constructor(bytes32[],uint256)" \
        "[$PUBKEY_LIST]" "$QUORUM")

    verify_with_forge \
        "$VERIFIER_ADDR" \
        "$CONSTRUCTOR_ARGS" \
        "contracts/EvmReceiptVerifier.sol:EvmReceiptVerifier"

elif [[ "$CONTRACT_TYPE" == "gateway" ]]; then
    # ── Verify X3ExternalGateway ──────────────────────────────────────────
    GATEWAY="${GATEWAY_ADDRESS:-}"
    VERIFIER="${VERIFIER_ADDRESS:-}"
    X3_ID="${X3_CHAIN_ID:-200}"
    MIN_CONFS="${MIN_X3_CONFIRMATIONS:-12}"

    if [[ -z "$GATEWAY" ]]; then
        echo "❌ GATEWAY_ADDRESS is required for gateway verification"
        exit 1
    fi
    if [[ -z "$VERIFIER" ]]; then
        echo "❌ VERIFIER_ADDRESS is required for gateway constructor args"
        exit 1
    fi

    echo "=== Verifying X3ExternalGateway ==="

    CONSTRUCTOR_ARGS=$(cast abi-encode \
        "constructor(address,uint256,uint256,uint256)" \
        "$VERIFIER" "$CHAIN_ID" "$X3_ID" "$MIN_CONFS")

    verify_with_forge \
        "$GATEWAY" \
        "$CONSTRUCTOR_ARGS" \
        "contracts/X3ExternalGateway.sol:X3ExternalGateway"

else
    echo "❌ Unknown contract type: $CONTRACT_TYPE (expected 'verifier' or 'gateway')"
    exit 1
fi

echo ""
echo "=== Verification submitted ==="
echo "Check explorer for verified source badge."
