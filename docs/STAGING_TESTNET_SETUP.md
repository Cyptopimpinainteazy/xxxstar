# X3 Staging Testnet Setup Guide

**Version:** 1.0  
**Status:** v0.4 RC-1 Internal Staging  
**Prerequisites:** Signed v0.4.0-rc.1 release tag published

---

## Overview

This guide sets up a private 5–7 validator staging testnet for RC-1 validation.
It follows the deployment policy: validators and bootnodes use **systemd + signed binaries**,
support services (explorer, indexer, monitoring) use **Docker/Kubernetes**.

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Staging Environment                    │
│                                                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐ │
│  │ Validator │  │ Validator │  │ Validator │  │ Bootnode │ │
│  │    #1     │  │    #2     │  │    #3     │  │          │ │
│  │  systemd  │  │  systemd  │  │  systemd  │  │  systemd │ │
│  └─────┬────┘  └─────┬────┘  └─────┬────┘  └─────┬────┘ │
│        │              │              │              │       │
│        └──────────────┴──────────────┴──────────────┘       │
│                          P2P                                │
│                                                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐ │
│  │ Explorer  │  │ Indexer  │  │ Faucet   │  │ Monitor  │ │
│  │  Docker   │  │  Docker  │  │  Docker  │  │  Docker  │ │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘ │
│                                                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐               │
│  │ Grafana   │  │Prometheus│  │ Loki     │               │
│  │  Docker   │  │  Docker  │  │  Docker  │               │
│  └──────────┘  └──────────┘  └──────────┘               │
└─────────────────────────────────────────────────────────┘
```

---

## Phase 1: Infrastructure Provisioning

### 1.1 Provision 7 VMs

| Node | Role | Specs | Count |
|---|---|---|---|
| Validator | Block production + finality | 4 vCPU, 16GB RAM, 100GB NVMe | 5 |
| Bootnode | P2P discovery | 2 vCPU, 4GB RAM, 50GB SSD | 1 |
| Support | Explorer, indexer, monitoring, faucet | 8 vCPU, 32GB RAM, 200GB SSD | 1 |

### 1.2 Network Requirements

| Direction | Port | Protocol | Purpose |
|---|---|---|---|
| Inbound | 30333 | TCP | P2P (validator + bootnode) |
| Inbound | 9933 | TCP | RPC (support node only) |
| Inbound | 9944 | TCP | WebSocket (support node only) |
| Outbound | 443 | TCP | GitHub releases (binary download) |

---

## Phase 2: Validator Setup

### 2.1 Download and Verify Release Binary

```bash
# On each validator node:
RELEASE_TAG="v0.4.0-rc.1"
REPO="Cyptopimpinainteazy/xxxstar"

# Download binary and checksums
wget "https://github.com/${REPO}/releases/download/${RELEASE_TAG}/x3-chain-node"
wget "https://github.com/${REPO}/releases/download/${RELEASE_TAG}/x3-chain-node.sha256"

# Verify checksum
sha256sum -c x3-chain-node.sha256

# Make executable
chmod +x x3-chain-node
sudo mv x3-chain-node /usr/local/bin/
```

### 2.2 Generate Session Keys

```bash
# Generate session keys (output on each validator)
./x3-chain-node key generate --output-type json --scheme Sr25519 > validator-key.json
./x3-chain-node key generate --output-type json --scheme Ed25519 > grandpa-key.json

# Extract addresses
VALIDATOR_ADDR=$(cat validator-key.json | python3 -c "import json,sys; d=json.load(sys.stdin); print(d['ss58Address'])")
GRANDPA_ADDR=$(cat grandpa-key.json | python3 -c "import json,sys; d=json.load(sys.stdin); print(d['ss58Address'])")

echo "Validator address: ${VALIDATOR_ADDR}"
echo "GRANDPA address:  ${GRANDPA_ADDR}"

# SAFELY STORE these keys offline. They will be needed for:
# 1. Inserting into the node keystore
# 2. Registering in the chain specification
```

### 2.3 Create Systemd Service

```bash
sudo cp packaging/systemd/x3-validator.service /etc/systemd/system/

# Edit the service file with your specific configuration:
sudo sed -i "s/--name X3Validator/--name staging-validator-${N}/" /etc/systemd/system/x3-validator.service
sudo sed -i "s|ExecStart=/usr/local/bin/x3-chain-node|ExecStart=/usr/local/bin/x3-chain-node --chain staging --bootnodes /ip4/BOOTNODE_IP/tcp/30333/p2p/BOOTNODE_PEER_ID|" /etc/systemd/system/x3-validator.service

# Insert session keys into keystore
# (Run once after first start to generate the data directory)
sudo systemctl start x3-validator
sleep 5
sudo systemctl stop x3-validator

# Insert keys
sudo /usr/local/bin/x3-chain-node key insert \
  --key-type babe \
  --chain staging \
  --scheme Sr25519 \
  --suri "<mnemonic phrase>" \
  --base-path /var/lib/x3-chain

sudo /usr/local/bin/x3-chain-node key insert \
  --key-type gran \
  --chain staging \
  --scheme Ed25519 \
  --suri "<mnemonic phrase>" \
  --base-path /var/lib/x3-chain

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable x3-validator
sudo systemctl restart x3-validator
sudo systemctl status x3-validator
```

### 2.4 Hardening

```bash
# Run the hardening script
sudo bash scripts/harden-validator.sh

# This configures:
# - Firewall (UFW): Ports 30333 (P2P), 22 (SSH only)
# - Kernel hardening: sysctl.conf
# - SSH: key-only, no root login
# - Log rotation: journald limits
# - Fail2ban: SSH brute-force protection
```

---

## Phase 3: Bootnode Setup

```bash
# On bootnode VM:
wget "https://github.com/${REPO}/releases/download/${RELEASE_TAG}/x3-chain-node"
sha256sum -c x3-chain-node.sha256
chmod +x x3-chain-node
sudo mv x3-chain-node /usr/local/bin/

# Copy bootnode systemd service
sudo cp packaging/systemd/x3-bootnode.service /etc/systemd/system/

# Edit bootnode config
sudo sed -i "s|ExecStart=/usr/local/bin/x3-chain-node|ExecStart=/usr/local/bin/x3-chain-node --chain staging --node-key-file /etc/x3-chain/node-key --no-mdns|" /etc/systemd/system/x3-bootnode.service

# Generate and save node key
/usr/local/bin/x3-chain-node key generate-node-key --file /etc/x3-chain/node-key
BOOTNODE_PEER_ID=$(/usr/local/bin/x3-chain-node key inspect-node-key --file /etc/x3-chain/node-key)
echo "Bootnode Peer ID: ${BOOTNODE_PEER_ID}"

sudo systemctl daemon-reload
sudo systemctl enable x3-bootnode
sudo systemctl start x3-bootnode
```

---

## Phase 4: Support Services (Docker)

On the support VM:

```bash
# Clone repo (for docker-compose.yml and configs)
git clone https://github.com/Cyptopimpinainteazy/xxxstar.git
cd xxxstar

# Start the support stack
docker compose -f docker/docker-compose.yml up -d

# Verify all services are healthy
docker compose -f docker/docker-compose.yml ps

# Check individual services:
curl http://localhost:3000  # Explorer
curl http://localhost:9090  # Prometheus
curl http://localhost:3001  # Grafana (admin/admin)
curl http://localhost:3100  # Loki
```

### 4.1 Configure Prometheus Targets

Edit `monitoring/prometheus/prometheus.yml` to add validator targets:

```yaml
scrape_configs:
  - job_name: 'x3-validators'
    static_configs:
      - targets:
        - 'validator1:9615'
        - 'validator2:9615'
        - 'validator3:9615'
        - 'validator4:9615'
        - 'validator5:9615'
```

### 4.2 Import Grafana Dashboards

```bash
# The monitoring/grafana/dashboards/ directory contains:
# - x3-validator-overview.json
# - x3-network-health.json
# - x3-runtime-metrics.json

# Import via API:
for dashboard in monitoring/grafana/dashboards/*.json; do
  curl -X POST -H "Authorization: Bearer ${GRAFANA_API_KEY}" \
    -H "Content-Type: application/json" \
    -d "@${dashboard}" \
    "http://localhost:3001/api/dashboards/db"
done
```

---

## Phase 5: Chain Specification

### 5.1 Build Staging Chain Spec

```bash
# Build the chain spec for staging
./target/release/x3-chain-node build-spec \
  --chain staging \
  --disable-default-bootnode \
  > chain-specs/staging-plain.json

# Add validators to the spec:
# Edit chain-specs/staging-plain.json to include initial validator set:
# "pallet_session": { "validators": ["<validator1>", "<validator2>", ...] }

# Convert to raw spec
./target/release/x3-chain-node build-spec \
  --chain chain-specs/staging-plain.json \
  --raw \
  > chain-specs/staging-raw.json
```

### 5.2 Genesis Configuration

```yaml
# validator_config/staging.toml
[genesis]
external_bridges_enabled = false
external_bridge_audit_gate = false

[genesis.validators]
# List of initial validator stash accounts
initial_authorities = [
  ["<validator1_stash>", "<validator1_controller>", "<grandpa_key>", "<babe_key>"],
  ["<validator2_stash>", "<validator2_controller>", "<grandpa_key>", "<babe_key>"],
]
```

---

## Phase 6: Verification Drills

### 6.1 Block Production Check

```bash
# On any node:
./target/release/x3-chain-node --chain staging --rpc-port 9933 &
curl -s http://localhost:9933 -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"chain_getHeader","params":[],"id":1}' | \
  python3 -c "import json,sys; d=json.load(sys.stdin); print(f'Best block: {d[\"result\"][\"number\"]}')"
```

### 6.2 Finality Check

```bash
# Check GRANDPA finality:
curl -s http://localhost:9933 -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"chain_getFinalizedHead","params":[],"id":1}' | \
  python3 -c "import json,sys; d=json.load(sys.stdin); print(f'Finalized: {d[\"result\"]}')"
```

### 6.3 Restore from Snapshot

```bash
# Backup
sudo systemctl stop x3-validator
sudo tar czf /backup/x3-chain-$(date +%Y%m%d-%H%M%S).tar.gz /var/lib/x3-chain/
sudo systemctl start x3-validator

# Restore
sudo systemctl stop x3-validator
sudo rm -rf /var/lib/x3-chain/chains/staging/db/
sudo tar xzf /backup/x3-chain-20260610-120000.tar.gz -C /
sudo systemctl start x3-validator

# Monitor recovery
journalctl -u x3-validator -f --since "5 minutes ago"
```

### 6.4 Incident Response Drill

```bash
# Test 1: Validator crash recovery
#   - Kill validator process
#   - Verify systemd auto-restarts
#   - Check block production resumes

# Test 2: Network partition
#   - Block P2P port on one validator: sudo ufw deny 30333
#   - Wait 30 seconds
#   - Reopen: sudo ufw allow 30333
#   - Verify finality recovers

# Test 3: Disk space warning
#   - Fill disk to 85%: dd if=/dev/zero of=/tmp/fill bs=1M count=1000
#   - Verify Prometheus alerts fire
#   - Clean up: rm /tmp/fill
```

---

## Phase 7: Sign-Off Checklist

| # | Item | Verified By |
|---|---|---|
| 7.1 | All 5 validators producing blocks | DevOps |
| 7.2 | GRANDPA finality at 100% | DevOps |
| 7.3 | Transaction submission works via RPC | QA |
| 7.4 | Cross-VM transfer works (native ↔ evm ↔ svm) | QA |
| 7.5 | Supply invariant holds (TotalIssuance == sum(accounts)) | QA |
| 7.6 | Indexer ingesting blocks | DevOps |
| 7.7 | Explorer showing live blocks | DevOps |
| 7.8 | Prometheus scraping all targets | DevOps |
| 7.9 | Alertmanager routing working | DevOps |
| 7.10 | Restore from snapshot passes | DevOps |
| 7.11 | Incident drill passes (kill + restart) | DevOps |
| 7.12 | Secret rotation drill passes | Security |
| 7.13 | Bridge relayer health passes (if bridge-enabled) | DevOps |
| 7.14 | EVM gateway contract verified on explorer | DevOps |
| 7.15 | Snapshot backup + restore drill passes | DevOps |
| 7.16 | Incident drill passes (kill + restart) | DevOps |
| 7.17 | Secret rotation drill passes | Security |
---

## Troubleshooting

### Validator not producing blocks
```bash
# Check node key
sudo journalctl -u x3-validator -n 50 --no-pager | grep -i "key\|peer"

# Verify session keys inserted
curl -s http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"author_hasSessionKeys","params":["0x..."],"id":1}'

# Check P2P connectivity
curl -s http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"system_peers","params":[],"id":1}' | \
  python3 -c "import json,sys; d=json.load(sys.stdin); print(f'Peers: {len(d[\"result\"])}')"
```

### Indexer not syncing
```bash
# Check PostgreSQL connection
docker compose -f docker/docker-compose.yml exec postgres pg_isready

# Check indexer logs
docker compose -f docker/docker-compose.yml logs --tail=50 indexer

# Rebuild indexer database
docker compose -f docker/docker-compose.yml stop indexer postgres
docker compose -f docker/docker-compose.yml rm -f postgres indexer
docker compose -f docker/docker-compose.yml up -d postgres
sleep 10
docker compose -f docker/docker-compose.yml up -d indexer