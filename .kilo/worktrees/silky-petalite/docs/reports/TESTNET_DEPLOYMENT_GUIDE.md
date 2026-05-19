# X3 Chain Testnet v1 - Deployment Guide

**Status**: Ready for Deployment  
**Version**: 0.1.0-testnet  
**Network**: X3 Chain Testnet v1  
**Launch Date**: November 2025

---

## Overview

X3 Chain Testnet v1 is a **developer preview network** designed for:
- Testing Comit submission and canonical ledger functionality
- Developer experimentation with dual-VM architecture
- Infrastructure validation and performance testing
- Community feedback and iteration

⚠️ **Important Limitations**:
- Uses **mock VM execution** (not real EVM/SVM yet)
- HTTP RPC only (WebSocket coming soon)
- Testnet tokens have **no economic value**
- Network may be reset during development

---

## Network Specifications

### Chain Configuration
| Parameter | Value |
|-----------|-------|
| **Chain ID** | `x3-testnet` |
| **Currency Symbol** | `tATLAS` |
| **Decimals** | 12 |
| **Block Time** | 6 seconds |
| **Consensus** | Aura + GRANDPA |
| **RPC Endpoint** | `http://127.0.0.1:9944` |

### Genesis Configuration
- **Initial Validators**: 3-5 authority nodes
- **Initial Supply**: 1,000,000,000 tATLAS
- **Existential Deposit**: 0.0001 tATLAS

---

## Pre-Deployment Checklist

### 1. Build Release Binary
```bash
# Clone repository
git clone https://github.com/Cyptopimpinainteazy/x3-chain.git X3_ATOMIC_STAR
cd X3_ATOMIC_STAR
cargo build --release
```

**Verify binary**:
```bash
./target/release/x3-chain-node --version
```

### 2. Generate Chain Specification

**Create testnet chain spec**:
```bash
./target/release/x3-chain-node build-spec \
  --chain local \
  --disable-default-bootnode \
  > x3-testnet.json
```

**Convert to raw format**:
```bash
./target/release/x3-chain-node build-spec \
  --chain x3-testnet.json \
  --raw \
  --disable-default-bootnode \
  > x3-testnet-raw.json
```

### 3. Generate Authority Keys

For each validator node, generate keys:

```bash
# Generate Aura key (sr25519)
./target/release/x3-chain-node key generate \
  --scheme sr25519 \
  --output-type json \
  > validator-1-aura.json

# Generate Grandpa key (ed25519)
./target/release/x3-chain-node key generate \
  --scheme ed25519 \
  --output-type json \
  > validator-1-grandpa.json
```

Repeat for each validator (validator-2, validator-3, etc.)

### 4. Configure Genesis Validators

Edit `x3-testnet.json` and add validator public keys:

```json
{
  "genesis": {
    "runtime": {
      "aura": {
        "authorities": [
          "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
          "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty"
        ]
      },
      "grandpa": {
        "authorities": [
          ["5FA9nQDVg267DEd8m1ZypXLBnvN7SFxYwV7ndqSYGiN9TTpu", 1],
          ["5GoNkf6WdbxCFnPdAnYYQyCjAKPJgLNxXwPjwTh6DGg6gN3E", 1]
        ]
      }
    }
  }
}
```

---

## Deployment Architecture

### Minimum Requirements
- **3 Validator Nodes** (authority set)
- **2 RPC Nodes** (public endpoints)
- **1 Bootnode** (peer discovery)

### Recommended Infrastructure

#### Validator Node Specs
- **CPU**: 4 cores
- **RAM**: 8 GB
- **Disk**: 100 GB SSD
- **Network**: 100 Mbps
- **OS**: Ubuntu 22.04 LTS

#### RPC Node Specs
- **CPU**: 2 cores
- **RAM**: 4 GB
- **Disk**: 100 GB SSD
- **Network**: 100 Mbps
- **OS**: Ubuntu 22.04 LTS

---

## Node Deployment

### Validator Node Setup

**1. Install binary**:
```bash
# Copy binary to validator server
scp target/release/x3-chain-node user@validator:/usr/local/bin/
```

**2. Create systemd service**:
```bash
sudo nano /etc/systemd/system/x3-validator.service
```

```ini
[Unit]
Description=X3 Chain Validator Node
After=network.target

[Service]
Type=simple
User=x3
WorkingDirectory=/home/x3
ExecStart=/usr/local/bin/x3-chain-node \
  --validator \
  --chain /home/x3/x3-testnet-raw.json \
  --base-path /home/x3/data \
  --name "Validator-01" \
  --rpc-port 9944 \
  --rpc-cors all \
  --log info,runtime=debug
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

**3. Insert keys**:
```bash
# Insert Aura key
x3-chain-node key insert \
  --base-path /home/x3/data \
  --chain x3-testnet-raw.json \
  --scheme sr25519 \
  --suri "YOUR_SECRET_PHRASE" \
  --key-type aura

# Insert Grandpa key
x3-chain-node key insert \
  --base-path /home/x3/data \
  --chain x3-testnet-raw.json \
  --scheme ed25519 \
  --suri "YOUR_SECRET_PHRASE" \
  --key-type gran
```

**4. Start validator**:
```bash
sudo systemctl enable x3-validator
sudo systemctl start x3-validator
sudo systemctl status x3-validator
```

**5. Check logs**:
```bash
sudo journalctl -u x3-validator -f
```

### RPC Node Setup

**1. Create systemd service**:
```bash
sudo nano /etc/systemd/system/x3-rpc.service
```

```ini
[Unit]
Description=X3 Chain RPC Node
After=network.target

[Service]
Type=simple
User=x3
WorkingDirectory=/home/x3
ExecStart=/usr/local/bin/x3-chain-node \
  --chain /home/x3/x3-testnet-raw.json \
  --base-path /home/x3/data \
  --name "RPC-Node-01" \
  --rpc-port 9944 \
  --rpc-external \
  --rpc-cors all \
  --rpc-methods Safe \
  --log info
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

**2. Start RPC node**:
```bash
sudo systemctl enable x3-rpc
sudo systemctl start x3-rpc
sudo systemctl status x3-rpc
```

### Bootnode Setup

**1. Generate node key**:
```bash
x3-chain-node key generate-node-key \
  --file /home/x3/node-key
```

**2. Get node peer ID**:
```bash
x3-chain-node key inspect-node-key \
  --file /home/x3/node-key
```

**3. Create systemd service**:
```bash
sudo nano /etc/systemd/system/x3-bootnode.service
```

```ini
[Unit]
Description=X3 Chain Bootnode
After=network.target

[Service]
Type=simple
User=x3
WorkingDirectory=/home/x3
ExecStart=/usr/local/bin/x3-chain-node \
  --chain /home/x3/x3-testnet-raw.json \
  --base-path /home/x3/data \
  --name "Bootnode-01" \
  --node-key-file /home/x3/node-key \
  --listen-addr /ip4/0.0.0.0/tcp/30333 \
  --log info
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

**4. Start bootnode**:
```bash
sudo systemctl enable x3-bootnode
sudo systemctl start x3-bootnode
```

---

## Network Monitoring

### Prometheus Metrics

**Configure Prometheus** to scrape node metrics:

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'x3-validators'
    static_configs:
      - targets:
          - 'validator-1:9615'
          - 'validator-2:9615'
          - 'validator-3:9615'

  - job_name: 'x3-rpc'
    static_configs:
      - targets:
          - 'rpc-1:9615'
          - 'rpc-2:9615'
```

### Health Checks

**Check node sync status**:
```bash
curl http://localhost:9944 -H "Content-Type: application/json" \
  -d '{"id":1,"jsonrpc":"2.0","method":"system_health","params":[]}'
```

**Check peer count**:
```bash
curl http://localhost:9944 -H "Content-Type: application/json" \
  -d '{"id":1,"jsonrpc":"2.0","method":"system_peers","params":[]}'
```

**Check block height**:
```bash
curl http://localhost:9944 -H "Content-Type: application/json" \
  -d '{"id":1,"jsonrpc":"2.0","method":"chain_getHeader","params":[]}'
```

---

## Public Endpoints

### Testnet RPC Endpoints

Once deployed, publish these endpoints for developers:

```
HTTP RPC:  http://rpc.testnet.x3-chain.io:9944
WebSocket: ws://ws.testnet.x3-chain.io:9944 (coming soon)
```

### Faucet Service

Deploy a faucet for distributing testnet tokens:

```bash
# Faucet API endpoint
POST https://faucet.testnet.x3-chain.io/claim
{
  "address": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"
}
```

**Faucet limits**:
- 100 tATLAS per request
- 1 request per address per day
- Maximum 1000 tATLAS per address

---

## Developer Onboarding

### Quick Start for Developers

**1. Get testnet tokens**:
```bash
curl -X POST https://faucet.testnet.x3-chain.io/claim \
  -H "Content-Type: application/json" \
  -d '{"address": "YOUR_ADDRESS"}'
```

**2. Query canonical balance**:
```bash
curl http://rpc.testnet.x3-chain.io:9944 \
  -H "Content-Type: application/json" \
  -d '{
    "id":1,
    "jsonrpc":"2.0",
    "method":"atlasKernel_getCanonicalBalance",
    "params":["YOUR_ADDRESS", 0, null]
  }'
```

**3. Check authorized accounts**:
```bash
curl http://rpc.testnet.x3-chain.io:9944 \
  -H "Content-Type: application/json" \
  -d '{
    "id":1,
    "jsonrpc":"2.0",
    "method":"atlasKernel_getAuthorizedAccounts",
    "params":[null]
  }'
```

### Example: Submit Comit (Future)

```bash
# Using polkadot.js CLI (once available)
polkadot-js-api \
  --ws ws://ws.testnet.x3-chain.io:9944 \
  tx.atlasKernel.submitComit \
  0x0102030405060708090a0b0c0d0e0f00112233445566778899aabbccddeeff00 \
  0x \
  0x0102 \
  0 \
  1000000000000 \
  0x0000000000000000000000000000000000000000000000000000000000000000 \
  --seed "YOUR_SEED_PHRASE"
```

---

## Security Considerations

### Testnet Security Model

⚠️ **This is a TESTNET - Not Production**:
- Testnet tokens have NO economic value
- Network may be reset at any time
- Do NOT use mainnet keys on testnet
- Do NOT store sensitive data on testnet
- Expect bugs and breaking changes

### Key Management

**For testnet validators**:
- Generate unique keys for testnet only
- Store keys securely (even for testnet)
- Use hardware wallet for production keys (when mainnet launches)

### Network Access

**RPC node security**:
- Use `--rpc-methods Safe` to restrict dangerous methods
- Enable CORS only for trusted origins in production
- Use reverse proxy (nginx) with rate limiting
- Monitor for unusual activity

---

## Troubleshooting

### Node Won't Start

**Check logs**:
```bash
sudo journalctl -u x3-validator -n 100
```

**Common issues**:
- Chain spec mismatch (ensure all nodes use same raw spec)
- Port conflicts (check 9944, 30333 not in use)
- Insufficient permissions (run as x3 user, not root)
- Database corruption (purge chain with `--purge-chain`)

### Node Not Syncing

**Check peer connections**:
```bash
curl http://localhost:9944 -H "Content-Type: application/json" \
  -d '{"id":1,"jsonrpc":"2.0","method":"system_peers","params":[]}'
```

**Common issues**:
- Firewall blocking P2P port (30333)
- No bootnode configured
- Clock skew (check system time with NTP)

### RPC Not Responding

**Check RPC binding**:
```bash
netstat -tlnp | grep 9944
```

**Common issues**:
- RPC not externally exposed (add `--rpc-external`)
- CORS blocking requests (add `--rpc-cors all`)
- Rate limiting triggered
- Node still syncing (wait for sync)

---

## Maintenance

### Database Backup

**Backup chain data**:
```bash
tar -czf x3-backup-$(date +%Y%m%d).tar.gz \
  /home/x3/data/chains/x3-testnet/db/
```

### Database Purge

**Purge and resync** (if corrupted):
```bash
sudo systemctl stop x3-validator
x3-chain-node purge-chain \
  --chain x3-testnet-raw.json \
  --base-path /home/x3/data
sudo systemctl start x3-validator
```

### Updating Node Software

**1. Build new version**:
```bash
git pull origin main
cargo build --release
```

**2. Stop node**:
```bash
sudo systemctl stop x3-validator
```

**3. Update binary**:
```bash
sudo cp target/release/x3-chain-node /usr/local/bin/
```

**4. Restart node**:
```bash
sudo systemctl start x3-validator
```

---

## Support & Communication

### Developer Resources
- **Documentation**: https://docs.x3-chain.io
- **GitHub**: https://github.com/x3-chain/x3-chain
- **Discord**: https://discord.gg/x3-chain
- **Telegram**: https://t.me/x3_chain

### Report Issues
- **Bug Reports**: https://github.com/x3-chain/x3-chain/issues
- **Security Issues**: security@x3-chain.io (private)
- **General Questions**: Discord #support channel

---

## Testnet Roadmap

### Phase 1: Initial Launch (Now)
- ✅ Launch validator set
- ✅ Deploy RPC nodes
- ✅ Open faucet
- ✅ Publish endpoints

### Phase 2: Developer Tools (2-4 weeks)
- [ ] WebSocket RPC support
- [ ] TypeScript SDK release
- [ ] Python SDK release
- [ ] CLI tools enhancement

### Phase 3: VM Integration (4-8 weeks)
- [ ] Real EVM integration (Frontier)
- [ ] Real SVM integration (Solana)
- [ ] Cross-VM bridge testing
- [ ] Performance optimization

### Phase 4: Mainnet Preparation (8-12 weeks)
- [ ] Security audit
- [ ] Economic model finalization
- [ ] Governance activation
- [ ] Mainnet launch

---

## Success Metrics

### Network Health
- [ ] 3+ validators online
- [ ] <2 block finality lag
- [ ] 99% uptime
- [ ] <100ms RPC latency

### Developer Adoption
- [ ] 10+ developers using testnet
- [ ] 100+ Comits submitted
- [ ] 5+ community contributions
- [ ] 1000+ faucet requests

---

## Conclusion

X3 Chain Testnet v1 is ready for deployment with:
- ✅ Stable consensus and networking
- ✅ Working RPC server
- ✅ Functional Comit submission
- ✅ Canonical ledger tracking

**Limitations** (clearly communicated):
- Mock VM execution (not real EVM/SVM yet)
- HTTP-only RPC (WebSocket coming soon)
- No economic value (testnet only)

**Deploy immediately** to gather community feedback while continuing development in parallel.

---

**Last Updated**: November 2025  
**Maintainer**: X3 Chain Core Team  
**License**: Apache 2.0
