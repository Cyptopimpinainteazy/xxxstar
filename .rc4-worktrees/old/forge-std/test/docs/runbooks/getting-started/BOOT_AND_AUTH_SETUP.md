# 🦾 X3 Chain - Boot & Authentication Setup

## Quick Start - Start "The Beast"

```bash
# Start X3 Intelligence Dashboard + GPU Validator
bash scripts/start-beast.sh
```

That's it! Both services will start with:
- ✅ X3 Intelligence Dashboard: http://localhost:5173
- ✅ GPU Validator Metrics: http://localhost:8000/metrics.json

## Login Credentials

```
Username: admin
Password: x3-chain-2026
```

**⚠️ IMPORTANT: Change these in production!** (Edit `.env.production`)

## 1. Quick Commands

Add to your shell (`~/.bashrc` or `~/.zshrc`):
```bash
source /path/to/x3-chain/x3-boot-config.sh
```

Then use:
```bash
beast-start          # Start all services
beast-stop           # Stop all services
beast-status         # Check if running
beast-logs-x3        # View X3 Intelligence logs
beast-logs-gpu       # View GPU Validator logs
beast-login          # Show login info
```

## 2. Auto-Start on Boot (Linux)

### Setup
```bash
sudo bash scripts/setup-autostart.sh
```

This installs systemd services that automatically start on boot.

### Managing Services
```bash
# Start immediately
sudo systemctl start x3-intelligence
sudo systemctl start ccgv-validator

# Check status
sudo systemctl status x3-intelligence
sudo systemctl status ccgv-validator

# View logs
sudo journalctl -u x3-intelligence -f
sudo journalctl -u ccgv-validator -f

# Stop services
sudo systemctl stop x3-intelligence
sudo systemctl stop ccgv-validator

# Disable auto-start
sudo systemctl disable x3-intelligence
sudo systemctl disable ccgv-validator
```

## 3. Manual Startup

```bash
# Terminal 1: Start X3 Intelligence Dashboard
cd apps/x3-intelligence
npm install
npm run dev

# Terminal 2: Start GPU Validator
cd cross-chain-gpu-validator
source .venv/bin/activate
export CCGV_USE_MOCK_RPC=true
python -m cross_chain_gpu_validator.cli serve --host 0.0.0.0 --port 8000
```

## 4. File Structure

```
x3-chain/
├── scripts/
│   ├── start-beast.sh              # Manual startup script
│   ├── stop-beast.sh               # Manual stop script
│   └── setup-autostart.sh          # Systemd installation
│
├── services/
│   ├── x3-intelligence.service     # Systemd config
│   └── ccgv-validator.service      # Systemd config
│
├── .env.production                 # Environment configuration
├── x3-boot-config.sh            # Shell aliases & functions
│
├── apps/x3-intelligence/
│   └── src/
│       ├── auth.ts                 # Authentication system
│       └── auth-router.ts          # Auth endpoints
│
└── cross-chain-gpu-validator/
    └── scripts/
        └── run-local-tests.sh      # E2E testing
```

## 5. Authentication System

### How It Works
- Simple username/password system
- Sessions stored in memory
- 24-hour session expiry
- HTTPS-only cookies in production

### API Endpoints
```
POST   /api/auth/login      # Login with credentials
POST   /api/auth/logout     # Logout & destroy session
GET    /api/auth/status     # Check auth status
GET    /api/dashboard       # Protected endpoint (requires auth)
```

### Example Login
```bash
curl -X POST http://localhost:5173/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"x3-chain-2026"}'
```

Response:
```json
{
  "success": true,
  "token": "abc123...",
  "expiresAt": "2026-02-12T04:30:00Z"
}
```

## 6. Production Security Checklist

Before deploying, change:

```bash
# 1. Session secret
SESSION_SECRET=your-random-long-string

# 2. Auth salt
AUTH_SALT=your-random-salt

# 3. Change default credentials
# Edit auth.ts and create new admin user

# 4. Use HTTPS
# Deploy behind nginx/reverse proxy with SSL

# 5. Environment
NODE_ENV=production

# 6. RPC endpoints (if using live networks)
CCGV_EVM_RPC=https://your-mainnet-rpc
CCGV_SVM_RPC=https://your-solana-rpc
```

## 7. Troubleshooting

### Services won't start
```bash
# Check if ports are in use
lsof -i :5173
lsof -i :8000

# Kill existing processes
pkill -f "npm run dev"
pkill -f "cross_chain_gpu_validator"
```

### Login not working
```bash
# Check auth is initialized
tail -f /tmp/x3-intelligence.log | grep AUTH

# Verify credentials in auth.ts
grep DEFAULT_USER /apps/x3-intelligence/src/auth.ts
```

### Services won't auto-start
```bash
# Check systemd status
sudo systemctl status x3-intelligence
sudo systemctl status ccgv-validator

# Check if services are enabled
sudo systemctl is-enabled x3-intelligence
sudo systemctl is-enabled ccgv-validator

# Check logs
sudo journalctl -u x3-intelligence -n 50
```

## 8. Environment Variables

Key configs in `.env.production`:

| Variable | Default | Purpose |
|----------|---------|---------|
| `SESSION_SECRET` | Change me! | Express session encryption |
| `AUTH_SALT` | Change me! | Password hash salt |
| `PORT` | 5173 | X3 Dashboard port |
| `CCGV_USE_MOCK_RPC` | true | Use mock RPC (no network) |
| `CCGV_LOG_LEVEL` | INFO | Validator logging level |
| `NODE_ENV` | production | Deployment environment |

## 9. Monitoring

### Check Service Status
```bash
# Quick status check
bash scripts/status-check.sh

# Or manually
curl http://localhost:5173          # Dashboard
curl http://localhost:8000/metrics.json  # Metrics
```

### View Logs
```bash
# All in one
tail -f /tmp/x3-intelligence.log /tmp/ccgv-validator.log

# Systemd logs
sudo journalctl -u x3-intelligence -u ccgv-validator -f
```

## 10. Next Steps

After first boot:
1. ✅ Access dashboard: http://localhost:5173
2. ✅ Login with admin/x3-chain-2026
3. ✅ Change password (in production)
4. ✅ Enable auto-start: `sudo bash scripts/setup-autostart.sh`
5. ✅ Test RPC endpoints
6. ✅ Run E2E tests: `./scripts/run-local-tests.sh`

---

**Ready to run?** Start with:
```bash
bash scripts/start-beast.sh
```

Then visit: **http://localhost:5173**

Done! 🚀
