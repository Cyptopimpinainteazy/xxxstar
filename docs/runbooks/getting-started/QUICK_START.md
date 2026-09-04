# 🚀 X3 Chain Quick Start Card

## Install Auto-Start Services

```bash
sudo bash /home/lojak/Desktop/x3-chain-master/deployment/scripts/install-services.sh
```

**What this does:**
- ✅ Creates "x3" system user
- ✅ Installs 3 systemd services
- ✅ Enables auto-start on boot
- ✅ Starts services immediately

**Services installed:**
| Service | Port | Purpose |
|---------|------|---------|
| `redis.service` | 6379 | Cache & sessions |
| `ccgv-validator.service` | 8000 | GPU validator |
| `x3-intelligence.service` | 5173 | Dashboard |

---

## Login Credentials

```
Username: admin
Password: x3-chain-2026
```

**⚠️ CHANGE FOR PRODUCTION!**

How to change:
1. Edit: `/apps/x3-intelligence/src/auth.ts`
2. Update the password in `DEFAULT_USERS`
3. Rebuild: `cd apps/x3-intelligence && npm run build`

---

## Access Dashboard

```
http://localhost:5173
```

After boot, the dashboard will:
- Require login automatically
- Show cross-chain metrics
- Display validator status
- Real-time transaction tracking

---

## Configuration

Create `.env` file from template:
```bash
cp deployment/env/.env.template /home/lojak/Desktop/x3-chain-master/.env
```

**Key variables to set:**
```
AUTH_SALT=your-secret-salt          # Password hashing
JWT_SECRET=your-jwt-secret          # Token signing
REDIS_HOST=localhost                # Session storage
CCGV_RPC_ETHEREUM=https://...       # Ethereum RPC
CCGV_RPC_SOLANA=https://...         # Solana RPC
```

---

## Essential Commands

```bash
# Check service status
sudo systemctl status x3-intelligence.service

# View logs
sudo journalctl -u x3-intelligence.service -f

# Restart service
sudo systemctl restart x3-intelligence.service

# Start all services manually (if systemd fails)
bash /home/lojak/Desktop/x3-chain-master/deployment/scripts/startup.sh
```

---

## After Reboot

On next system restart, services will:
1. ✅ Start automatically
2. ✅ Load configuration from .env
3. ✅ Listen on assigned ports
4. ✅ Be accessible at http://localhost:5173

---

## Troubleshooting

**Services not running?**
```bash
sudo journalctl -u ccgv-validator.service
sudo journalctl -u x3-intelligence.service
sudo journalctl -u redis.service
```

**Port already in use?**
```bash
sudo lsof -i :5173    # Dashboard
sudo lsof -i :8000    # Validator
sudo lsof -i :6379    # Redis
```

**Manual startup fallback:**
```bash
bash /home/lojak/Desktop/x3-chain-master/deployment/scripts/startup.sh
```

---

## Architecture

```
Browser (localhost:5173)
    ↓ (Login required)
Dashboard (authenticated)
    ↓ (API calls with JWT)
Backend (localhost:8000)
    ↓ (Session storage)
Redis (localhost:6379)
```

---

## File Locations

| Component | Path |
|-----------|------|
| Login UI | `apps/x3-intelligence/src/components/LoginPage.tsx` |
| Config | `.env` (create from template) |
| Services | `/etc/systemd/system/*.service` |
| Logs | `/var/log/x3/` |
| Startup | `deployment/scripts/install-services.sh` |

---

## Security Highlights

✅ JWT tokens (24hr expiry)
✅ Password hashing (SHA256 + salt)
✅ Secure logout
✅ CORS headers
✅ Token validation per request

---

## Documentation

📖 Full guide: `docs/runbooks/getting-started/AUTHENTICATION_SETUP.md`
📋 Test suite: `cross-chain-gpu-validator/tests/`
🔧 Config template: `deployment/env/.env.template`

---

**Status:** ✅ All systems ready for auto-start after `sudo bash deployment/scripts/install-services.sh`

