# X3 Desktop App Store - X3 Treasury Integration

## Overview

The X3 Desktop App Store integrates 10 third-party blockchain applications with automatic **50% revenue sharing to the X3 Treasury**. All mining rewards, trading profits, transaction fees, and earnings are automatically split, with half routing to the X3 ecosystem treasury.

## 🏦 Treasury Configuration

### X3 Treasury Addresses

```typescript
// Primary X3 Treasury (X3 Chain)
X3_TREASURY_ADDRESS: "X3Treasury_DefaultAddress_REPLACE_IN_PRODUCTION"

// Multi-chain addresses
Ethereum:   0xX3_TREASURY_ETH
BSC:        0xX3_TREASURY_BSC
Polygon:    0xX3_TREASURY_POLYGON
Arbitrum:   0xX3_TREASURY_ARB
Avalanche:  0xX3_TREASURY_AVAX
Solana:     X3Treasury_SOL_ADDRESS
```

### Revenue Split

**All integrated apps follow this split:**
- **50%** → X3 Treasury (community fund)
- **50%** → User (you)

This ensures sustainable ecosystem growth while rewarding participants.

---

## 📦 Integrated Applications

### 1. **Arbitrage Bot** 🤖
- **Type**: Multi-chain arbitrage trading
- **Chain**: Multi-chain (ETH, BSC, Polygon, Arbitrum)
- **Treasury Integration**: ✅ Enabled
- **Revenue Source**: Trading profits
- **Location**: `app-store/Arbitrage-Bot/`

#### Integration Files:
- `x3_treasury_integration.py` - Python treasury wrapper
- Integration automatically splits all arbitrage profits 50/50

#### Launch Command:
```bash
python arbitrage_bot.py --treasury-enabled
```

---

### 2. **Fuego GTR Wallet** 🔥
- **Type**: Multi-chain crypto wallet
- **Chain**: Multi-chain
- **Treasury Integration**: ✅ Enabled
- **Revenue Source**: Transaction fees
- **Location**: `app-store/fuego-GTR-wallet/`

#### Integration Files:
- `x3-treasury-integration.ts` - TypeScript wallet wrapper
- Hooks into all transaction flows to route fees

#### Launch Command:
```bash
cd fuego-GTR-wallet && npm run tauri dev
```

---

### 3. **PancakeSwap Wizard** 🥞
- **Type**: Automated PancakeSwap trading
- **Chain**: Binance Smart Chain (BSC)
- **Treasury Integration**: ✅ Enabled
- **Revenue Source**: Trading profits, LP rewards, yield farming
- **Location**: `app-store/pancake-wizard/`

#### Integration Files:
- `x3-treasury-config.json` - Configuration file
- `x3-treasury-integration.ts` - Trading bot wrapper

#### Launch Command:
```bash
cd pancake-wizard && npm run start:treasury
```

#### Features:
- Arbitrage trading → 50% profits to treasury
- Liquidity provision → 50% rewards to treasury
- Yield farming → 50% earnings to treasury
- Auto-compounding with treasury split

---

### 4. **Meme Core Bundler** 🎭
- **Type**: Solana meme coin launcher
- **Chain**: Solana
- **Treasury Integration**: ✅ Enabled
- **Revenue Source**: Launch fees, bundling fees
- **Location**: `app-store/meme-core-bundler-solana/`

#### Launch Command:
```bash
./meme-core_2.1.0.exe --treasury-mode
```

---

### 5. **AERA Project Suite** ⛏️
- **Type**: Full blockchain ecosystem (miner + wallet + node)
- **Chain**: Multi-chain
- **Treasury Integration**: ✅ Enabled
- **Revenue Source**: Mining rewards
- **Location**: `app-store/AERA-Project/`

#### Integration Files:
- `aera-miner/x3_treasury_config.json` - Miner configuration
- `aera-miner/start-miner-with-treasury.sh` - Launch script with treasury

#### Launch Command:
```bash
cd AERA-Project/aera-miner && ./start-miner-with-treasury.sh
```

#### Features:
- **50% of all mining rewards** automatically route to X3 Treasury
- Auto-transfer every 100 blocks
- Full transaction logging
- Configurable transfer thresholds

---

### 6. **Mynta Wallet** 💎
- **Type**: Modern multi-chain wallet
- **Chain**: Multi-chain
- **Treasury Integration**: ✅ Enabled
- **Revenue Source**: Transaction fees, swap fees
- **Location**: `app-store/mynta-wallet/`

#### Launch Command:
```bash
cd mynta-wallet && npm run tauri dev
```

---

### 7. **AgenC Operator** 🤖
- **Type**: AI agent orchestration
- **Chain**: Multi-chain
- **Treasury Integration**: ✅ Enabled
- **Revenue Source**: Agent earnings, service fees
- **Location**: `app-store/AgenC-Operator/`

#### Launch Command:
```bash
cd AgenC-Operator && cargo run -- --treasury-enabled
```

---

### 8. **AI Blockchain Assistant** 🧠
- **Type**: AI-powered blockchain operations
- **Chain**: Multi-chain
- **Treasury Integration**: ✅ Enabled
- **Revenue Source**: Service fees, automation fees
- **Location**: `app-store/ai-blockchain-assistant/`

#### Launch Command:
```bash
cd ai-blockchain-assistant && cargo run --release -- --treasury-mode
```

---

### 9. **Blum Tap Clicker** 👆
- **Type**: Auto-clicker for Blum points
- **Chain**: Multi-chain
- **Treasury Integration**: ✅ Enabled
- **Revenue Source**: Earned tokens
- **Location**: `app-store/Blumtap/`

#### Launch Command:
```bash
cd Blumtap && python -m blum_tap_clicker --treasury-wallet
```

---

### 10. **Hardware Info Plugin** ⚙️
- **Type**: Tauri hardware monitoring plugin
- **Chain**: N/A (utility)
- **Treasury Integration**: ❌ Not applicable
- **Purpose**: Mining optimization, hardware monitoring
- **Location**: `app-store/tauri-plugin-hwinfo/`

#### Usage:
```bash
cd tauri-plugin-hwinfo && npm run build
```

---

## 🚀 Quick Start

### 1. Configure Treasury Addresses

Before running any app in production, configure the treasury addresses:

```bash
# Set environment variables
export X3_TREASURY_ADDRESS="your_x3_treasury_address"
export X3_TREASURY_ETH="your_ethereum_treasury"
export X3_TREASURY_BSC="your_bsc_treasury"
export X3_TREASURY_SOL="your_solana_treasury"
```

Or edit the config files directly:
- `/src/config/treasury.config.ts` (main treasury config)
- Individual app configs in each `app-store/*/` directory

### 2. Launch X3 Desktop

```bash
cd apps/x3-desktop
npm run tauri:dev
```

### 3. Open App Store

Navigate to `/appstore` route in the desktop app, or add a navigation button:

```typescript
import { useNavigate } from "react-router-dom";

const navigate = useNavigate();
navigate("/appstore");
```

### 4. Install & Launch Apps

From the App Store UI:
1. Browse available apps
2. Click on an app to see details
3. Click "Install App" (if not already installed)
4. Click "Launch App" to start with treasury integration

---

## 📊 Treasury Monitoring

### View Treasury Statistics

Each app logs treasury transactions to:
- `logs/treasury_transfers.log`
- `logs/treasury_integration.log`
- LocalStorage: `x3-desktop:treasury-log`

### Access Treasury Stats from Code:

```typescript
import { logTreasuryTransaction, X3_TREASURY_CONFIG } from "@/config/treasury.config";
import { getTreasuryIntegratedApps } from "@/config/appstore.config";

// Get all apps with treasury integration
const treasuryApps = getTreasuryIntegratedApps();

// Log a treasury transaction
logTreasuryTransaction({
  timestamp: new Date().toISOString(),
  appId: "arbitrage-bot",
  chain: "ethereum",
  totalAmount: 100,
  treasuryAmount: 50,
  userAmount: 50,
  txHash: "0x123...",
  status: "completed"
});
```

### View in UI:

The App Store page displays:
- Total apps with treasury integration
- Treasury share percentage (50%)
- Per-app treasury status
- Revenue split visualization

---

## 🔧 Development

### Adding New Apps

1. **Clone the repository** to `app-store/`:
```bash
cd app-store
git clone https://github.com/user/new-app
```

2. **Create treasury integration**:
   - Add `x3-treasury-config.json` or `x3_treasury_integration.py`
   - Wrap revenue-generating functions with treasury split
   - Log all transactions to treasury

3. **Add to App Store config**:

Edit `src/config/appstore.config.ts`:

```typescript
{
  id: "new-app",
  name: "New App",
  description: "Description with treasury integration",
  category: "trading",
  chain: "ethereum",
  version: "1.0.0",
  author: "author-name",
  repositoryUrl: "https://github.com/user/new-app",
  icon: "🆕",
  installed: true,
  enabled: true,
  treasuryIntegrated: true, // Set to true!
  features: ["50% profit to X3 Treasury", ...],
  requirements: ["Node.js 18+"],
  launchCommand: "npm run start:treasury",
  configPath: "app-store/new-app/x3-treasury-config.json",
  size: "50 MB"
}
```

4. **Test the integration**:
```bash
# Run the app with treasury integration
cd app-store/new-app
npm run start:treasury
```

5. **Verify treasury transactions** are logged.

---

## 🔒 Security

### Treasury Address Validation

All integrations validate treasury addresses before transfers:

```typescript
import { validateTreasuryConfig } from "@/config/treasury.config";

const isValid = validateTreasuryConfig(X3_TREASURY_CONFIG);
if (!isValid) {
  throw new Error("Invalid treasury configuration");
}
```

### Production Checklist

Before deploying to production:

- [ ] Replace all default treasury addresses with real addresses
- [ ] Test treasury transfers on testnet
- [ ] Verify treasury share calculations (should be exactly 50%)
- [ ] Enable transaction logging
- [ ] Set up monitoring alerts for failed transfers
- [ ] Backup treasury private keys securely
- [ ] Document recovery procedures

---

## 📈 Revenue Tracking

### Per-App Revenue

Each integrated app tracks:
- Total revenue generated
- Treasury share (50%)
- User share (50%)
- Number of transactions
- Failed transactions
- Token breakdown

### Global Treasury Stats

Access global statistics:

```typescript
import { APP_STORE_APPS } from "@/config/appstore.config";

const treasuryApps = APP_STORE_APPS.filter(app => app.treasuryIntegrated);
console.log(`${treasuryApps.length} apps integrated with treasury`);
```

### Example Treasury Dashboard

```typescript
function TreasuryDashboard() {
  const stats = {
    totalApps: 10,
    treasuryIntegrated: 9,
    totalRevenue: "12,500 USD",
    treasuryShare: "6,250 USD",
    userShare: "6,250 USD"
  };

  return (
    <div>
      <h2>X3 Treasury Dashboard</h2>
      <div>Total Apps: {stats.totalApps}</div>
      <div>Treasury Integrated: {stats.treasuryIntegrated}</div>
      <div>Total Revenue: {stats.totalRevenue}</div>
      <div>Treasury Share (50%): {stats.treasuryShare}</div>
      <div>User Share (50%): {stats.userShare}</div>
    </div>
  );
}
```

---

## 🐛 Troubleshooting

### Treasury Transfers Failing

1. Check treasury address is configured:
```bash
echo $X3_TREASURY_ADDRESS
```

2. Verify network connectivity
3. Check gas/fees are sufficient
4. Review logs: `logs/treasury_integration.log`

### App Not Launching

1. Check dependencies are installed:
```bash
cd app-store/app-name
npm install  # or pip install requirements.txt
```

2. Verify config files exist
3. Check launch command in `appstore.config.ts`
4. Review app-specific logs

### Treasury Address Shows "REPLACE"

You need to configure production treasury addresses:

1. Edit `src/config/treasury.config.ts`
2. Set environment variables
3. Update app-specific configs
4. Restart X3 Desktop

---

## 📚 Additional Resources

- [X3 Chain Documentation](../../docs/)
- [X3 Treasury Specification](../../docs/x3-treasury.md)
- [App Store API Reference](../../docs/appstore-api.md)
- [Treasury Integration Guide](../../docs/treasury-integration.md)

---

## 🤝 Contributing

To add new apps to the store:

1. Fork the X3 Chain repository
2. Add your app to `app-store/`
3. Implement X3 treasury integration (50% split required)
4. Add app metadata to `appstore.config.ts`
5. Test treasury transfers
6. Submit a pull request

All apps must integrate treasury revenue sharing to be accepted.

---

## 📄 License

Each app in the store maintains its own license. See individual app directories for details.

The treasury integration framework is part of X3 Chain and follows the repository's license.

---

## 🎯 Summary

**10 apps integrated** with automatic 50% revenue sharing to X3 Treasury:

1. ✅ Arbitrage Bot - Trading profits
2. ✅ Fuego GTR Wallet - Transaction fees
3. ✅ PancakeSwap Wizard - DeFi earnings
4. ✅ Meme Core Bundler - Launch fees
5. ✅ AERA Miner - Mining rewards
6. ✅ Mynta Wallet - Transaction fees
7. ✅ AgenC Operator - Agent earnings
8. ✅ AI Blockchain Assistant - Service fees
9. ✅ Blum Tap - Earned tokens
10. ⚙️ HWInfo Plugin - Utility (no treasury)

**Total treasury integration: 90%** (9/10 apps)

All earnings automatically split:
- **50% → X3 Treasury** (ecosystem growth)
- **50% → Users** (participant rewards)

---

**Built with ❤️ for the X3 Chain ecosystem**
