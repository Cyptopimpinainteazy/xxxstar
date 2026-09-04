# X3 Chain Testnet v1 - Deployment Package Ready! 🚀

**Status**: Day -2 Complete, Day -1 In Progress (Build Running)  
**Date**: November 8, 2025  
**Deployment Team**: Ready to execute

---

## 📦 Deployment Package Created

### ✅ Infrastructure Setup (Day -2) - COMPLETE

**Created Files:**
- ✅ `deployment/infrastructure-setup.sh` - Main setup script
- ✅ `deployment/inventory.yaml` - Infrastructure inventory template
- ✅ `deployment/provision-digitalocean.sh` - DigitalOcean automation
- ✅ `docs/docs/deployment/provision-aws.md` - AWS EC2 guide
- ✅ `docs/docs/deployment/provision-manual.md` - Manual/VPS provider guide
- ✅ `docs/docs/deployment/dns-config.md` - DNS configuration guide
- ✅ `deployment/configure-firewall.sh` - Firewall setup script
- ✅ `~/.ssh/x3-testnet-deploy` - SSH key generated

**What You Need to Do Next:**
1. Choose your infrastructure provider:
   - **DigitalOcean**: Run `./deployment/provision-digitalocean.sh`
   - **AWS EC2**: Follow `docs/docs/deployment/provision-aws.md`
   - **Manual/VPS**: Follow `docs/docs/deployment/provision-manual.md`

2. Update `deployment/inventory.yaml` with actual IPs after provisioning

3. Configure DNS records using `docs/docs/deployment/dns-config.md`

4. Run firewall setup on each node:
   ```bash
   ssh x3@NODE_IP 'bash -s' < deployment/configure-firewall.sh validator
   ```

---

### ⏳ Build & Key Generation (Day -1) - IN PROGRESS

**Currently Running:**
- 🔨 `cargo build --release` (10-30 minutes)
- Output: `deployment/build.log`

**Created Files:**
- ✅ `deployment/build-and-keygen.sh` - Build and key generation script

**What Will Be Generated (after build completes):**
- `target/release/x3-chain-node` - Release binary (~200MB)
- `deployment/chain-specs/x3-testnet-raw.json` - Chain specification
- `deployment/keys/validator-0X-summary.txt` - Validator keys (3-5 sets)
- `deployment/keys/bootnode-info.txt` - Bootnode configuration
- `deployment/keys/sudo-key.txt` - Development sudo key

**What You Need to Do After Build:**
1. Wait for `cargo build --release` to finish
2. Run `./deployment/build-and-keygen.sh` to generate keys
3. **CRITICAL**: Backup keys immediately (encrypted!)
   ```bash
   tar czf - deployment/keys | gpg -e -r admin@x3-chain.io \
     > x3-testnet-keys-$(date +%Y%m%d).tar.gz.gpg
   ```

---

### 📋 Node Deployment (Day 1) - READY

**Created Files:**
- ✅ `deployment/deploy-nodes-day1.sh` - Automated deployment script

**What This Script Does:**
1. **Deploy Bootnode** (first!):
   - Copies binary and chain spec
   - Installs systemd service
   - Starts bootnode
   - Extracts peer ID for validators

2. **Deploy Validators** (3-5 nodes):
   - Copies binary and chain spec
   - Installs systemd service
   - Starts validator
   - Inserts Aura + GRANDPA authority keys via RPC
   - Verifies keys loaded

3. **Verify Network**:
   - Checks peer connections
   - Monitors for block production
   - Confirms finalization

**What You Need to Do:**
1. Ensure infrastructure provisioned (Day -2)
2. Ensure build complete and keys generated (Day -1)
3. Run: `./deployment/deploy-nodes-day1.sh`
4. Follow prompts to enter IPs for each node
5. Monitor logs for first blocks!

---

## 🚀 Quick Start Deployment

### Option 1: Automated (Recommended for DigitalOcean)

```bash
# Day -2: Provision infrastructure
./deployment/infrastructure-setup.sh
./deployment/provision-digitalocean.sh  # Or follow AWS/manual guide

# Update inventory with actual IPs
vim deployment/inventory.yaml

# Configure DNS
# Follow docs/docs/deployment/dns-config.md

# Day -1: Build and generate keys (CURRENTLY RUNNING)
# Wait for cargo build --release to finish...
./deployment/build-and-keygen.sh

# BACKUP KEYS!
tar czf - deployment/keys | gpg -e -r your@email.com \
  > x3-keys-backup.tar.gz.gpg

# Day 1: Deploy nodes
./deployment/deploy-nodes-day1.sh

# Days 2-5: Continue with remaining scripts (to be created)
```

### Option 2: Manual Step-by-Step

1. **Provision VMs** (your cloud provider apps/dash-legacy-2-legacy-2board)
   - 3-5 validators (4GB RAM, 2 vCPU)
   - 2+ RPC nodes (8GB RAM, 4 vCPU)
   - 1 bootnode (2GB RAM, 1 vCPU)
   - 1 monitoring (4GB RAM, 2 vCPU)

2. **Configure DNS**
   - Point rpc.testnet.x3-chain.io → RPC load balancer
   - Point bootnode.testnet.x3-chain.io → Bootnode IP
   - Point faucet.testnet.x3-chain.io → Faucet server
   - Point metrics.testnet.x3-chain.io → Grafana server

3. **Build Binary**
   ```bash
   cargo build --release
   ```

4. **Generate Keys**
   ```bash
   # Install subkey if not present
   cargo install --force --git https://github.com/paritytech/substrate subkey
   
   # Generate validator keys
   subkey generate --scheme Sr25519  # Aura
   subkey generate --scheme Ed25519  # GRANDPA
   # Repeat for each validator
   ```

5. **Generate Chain Spec**
   ```bash
   ./target/release/x3-chain-node build-spec \
     --chain local > x3-testnet-plain.json
   
   # Edit: name, id, bootnodes, initial authorities
   
   ./target/release/x3-chain-node build-spec \
     --chain x3-testnet-plain.json --raw \
     > x3-testnet-raw.json
   ```

6. **Deploy Nodes** (manually SSH to each)
   - Copy binary: `/usr/local/bin/x3-chain-node`
   - Copy chain spec: `/etc/x3/x3-testnet-raw.json`
   - Create systemd service
   - Start services
   - Insert keys via RPC

---

## 📊 Current Status

| Task | Status | Time Estimate | Notes |
|------|--------|---------------|-------|
| **Day -2: Infrastructure** | ✅ Complete | 2-4 hours | Scripts created, ready to provision |
| **Day -1: Build & Keys** | ⏳ In Progress | 10-30 min build | `cargo build --release` running |
| **Day 1: Deploy Nodes** | 📋 Ready | 2-3 hours | Script ready, pending build |
| **Day 2: RPC + Faucet** | 📝 Planned | 2-3 hours | Script to be created |
| **Day 3: Monitoring** | 📝 Planned | 2-3 hours | Script to be created |
| **Day 4: Testing** | 📝 Planned | 4-6 hours | Comprehensive testing |
| **Day 5: Launch** | 🎉 Planned | 1-2 hours | Public announcement |

**Total Time to Launch**: 5-7 days (with infrastructure setup)

---

## 🔐 Security Checklist

### Keys Management
- ✅ Keys directory `.gitignored` automatically
- ⏳ Backup keys encrypted (do after generation)
- ⏳ Distribute to validators via encrypted channel
- ⏳ Store backups in 3 locations (cloud + USB + vault)

### Infrastructure Security
- ⏳ SSH keys deployed (do on each node)
- ⏳ Firewall configured (use `configure-firewall.sh`)
- ⏳ Restrict SSH to admin IPs only
- ⏳ RPC ports only accessible via load balancer
- ⏳ Prometheus metrics restricted to monitoring server

### Network Security
- ⏳ DDoS protection (Cloudflare for RPC endpoints)
- ⏳ Rate limiting on public RPC (1000 req/min)
- ⏳ Faucet captcha configured
- ⏳ Faucet rate limits (100 tATLAS per 24h)

---

## 📁 File Structure

```
deployment/
├── infrastructure-setup.sh          ✅ Created
├── inventory.yaml                   ✅ Created (needs IPs)
├── provision-digitalocean.sh        ✅ Created
├── provision-aws.md                 ✅ Created
├── provision-manual.md              ✅ Created
├── dns-config.md                    ✅ Created
├── configure-firewall.sh            ✅ Created
├── build-and-keygen.sh              ✅ Created
├── deploy-nodes-day1.sh             ✅ Created
├── build.log                        ⏳ In progress
├── chain-specs/
│   ├── x3-dev-plain.json         ⏳ Will be generated
│   ├── x3-testnet-plain.json     ⏳ Will be generated
│   ├── x3-testnet-raw.json       ⏳ Will be generated (deploy this)
│   └── x3-staging-plain.json     ⏳ Will be generated
├── keys/
│   ├── .gitignore                   ⏳ Will be generated
│   ├── KEYS_MANIFEST.md             ⏳ Will be generated
│   ├── validator-01-summary.txt     ⏳ Will be generated
│   ├── validator-02-summary.txt     ⏳ Will be generated
│   ├── validator-03-summary.txt     ⏳ Will be generated
│   ├── bootnode-info.txt            ⏳ Will be generated
│   └── sudo-key.txt                 ⏳ Will be generated
└── x3-chain-node                ⏳ Will be copied from target/release/
```

---

## ⚡ Next Actions (Priority Order)

### Immediate (Now)
1. ✅ Wait for `cargo build --release` to complete (check with: `tail -f deployment/build.log`)
2. Provision VMs using your preferred method:
   - DigitalOcean: `./deployment/provision-digitalocean.sh`
   - AWS: Follow `docs/docs/deployment/provision-aws.md`
   - Manual: Follow `docs/docs/deployment/provision-manual.md`

### After Build Completes
3. Run `./deployment/build-and-keygen.sh`
4. **IMMEDIATELY backup keys** (encrypted!)
5. Update `deployment/inventory.yaml` with actual IPs

### Day 1 (After Infrastructure Ready)
6. Configure DNS records
7. Run firewall setup on each node
8. Run `./deployment/deploy-nodes-day1.sh`
9. Monitor logs for first blocks!

---

## 🆘 Troubleshooting

### Build Taking Too Long
```bash
# Check build progress
tail -f deployment/build.log

# Typical times:
# - Fast machine (16+ cores): 5-10 min
# - Medium machine (4-8 cores): 10-20 min
# - Slow machine (2 cores): 20-30 min
```

### Build Fails
```bash
# Common fixes:
# 1. Update Rust
rustup update stable

# 2. Clean and retry
cargo clean
cargo build --release
```

### Can't SSH to VMs
```bash
# Check SSH key
ssh -i ~/.ssh/x3-testnet-deploy x3@VM_IP

# Add key to agent if needed
ssh-add ~/.ssh/x3-testnet-deploy
```

### Firewall Blocks Deployment
```bash
# Temporarily disable for setup (re-enable after!)
ssh x3@VM_IP 'sudo ufw disable'

# Deploy, then re-enable
ssh x3@VM_IP 'sudo ufw enable'
```

---

## 📞 Support Channels

**During Deployment:**
- **Technical Issues**: Check docs/reports/docs/runbooks/deployment/DEPLOYMENT_GUIDE.md troubleshooting section
- **Script Errors**: Review script output, check prerequisites
- **Infrastructure**: Consult your cloud provider docs

**After Launch:**
- **Developer Support**: Discord #testnet-support
- **Bug Reports**: GitHub issues
- **Security Issues**: security@x3-chain.io (private)

---

## 🎉 Ready to Deploy!

All infrastructure scripts are created and ready. Build is currently running.

**Estimated Time to Public Launch**: 5-7 days

**Next Milestone**: Complete Day -1 (build + keys) → Proceed to Day 1 (node deployment)

---

**Status**: 🟢 ON TRACK  
**Build Progress**: Check `deployment/build.log`  
**Last Updated**: November 8, 2025

**Let's launch X3 Chain Testnet v1! 🚀**
