# 🚀 X3 Chain: In-House Deployment - Quick Start

## You Have 3 Options:

### 🎮 Option 1: FASTEST - Dev Mode (Single Process)
**Perfect for testing everything on your current machine RIGHT NOW:**

```bash
# From repository root
./target/release/x3-chain-node --dev --tmp --rpc-external --rpc-cors all
```

**That's it!** Connect at: `ws://localhost:9944`

---

### 🏠 Option 2: EASY - Local Testnet (Automated Script)
**Full 3-validator testnet on ONE machine with systemd services:**

```bash
cd deployment
./deploy-local-testnet.sh
```

**What it does:**
- ✅ Installs binary to `/usr/local/bin/`
- ✅ Creates 4 systemd services (bootnode + 3 validators)
- ✅ Deploys all validator keys
- ✅ Starts everything automatically
- ✅ Verifies block production

**Check status:**
```bash
sudo systemctl status x3-bootnode
sudo journalctl -u x3-validator-01 -f
```

---

### 🌐 Option 3: PRODUCTION - Multi-Server (Custom Setup)
**Deploy across multiple physical/virtual servers:**

1. **Edit server IPs** in `deploy-multi-server.sh`:
```bash
declare -A SERVERS=(
    ["bootnode"]="user@192.168.1.10"
    ["validator-01"]="user@192.168.1.11"
    ["validator-02"]="user@192.168.1.12"
    ["validator-03"]="user@192.168.1.13"
)
```

2. **Run deployment:**
```bash
cd deployment
./deploy-multi-server.sh
```

**Prerequisites:**
- SSH access to all servers (passwordless)
- Ubuntu 22.04+ on all servers
- Ports 30333 (P2P) and 9944 (RPC) open

---

## 📚 Full Documentation

If you are deploying on the currently owned rack plus optional remote VPS hosts, read **[HARDWARE_ROLE_PLAN.md](HARDWARE_ROLE_PLAN.md)** before touching `deploy-multi-server.sh`. That document assigns actual X3 roles to the Threadripper workstation, Lenovo servers, DL380p, R710, and Xserve, and it separates the single-site lab topology from the minimum public-testnet topology.

See **[IN_HOUSE_DEPLOYMENT.md](IN_HOUSE_DEPLOYMENT.md)** for:
- Server requirements
- Manual setup steps
- Firewall configuration
- Monitoring setup
- Troubleshooting guide

---

## 🎯 What Do You Want?

**Tell me which option fits your setup:**

1. **Just testing locally?** → Use Option 1 (dev mode)
2. **Want full testnet on this machine?** → Use Option 2 (automated)
3. **Have multiple servers ready?** → Use Option 3 (multi-server)

**Or tell me your server setup and I'll customize the deployment!**

---

## ⚡ System Requirements

### Minimum (Per Node):
- CPU: 4 cores
- RAM: 8GB
- Disk: 100GB SSD
- Network: 100Mbps+

### Recommended (Validators):
- CPU: 8+ cores
- RAM: 16GB+
- Disk: 500GB NVMe
- Network: 1Gbps

---

## 🆘 Quick Troubleshooting

**Binary won't start?**
```bash
ldd /usr/local/bin/x3-chain-node  # Check dependencies
./target/release/x3-chain-node --version  # Test binary
```

**Nodes not connecting?**
```bash
# Check bootnode peer ID
x3-chain-node key inspect-node-key --file deployment/keys/bootnode-key.txt

# Test connectivity
telnet BOOTNODE_IP 30333
```

**Not producing blocks?**
```bash
# Check if keys are loaded
ls -la /var/lib/x3-chain/validator-01/chains/x3_testnet/keystore/

# Check logs
sudo journalctl -u x3-validator-01 -n 100
```

---

## 🎉 Ready to Launch?

**Choose your path and let's GO! 🚀**
