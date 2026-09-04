# 🎯 X3 Chain Deployment - Complete Guide

## 📚 Documentation Index

### Quick Start
- **[QUICKSTART.md](QUICKSTART.md)** - Choose your deployment path (3 options)
- **[HARDWARE_ROLE_PLAN.md](HARDWARE_ROLE_PLAN.md)** - Exact role mapping for the current rack and the minimum VPS extension for a credible public testnet
- **[RACK_BRINGUP_RUNBOOK.md](RACK_BRINGUP_RUNBOOK.md)** - Host-by-host bring-up using reusable systemd env files and strict firewall inputs
- **[IN_HOUSE_DEPLOYMENT.md](IN_HOUSE_DEPLOYMENT.md)** - Complete in-house deployment guide
- **[PUBLIC_RPC.md](PUBLIC_RPC.md)** - Run public RPC (dev/local/staging/testnet) safely

### Deployment Scripts
- **[../../deployment/deploy-local-testnet.sh](../../deployment/deploy-local-testnet.sh)** - Automated single-server deployment
- **[../../deployment/deploy-multi-server.sh](../../deployment/deploy-multi-server.sh)** - Multi-server deployment automation
- **[../../deployment/manage-testnet.sh](../../deployment/manage-testnet.sh)** - Control and monitor your testnet

### Build & Keys
- **[../../deployment/build-and-keygen.sh](../../deployment/build-and-keygen.sh)** - Build binary and generate validator keys
- **[inventory.yaml](../../deployment/inventory.yaml)** - Hardware-backed inventory with real roles and null fields that must be filled before deployment
- **[servers.env.example](../../deployment/servers.env.example)** - Copy to `deployment/servers.env` and set real SSH targets for multi-server deployment
- **[x3-chain-node@.service](../../deployment/systemd/x3-chain-node@.service)** - Reusable node service template for rack hosts
- **[systemd/examples](../../deployment/systemd/examples)** - Rack-specific env examples for the current hardware plan
- **[keys/](../../deployment/keys/)** - Validator keys and bootnode key (KEEP SECURE!)
- **[chain-specs/](../../deployment/chain-specs/)** - Chain specification files

---

## 🚀 Three Deployment Paths

### 1️⃣ Dev Mode (Instant Testing)
**Single command, immediate results:**
```bash
./target/release/x3-chain-node --dev --tmp --rpc-external
```
- ✅ Perfect for development
- ✅ Instant startup
- ✅ Single validator
- ⚠️ Data deleted on exit

### 2️⃣ Local Testnet (Production-Like)
**Full testnet on one machine:**
```bash
cd deployment
./deploy-local-testnet.sh
```
- ✅ 3 validators + bootnode
- ✅ Systemd services
- ✅ Persistent data
- ✅ Full consensus

### 3️⃣ Multi-Server (Distributed)
**Production deployment across servers:**
```bash
# Edit server IPs first!
vim deploy-multi-server.sh

# Then deploy
./deploy-multi-server.sh
```
- ✅ Distributed validators
- ✅ Geographic diversity
- ✅ Production-ready
- ✅ High availability

---

## 🛠️ Management Commands

After deploying with **deploy-local-testnet.sh**, use:

```bash
# Check status
./manage-testnet.sh status

# View logs
./manage-testnet.sh logs x3-validator-01

# Restart everything
./manage-testnet.sh restart

# Run health checks
./manage-testnet.sh health

# Emergency: Delete all data
./manage-testnet.sh purge
```

---

## 📊 Monitoring Endpoints

After deployment, your RPC endpoints are:

| Service | Port | URL |
|---------|------|-----|
| Bootnode | 9944 | http://localhost:9944 |
| Validator-01 | 9945 | http://localhost:9945 |
| Validator-02 | 9946 | http://localhost:9946 |
| Validator-03 | 9947 | http://localhost:9947 |

### Connect via Polkadot.js
1. Open: https://polkadot.js.org/apps/
2. Settings → Custom endpoint
3. Enter: `ws://localhost:9944` (or your server IP)
4. Save & Connect

---

## 🔐 Security Considerations

### Keys Location
```
deployment/keys/
├── validator-01-keys/
│   ├── keystore/        # SENSITIVE: Validator keys
│   └── session-keys.txt # Session key hex
├── validator-02-keys/
├── validator-03-keys/
└── bootnode-key.txt     # Bootnode secret key
```

**⚠️ CRITICAL:**
- **NEVER** commit keys to git (already in .gitignore)
- **BACKUP** keys securely (encrypted, offline storage)
- **ROTATE** keys periodically for production

### Firewall Configuration

**Required ports:**
- `30333`: P2P communication (ALL validators/bootnodes)
- `9944-9947`: RPC (ONLY from trusted IPs)
- `9615`: Prometheus metrics (OPTIONAL, trusted networks only)

**Ubuntu UFW example:**
```bash
# Allow P2P from anywhere
sudo ufw allow 30333/tcp

# Allow RPC only from your IP
sudo ufw allow from YOUR_IP to any port 9944
```

---

## 📈 Performance Tuning

### Validator Nodes
```bash
# Increase file descriptors
ulimit -n 65536

# Use archive pruning for full node
--pruning archive

# Enable telemetry (optional)
--telemetry-url 'wss://telemetry.polkadot.io/submit/ 0'
```

### Database Optimization
```bash
# Use RocksDB (default, optimized)
--database rocksdb

# Increase cache for better performance
--db-cache 2048  # MB
```

---

## 🐛 Troubleshooting

### Services Won't Start
```bash
# Check systemd errors
sudo systemctl status x3-bootnode
sudo journalctl -u x3-bootnode -n 50

# Check binary
/usr/local/bin/x3-chain-node --version

# Check permissions
ls -la /var/lib/x3-chain/
```

### Nodes Not Connecting
```bash
# Verify bootnode peer ID
x3-chain-node key inspect-node-key \
  --file deployment/keys/bootnode-key.txt

# Test network connectivity
telnet BOOTNODE_IP 30333

# Check firewall
sudo ufw status
sudo iptables -L -n
```

### No Block Production
```bash
# Check validator keys
ls -la /var/lib/x3-chain/validator-01/chains/x3_testnet/keystore/

# Verify session keys via RPC
curl -s http://localhost:9945 -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "author_rotateKeys"}' | jq

# Check active validators
curl -s http://localhost:9944 -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "session_validators"}' | jq
```

### High Resource Usage
```bash
# Check CPU/Memory
top -p $(pgrep -d',' x3-chain-node)

# Reduce pruning
--pruning 1000  # Keep last 1000 blocks

# Limit peers
--in-peers 25 --out-peers 25
```

---

## 📋 Pre-Deployment Checklist

### System Requirements
- [ ] Ubuntu 22.04+ (or compatible Linux)
- [ ] 8GB+ RAM per validator
- [ ] 100GB+ SSD storage
- [ ] Static IP or dynamic DNS
- [ ] Open ports: 30333, 9944

### Prerequisites
- [ ] Binary built: `./target/release/x3-chain-node`
- [ ] Keys generated: `deployment/keys/*`
- [ ] Chain spec created: `deployment/chain-specs/x3-testnet-raw.json`
- [ ] SSH access configured (for multi-server)
- [ ] Firewall rules planned

### Post-Deployment Verification
- [ ] All services running: `./manage-testnet.sh status`
- [ ] Blocks being produced (check logs)
- [ ] Peers connected (3+ peers each)
- [ ] RPC responding: `curl http://localhost:9944`
- [ ] Polkadot.js connects successfully

---

## 🎓 Next Steps After Deployment

### Day 1: Core Infrastructure ✅
- [x] Deploy validators
- [x] Verify block production
- [x] Check consensus

### Day 2: RPC & Services
- [ ] Deploy dedicated RPC nodes
- [ ] Set up load balancer (optional)
- [ ] Deploy faucet service
- [ ] Public RPC endpoints

### Day 3: Developer Tools
- [ ] Deploy block explorer
- [ ] Set up monitoring (Prometheus/Grafana)
- [ ] Create developer documentation
- [ ] Test EVM integration

### Day 4: Testing & Security
- [ ] Load testing
- [ ] Security audit
- [ ] Penetration testing
- [ ] Backup & recovery procedures

### Day 5: PUBLIC LAUNCH 🚀
- [ ] Final verification
- [ ] Announce to community
- [ ] Open public access
- [ ] Monitor initial adoption

---

## 📞 Support & Resources

### Logs Location
```bash
# Systemd logs
sudo journalctl -u x3-bootnode -f
sudo journalctl -u x3-validator-01 -f

# Or use management script
./manage-testnet.sh logs x3-bootnode
```

### Configuration Files
```
/etc/systemd/system/x3-bootnode.service
/etc/systemd/system/x3-validator-01.service
/etc/systemd/system/x3-validator-02.service
/etc/systemd/system/x3-validator-03.service
```

### Data Directories
```
/var/lib/x3-chain/bootnode/
/var/lib/x3-chain/validator-01/
/var/lib/x3-chain/validator-02/
/var/lib/x3-chain/validator-03/
```

---

## 🎉 You're Ready!

**Choose your deployment path:**

1. **Quick test?** → Run dev mode: `./target/release/x3-chain-node --dev --tmp`
2. **Local testnet?** → Run: `./deploy-local-testnet.sh`
3. **Multi-server?** → Edit and run: `./deploy-multi-server.sh`

**Questions? Check:**
- [QUICKSTART.md](QUICKSTART.md) for fast paths
- [IN_HOUSE_DEPLOYMENT.md](IN_HOUSE_DEPLOYMENT.md) for detailed guide
- [../../deployment/manage-testnet.sh](manage-testnet.sh) for operations

---

**Let's launch this testnet! 🚀**
