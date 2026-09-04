#!/bin/bash

# X3 Chain MCP - Quick API Keys Setup Script
# This script helps you set up the essential API keys for development

echo "🔑 X3 Chain MCP - API Keys Setup"
echo "===================================="
echo ""

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}IMMEDIATE - Get These First (Free):${NC}"
echo "===================================="

echo ""
echo -e "${GREEN}1. Infura Project ID${NC}"
echo "   → Go to: https://infura.io"
echo "   → Sign up for free account"
echo "   → Create new project"
echo "   → Copy Project ID"
echo "   → Replace YOUR_PROJECT_ID in mcp-config.json"
echo ""

echo -e "${GREEN}2. CoinGecko API Key (Demo)${NC}"
echo "   → Demo key: CG-1234567890abcdefghijklmnopqrstuvwxyz"
echo "   → Or get free key: https://www.coingecko.com/en/api"
echo ""

echo -e "${GREEN}3. Dune Analytics API Key${NC}"
echo "   → Get from: https://dune.com/api"
echo "   → Free tier available"
echo ""

echo -e "${GREEN}4. GoPlus Security API Key${NC}"
echo "   → Get from: https://gopluslabs.io"
echo "   → Free tier available"
echo ""

echo ""
echo -e "${YELLOW}Generate These Keys Locally:${NC}"
echo "============================"

echo -e "${GREEN}5. Generate Wallet Keystore${NC}"
echo "   → This will be your main X3 wallet"
echo '   → Run: node -e "console.log(require('crypto').randomBytes(32).toString('hex'))"'
echo ""

echo -e "${GREEN}6. Generate PostgreSQL Password${NC}"
echo "   → Create strong password for database"
echo '   → Run: openssl rand -hex 32'
echo ""

echo -e "${GREEN}7. Generate Git SSH Key${NC}"
echo "   → For repository access"
echo "   → Run: ssh-keygen -t ed25519 -C 'x3-chain'"
echo ""

echo ""
echo -e "${YELLOW}OPTIONAL - For Production:${NC}"
echo "=========================="
echo "→ Ethereum RPC Key (Alchemy/QuickNode)"
echo "→ Solana RPC Key (Helius/GenesysGo)"
echo "→ OpenAI API Key (for LLM features)"
echo "→ Multi-signature wallet setup"
echo ""

echo ""
echo -e "${RED}SECURITY REMINDERS:${NC}"
echo "==================="
echo "⚠️  Never commit API keys to repositories"
echo "⚠️  Use Vault for all secrets in production"
echo "⚠️  Set up monitoring for API usage"
echo "⚠️  Rotate keys regularly"
echo ""

echo ""
echo -e "${GREEN}Quick validation:${NC}"
echo "→ Test with minimal services first"
echo "→ Start with testnets (Goerli, Sepolia)"
echo "→ Monitor costs and rate limits"
echo ""

echo "Setup complete! Check API_KEYS_REQUIRED.md for detailed instructions."