# X3 Infrastructure Auto-Start Setup

## ✅ Current Status

**Running Services:**
- ✅ x3-desktop (Vite dev server on port 5173)
- ✅ cloudflared-tunnel (routes x3star.net → localhost backends)

**Inactive:**
- ⚠️ x3-chain-node (build failed — needs fix)

**Tunnel Status:**
- ✅ x3star.net → working
- ⚠️ RPC endpoints → waiting for node binary

---

## 🚀 Auto-Start Setup (Already Enabled)

All services are configured to start automatically on system reboot via systemd:

```bash
# User-level systemd services (no sudo needed)
~/.config/systemd/user/x3-desktop.service
~/.config/systemd/user/x3fronend.service
~/.config/systemd/user/cloudflared-tunnel.service
~/.config/systemd/user/x3-chain-node.service
```

### Bootstrap Process on Reboot

1. System boots → systemd user manager starts
2. loginctl enable-linger ensures user session persists
3. Services start in order:
   - x3-desktop starts first (port 5173)
   - x3fronend starts next (port 4174)
   - cloudflared-tunnel starts after (waits for both frontends)
   - x3-chain-node starts independently (skips if binary missing)
4. All services auto-restart if they crash (10s backoff)

### Service Unit Templates

Copy or symlink the provided templates into `~/.config/systemd/user/`:

```bash
mkdir -p ~/.config/systemd/user
cp scripts/systemd/x3-desktop.service ~/.config/systemd/user/
cp scripts/systemd/x3fronend.service ~/.config/systemd/user/
cp scripts/systemd/cloudflared-tunnel.service ~/.config/systemd/user/
```

Then reload user systemd units:

```bash
systemctl --user daemon-reload
```

### Verify Auto-Start is Enabled

```bash
# Check if linger is enabled
loginctl show-user lojak | grep Linger

# Check if services are enabled
systemctl --user list-unit-files | grep x3-

# Expected output: all should show "enabled"
```

---

## 🎮 Control Services

### Full Infrastructure Manager Script

```bash
./x3-infrastructure.sh status              # Show all service status + health
./x3-infrastructure.sh start               # Start all services
./x3-infrastructure.sh stop                # Stop all services
./x3-infrastructure.sh restart             # Restart all services
./x3-infrastructure.sh logs [service]      # Stream logs (desktop|node|tunnel)
./x3-infrastructure.sh enable              # Enable auto-start at boot
./x3-infrastructure.sh disable             # Disable auto-start at boot
./x3-infrastructure.sh boot-test           # Simulate reboot (stop/restart all)
```

### Manual Control (systemctl)

```bash
# Start individual services
systemctl --user start x3-desktop.service
systemctl --user start cloudflared-tunnel.service
systemctl --user start x3-chain-node.service

# Check status
systemctl --user status x3-desktop.service

# Stream logs
journalctl --user -u x3-desktop.service -f

# Enable/disable auto-start
systemctl --user enable x3-desktop.service
systemctl --user disable x3-desktop.service
```

### Local development without Cloudflare

When debugging the desktop or HTML frontend, use the direct local URLs instead of the public Cloudflare domain:

- Desktop app: http://localhost:5173
- HTML site: http://127.0.0.1:4174

This avoids Cloudflare Rocket Loader and CSP-related interference during development.

To start both local frontends together:

```bash
chmod +x scripts/dev-local.sh
./scripts/dev-local.sh
```

---

## 🔗 Tunnel Routing

The cloudflared tunnel routes all traffic from x3star.net and x3star.org to local services:

```
x3star.net:443  ──→  localhost:4174  (x3fronend HTML site)
x3star.org:443  ──→  localhost:5173  (x3-desktop)
rpc.x3star.net  ──→  localhost:9933  (Substrate RPC HTTP)
ws.x3star.net   ──→  localhost:9944  (Substrate RPC WS)
```

**Config Location:** `~/.cloudflared/config.yml`

---

## 📊 Service Details

### x3-desktop.service
- **Command:** `npm run dev`
- **Port:** 5173
- **WorkDir:** apps/x3-desktop
- **Memory:** 4GB max
- **Restart:** always, 10s delay on crash

### x3fronend.service
- **Command:** `PORT=4174 npm run server`
- **Port:** 4174
- **WorkDir:** x3fronend
- **Restart:** always, 10s delay on crash

### cloudflared-tunnel.service
- **Command:** `cloudflared --config ~/.cloudflared/config.yml tunnel run atlas-sphere`
- **Tunnel ID:** 6c118620-18cf-4795-80a8-6d44d37aecaa
- **Depends On:** x3-desktop, x3fronend
- **Restart:** always, 10s delay on crash

### x3-chain-node.service
- **Command:** `x3-chain-node --dev --rpc-external --ws-external`
- **Ports:** 9933 (HTTP RPC), 9944 (WS RPC)
- **Binary:** target/release/x3-chain-node
- **Memory:** 8GB max
- **Restart:** every 30s on crash (but skips if binary missing)

---

## ⚠️ Node Build Issue

The x3-chain-node binary build failed with Rust compilation errors in `serde_core`.

**To Fix:**

```bash
# Option 1: Clean rebuild
cargo clean
cargo build --release -p x3-chain-node

# Option 2: Debug the error
cargo build --release -p x3-chain-node 2>&1 | tail -50

# Once built, enable the service
systemctl --user start x3-chain-node.service
```

---

## 🧪 Testing After Reboot

After a system reboot, verify services started automatically:

```bash
# Check all services
./x3-infrastructure.sh status

# Or manually
ps aux | grep -E "vite|cloudflared|x3-chain-node"

# Test endpoints
curl https://x3star.net -I          # Should return 200
curl http://localhost:5173 -I       # Should return 200
curl http://localhost:9933 -I       # Should return when node running
```

---

## 📝 Service Logs

All services log to systemd journal:

```bash
# View x3-desktop logs
journalctl --user -u x3-desktop.service -n 100

# Stream desktop logs live
journalctl --user -u x3-desktop.service -f

# View all logs for all services
journalctl --user -n 200 -f \
  -u x3-desktop.service \
  -u cloudflared-tunnel.service \
  -u x3-chain-node.service
```

---

## ✨ What Starts Automatically on Reboot

After system reboot:

1. ✅ x3-desktop starts on :5173
2. ✅ cloudflared tunnel routes x3star.net → localhost:5173
3. ⚠️ x3-chain-node starts (if binary is built)
4. ✅ Live-data script connects to chain RPC and updates all 40 HTML pages

**Accessed at:**
- 🌐 https://x3star.net (Tauri desktop app)
- 🌐 https://x3star.org (HTML site)
- 🔗 https://rpc.x3star.net (Substrate HTTP RPC)
- 🌊 wss://ws.x3star.net (Substrate WebSocket RPC)

---

## 🛠️ Troubleshooting

### Service won't start
```bash
systemctl --user status x3-desktop.service
journalctl --user -u x3-desktop.service
```

### Port already in use
```bash
ss -tlnp | grep 5173  # Find what's using port
kill -9 <PID>         # Kill the process
systemctl --user restart x3-desktop.service
```

### Tunnel not connecting
```bash
systemctl --user restart cloudflared-tunnel.service
sleep 3
curl -I https://x3star.net
```

### Services didn't start on reboot
```bash
# Check if linger is still enabled
loginctl show-user lojak | grep Linger

# Check if services are still enabled
systemctl --user list-unit-files | grep x3-
```

---

## 📋 Files Involved

**Systemd Services:**
- `~/.config/systemd/user/x3-desktop.service`
- `~/.config/systemd/user/cloudflared-tunnel.service`
- `~/.config/systemd/user/x3-chain-node.service`

**Tunnel Config:**
- `~/.cloudflared/config.yml` (routes x3star.net domains)

**Infrastructure Manager:**
- `./x3-infrastructure.sh` (control script)

**Live Data:**
- `x3fronend/js/x3-live-data.js` (connects to RPC, updates pages)

---

**Last Updated:** 2026-05-09  
**Status:** ✅ Desktop + Tunnel running, ⚠️ Node build pending
