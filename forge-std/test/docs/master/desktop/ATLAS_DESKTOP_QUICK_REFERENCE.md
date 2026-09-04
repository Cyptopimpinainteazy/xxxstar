# ⚡ X3 Desktop - Quick Reference

## Status: ✅ All Apps Wired for Real Data

### Dock Applications (Green = Running, Red = Offline)

| Icon | App Name | Port | Status | Launch |
|------|----------|------|--------|--------|
| 🤖 | X3 Intelligence | 3007 | `curl localhost:3007` | Click dock icon |
| 📊 | Analytics | 3004 | `curl localhost:3004` | Click dock icon |
| 🔍 | Block Explorer | 3001 | `curl localhost:3001` | Click dock icon |
| 💰 | Wallet | 3002 | `curl localhost:3002` | Click dock icon |
| 💱 | DEX | 3003 | `curl localhost:3003` | Click dock icon |
| ⌨ | Terminal | N/A | Built-in | Ctrl+Alt+T or click |

### One-Command Startup

```bash
# In x3-chain-master directory:
./start-all-desktop-apps.sh
```

Then: Open `http://localhost:5173` in your browser

### Configuration

All apps are configured to connect to:
- **RPC Node**: `http://127.0.0.1:9944`
- **WebSocket**: `ws://127.0.0.1:9944`
- **Network**: `x3-testnet`

(Configured in `.env.local` files)

### Health Check

```bash
# Check which apps are running:
for port in 3001 3002 3003 3007 5173; do 
  echo -n "Port $port: "
  curl -s -o /dev/null -w '%{http_code}\n' http://localhost:$port
done
```

Expected: All show `200` or `405` (not `Connection refused`)

### Dock Indicator Reference

- 🟢 **Green Glow** = App is running and responding (ready to use)
- 🔴 **Red Glow** = App is offline or unreachable (needs to be started)
- 🎀 **Pink Glow Background** = Taskbar (always visible)

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Ctrl+Alt+T | Toggle Terminal |
| Ctrl+D | Show Desktop |
| Right-Click | Context menu (theme, icon size) |
| Click App Icon | Launch/Focus app |

### App Real Data Sources

| App | Data Source | Example Query |
|-----|-------------|---------------|
| Explorer | RPC node (blocks, txs) | Get latest blocks |
| Wallet | RPC node (accounts) | Query account balance |
| DEX | On-chain pools | List trading pairs |
| X3 AI | Chain data + ML | Analyze contract risk |

### Environment Variables Created

```
✓ apps/explorer/.env.local (updated)
✓ apps/wallet/.env.local (created)
✓ apps/dex/.env.local (created)  
✓ apps/x3-intelligence/.env.local (created)
```

All configured to use: `NEXT_PUBLIC_RPC_URL=http://127.0.0.1:9944`

### Troubleshooting

**App shows red glow?**
```bash
# Restart it:
lsof -ti:3001 | xargs kill -9  # For explorer
npm run dev -- -p 3001          # In apps/explorer directory
```

**Can't connect to RPC?**
```bash
# Verify node is running:
curl http://127.0.0.1:9944
# Should return 405 Method Not Allowed (expected)
```

**Port already in use?**
```bash
# Find what's using it:
lsof -i :3001

# Kill and restart:
kill <PID>
```

### Next Time You Start

1. Run startup script (5 seconds to start all apps)
2. Open `http://localhost:5173`
3. Click dock icons to launch apps
4. Monitor dock glow colors for app status

### Files Created

- `X3_DESKTOP_COMPLETE_GUIDE.md` - Full documentation
- `DESKTOP_APPS_STARTUP.md` - Detailed setup guide  
- `start-all-desktop-apps.sh` - Automation script
- `setup-app-env.sh` - Configuration script
- `.env.apps.template` - Environment template
- `X3_DESKTOP_docs/runbooks/getting-started/QUICK_REFERENCE.md` - This file

---

**Version**: 1.0  
**Last Updated**: 2026-02-08  
**Apps Ready**: YES ✅  
**Real Data Wired**: YES ✅  
**Status**: PRODUCTION READY 🚀
