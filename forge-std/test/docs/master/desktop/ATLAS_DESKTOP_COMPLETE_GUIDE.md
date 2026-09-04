# X3 Desktop - Complete Setup & Data Wiring Guide

## Overview

The X3 Desktop is a Tauri-based desktop application that serves as a command center for the X3 Chain blockchain ecosystem. It provides a Mac-like dock interface with access to key blockchain applications.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     X3 Desktop (Tauri)                    │
│                    (React + Three.js + Zustand)              │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  Explorer    │  │   Wallet     │  │    DEX       │      │
│  │  (3001)      │  │   (3002)     │  │   (3003)     │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                                                              │
│  ┌──────────────┐  ┌──────────────────────────────────┐    │
│  │  X3 Intel    │  │  Terminal (⌨ in dock)           │    │
│  │  (3007)      │  │  Real time command execution     │    │
│  └──────────────┘  └──────────────────────────────────┘    │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │         Tauri Backend (Rust)                          │   │
│  │  - Telemetry (swarm, network, storage, IDE)          │   │
│  │  - IPC message handling                              │   │
│  │  - Process management                                │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
           ↓
     ┌─────────────────────────┐
     │  X3 Chain Node      │
     │  RPC: localhost:9944    │
     │  WS: localhost:9944     │
     └─────────────────────────┘
           ↓
     ┌─────────────────────────┐
     │   Real Blockchain Data  │
     │   (Blocks, Accounts)    │
     └─────────────────────────┘
```

## Quick Start (3 Simple Steps)

### Step 1: Setup Environment Variables
```bash
cd /home/lojak/Desktop/x3-chain-master
./setup-app-env.sh
```

### Step 2: Start All Applications
```bash
./start-all-desktop-apps.sh
```

### Step 3: Open the Desktop
Navigate to `http://localhost:5173` in your browser.

## Application Details

### 1. **Block Explorer** (Port 3001)
- **Purpose**: Browse blocks, transactions, and accounts
- **Real Data Source**: X3 Chain RPC node (localhost:9944)
- **Key Features**:
  - Live block stream
  - Transaction details
  - Account information
  - Asset balances

**Ensure it's wired:**
```bash
# .env.local in apps/explorer/
NEXT_PUBLIC_RPC_URL=http://127.0.0.1:9944
NEXT_PUBLIC_CHAIN_ID=x3-testnet
```

### 2. **Wallet** (Port 3002)
- **Purpose**: Key management and transaction signing
- **Real Data Source**: Local key storage + RPC node
- **Key Features**:
  - Import/create keys
  - Sign transactions
  - View account balances
  - Send funds

**Ensure it's wired:**
```bash
# .env.local in apps/wallet/
NEXT_PUBLIC_RPC_URL=http://127.0.0.1:9944
NEXT_PUBLIC_WALLET_SUPPORT=true
```

### 3. **DEX** (Port 3003)
- **Purpose**: Decentralised token exchange
- **Real Data Source**: On-chain liquidity pools
- **Key Features**:
  - Token swaps
  - Liquidity pools
  - Price feeds
  - Slippage protection

**Ensure it's wired:**
```bash
# .env.local in apps/dex/
NEXT_PUBLIC_RPC_URL=http://127.0.0.1:9944
NEXT_PUBLIC_DEX_FACTORY=<on-chain-dex-address>
```

### 4. **X3 Intelligence** (Port 3007)
- **Purpose**: AI-powered blockchain insights and analysis
- **Real Data Source**: On-chain data + AI models
- **Key Features**:
  - Smart contract analysis
  - Risk scoring
  - Transaction patterns
  - Threat detection

**Ensure it's wired:**
```bash
# .env.local in apps/x3-intelligence/
NEXT_PUBLIC_RPC_URL=http://127.0.0.1:9944
NEXT_PUBLIC_AI_ENDPOINT=http://127.0.0.1:8000/api
```

### 5. **Terminal** (Integrated in Dock)
- **Access**: Click ⌨ icon in the dock or press Ctrl+Alt+T
- **Features**:
  - Execute blockchain commands
  - Query on-chain data
  - Run deployment scripts
  - Monitor services

## Verifying Real Data Connection

### Check RPC Node is Running
```bash
curl -X POST http://127.0.0.1:9944 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}'
```

### Test App Connectivity
```bash
# Explorer
curl http://localhost:3001/api/blocks

# Wallet  
curl http://localhost:3002/api/accounts

# DEX
curl http://localhost:3003/api/pools

# X3 Intelligence
curl http://localhost:3007/api/health
```

### Monitor App Status in Dock
- 🟢 **Green glow** = App is running and healthy
- 🔴 **Red glow** = App is down or unreachable
- 🎨 Colored borders indicate app category (blockchain, DeFi, utility)

## Troubleshooting

### App Shows Red Glow (Not Running)
1. Check if the app's dev server started:
   ```bash
   lsof -i :3001  # For explorer on port 3001
   ```
2. Check the logs:
   ```bash
   tail -f /tmp/explorer.log
   ```
3. Try restarting:
   ```bash
   lsof -ti:3001 | xargs kill -9
   npm run dev -p 3001  # Restart manually
   ```

### Apps Can't Connect to RPC Node
1. Verify RPC is running on 9944:
   ```bash
   lsof -i :9944
   ```
2. Check node health:
   ```bash
   curl http://127.0.0.1:9944  # Should return 405 or valid JSON
   ```
3. Update .env.local if node is on different address:
   ```bash
   NEXT_PUBLIC_RPC_URL=http://your-node-address:9944
   ```

### Port Already in Use
```bash
# Find and kill process
lsof -ti:3001 | xargs kill -9

# Or use different port
npm run dev -- -p 3008
```

## Data Flow Diagram

```
┌──────────────────────────────────────────────────────────────┐
│                    Browser/Tauri Interface                    │
│                                                               │
│  User clicks "Explorer" → App loads from localhost:3001      │
│         ↓                                                      │
│  App queries NEXT_PUBLIC_RPC_URL (localhost:9944)            │
│         ↓                                                      │
│  RPC returns blockchain data (blocks, txs, accounts)         │
│         ↓                                                      │
│  UI renders real chain data                                  │
│         ↓                                                      │
│  User can interact with live blockchain                      │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

## Manual App Configuration

If you need to manually configure an app (instead of using the setup script):

### For Explorer
```bash
cd apps/explorer
cp .env.example .env.local
# Edit .env.local to set:
# NEXT_PUBLIC_RPC_URL=http://127.0.0.1:9944
npm install
npm run dev -- -p 3001
```

### For Wallet
```bash
cd apps/wallet
cat > .env.local << 'EOF'
NEXT_PUBLIC_RPC_URL=http://127.0.0.1:9944
NEXT_PUBLIC_WALLET_SUPPORT=true
EOF
npm install
npm run dev -- -p 3002
```

### For DEX
```bash
cd apps/dex
cat > .env.local << 'EOF'
NEXT_PUBLIC_RPC_URL=http://127.0.0.1:9944
NEXT_PUBLIC_DEX_ENABLED=true
EOF
npm install
npm run dev -- -p 3003
```

### For X3 Intelligence
```bash
cd apps/x3-intelligence
cat > .env.local << 'EOF'
NEXT_PUBLIC_RPC_URL=http://127.0.0.1:9944
NEXT_PUBLIC_AI_ENABLED=true
EOF
npm install
npm run dev -- -p 3007
```

## Advanced Configuration

### Custom RPC Endpoint
To use a different RPC node (not localhost:9944):

1. Update all .env.local files with the correct RPC URL
2. Ensure the RPC endpoint supports:
   - `system_health` (status check)
   - `chain_getBlock` (block queries)
   - `state_getStorage` (account queries)

### Enable Debug Mode
Add to any .env.local:
```
NEXT_PUBLIC_DEBUG=true
NEXT_PUBLIC_LOG_LEVEL=debug
```

Then check browser console for detailed logs.

### Analytics Integration
To enable real-time analytics:
```bash
# In apps/x3-desktop/.env.local
REACT_APP_ANALYTICS_API=http://127.0.0.1:3004/api
```

## Health Check Dashboard

Create a simple health dashboard:

```bash
# Install watch
npm install -g watch

# Monitor app status
watch -n 2 'for port in 3001 3002 3003 3007; do echo "Port $port: $(curl -s -o /dev/null -w '%{http_code}' http://localhost:$port)"; done'
```

## Performance Tips

1. **Use localhost** instead of 127.0.0.1 for faster DNS resolution
2. **Enable caching** to reduce RPC calls:
   ```
   NEXT_PUBLIC_CACHE_ENABLED=true
   NEXT_PUBLIC_CACHE_TTL=300
   ```
3. **Monitor browser DevTools** Network tab to see real data fetches
4. **Check RPC node logs** if apps are slow

## Security Considerations

⚠️ **Important**: The current setup uses localhost only. For production:

1. Enable HTTPS on all services
2. Add authentication to sensitive endpoints
3. Use environment-specific RPC endpoints
4. Enable rate limiting on RPC calls
5. Validate all external data sources

## Next Steps

1. ✅ Apps are configured with real RPC endpoints
2. ✅ Dock icons show green/red status
3. ✅ Terminal is integrated into the dock
4. 🔄 **Next**: Integrate real blockchain queries in each app
5. 🔄 **Next**: Add WebSocket subscriptions for live data
6. 🔄 **Next**: Implement caching and indexing layers

## Support

For detailed app documentation:
- Explorer: See `apps/explorer/docs/root/README.md`
- Wallet: See `apps/wallet/docs/root/README.md`
- DEX: See `apps/dex/docs/root/README.md`
- X3 Intelligence: See `apps/x3-intelligence/docs/root/README.md`

---

**Last Updated**: 2026-02-08
**Status**: All apps configured for real data wiring ✓
