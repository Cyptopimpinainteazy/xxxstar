# X3Star Production Deployment

**Status:** ✅ LIVE

## Deployment Details

### Production Server
- **Host:** `localhost:8080` (production static server)
- **Runtime:** Node.js CommonJS server
- **Startup:** `cd production && node server.cjs`
- **Files Served:** All x3star-*.html pages + js/css assets
- **CORS:** Enabled for cross-origin requests

### Domain Configuration
- **Root Domain:** `x3star.net` → Production Server (port 8080)
- **WWW:** `www.x3star.net` → Production Server (port 8080)
- **RPC:** `rpc.x3star.net` → Blockchain Node (port 9933)
- **WebSocket:** `ws.x3star.net` → Blockchain Node (port 9933)

### Cloudflare Tunnel
- **Tunnel:** `atlas-sphere` (6c118620-18cf-4795-80a8-6d44d37aecaa)
- **Config:** `/home/lojak/.cloudflared/config.yml`
- **Status:** Active with 4+ connections

### Blockchain Node
- **Port:** 9933 (JSON-RPC)
- **Status:** Producing blocks (current: #7568)
- **Peers:** 0 (dev mode)
- **Sync:** Not syncing (expected)

## Live Pages

All 40+ pages are now live:
- **Dashboard:** `/x3star-dashboard.html`
- **Admin:** `/x3star-admin-dashboard.html`
- **RPC Tester:** `/test-rpc-connection.html`
- **Landing:** `/x3star-landing.html`
- Plus: ecosystem, governance, tokenomics, validators, wallet, etc.

## Endpoints

### Frontend
- `https://x3star.net/` - Production site (via Cloudflare tunnel)
- `https://www.x3star.net/` - Production site (via Cloudflare tunnel)

### Blockchain
- `https://rpc.x3star.net/` - JSON-RPC endpoint (via tunnel)
- `wss://ws.x3star.net/` - WebSocket endpoint (via tunnel)

## Monitoring

### Health Check
```bash
curl https://rpc.x3star.net \
  -H 'Content-Type: application/json' \
  -d '{"id":1,"jsonrpc":"2.0","method":"system_health","params":[]}'
```

### Admin Dashboard
- **URL:** `https://x3star.net/x3star-admin-dashboard`
- **Features:**
  - Network connectivity tests
  - Blockchain status monitoring
  - RPC diagnostics
  - Activity logging

## Configuration Files

- `production/server.cjs` - Static file server
- `production/public/` - All static assets
- `/home/lojak/.cloudflared/config.yml` - Tunnel routing rules

## Deployment Date
- **Date:** May 9, 2026
- **Block Height:** #7568
- **Live Time:** ~12:42 UTC

## Next Steps (Optional)
- [ ] Set up monitoring/alerts
- [ ] Configure CDN caching
- [ ] Enable analytics
- [ ] Set up SSL/TLS certificates
- [ ] Deploy backend services (if needed)
