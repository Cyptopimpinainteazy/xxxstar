# X3 Chain Testnet v1 - Quick Start Guide

**⚡ Get started with X3 Chain Testnet in 5 minutes!**

---

## 🌐 Network Information

| **Parameter**          | **Value**                                  |
|------------------------|--------------------------------------------|
| **Network Name**       | X3 Chain Testnet v1                    |
| **Chain ID**           | `x3-testnet`                            |
| **RPC Endpoint**       | `http://rpc.testnet.x3-chain.io:9944`  |
| **WebSocket**          | Coming Soon                                |
| **Faucet**             | `https://faucet.testnet.x3-chain.io`   |
| **Block Time**         | ~6 seconds                                 |
| **Consensus**          | Aura + GRANDPA                             |

---

## 💰 Get Test Tokens

### Option 1: Web Faucet (Recommended)
1. Visit: `https://faucet.testnet.x3-chain.io`
2. Enter your account address
3. Complete captcha
4. Receive **100 tATLAS** instantly

### Option 2: Discord Bot
```
!faucet <your-address>
```
Join Discord: https://discord.gg/x3-chain

**Limits:** 100 tATLAS per request, 1 request per 24 hours per address

---

## 🔌 Connect via RPC

### Health Check
```bash
curl -X POST http://rpc.testnet.x3-chain.io:9944 \
  -H "Content-Type: application/json" \
  -d '{
    "id": 1,
    "jsonrpc": "2.0",
    "method": "system_health",
    "params": []
  }'
```

**Expected Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "isSyncing": false,
    "peers": 12,
    "shouldHavePeers": true
  },
  "id": 1
}
```

### Get Chain Info
```bash
curl -X POST http://rpc.testnet.x3-chain.io:9944 \
  -H "Content-Type: application/json" \
  -d '{
    "id": 1,
    "jsonrpc": "2.0",
    "method": "system_chain",
    "params": []
  }'
```

---

## 🧪 Try X3 Kernel RPC Methods

### 1. Get Canonical Balance
```bash
curl -X POST http://rpc.testnet.x3-chain.io:9944 \
  -H "Content-Type: application/json" \
  -d '{
    "id": 1,
    "jsonrpc": "2.0",
    "method": "atlasKernel_getCanonicalBalance",
    "params": [
      "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
      1,
      null
    ]
  }'
```

**Params:**
- `account` (SS58 address): Account to query
- `asset_id` (number): Asset ID (1 = native token)
- `at` (optional block hash): Query at specific block

### 2. List Authorized Accounts
```bash
curl -X POST http://rpc.testnet.x3-chain.io:9944 \
  -H "Content-Type: application/json" \
  -d '{
    "id": 1,
    "jsonrpc": "2.0",
    "method": "atlasKernel_getAuthorizedAccounts",
    "params": [null]
  }'
```

### 3. Check Authorization Status
```bash
curl -X POST http://rpc.testnet.x3-chain.io:9944 \
  -H "Content-Type: application/json" \
  -d '{
    "id": 1,
    "jsonrpc": "2.0",
    "method": "atlasKernel_isAuthorized",
    "params": [
      "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
      null
    ]
  }'
```

### 4. Get Current Validators
```bash
curl -X POST http://rpc.testnet.x3-chain.io:9944 \
  -H "Content-Type: application/json" \
  -d '{
    "id": 1,
    "jsonrpc": "2.0",
    "method": "atlasKernel_getAuthorities",
    "params": [null]
  }'
```

### 5. Get Asset Metadata
```bash
curl -X POST http://rpc.testnet.x3-chain.io:9944 \
  -H "Content-Type: application/json" \
  -d '{
    "id": 1,
    "jsonrpc": "2.0",
    "method": "atlasKernel_getAssetMetadata",
    "params": [1, null]
  }'
```

---

## 📦 Submit a Comit (Cross-Domain Transaction)

### Step 1: Create Comit Payload
```json
{
  "nonce": 1,
  "evm_calls": [
    {
      "to": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
      "value": "1000000000000000000",
      "data": "0x"
    }
  ],
  "svm_instructions": [],
  "fee": 10000,
  "signature": "0x..."
}
```

### Step 2: Submit via Extrinsic
```bash
# Using polkadot.js CLI
polkadot-js-api tx.atlasKernel.submitComit \
  --seed "//Alice" \
  --params '[{"nonce":1,"evm_calls":[],"svm_instructions":[],"fee":10000}]' \
  --ws ws://rpc.testnet.x3-chain.io:9944
```

**Note:** WebSocket support coming soon; use local node for now.

---

## 🛠️ Run Local Node (Connect to Testnet)

### Option 1: Binary Release
```bash
# Download latest release
wget https://github.com/x3-chain/x3-chain-node/releases/latest/download/x3-chain-node-linux-amd64

# Make executable
chmod +x x3-chain-node-linux-amd64

# Run testnet sync
./x3-chain-node-linux-amd64 \
  --chain testnet \
  --bootnodes /dns/bootnode.testnet.x3-chain.io/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp \
  --rpc-port 9944
```

### Option 2: Build from Source
```bash
# Clone repository
git clone https://github.com/x3-chain/x3-chain-node.git
cd x3-chain-node

# Build release
cargo build --release

# Run testnet sync
./target/release/x3-chain-node \
  --chain testnet \
  --bootnodes /dns/bootnode.testnet.x3-chain.io/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp
```

---

## 📚 Available RPC Methods

### Standard Substrate RPC
- `system_health` - Node health status
- `system_chain` - Chain name
- `system_version` - Node version
- `chain_getBlock` - Get block by hash
- `chain_getBlockHash` - Get block hash by number
- `state_getStorage` - Query storage directly

### X3 Kernel RPC
- `atlasKernel_getCanonicalBalance` - Query canonical ledger balance
- `atlasKernel_getAssetMetadata` - Get asset symbol and decimals
- `atlasKernel_isAuthorized` - Check authorization status
- `atlasKernel_getAuthorizedAccounts` - List authorized accounts
- `atlasKernel_getAuthorities` - Get current validator set

---

## ⚠️ Important Limitations (Testnet v1)

1. **Mock VM Execution**: EVM and SVM executors use mock receipts; real execution coming in v2
2. **HTTP Only**: WebSocket RPC support coming soon
3. **No Economic Value**: tATLAS tokens have no real-world value
4. **Network Resets**: Testnet may be reset without notice during development
5. **Rate Limits**: Faucet limited to 100 tATLAS per 24 hours
6. **Public RPC Limits**: 1000 requests/minute per IP

---

## 🆘 Troubleshooting

### "Connection refused" Error
- Check RPC endpoint URL (must include port `:9944`)
- Verify network connectivity
- Try fallback RPC: `http://rpc2.testnet.x3-chain.io:9944`

### "Insufficient balance" Error
- Request tokens from faucet
- Check balance with `atlasKernel_getCanonicalBalance`
- Wait for faucet cooldown (24 hours)

### Node Won't Sync
- Check bootnodes are reachable
- Ensure firewall allows port 30333
- Try different bootnode from list in `docs/reports/docs/runbooks/deployment/DEPLOYMENT_GUIDE.md`

### RPC Returns "Method not found"
- Verify method name spelling (case-sensitive)
- Check if method is X3 Kernel-specific (`atlasKernel_` prefix)
- Ensure using JSON-RPC 2.0 format

---

## 🤝 Join the Community

- **Discord**: https://discord.gg/x3-chain
- **Telegram**: https://t.me/x3_chain
- **GitHub**: https://github.com/x3-chain/x3-chain-node
- **Twitter**: https://twitter.com/x3_chain
- **Forum**: https://forum.x3-chain.io

---

## 📖 Additional Resources

- **Full Deployment Guide**: `docs/reports/docs/runbooks/deployment/DEPLOYMENT_GUIDE.md`
- **Technical Architecture**: `docs/ARCHITECTURE.md`
- **API Documentation**: `docs/root/README.md` (sections 10-13)
- **Testnet Announcement**: `docs/reports/TESTNET_ANNOUNCEMENT.md`

---

## 🚀 Next Steps

1. ✅ Get test tokens from faucet
2. ✅ Try X3 Kernel RPC methods
3. ✅ Submit your first Comit
4. ✅ Run a local sync node
5. ✅ Join Discord and share feedback
6. ✅ Star the GitHub repo
7. ✅ Build an app on X3 Chain!

**Happy Building! 🎉**
