# Validator Onboarding Runbook

**Document Version:** 1.0  
**Date:** April 26, 2026  
**Status:** PUBLIC (Community Validator Guide)  
**Target Audience:** Prospective validators, community members, node operators

---

## Executive Summary

This runbook provides **complete instructions** for setting up and operating an X3 ATOMIC STAR validator node. Whether you're joining the testnet or preparing for mainnet, this guide covers everything from hardware selection to slashing prevention.

**What You'll Learn:**
- Hardware requirements and provider recommendations
- Complete node installation (from source or Docker)
- Key generation and management (including HSM)
- Validator registration and staking
- Monitoring and alerting setup
- Operations procedures (upgrades, maintenance)
- Common troubleshooting scenarios

**Estimated Setup Time:**
- Basic setup: 2-4 hours
- With monitoring: 4-6 hours
- With HSM: 6-8 hours

**Cost Estimates:**
- Hardware: $200-500/month (cloud) or $3k-8k (dedicated)
- Stake: Variable (minimum 500k X3 for testnet)
- Operations: $50-200/month (monitoring, backups)

---

## Table of Contents

1. [Prerequisites & Hardware Requirements](#1-prerequisites--hardware-requirements)
2. [Software Installation](#2-software-installation)
3. [Key Management](#3-key-management)
4. [Node Configuration](#4-node-configuration)
5. [Validator Registration](#5-validator-registration)
6. [Monitoring & Alerting](#6-monitoring--alerting)
7. [Operations Procedures](#7-operations-procedures)
8. [Slashing Prevention](#8-slashing-prevention)
9. [Troubleshooting Guide](#9-troubleshooting-guide)
10. [FAQ](#10-faq)

---

## 1. Prerequisites & Hardware Requirements

### Minimum Requirements (Testnet)

```yaml
cpu:
  cores: 4
  architecture: "x86_64 (Intel/AMD) or ARM64"
  recommendation: "Intel Xeon, AMD EPYC, or AWS Graviton"

memory:
  minimum: "16 GB RAM"
  recommended: "32 GB RAM"
  swap: "4 GB (in case of memory spikes)"

storage:
  type: "NVMe SSD (required - HDD will NOT work)"
  minimum: "500 GB"
  recommended: "1 TB"
  iops: "10,000+ IOPS"
  note: "Database grows ~10-50 GB/month"

network:
  bandwidth: "100 Mbps minimum, 1 Gbps recommended"
  latency: "<100ms to other validators"
  ports_required:
    - 30333: "P2P (libp2p) - must be publicly accessible"
    - 9944: "RPC (websocket) - localhost only"
    - 9933: "RPC (http) - localhost only"
    - 9615: "Prometheus metrics (optional)"

os:
  recommended: "Ubuntu 22.04 LTS"
  alternatives: ["Debian 12", "Fedora 38+", "Arch Linux"]
  not_supported: ["Windows", "macOS"]
```

### Recommended Requirements (Mainnet)

```yaml
cpu:
  cores: 8+
  model: "AMD EPYC 7002+ or Intel Xeon Gold"

memory:
  size: "64 GB RAM"

storage:
  size: "2 TB NVMe SSD"
  raid: "RAID 1 (mirrored) for redundancy"

network:
  bandwidth: "1 Gbps symmetric"
  redundancy: "Dual ISP (failover)"
```

### Cloud Provider Recommendations

| Provider | Instance Type | vCPU | RAM | Storage | Network | Cost/Month |
|----------|--------------|------|-----|---------|---------|------------|
| **AWS** | c6i.2xlarge | 8 | 16 GB | +1TB EBS | 5 Gbps | ~$280 |
| **AWS** | m6i.2xlarge | 8 | 32 GB | +1TB EBS | 10 Gbps | ~$350 |
| **Google Cloud** | n2-standard-8 | 8 | 32 GB | +1TB PD-SSD | 10 Gbps | ~$320 |
| **DigitalOcean** | Premium 8vCPU | 8 | 16 GB | +1TB NVMe | 6 Gbps | ~$240 |
| **Hetzner** | AX51-NVMe | 8 | 64 GB | 2x512GB NVMe | 1 Gbps | ~$70 (🏆 best value) |
| **OVH** | Rise-3 | 8 | 32 GB | 2x512GB NVMe | 1 Gbps | ~$85 |

**🏆 Recommended for Budget:** Hetzner dedicated servers (excellent price/performance)  
**🏆 Recommended for Enterprise:** AWS/GCP with auto-scaling, backups, DDoS protection

### Dedicated Hardware (Own Servers)

**Budget Build (~$3k):**
- CPU: AMD Ryzen 9 5950X (16 cores)
- RAM: 64 GB DDR4-3200
- Storage: 2x 2TB Samsung 980 Pro NVMe (RAID 1)
- Network: 1 Gbps fiber
- UPS: CyberPower 1500VA

**Production Build (~$8k):**
- CPU: AMD EPYC 7443P (24 cores)
- RAM: 128 GB DDR4-3200 ECC
- Storage: 4x 2TB Samsung PM9A3 NVMe (RAID 10)
- Network: 10 Gbps fiber with redundant uplinks
- UPS: APC Smart-UPS 3000VA with monitoring

### Datacenter Requirements

✅ **Required:**
- 99.9%+ uptime SLA
- DDoS protection (10+ Gbps)
- Backup power (UPS + generator)
- 24/7 on-site staff (if colo)
- Physical security (cameras, access control)

✅ **Recommended:**
- Multiple upstream providers (BGP routing)
- Geographic diversity (backup validator in different region)
- SOC 2 Type II or ISO 27001 certified

### Geographic Distribution (Mainnet)

**For decentralization, validators should be spread across:**
- North America: 20-30%
- Europe: 30-40%
- Asia-Pacific: 20-30%
- Other regions: 10-20%

**Anti-patterns to avoid:**
- All validators on AWS (single provider risk)
- All validators in same country (regulatory risk)
- All validators in same datacenter (single point of failure)

---

## 2. Software Installation

### Pre-Installation Checklist

- [ ] Fresh Ubuntu 22.04 LTS installation
- [ ] Root or sudo access
- [ ] Firewall configured (allow port 30333)
- [ ] Static IP address configured
- [ ] DNS records updated (optional but recommended)

### Option A: Install from Pre-Built Binary (Fastest)

**Recommended for most users**

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install dependencies
sudo apt install -y \
  curl \
  wget \
  git \
  build-essential \
  libssl-dev \
  pkg-config \
  jq

# Download latest release
cd /tmp
LATEST_VERSION="v1.0.0"  # Check https://github.com/X3/releases for latest
wget https://github.com/x3network/x3-chain/releases/download/${LATEST_VERSION}/x3-chain-node-linux-amd64.tar.gz

# Verify checksum
wget https://github.com/x3network/x3-chain/releases/download/${LATEST_VERSION}/x3-chain-node-linux-amd64.tar.gz.sha256
sha256sum -c x3-chain-node-linux-amd64.tar.gz.sha256
# Should output: x3-chain-node-linux-amd64.tar.gz: OK

# Extract and install
tar -xzf x3-chain-node-linux-amd64.tar.gz
sudo mv x3-chain-node /usr/local/bin/
sudo chmod +x /usr/local/bin/x3-chain-node

# Verify installation
x3-chain-node --version
# Expected: x3-chain-node 1.0.0-dc9d1bd
```

### Option B: Build from Source (Most Secure)

**Recommended for production mainnet validators**

```bash
# Install Rust (required for building)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Verify Rust installation
rustc --version
# Expected: rustc 1.75.0 or newer

# Install additional build dependencies
sudo apt install -y \
  clang \
  libclang-dev \
  cmake \
  protobuf-compiler

# Clone repository
git clone https://github.com/x3network/x3-chain.git
cd x3-chain

# Checkout specific version (NEVER use main/master for validators)
git checkout tags/v1.0.0

# Verify git tag signature (optional but recommended)
git verify-tag v1.0.0

# Build in release mode (takes 30-60 minutes)
cargo build --release

# Move binary to system path
sudo mv target/release/x3-chain-node /usr/local/bin/
sudo chmod +x /usr/local/bin/x3-chain-node

# Verify build
x3-chain-node --version
```

### Option C: Docker (Advanced)

```bash
# Install Docker
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh

# Pull official image
docker pull x3network/x3-chain:v1.0.0

# Verify image
docker run --rm x3network/x3-chain:v1.0.0 --version

# Create data directory
sudo mkdir -p /var/lib/x3-data
sudo chown 1000:1000 /var/lib/x3-data

# Run container
docker run -d \
  --name x3-validator \
  --restart unless-stopped \
  -p 30333:30333 \
  -v /var/lib/x3-data:/data \
  x3network/x3-chain:v1.0.0 \
  --base-path /data \
  --chain mainnet \
  --validator \
  --name "MyValidator"
```

### System Service Setup (systemd)

**Create service file:**

```bash
sudo nano /etc/systemd/system/x3-validator.service
```

**Add configuration:**

```ini
[Unit]
Description=X3 ATOMIC STAR Validator Node
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=x3
Group=x3
ExecStart=/usr/local/bin/x3-chain-node \
  --base-path /var/lib/x3-data \
  --chain mainnet \
  --validator \
  --name "MyValidator-City" \
  --port 30333 \
  --rpc-port 9944 \
  --ws-port 9945 \
  --rpc-cors all \
  --rpc-methods Safe \
  --prometheus-port 9615 \
  --prometheus-external \
  --telemetry-url 'wss://telemetry.x3.network/submit 0' \
  --bootnodes /dns/bootnode-1.x3.network/tcp/30333/p2p/12D3KooWXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX \
  --bootnodes /dns/bootnode-2.x3.network/tcp/30333/p2p/12D3KooWXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX \
  --bootnodes /dns/bootnode-3.x3.network/tcp/30333/p2p/12D3KooWXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX

Restart=always
RestartSec=10
LimitNOFILE=65536

StandardOutput=append:/var/log/x3-validator.log
StandardError=append:/var/log/x3-validator.log

[Install]
WantedBy=multi-user.target
```

**Create user and directories:**

```bash
# Create dedicated user
sudo useradd -r -s /bin/false x3

# Create data directory
sudo mkdir -p /var/lib/x3-data
sudo chown x3:x3 /var/lib/x3-data
sudo chmod 700 /var/lib/x3-data

# Create log directory
sudo mkdir -p /var/log
sudo touch /var/log/x3-validator.log
sudo chown x3:x3 /var/log/x3-validator.log
```

**Enable and start service:**

```bash
# Reload systemd
sudo systemctl daemon-reload

# Enable service (start on boot)
sudo systemctl enable x3-validator

# Start service
sudo systemctl start x3-validator

# Check status
sudo systemctl status x3-validator

# View logs
sudo journalctl -u x3-validator -f
```

### Firewall Configuration

```bash
# Using UFW (Ubuntu)
sudo ufw allow 22/tcp   # SSH
sudo ufw allow 30333/tcp  # P2P
sudo ufw allow 9615/tcp   # Prometheus (if remote monitoring)
sudo ufw enable

# Using iptables
sudo iptables -A INPUT -p tcp --dport 22 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 30333 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 9615 -j ACCEPT
sudo iptables-save | sudo tee /etc/iptables/rules.v4
```

---

## 3. Key Management

### Understanding X3 Keys

X3 validators use **5 types of keys:**

| Key Type | Algorithm | Purpose | Rotation | Storage |
|----------|-----------|---------|----------|---------|
| **Stash** | SR25519 | Holds stake, receives rewards | Never | Cold storage (offline) |
| **Controller** | SR25519 | Manages validator operations | Annually | Warm storage (secure server) |
| **BABE** | SR25519 | Block production (consensus) | Per session | Node keystore |
| **GRANDPA** | ED25519 | Finality voting (consensus) | Per session | Node keystore |
| **ImOnline** | SR25519 | Heartbeat messages | Per session | Node keystore |

### Key Generation Workflow

```
1. Generate STASH key (offline, secure location)
   ↓
2. Generate CONTROLLER key (can be same as stash for simplicity)
   ↓
3. Start node and generate SESSION keys (automated)
   ↓
4. Register session keys on-chain (links validator to node)
   ↓
5. Bond stake + nominate self as validator
```

### Step 1: Generate Stash Key (OFFLINE)

**🔴 CRITICAL: Do this on an air-gapped machine (no internet)**

```bash
# Install x3-chain-node on offline machine
# (transfer binary via USB)

# Generate stash key
x3-chain-node key generate --scheme sr25519 --output-type json > stash-key.json

# View key details
cat stash-key.json

# Example output:
{
  "secretPhrase": "word1 word2 word3 ... word12",
  "secretSeed": "0x...",
  "publicKey": "0x...",
  "ss58Address": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"
}

# ⚠️ CRITICAL: Record the following SECURELY:
# 1. secretPhrase (12 words) - write on paper, store in safe
# 2. ss58Address - this is your public validator address
```

**Key Security Best Practices:**

```bash
# Option A: Paper backup (recommended)
# 1. Write 12-word seed phrase on paper
# 2. Store in fireproof safe or safety deposit box
# 3. Consider splitting across 2 locations (Shamir Secret Sharing)

# Option B: Hardware wallet (if supported)
# 1. Import seed into Ledger/Trezor
# 2. Use hardware wallet to sign validator operations

# Option C: HSM (enterprise only)
# 1. Store key in Hardware Security Module
# 2. Access via PKCS#11 interface
# Example: YubiHSM, Thales Luna, AWS CloudHSM
```

### Step 2: Generate Session Keys (ON VALIDATOR NODE)

```bash
# Start node (must be running to generate session keys)
sudo systemctl start x3-validator

# Wait 30 seconds for node to initialize

# Generate session keys via RPC
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "author_rotateKeys"}' \
  http://localhost:9944

# Example output:
{
  "jsonrpc":"2.0",
  "result":"0x[384 hex characters representing 5 session keys]",
  "id":1
}

# Save the result (you'll need it for validator registration)
SESSION_KEYS="0x[paste the 384 hex chars here]"
echo "Session Keys: $SESSION_KEYS" > ~/session-keys.txt
```

**What just happened?**
- Node generated 5 keys (BABE, GRANDPA, ImOnline, Authority Discovery, + 1 more)
- Keys stored in `/var/lib/x3-data/chains/x3_mainnet/keystore/`
- You received the **public** session keys (384 hex chars)
- The **private** session keys remain on the node (never leave the server)

### Step 3: Key Backup

```bash
# Backup session keys (in case node crashes)
sudo tar -czf ~/session-keys-backup-$(date +%Y%m%d).tar.gz \
  /var/lib/x3-data/chains/x3_mainnet/keystore/

# Encrypt backup
gpg --symmetric --cipher-algo AES256 ~/session-keys-backup-*.tar.gz

# Move encrypted backup to secure location
# DO NOT store on same server (defeats backup purpose)

# Example: Copy to S3 with encryption
aws s3 cp ~/session-keys-backup-*.tar.gz.gpg \
  s3://my-validator-backups/session-keys/ \
  --sse AES256
```

### Step 4: Key Rotation (Best Practice)

**Session keys:** Rotate every era (24 hours on mainnet)  
**Controller key:** Rotate annually  
**Stash key:** NEVER rotate (or you lose your stake)

```bash
# Rotate session keys (automated by node, but can force manually):
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "author_rotateKeys"}' \
  http://localhost:9944

# Update session keys on-chain:
# (Use Polkadot.js UI → Staking → Session → Set Keys)
# Or via CLI:
x3-chain-node send-extrinsic \
  --suri "//YourControllerSeed" \
  session setKeys \
  [NEW_SESSION_KEYS] \
  []
```

---

## 4. Node Configuration

### Network Configuration

**Chain Specs:**
- **Mainnet:** `--chain mainnet` or path to `x3-mainnet-raw.json`
- **Testnet:** `--chain testnet` or path to `x3-testnet-raw.json`

**Bootnodes:**
```bash
# Mainnet bootnodes (always up-to-date list at https://docs.x3.network/nodes)
--bootnodes /dns/bootnode-1.x3.network/tcp/30333/p2p/12D3KooW...
--bootnodes /dns/bootnode-2.x3.network/tcp/30333/p2p/12D3KooW...
--bootnodes /dns/bootnode-3.x3.network/tcp/30333/p2p/12D3KooW...
```

### Performance Tuning

**Database Backend:**
```bash
# RocksDB (default, recommended)
--database rocksdb

# ParityDB (experimental, faster writes)
--database paritydb
```

**Caching:**
```bash
# Cache size (adjust based on RAM)
--state-cache-size 4096  # 4 GB cache (for 32 GB RAM server)
```

**Pruning:**
```bash
# Archive node (keep all state, requires 5+ TB)
--pruning archive

# Validator node (keep last 256 blocks, ~500 GB)
--pruning 256

# Minimal (keep last 64 blocks, risky for validators)
--pruning 64
```

**Telemetry:**
```bash
# Enable telemetry (appears on https://telemetry.x3.network)
--telemetry-url 'wss://telemetry.x3.network/submit 0'

# Disable telemetry (for privacy)
--no-telemetry
```

### Example Production Config

```bash
x3-chain-node \
  --base-path /var/lib/x3-data \
  --chain mainnet \
  --validator \
  --name "MyOrg-Validator-NYC" \
  \
  --port 30333 \
  --rpc-port 9944 \
  --ws-port 9945 \
  --prometheus-port 9615 \
  --prometheus-external \
  \
  --rpc-cors all \
  --rpc-methods Safe \
  --rpc-max-connections 100 \
  \
  --database rocksdb \
  --state-cache-size 4096 \
  --pruning 256 \
  \
  --max-runtime-instances 8 \
  --runtime-cache-size 2 \
  \
  --telemetry-url 'wss://telemetry.x3.network/submit 0' \
  \
  --bootnodes /dns/bootnode-1.x3.network/tcp/30333/p2p/12D3KooW... \
  --bootnodes /dns/bootnode-2.x3.network/tcp/30333/p2p/12D3KooW... \
  --bootnodes /dns/bootnode-3.x3.network/tcp/30333/p2p/12D3KooW...
```

---

## 5. Validator Registration

### Prerequisites

- [ ] Node running and synced (check: `curl -s localhost:9944 | jq '.result.isSyncing'` should be `false`)
- [ ] Stash account funded (minimum 500k X3 + transaction fees)
- [ ] Session keys generated and saved

### Step-by-Step Registration

**Option A: Using Polkadot.js UI (Easiest)**

1. **Open Polkadot.js Apps:**
   - Navigate to https://polkadot.js.org/apps/
   - Connect to X3: Settings → Custom Endpoint → `wss://rpc.x3.network`

2. **Import Stash Account:**
   - Accounts → Add Account → Import from seed
   - Paste your 12-word seed phrase
   - Set password
   - Save

3. **Bond Tokens:**
   - Network → Staking → Account Actions → + Stash
   - Stash account: Select your imported account
   - Controller account: Same as stash (or different for advanced users)
   - Value bonded: Enter amount (e.g., 500,000 X3)
   - Payment destination: Stash account (increase stake) or Stash account (do not increase)
   - Click "Bond" and sign transaction

4. **Set Session Keys:**
   - Network → Staking → Account Actions → Session Key (next to your stash)
   - Paste your session keys (384 hex chars from Step 3.2)
   - Click "Set Session Key" and sign transaction

5. **Validate:**
   - Network → Staking → Account Actions → Validate (next to your stash)
   - Commission: Set your commission (e.g., 10% = you keep 10% of rewards, nominators get 90%)
   - Click "Validate" and sign transaction

6. **Wait for Next Era:**
   - Validators are selected at the start of each era (~24 hours)
   - Check status: Network → Staking → Waiting
   - Once active: Network → Staking → Overview (you'll appear in the list)

**Option B: Using CLI (Advanced)**

```bash
# 1. Bond tokens
x3-chain-node send-extrinsic \
  --suri "//YourStashSeed" \
  staking bond \
  [CONTROLLER_ADDRESS] \
  500000000000000000 \  # 500k X3 (12 decimals)
  Staked

# 2. Set session keys
x3-chain-node send-extrinsic \
  --suri "//YourControllerSeed" \
  session setKeys \
  [YOUR_SESSION_KEYS] \
  []

# 3. Validate
x3-chain-node send-extrinsic \
  --suri "//YourControllerSeed" \
  staking validate \
  10  # 10% commission
```

### Commission and Rewards

**Setting Commission:**
- **0-5%:** Very low commission, attracts nominators but lower validator profit
- **5-10%:** Standard commission range (recommended)
- **10-20%:** Higher commission, may discourage nominators
- **>20%:** Very high commission, rarely gets nominated

**Reward Calculation:**
```
Era Rewards = Base Reward + Transaction Fees + Tips
Validator Share = Era Rewards * Commission %
Nominator Share = Era Rewards * (1 - Commission %)

Example with 10% commission:
Era Rewards: 1000 X3
Validator Share: 1000 * 0.10 = 100 X3
Nominator Share: 1000 * 0.90 = 900 X3 (split among nominators by stake)
```

---

## 6. Monitoring & Alerting

### Prometheus + Grafana Setup

**Install Prometheus:**

```bash
# Download Prometheus
cd /tmp
wget https://github.com/prometheus/prometheus/releases/download/v2.45.0/prometheus-2.45.0.linux-amd64.tar.gz
tar -xzf prometheus-2.45.0.linux-amd64.tar.gz
sudo mv prometheus-2.45.0.linux-amd64 /opt/prometheus

# Create config
sudo nano /opt/prometheus/prometheus.yml
```

**Prometheus Config:**

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'x3-validator'
    static_configs:
      - targets: ['localhost:9615']
        labels:
          instance: 'x3-validator-nyc'
```

**Start Prometheus:**

```bash
# Create systemd service
sudo nano /etc/systemd/system/prometheus.service

# Add:
[Unit]
Description=Prometheus
After=network.target

[Service]
User=prometheus
Group=prometheus
Type=simple
ExecStart=/opt/prometheus/prometheus \
  --config.file=/opt/prometheus/prometheus.yml \
  --storage.tsdb.path=/var/lib/prometheus/

[Install]
WantedBy=multi-user.target

# Create user and start
sudo useradd -r -s /bin/false prometheus
sudo mkdir -p /var/lib/prometheus
sudo chown prometheus:prometheus /var/lib/prometheus
sudo systemctl enable prometheus
sudo systemctl start prometheus
```

**Install Grafana:**

```bash
# Add Grafana repo
wget -q -O - https://packages.grafana.com/gpg.key | sudo apt-key add -
echo "deb https://packages.grafana.com/oss/deb stable main" | sudo tee /etc/apt/sources.list.d/grafana.list

# Install
sudo apt update
sudo apt install -y grafana

# Start
sudo systemctl enable grafana-server
sudo systemctl start grafana-server

# Access: http://your-ip:3000
# Default login: admin/admin (change immediately)
```

**Import X3 Validator Dashboard:**

1. Login to Grafana (http://your-ip:3000)
2. Configuration → Data Sources → Add data source → Prometheus
3. URL: http://localhost:9090
4. Save & Test
5. Import dashboard:
   - Dashboards → Import
   - Download: https://github.com/x3network/monitoring/x3-validator-dashboard.json
   - Upload JSON
   - Select Prometheus data source
   - Import

**Key Metrics to Monitor:**

| Metric | Threshold | Alert |
|--------|-----------|-------|
| `x3_block_height` | Increasing | Stalled if no change for 2 min |
| `x3_finalized_height` | Increasing | Stalled if lag >50 blocks |
| `x3_peers` | >5 | <3 peers for 5 min |
| `x3_validator_is_active` | 1 (true) | 0 (false) for 5 min |
| `x3_block_produced` | Per epoch | Miss >3 blocks per epoch |
| System CPU | <80% | >90% for 5 min |
| System RAM | <80% | >90% for 5 min |
| System Disk | <80% | >90% used |

### AlertManager Setup

```bash
# Install AlertManager
cd /tmp
wget https://github.com/prometheus/alertmanager/releases/download/v0.26.0/alertmanager-0.26.0.linux-amd64.tar.gz
tar -xzf alertmanager-0.26.0.linux-amd64.tar.gz
sudo mv alertmanager-0.26.0.linux-amd64 /opt/alertmanager

# Create config
sudo nano /opt/alertmanager/alertmanager.yml
```

**AlertManager Config:**

```yaml
global:
  resolve_timeout: 5m
  slack_api_url: 'https://hooks.slack.com/services/YOUR/SLACK/WEBHOOK'

route:
  group_by: ['alertname', 'severity']
  group_wait: 30s
  group_interval: 5m
  repeat_interval: 12h
  receiver: 'slack-critical'
  routes:
    - match:
        severity: critical
      receiver: 'slack-critical'
    - match:
        severity: warning
      receiver: 'slack-warnings'

receivers:
  - name: 'slack-critical'
    slack_configs:
      - channel: '#validator-alerts-critical'
        title: '🚨 CRITICAL: {{ .CommonAnnotations.summary }}'
        text: '{{ range .Alerts }}{{ .Annotations.description }}{{ end }}'
        
  - name: 'slack-warnings'
    slack_configs:
      - channel: '#validator-alerts'
        title: '⚠️ Warning: {{ .CommonAnnotations.summary }}'
        text: '{{ range .Alerts }}{{ .Annotations.description }}{{ end }}'
```

**Prometheus Alert Rules:**

```yaml
# /opt/prometheus/rules.yml
groups:
  - name: x3_validator_alerts
    interval: 30s
    rules:
      # Critical: Validator offline
      - alert: ValidatorOffline
        expr: x3_validator_is_active == 0
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Validator is offline"
          description: "Validator {{ $labels.instance }} has been offline for 5+ minutes"

      # Critical: No blocks produced
      - alert: NoBlocksProduced
        expr: rate(x3_block_height[5m]) == 0
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "No blocks produced"
          description: "Validator {{ $labels.instance }} hasn't produced any blocks in 2+ minutes"

      # Warning: Low peer count
      - alert: LowPeerCount
        expr: x3_peers < 3
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Low peer count"
          description: "Validator {{ $labels.instance }} has only {{ $value }} peers"

      # Warning: High CPU usage
      - alert: HighCPU
        expr: 100 - (avg by (instance) (rate(node_cpu_seconds_total{mode="idle"}[5m])) * 100) > 90
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High CPU usage"
          description: "CPU usage is {{ $value }}% on {{ $labels.instance }}"

      # Warning: High disk usage
      - alert: HighDiskUsage
        expr: (node_filesystem_size_bytes{mountpoint="/"} - node_filesystem_avail_bytes{mountpoint="/"}) / node_filesystem_size_bytes{mountpoint="/"} * 100 > 90
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High disk usage"
          description: "Disk usage is {{ $value }}% on {{ $labels.instance }}"
```

---

## 7. Operations Procedures

### Node Upgrades

**When to Upgrade:**
- Security patches: IMMEDIATELY
- Runtime upgrades: Before deadline (usually 1 week notice)
- Feature releases: Optional (test on testnet first)

**Upgrade Procedure:**

```bash
# 1. Download new binary
cd /tmp
wget https://github.com/x3network/x3-chain/releases/download/v1.1.0/x3-chain-node-linux-amd64.tar.gz
sha256sum -c x3-chain-node-linux-amd64.tar.gz.sha256

# 2. Extract
tar -xzf x3-chain-node-linux-amd64.tar.gz

# 3. Backup old binary
sudo cp /usr/local/bin/x3-chain-node /usr/local/bin/x3-chain-node.backup

# 4. Install new binary
sudo mv x3-chain-node /usr/local/bin/
sudo chmod +x /usr/local/bin/x3-chain-node

# 5. Restart node
sudo systemctl restart x3-validator

# 6. Verify upgrade
x3-chain-node --version
journalctl -u x3-validator -f

# 7. Monitor for 30 minutes
# Check block production, finality, peers

# 8. If issues, rollback:
sudo mv /usr/local/bin/x3-chain-node.backup /usr/local/bin/x3-chain-node
sudo systemctl restart x3-validator
```

### Database Maintenance

**Pruning Old State:**

```bash
# Stop node
sudo systemctl stop x3-validator

# Prune database (removes old state, keeps recent)
x3-chain-node purge-chain \
  --base-path /var/lib/x3-data \
  --chain mainnet \
  --pruning 256

# Restart
sudo systemctl start x3-validator
```

**Full Re-Sync:**

```bash
# Only if database corrupted

# 1. Backup keys
sudo cp -r /var/lib/x3-data/chains/x3_mainnet/keystore ~/keystore-backup

# 2. Stop node
sudo systemctl stop x3-validator

# 3. Delete database
sudo rm -rf /var/lib/x3-data/chains/x3_mainnet/db

# 4. Restore keys
sudo cp -r ~/keystore-backup/* /var/lib/x3-data/chains/x3_mainnet/keystore/

# 5. Restart (will sync from genesis)
sudo systemctl start x3-validator

# 6. Monitor sync progress
journalctl -u x3-validator -f | grep "Syncing"
# Will take 6-24 hours depending on network speed
```

### Emergency Maintenance

**Planned Downtime:**

1. Announce in community channel (24+ hours notice)
2. Set validator to "Chill" state (stops validating at next era)
```bash
# Via Polkadot.js UI:
# Staking → Account Actions → Stop (chill)
```
3. Wait for era boundary (up to 24 hours)
4. Perform maintenance
5. Set validator to "Validate" state
6. Wait for next era to rejoin active set

**Unplanned Outage:**

- Accept slashing risk (minor if <1 hour downtime)
- Restore service ASAP
- Post incident report
- Compensate nominators if slashed (optional, goodwill gesture)

---

## 8. Slashing Prevention

### What Causes Slashing?

| Offense | Penalty | Description |
|---------|---------|-------------|
| **Unresponsiveness** | 0.01% | Offline for 1+ sessions (~4 hours) |
| **Equivocation** | 0.1-100% | Double-signing blocks or GRANDPA votes |
| **Invalid Block** | 100% | Producing invalid blocks |

### Preventing Unresponsiveness

✅ **Do:**
- Maintain 99.9%+ uptime (monitor with alerts)
- Have backup power (UPS + generator)
- Monitor ImOnline heartbeats
- Set up failover validator (advanced)

❌ **Don't:**
- Run validator on unreliable hardware
- Perform maintenance during active validation
- Ignore monitoring alerts

### Preventing Equivocation

✅ **Do:**
- **NEVER run same validator keys on 2 nodes simultaneously**
- Use session key rotation
- Wipe database before restoring keys to new server
- Use validator failover tools (e.g., Polkadot Validator Manager)

❌ **Don't:**
- Clone validator VM and run both
- Restore backup while original is still running
- Run validator in high-availability cluster without proper safeguards

### Failover Setup (Advanced)

**Active-Passive Failover:**

```
Primary Validator (Active)
  ↓
Monitoring (checks every 30s)
  ↓
If primary offline >2 minutes:
  1. Wipe primary database (prevent double-signing)
  2. Restore keys to secondary
  3. Start secondary validator
```

**Implementation:**

```bash
# Primary validator health check script
# /opt/x3-monitoring/health-check.sh

#!/bin/bash
PRIMARY_URL="http://primary-validator:9944"
FAILOVER_TRIGGER="/tmp/x3-failover-trigger"

# Check if primary is responsive
if ! curl -s --max-time 5 $PRIMARY_URL > /dev/null; then
    # Primary is down
    if [ ! -f $FAILOVER_TRIGGER ]; then
        # First failure detected
        touch $FAILOVER_TRIGGER
        echo "Primary failure detected at $(date)" >> /var/log/x3-failover.log
    else
        # Check if down for >2 minutes
        TRIGGER_AGE=$(( $(date +%s) - $(stat -c %Y $FAILOVER_TRIGGER) ))
        if [ $TRIGGER_AGE -gt 120 ]; then
            # Trigger failover
            echo "Triggering failover at $(date)" >> /var/log/x3-failover.log
            /opt/x3-monitoring/failover.sh
        fi
    fi
else
    # Primary is healthy
    rm -f $FAILOVER_TRIGGER
fi
```

**⚠️ WARNING:** Failover is complex and risky. Test thoroughly on testnet before using on mainnet.

---

## 9. Troubleshooting Guide

### Node Won't Start

**Error: "Address already in use"**
```bash
# Another process is using the port
sudo lsof -i :30333
# Kill the process or change port
```

**Error: "Database lock"**
```bash
# Node crashed without releasing lock
sudo rm /var/lib/x3-data/chains/x3_mainnet/db/LOCK
```

**Error: "Permission denied"**
```bash
# Fix ownership
sudo chown -R x3:x3 /var/lib/x3-data
```

### Node Not Syncing

**Check network connectivity:**
```bash
# Test bootnode connectivity
nc -zv bootnode-1.x3.network 30333

# Check firewall
sudo ufw status
```

**Check peer count:**
```bash
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "system_health"}' \
  http://localhost:9944

# Should have >5 peers
```

**Force sync from specific peer:**
```bash
# Add reserved peer
curl -X POST http://localhost:9944 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "system_addReservedPeer",
    "params": ["/ip4/PEER_IP/tcp/30333/p2p/PEER_ID"],
    "id": 1
  }'
```

### Validator Not Producing Blocks

**Check validator status:**
```bash
# Via RPC
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "system_health"}' \
  http://localhost:9944

# Via Polkadot.js UI
# Network → Staking → Accounts → Check if in "Active" list
```

**Check session keys:**
```bash
# Verify session keys are set correctly
# Polkadot.js UI → Network → Staking → Accounts
# Your validator should show session keys set
```

**Common issues:**
- Not in active set (wait for next era)
- Session keys not set correctly (re-run `author_rotateKeys` and update)
- Node not synced (check sync status)
- Insufficient stake (need minimum 500k X3 + nominated stake)

### High Resource Usage

**CPU usage >90%:**
```bash
# Check what's consuming CPU
top -u x3

# If x3-chain-node:
# - Normal during sync (60-90%)
# - Normal during block production (30-60%)
# - Abnormal at idle (>70%) - investigate logs for loops/errors
```

**Memory usage >90%:**
```bash
# Check memory
free -h

# Restart node (releases memory)
sudo systemctl restart x3-validator

# Reduce cache size if persistent
# Edit service file → add: --state-cache-size 2048
```

**Disk full:**
```bash
# Check disk usage
df -h

# Clean logs
sudo journalctl --vacuum-time=7d

# Prune old state (see Section 7.2)
```

---

## 10. FAQ

**Q: How much can I earn as a validator?**

A: Depends on:
- Total staked by you + nominators
- Commission rate
- Network inflation rate
- Era rewards

Estimated APY: 10-20% on staked X3 (subject to change)

Example:
- Your stake: 500k X3
- Nominated stake: 1.5M X3
- Total stake: 2M X3
- Commission: 10%
- Era rewards: 1000 X3

Your earnings per era:
- Validator commission: 1000 * 0.10 = 100 X3
- Your staking rewards: (1000 * 0.90) * (500k/2M) = 225 X3
- Total: 325 X3 per era
- Per year (365 eras): ~119k X3 (~24% APY)

**Q: What happens if I get slashed?**

A: You lose a percentage of your staked X3 (and nominators' stake). Severity depends on offense:
- Minor (unresponsiveness): 0.01% (~100 X3 on 1M stake)
- Major (equivocation): 0.1-100% (up to total stake)

**Q: Can I run multiple validators?**

A: Yes, but each needs:
- Unique stash/controller accounts
- Separate node with unique session keys
- Separate stake (minimum per validator)

**Q: Can I stake less than 500k X3?**

A: No, 500k is the minimum for testnet validators. Mainnet minimum TBD (likely higher).

**Q: Do I need to be online 24/7?**

A: Yes. Validators must maintain 99.9%+ uptime to avoid slashing and maximize rewards. Even 1 hour offline per week can impact rewards.

**Q: Can I run a validator from home?**

A: Technically yes, but NOT recommended for mainnet:
- Residential ISP uptime typically <99%
- Power outages without UPS/generator
- DDoS vulnerability (exposed IP)
- Bandwidth caps
Better to use datacenter with SLA

**Q: How do I increase my nominated stake?**

A: You don't control this directly. Nominators choose to stake with you based on:
- High uptime (>99.9%)
- Reasonable commission (5-10%)
- Good reputation
- Communication (social media, blog updates)

**Q: Can I change commission after starting?**

A: Yes, but changes take effect at next era boundary. Announce changes to nominators.

**Q: What if I want to stop being a validator?**

A: 
1. Set validator to "Chill" state
2. Wait for next era
3. Unbond stake (requires 28-day unbonding period)
4. After 28 days, withdraw funds

**Q: Where can I get help?**

A: 
- Discord: https://discord.gg/x3-network (channel: #validators)
- Telegram: https://t.me/x3network_validators
- Forum: https://forum.x3.network/c/validators
- Email: validators@x3.network

---

## Conclusion

Congratulations! You're now equipped to run a production-grade X3 ATOMIC STAR validator.

**Next Steps:**

1. ✅ Test setup on testnet first (https://testnet.x3.network)
2. ✅ Run for 1+ week without issues
3. ✅ Join validator community (Discord/Telegram)
4. ✅ Review security hardening guide
5. ✅ Set up monitoring and alerts
6. ✅ Register for mainnet when ready

**Important Reminders:**

- ⚠️ NEVER share your seed phrase/private keys
- ⚠️ NEVER run same validator keys on 2 nodes
- ⚠️ Always test on testnet before mainnet
- ⚠️ Keep software updated (security patches)
- ⚠️ Monitor your node 24/7 (set up alerts)

**Welcome to the X3 validator community! 🎉**

---

**END OF RUNBOOK**

*This document is classified as: PUBLIC*  
*Last updated: April 26, 2026*  
*Version: 1.0*  
*Owner: X3 Core Team*
