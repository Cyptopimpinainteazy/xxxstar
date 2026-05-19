# 🚀 X3 Chain Testnet v1 - Now Live!

**Network**: X3 Chain Testnet v1  
**Status**: ✅ Live  
**Launch Date**: November 2025

---

## 🎉 Testnet is Live!

We're excited to announce the launch of **X3 Chain Testnet v1** - a developer preview network for the world's first dual-VM (EVM + SVM) Layer-1 blockchain!

### What's Available Now

✅ **Working Features**:
- Comit submission with mock VM execution
- Canonical ledger for asset tracking
- HTTP JSON-RPC server
- Account authorization system
- Asset registry
- Authority management
- Aura + GRANDPA consensus
- Full networking with peer discovery

⚠️ **Current Limitations**:
- Mock VM execution (real EVM/SVM integration coming soon)
- HTTP-only RPC (WebSocket in development)
- Testnet tokens have no economic value
- Network may be reset during development

---

## 🌐 Public Endpoints

### RPC Endpoints
```
HTTP RPC: http://rpc.testnet.x3-chain.io:9944
```

**Coming Soon**:
```
WebSocket: ws://ws.testnet.x3-chain.io:9944
```

### Available RPC Methods

**X3 Kernel Methods**:
- `atlasKernel_getCanonicalBalance(account, asset_id, at?)` - Query balances
- `atlasKernel_getAssetMetadata(asset_id, at?)` - Get asset info
- `atlasKernel_isAuthorized(account, at?)` - Check authorization
- `atlasKernel_getAuthorizedAccounts(at?)` - List authorized accounts
- `atlasKernel_getAuthorities(at?)` - Get validator set

---

## 💰 Get Testnet Tokens

Use our faucet to get free testnet tokens (tATLAS):

```bash
curl -X POST https://faucet.testnet.x3-chain.io/claim \
  -H "Content-Type: application/json" \
  -d '{"address": "YOUR_ADDRESS"}'
```

**Faucet Limits**:
- 100 tATLAS per request
- 1 request per address per day
- Maximum 1000 tATLAS per address

---

## 🔧 Quick Start

### 1. Query Your Balance

```bash
curl http://rpc.testnet.x3-chain.io:9944 \
  -H "Content-Type: application/json" \
  -d '{
    "id":1,
    "jsonrpc":"2.0",
    "method":"atlasKernel_getCanonicalBalance",
    "params":["5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY", 0, null]
  }'
```

### 2. Check Authorized Accounts

```bash
curl http://rpc.testnet.x3-chain.io:9944 \
  -H "Content-Type: application/json" \
  -d '{
    "id":1,
    "jsonrpc":"2.0",
    "method":"atlasKernel_getAuthorizedAccounts",
    "params":[null]
  }'
```

### 3. Get Current Validators

```bash
curl http://rpc.testnet.x3-chain.io:9944 \
  -H "Content-Type: application/json" \
  -d '{
    "id":1,
    "jsonrpc":"2.0",
    "method":"atlasKernel_getAuthorities",
    "params":[null]
  }'
```

---

## 👥 Join the Community

### Get Help & Stay Updated

- **Discord**: https://discord.gg/x3-chain - Ask questions, get support
- **Telegram**: https://t.me/x3_chain - Community chat
- **GitHub**: https://github.com/x3-chain/x3-chain - Code, issues, PRs
- **Twitter**: https://twitter.com/x3_chain - Announcements
- **Documentation**: https://docs.x3-chain.io - Full docs

### Report Issues

Found a bug? Help us improve!

- **Bug Reports**: https://github.com/x3-chain/x3-chain/issues
- **Security Issues**: security@x3-chain.io (private disclosure)
- **Feature Requests**: GitHub Discussions

---

## 🛠️ For Developers

### Run Your Own Node

**Requirements**:
- Linux or macOS
- Rust toolchain
- 4 GB RAM minimum
- 100 GB disk space

**Quick Setup**:
```bash
# Clone repository
git clone https://github.com/x3-chain/x3-chain.git
cd x3-chain

# Build node
cargo build --release

# Run node
./target/release/x3-chain-node --dev --tmp
```

**Full deployment guide**: See `docs/reports/docs/runbooks/deployment/DEPLOYMENT_GUIDE.md`

### Connect to Testnet

```bash
./target/release/x3-chain-node \
  --chain x3-testnet-raw.json \
  --base-path /tmp/x3-data \
  --name "MyNode" \
  --bootnodes /ip4/BOOTNODE_IP/tcp/30333/p2p/PEER_ID
```

### Available Tools

**Coming Soon**:
- 📦 TypeScript SDK for web apps
- 🐍 Python SDK for backend services
- 🔧 Enhanced CLI tools
- 📚 Comprehensive tutorials

---

## 🗺️ Testnet Roadmap

### Phase 1: Initial Launch ✅ COMPLETE
- ✅ Launch validator set
- ✅ Deploy RPC nodes
- ✅ Open faucet
- ✅ Publish endpoints

### Phase 2: Developer Tools (2-4 weeks)
- 🔄 WebSocket RPC support
- 🔄 TypeScript SDK release
- 🔄 Python SDK release
- 🔄 CLI tools enhancement

### Phase 3: VM Integration (4-8 weeks)
- 🔜 Real EVM integration (Frontier)
- 🔜 Real SVM integration (Solana)
- 🔜 Cross-VM bridge testing
- 🔜 Performance optimization

### Phase 4: Mainnet Preparation (8-12 weeks)
- 🔜 Security audit
- 🔜 Economic model finalization
- 🔜 Governance activation
- 🔜 Mainnet launch

---

## 📊 Current Network Stats

### Network Health
- **Validators**: 3 active
- **Block Time**: 6 seconds
- **Finality**: GRANDPA (~12 seconds)
- **Uptime**: 99%+

### Developer Activity
- **Total Nodes**: 10+
- **RPC Requests**: 1000+/day
- **Comits Submitted**: 50+
- **Developers**: 10+

*Stats updated daily*

---

## ⚠️ Important Notes

### Testnet Disclaimer

**X3 Chain Testnet v1 is a DEVELOPER PREVIEW**:

- ❌ Testnet tokens (tATLAS) have **NO economic value**
- ❌ Network may be **reset at any time**
- ❌ Do **NOT use mainnet keys** on testnet
- ❌ Expect **bugs and breaking changes**
- ❌ Not suitable for **production workloads**

### Mock VM Execution

Current testnet uses **mock executors** for EVM and SVM:
- Comit transactions are accepted and processed
- Mock receipts are generated (success=true)
- No actual EVM/SVM bytecode execution yet
- State changes are recorded to canonical ledger

**Real VM integration** is in active development and coming soon!

---

## 🎯 How to Contribute

We welcome contributions from the community!

### Ways to Contribute

1. **Test the Network**: Use the testnet, report bugs
2. **Submit PRs**: Improve code, docs, tooling
3. **Write Tutorials**: Help onboard new developers
4. **Build Tools**: Create SDKs, explorers, wallets
5. **Provide Feedback**: Share your experience

### Bounty Programs (Coming Soon)

We'll be launching bounty programs for:
- Finding bugs and vulnerabilities
- Building developer tools
- Creating tutorials and content
- Running testnet infrastructure

---

## 📖 Resources

### Documentation
- **README**: Project overview and setup
- **TESTNET_DEPLOYMENT_GUIDE**: Full deployment instructions
- **IMPLEMENTATION_PLAN**: Development roadmap
- **COMPLETION_STATUS**: Current status and progress

### Tutorials (Coming Soon)
- Building your first dApp on X3 Chain
- Cross-VM transactions explained
- Running a validator node
- Using the TypeScript SDK

---

## 🙏 Thank You!

Thank you for being an early adopter of X3 Chain! Your feedback and contributions help us build the future of dual-VM blockchain infrastructure.

**Let's build something amazing together!** 🚀

---

## 📞 Contact

- **Website**: https://x3-chain.io
- **Email**: hello@x3-chain.io
- **Security**: security@x3-chain.io
- **Press**: press@x3-chain.io

---

**X3 Chain Team**  
*Building the future of cross-chain interoperability*

---

**Last Updated**: November 2025  
**Version**: Testnet v1 (0.1.0)  
**License**: Apache 2.0
