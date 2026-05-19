#!/bin/bash
# Example: Jury Decision Verification Script
# Usage: ./verify_jury_decision.sh <session_id> <expected_hash>

set -euo pipefail

RPC_URL="http://localhost:9944"
JURY_SERVICE_URL="http://localhost:8080"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Validate arguments
if [ $# -lt 2 ]; then
    echo "Usage: $0 <session_id> <expected_hash>"
    echo "Example: $0 session-20260208-001 0xabc123..."
    exit 1
fi

SESSION_ID="$1"
EXPECTED_HASH="$2"

echo "=================================================="
echo "Jury Decision Verification"
echo "=================================================="
echo "Session ID:     $SESSION_ID"
echo "Expected Hash:  $EXPECTED_HASH"
echo ""

# Step 1: Check RPC connectivity
echo "1. Checking RPC connectivity..."
if curl -s -X POST "$RPC_URL" \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}' | grep -q "isSynced"; then
    echo -e "   ${GREEN}✓${NC} RPC connected"
else
    echo -e "   ${RED}✗${NC} RPC not responding"
    exit 1
fi

# Step 2: Check jury service
echo "2. Checking jury service..."
if curl -s "$JURY_SERVICE_URL/health" | grep -q "ok"; then
    echo -e "   ${GREEN}✓${NC} Jury service healthy"
else
    echo -e "   ${YELLOW}⚠${NC} Jury service not fully ready"
fi

# Step 3: Query decision status from service
echo "3. Querying decision status..."
DECISION_STATUS=$(curl -s "$JURY_SERVICE_URL/api/anchor/$SESSION_ID/status")

STATUS=$(echo "$DECISION_STATUS" | grep -o '"status":"[^"]*"' | cut -d'"' -f4)
BLOCK_NUMBER=$(echo "$DECISION_STATUS" | grep -o '"block_number":[0-9]*' | cut -d':' -f2)

echo "   Status: $STATUS"
if [ -n "$BLOCK_NUMBER" ]; then
    echo "   Block: #$BLOCK_NUMBER"
fi

# Step 4: Query blockchain for on-chain hash
echo "4. Querying blockchain for on-chain hash..."
RPC_RESULT=$(curl -s -X POST "$RPC_URL" \
    -H "Content-Type: application/json" \
    -d "{
        \"jsonrpc\": \"2.0\",
        \"method\": \"query.atlasJuryAnchor.getJuryDecision\",
        \"params\": [\"$SESSION_ID\"],
        \"id\": 1
    }")

ON_CHAIN_HASH=$(echo "$RPC_RESULT" | grep -o '"decision_hash":"[^"]*"' | cut -d'"' -f4)

if [ -z "$ON_CHAIN_HASH" ]; then
    echo -e "   ${RED}✗${NC} No decision found on blockchain"
    exit 1
else
    echo -e "   ${GREEN}✓${NC} Found on-chain: ${ON_CHAIN_HASH:0:20}..."
fi

# Step 5: Verify hashes match
echo "5. Verifying hash match..."
if [ "$ON_CHAIN_HASH" = "$EXPECTED_HASH" ]; then
    echo -e "   ${GREEN}✓ VERIFIED${NC} - Hashes match!"
else
    echo -e "   ${RED}✗ MISMATCH${NC}"
    echo "      Expected: $EXPECTED_HASH"
    echo "      On-Chain: $ON_CHAIN_HASH"
    exit 1
fi

# Step 6: Check decision events
echo "6. Checking decision events..."
EVENTS=$(curl -s -X POST "$RPC_URL" \
    -H "Content-Type: application/json" \
    -d "{
        \"jsonrpc\": \"2.0\",
        \"method\": \"query.system.events\",
        \"params\": [],
        \"id\": 1
    }")

if echo "$EVENTS" | grep -q "JuryDecisionAnchored"; then
    echo -e "   ${GREEN}✓${NC} JuryDecisionAnchored event found"
else
    echo -e "   ${YELLOW}⚠${NC} No events found"
fi

echo ""
echo "=================================================="
echo -e "${GREEN}✓ SUCCESS${NC} - Decision verified!"
echo "=================================================="
echo ""
echo "Summary:"
echo "  Session:    $SESSION_ID"
echo "  Hash:       $ON_CHAIN_HASH"
echo "  Status:     $STATUS"
if [ -n "$BLOCK_NUMBER" ]; then
    echo "  Block:      #$BLOCK_NUMBER"
fi
echo ""
echo "Next Steps:"
echo "  - View decision on blockchain: https://explorer.x3.io/block/$BLOCK_NUMBER"
echo "  - Share verification proof with stakeholders"
echo "  - Archive decision record"
