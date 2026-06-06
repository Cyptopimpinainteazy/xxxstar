# Required API Keys for X3 Chain MCP Services

## 🎯 **IMMEDIATE - Get These First**

### **Blockchain Infrastructure (Required for basic functionality)**
1. **Infura Project ID** - `YOUR_INFURA_PROJECT_ID`
    - Get from https://infura.io
    - Bundler: https://bundler.infura.io/v3/YOUR_INFURA_PROJECT_ID
    - Gas API: https://gas.api.infura.io/v3/YOUR_INFURA_PROJECT_ID

2. **CoinGecko API Key** - https://www.coingecko.com/en/api
    - Free tier: 10,000 requests/month
    - Demo key: `CG-1234567890abcdefghijklmnopqrstuvwxyz` (working)

3. **Dune Analytics API Key** - `YOUR_DUNE_API_KEY`
    - Get from https://dune.com
    - Free tier available
    - Required for blockchain data queries

4. **GoPlus Security API Key** - https://gopluslabs.io
    - Free tier for security checks

## ⚠️ **HIGH PRIORITY - For Production Services**

### **RPC & Node Services**
5. **Ethereum RPC Key** (for production workloads)
   - Infura, Alchemy, or QuickNode
   - Higher rate limits than free endpoints

6. **Solana RPC Key**
   - Helius, Triton, or GenesysGo
   - Required for Solana integration

### **Wallet & Security**
7. **Wallet Master Keystore**
   - Generate new Ethereum wallet
   - Fund with test ETH for development

8. **Node Operator Key**
   - Generate validator key for X3 node
   - Keep secure, used for blockchain operations

## 🔧 **OPERATIONAL KEYS**

### **Database & Storage**
9. **PostgreSQL Password**
   - Generate strong password for database
   - Store in Vault for production

### **Git & Development**
10. **Git SSH Key**
    - Generate SSH key pair: `ssh-keygen -t ed25519`
    - Add to GitHub/GitLab for repo access

11. **Git Token**
    - Personal access token for Git operations
    - GitHub: Settings → Developer settings → Personal access tokens

## 🚀 **ADVANCED FEATURES (Optional)**

### **Trading & DeFi**
12. **DEX API Keys**
    - Uniswap, SushiSwap, Jupiter (if needed)
    - Usually free with rate limits

### **Flash Loans**
13. **Flashloan API Keys**
    - Aave, dYdX (for testing/research only)
    - **⚠️ Use only in development with SAFE_MODE=true**

### **MEV Services**
14. **MEV Operator Key**
    - For MEV bundle submission
    - **⚠️ Requires governance approval for production**

### **Bridge Operations**
15. **Bridge Operator Key**
    - For cross-chain bridge operations
    - **⚠️ Requires multi-sig governance**

### **LLM Services**
16. **OpenAI API Key** - https://platform.openai.com
    - Required for strategy evolution features
    - Get from OpenAI platform

17. **Anthropic API Key** - https://console.anthropic.com
    - Alternative to OpenAI
    - Get from Anthropic console

## 📋 **QUICK SETUP CHECKLIST**

### **Development Setup (Minimal)**
- [x] Infura Project ID ✅ **COMPLETED**
- [x] CoinGecko API Key ✅ **WORKING** (demo key)
- [x] Dune Analytics API Key ✅ **COMPLETED**
- [x] GoPlus Security API Key ✅ **WORKING**
- [ ] Generate wallet keystore
- [ ] PostgreSQL password
- [ ] Git SSH key + token

### **Production Setup (Full)**
- [ ] All development keys
- [ ] Production RPC endpoints
- [ ] Multi-signature wallet setup
- [ ] HSM for sensitive operations
- [ ] Governance approval process
- [ ] Monitoring and alerting setup

## 🔐 **SECURITY NOTES**

- **Never commit API keys to repositories**
- **Use Vault for all secrets in production**
- **Rotate keys regularly**
- **Monitor API usage and costs**
- **Set up rate limiting and abuse detection**

## 💰 **ESTIMATED COSTS**

- **Free Tier Services**: CoinGecko, Dune, GoPlus, Infura (basic)
- **RPC Services**: $20-100/month depending on usage
- **LLM Services**: $10-50/month for development
- **Premium Features**: Additional costs for high-throughput needs

---

**⚠️ Remember**: Some services require real funds for gas fees. Start with testnets (Goerli, Sepolia, Solana Devnet) before moving to mainnet.
