# X3 Desktop - App Startup Guide

This guide helps you launch all the applications that appear in the X3 Desktop dock and ensure they're wired with real data.

## Quick Start - Launch All Apps

Run this script to start all services:

```bash
cd /home/lojak/Desktop/x3-chain-master
./start-all-desktop-apps.sh
```

Or manually start individual apps as needed.

## App Configuration

The X3 Desktop dock includes these apps that need to be running on specific ports:

| App ID | Name | Port | Type | Location |
|--------|------|------|------|----------|
| explorer | Block Explorer | 3001 | Next.js | `apps/explorer/` |
| wallet | Wallet | 3002 | Next.js | `apps/wallet/` |
| dex | Decentralised Exchange | 3003 | Next.js | `apps/dex/` |
| analytics | Analytics | 3004 | Python/Service | `apps/analytics-service/` |
| x3-intelligence | X3 Intelligence | 3007 | Next.js | `apps/x3-intelligence/` |

## Manual Startup

### 1. Block Explorer (Port 3001)
```bash
cd apps/explorer
npm install
npm run dev -- -p 3001
```

### 2. Wallet (Port 3002)
```bash
cd apps/wallet
npm install
npm run dev -- -p 3002
```

### 3. DEX (Port 3003)
```bash
cd apps/dex
npm install
npm run dev -- -p 3003
```

### 4. Analytics (Port 3004)
```bash
cd apps/analytics-service
# Check if it needs setup
# May require Python/Flask or Node.js
python main.py  # or npm run dev
```

### 5. X3 Intelligence (Port 3007)
```bash
cd apps/x3-intelligence
npm install
npm run dev -- -p 3007
```

## Real Data Wiring

Each app should be configured to connect to:

- **RPC Node**: `http://127.0.0.1:9944` (X3 Kernel)
- **WebSocket**: `ws://127.0.0.1:9944`
- **API Gateway**: `http://127.0.0.1:8080` (if available)

### Environment Variables

Create `.env.local` in each app directory:

```env
NEXT_PUBLIC_RPC_URL=http://127.0.0.1:9944
NEXT_PUBLIC_API_URL=http://127.0.0.1:8080
NEXT_PUBLIC_WS_URL=ws://127.0.0.1:9944
```

## Troubleshooting

### Port Already in Use
```bash
# Find and kill process using port 3001
lsof -ti:3001 | xargs kill -9
```

### App Won't Load
1. Check if the dev server is running: `curl http://localhost:3001`
2. Check browser console for errors (F12)
3. Verify all dependencies are installed: `npm install`
4. Check firewall settings

### Real Data Not Loading
1. Verify RPC node is running on 127.0.0.1:9944
2. Check app's network tab in DevTools
3. Ensure app has correct RPC endpoint in .env.local

## Status Indicators in Dock

- 🟢 **Green Glow** = App is running and responsive
- 🔴 **Red Glow** = App is not running or unreachable
- The glow effect updates automatically every 5 seconds

## Next Steps

1. Start the Tauri dev server (already running): `npm run tauri:dev`
2. Launch individual apps from the dock
3. Apps will open in your browser automatically
4. Monitor the dock indicator colors to see which apps are available

## Notes

- The desktop backend (`src-tauri`) provides telemetry data for swarm health, network control, and storage monitoring
- Each app should have its own data fetching logic wired to the RPC node or API gateway
- See individual app READMEs for app-specific configuration
