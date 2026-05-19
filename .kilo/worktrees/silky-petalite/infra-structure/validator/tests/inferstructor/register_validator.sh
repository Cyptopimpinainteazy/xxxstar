#!/usr/bin/env bash
# Quick registration script for validators
# Usage: ./register_validator.sh <chain> <email> [sla_tier]

set -euo pipefail

CHAIN="${1:-}"
EMAIL="${2:-}"
SLA_TIER="${3:-pro}"

REGISTRY_URL="${REGISTRY_URL:-http://localhost:7001}"

if [ -z "$CHAIN" ] || [ -z "$EMAIL" ]; then
    echo "Usage: $0 <chain> <email> [sla_tier]"
    echo ""
    echo "Examples:"
    echo "  $0 solana validator@example.com pro"
    echo "  $0 ethereum validator@example.com enterprise"
    echo "  $0 arbitrum validator@example.com basic"
    echo ""
    echo "SLA Tiers:"
    echo "  basic      - 100K TPS, \$10/M tx"
    echo "  pro        - 1M TPS, \$50/M tx (default)"
    echo "  enterprise - Unlimited TPS, \$200/M tx"
    exit 1
fi

echo "🚀 Registering validator for $CHAIN..."
echo "📧 Email: $EMAIL"
echo "🎯 SLA Tier: $SLA_TIER"
echo ""

# Check if registry is running
if ! curl -sf "$REGISTRY_URL/health" > /dev/null 2>&1; then
    echo "❌ Registry service not running at $REGISTRY_URL"
    echo ""
    echo "Start it with:"
    echo "  cd cross-chain-gpu-validator/tests/inferstructor"
    echo "  python3 validator_registry.py &"
    exit 1
fi

# Register
RESPONSE=$(curl -sf -X POST "$REGISTRY_URL/api/validators/register" \
    -H "Content-Type: application/json" \
    -d "{
        \"chain\": \"$CHAIN\",
        \"email\": \"$EMAIL\",
        \"sla_tier\": \"$SLA_TIER\"
    }")

if [ $? -ne 0 ]; then
    echo "❌ Registration failed"
    exit 1
fi

# Parse response
VALIDATOR_ID=$(echo "$RESPONSE" | python3 -c "import sys, json; print(json.load(sys.stdin)['credentials']['validator_id'])")
API_KEY=$(echo "$RESPONSE" | python3 -c "import sys, json; print(json.load(sys.stdin)['credentials']['api_key'])")
API_SECRET=$(echo "$RESPONSE" | python3 -c "import sys, json; print(json.load(sys.stdin)['credentials']['api_secret'])")
MAX_TPS=$(echo "$RESPONSE" | python3 -c "import sys, json; print(json.load(sys.stdin)['credentials']['max_tps'])")
JWT_TOKEN=$(echo "$RESPONSE" | python3 -c "import sys, json; print(json.load(sys.stdin)['credentials']['jwt_token'])")

echo "✅ Registration successful!"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔑 SAVE THESE CREDENTIALS (they won't be shown again):"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Validator ID: $VALIDATOR_ID"
echo "API Key:      $API_KEY"
echo "API Secret:   $API_SECRET"
echo "Max TPS:      $MAX_TPS"
echo "JWT Token:    ${JWT_TOKEN:0:50}..."
echo ""

# Save to .env file
ENV_FILE=".env.validator.$VALIDATOR_ID"
cat > "$ENV_FILE" <<EOF
# Inferstructor Validator Credentials
# Generated: $(date)
# Validator ID: $VALIDATOR_ID
# Chain: $CHAIN
# SLA Tier: $SLA_TIER

export INFRA_VALIDATOR_ID="$VALIDATOR_ID"
export INFRA_API_KEY="$API_KEY"
export INFRA_API_SECRET="$API_SECRET"
export INFRA_JWT_TOKEN="$JWT_TOKEN"
export INFRA_MAX_TPS="$MAX_TPS"

# Endpoints
export INFRA_BRIDGE_URL="http://localhost:9999"
export INFRA_TOLL_BOOTH_URL="http://localhost:7000"
export INFRA_DASHBOARD_URL="http://localhost:8080"
EOF

echo "💾 Credentials saved to: $ENV_FILE"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📋 NEXT STEPS:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "1. Load credentials:"
echo "   source $ENV_FILE"
echo ""
echo "2. Test connection:"
echo "   curl -H \"X-API-Key: \$INFRA_API_KEY\" http://localhost:9999/health"
echo ""
echo "3. Send test transaction:"
echo "   curl -X POST http://localhost:9999/accelerate \\"
echo "     -H \"X-API-Key: \$INFRA_API_KEY\" \\"
echo "     -H \"Content-Type: application/json\" \\"
echo "     -d '{\"tx_hash\": \"test123\", \"tx_data\": \"48656c6c6f\", \"chain\": \"$CHAIN\"}'"
echo ""
echo "4. Run full performance test:"
echo "   cd cross-chain-gpu-validator/tests/inferstructor"
echo "   ./run_300x_test.sh --duration 5m"
echo ""
echo "5. View dashboard:"
echo "   open http://localhost:8080"
echo ""
echo "📚 Full documentation: VALIDATOR_QUICKSTART.md"
echo ""
