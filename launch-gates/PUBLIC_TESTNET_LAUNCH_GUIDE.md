# X3 Public Incentivized Testnet - Complete Launch Guide

## Overview

Public testnet is **MANDATORY** before mainnet. This is not optional.

**Why?**
- Internal testing != economic reality
- Need real validators with skin in the game
- Need chaos testing with real participants
- Need to prove validator economics work
- Need to find bugs that only appear at scale

**Timeline:** 8-12 weeks minimum
**Budget:** $50k-$200k in incentives
**Goal:** 50-200 external validators

---

## Pre-Launch Preparation (Weeks 1-4)

### 1. Infrastructure Setup

#### Genesis Configuration
```bash
# Create testnet chain spec
./target/release/x3-node build-spec \
    --chain testnet \
    --raw \
    > testnet-spec-raw.json

# Genesis parameters
{
    "name": "X3 Incentivized Testnet",
    "id": "x3_testnet",
    "chainType": "Live",
    "bootNodes": [
        "/dns/boot1.testnet.x3.network/tcp/30333/p2p/12D3KooW...",
        "/dns/boot2.testnet.x3.network/tcp/30333/p2p/12D3KooW...",
        "/dns/boot3.testnet.x3.network/tcp/30333/p2p/12D3KooW..."
    ],
    "telemetryEndpoints": [
        ["wss://telemetry.x3.network/submit", 0]
    ]
}
```

#### Bootnode Setup
```bash
# Run 3+ bootnodes in different geographic regions
# Bootnode 1 (US East)
./target/release/x3-node \
    --chain testnet-spec-raw.json \
    --name "X3 Bootnode US-East" \
    --base-path /data/bootnode1 \
    --validator \
    --rpc-cors all \
    --telemetry-url "wss://telemetry.x3.network/submit 0"

# Bootnode 2 (EU)
# Bootnode 3 (Asia Pacific)
```

#### RPC Endpoints
```bash
# Public RPC nodes (3+ required)
./target/release/x3-node \
    --chain testnet-spec-raw.json \
    --name "X3 RPC Node" \
    --base-path /data/rpc \
    --rpc-external \
    --rpc-cors all \
    --rpc-methods Safe \
    --pruning archive  # For indexer
```

### 2. Monitoring & Observability

#### Telemetry
```bash
# Deploy telemetry server
docker run -d \
    -p 8000:8000 \
    -v /data/telemetry:/data \
    parity/substrate-telemetry-backend

# Telemetry frontend
docker run -d \
    -p 80:80 \
    -e TELEMETRY_URL=ws://telemetry.x3.network:8000/feed \
    parity/substrate-telemetry-frontend
```

#### Block Explorer
```bash
# Deploy Subscan or Polkadot.js Apps
# Option 1: Subscan (commercial)
# Option 2: Polkadot.js Apps (free, self-hosted)

git clone https://github.com/polkadot-js/apps
cd apps
npm install
npm run build
# Deploy to x3-explorer.network
```

#### Metrics & Alerts
```yaml
# Prometheus + Grafana setup
version: '3'
services:
  prometheus:
    image: prom/prometheus
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
    ports:
      - 9090:9090
      
  grafana:
    image: grafana/grafana
    ports:
      - 3000:3000
    volumes:
      - grafana-storage:/var/lib/grafana
```

### 3. Faucet Setup

```javascript
// Testnet faucet API
// Rate limit: 100 X3 tokens per address per day
// Requires CAPTCHA to prevent abuse

const express = require('express');
const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');

app.post('/faucet/drip', async (req, res) => {
    const { address, captcha } = req.body;
    
    // Verify CAPTCHA
    // Check rate limit
    // Send 100 X3 tokens
    
    const api = await ApiPromise.create({ provider: wsProvider });
    const keyring = new Keyring({ type: 'sr25519' });
    const faucetAccount = keyring.addFromUri('//FaucetSecret');
    
    const transfer = api.tx.balances.transfer(address, 100_000_000_000_000); // 100 X3
    const hash = await transfer.signAndSend(faucetAccount);
    
    res.json({ txHash: hash.toString() });
});
```

### 4. Documentation

Create comprehensive validator docs:

#### `/docs/testnet/validator-guide.md`
```markdown
# X3 Testnet Validator Guide

## Hardware Requirements
- CPU: 4+ cores
- RAM: 16GB minimum, 32GB recommended
- Storage: 500GB SSD (NVMe preferred)
- Network: 100 Mbps symmetric

## Installation

### Quick Start
\`\`\`bash
curl -sSf https://testnet.x3.network/install.sh | bash
\`\`\`

### Manual Installation
\`\`\`bash
# Build from source
git clone https://github.com/yourorg/x3-node
cd x3-node
cargo build --release

# Generate session keys
./target/release/x3-node key generate-session-keys

# Start validator
./target/release/x3-node \
    --chain testnet \
    --validator \
    --name "My Validator Name"
\`\`\`

## Session Key Registration
1. Get tokens from faucet: https://faucet.testnet.x3.network
2. Bond tokens: Minimum 1000 X3
3. Set session keys via extrinsic
4. Start validating

## Monitoring
- Telemetry: https://telemetry.testnet.x3.network
- Explorer: https://explorer.testnet.x3.network
- Your validator dashboard: Check "My Validators" tab
```

---

## Testnet Launch (Week 5)

### Genesis Ceremony

```bash
# Day -7: Announce genesis time (UTC)
# Day -3: Final spec published
# Day -1: Bootnodes start syncing
# Day 0 00:00 UTC: GENESIS BLOCK

# Genesis validators (team-controlled, 5-10 validators)
./target/release/x3-node \
    --chain testnet-spec-raw.json \
    --validator \
    --name "Genesis Validator 1" \
    --port 30333 \
    --rpc-port 9933

# Monitor finality
watch -n 1 'curl -s http://localhost:9933 -H "Content-Type: application/json" \
    -d '"'"'{"id":1,"jsonrpc":"2.0","method":"chain_getFinalizedHead"}'"'"' | jq'
```

### Day 1-7: Early Validators

**Goal:** Get to 20+ validators

**Marketing:**
- Twitter/X announcement
- Discord server
- Telegram group
- Reddit post (r/substrate, r/cryptocurrency)
- Email existing community

**Onboarding:**
- 1-on-1 support in Discord
- Live debugging sessions
- Video walkthrough
- FAQ updates

**Early Bird Bonus:**
- First 20 validators: +50% rewards
- Incentive: Join early, get more

---

## Phase 1: Network Stability (Weeks 5-6)

### Goals
- [ ] 50+ active validators
- [ ] Finality < 2 seconds
- [ ] No chain halts
- [ ] No equivocations

### Monitoring Metrics

```yaml
# Critical metrics
- block_production_rate: >0.95  # 95%+ blocks produced
- finality_lag: <2s  # Sub-2-second finality
- validator_uptime: >99%
- missed_blocks: <1%
- equivocations: 0
- chain_halts: 0
```

### Tests

#### Test 1: Block Production
```bash
# Verify all validators producing blocks
./scripts/testnet-block-production-audit.sh

# Expected: All validators in active set producing blocks
```

#### Test 2: Finality
```bash
# Measure finality lag
./scripts/testnet-measure-finality.sh

# Expected: <2 second average, <5 second p99
```

#### Test 3: Validator Rotation
```bash
# Verify session transitions work
# Expected: No finality stalls during session changes
```

---

## Phase 2: Stress Testing (Weeks 7-8)

### Load Testing

#### Test 1: High TPS
```bash
# Spam transactions to reach 10,000 TPS
./scripts/spam-testnet.sh \
    --accounts 1000 \
    --tps 10000 \
    --duration 3600  # 1 hour

# Monitor:
# - No missed blocks
# - No finality stalls
# - No validator crashes
```

#### Test 2: Bridge Load
```bash
# Spam bridge operations
./scripts/spam-bridge.sh \
    --messages 10000 \
    --concurrent 100

# Verify:
# - All messages settle
# - No replays
# - No orphaned collateral
```

#### Test 3: Atomic Swap Load
```bash
# Concurrent atomic swaps
./scripts/spam-atomic-swaps.sh \
    --swaps 5000 \
    --concurrent 50

# Verify:
# - All swaps settle or rollback
# - No partial settlements
# - Canonical supply conserved
```

---

## Phase 3: Chaos Engineering (Weeks 9-10)

### Failure Injection

#### Test 1: Validator Crashes
```bash
# Kill random validators
for i in {1..10}; do
    VALIDATOR=$(shuf -n 1 validator-list.txt)
    ssh $VALIDATOR "killall -9 x3-node"
    sleep 300  # Wait 5 minutes
done

# Verify:
# - Chain continues producing blocks
# - Finality maintained
# - No data loss
# - Validator can rejoin
```

#### Test 2: Network Partitions
```bash
# Partition 30% of validators
./scripts/simulate-network-partition.sh --percentage 30 --duration 600

# Verify:
# - Majority partition continues
# - Minority partition halts
# - Rejoin after partition heals
```

#### Test 3: Byzantine Validators
```bash
# Simulate malicious behavior
./scripts/byzantine-validator.sh \
    --type equivocate \
    --validator test-byzantine-1

# Verify:
# - Slashing occurs
# - Chain continues
# - Offender ejected
```

---

## Phase 4: Economic Testing (Weeks 11-12)

### Validator Rewards

```bash
# Verify rewards distribution
./scripts/audit-validator-rewards.sh

# Check:
# - All validators receive rewards
# - Rewards proportional to stake
# - No reward leakage
```

### Slashing

```bash
# Verify slashing works
./scripts/test-slashing.sh

# Scenarios:
# - Equivocation → slash
# - Offline → slash (if enabled)
# - Invalid block → slash
```

### Governance

```bash
# Test governance proposals
./scripts/test-governance.sh

# Scenarios:
# - Create proposal
# - Vote
# - Execute after delay
# - Verify no bypass possible
```

---

## Incentive Distribution

### Reward Tiers

#### Top Validators (Based on Uptime)
```
Top 1:  $2,000
Top 2:  $1,500
Top 3:  $1,000
Top 10: $500 each
Top 50: $200 each
All 50+: $50 each
```

#### Bug Bounties
```
Critical:  $5,000
High:      $1,000
Medium:    $250
Low:       $50
```

#### Community Contributions
```
Best monitoring dashboard:  $2,000
Best validator tutorial:    $500
Best tooling:              $1,000
Best analytics:            $500
```

### Payout Process

```markdown
# Week 12: Testnet concludes
# Week 13: Calculate final scores
# Week 14: Announce winners
# Week 15: Distribute payouts

Payout methods:
- Stablecoins (USDC/USDT)
- Bitcoin
- Ethereum
- Future X3 mainnet tokens (vested)
```

---

## Success Criteria

Testnet is **successful** if:

- ✅ Ran for 8+ weeks
- ✅ 50+ external validators participated
- ✅ No critical bugs found
- ✅ No chain halts
- ✅ No canonical supply issues
- ✅ No bridge replays
- ✅ No atomic swap partial settlements
- ✅ Finality maintained <2s
- ✅ Survived chaos testing
- ✅ Validator rewards worked
- ✅ Slashing worked
- ✅ Governance worked

If ANY of the above fail → **extend testnet** and retest.

---

## Post-Testnet Actions

After successful testnet:

1. **Conduct Retrospective**
   - What worked?
   - What broke?
   - What surprised us?
   - What needs fixing?

2. **Fix All Bugs**
   - Prioritize by severity
   - Retest all fixes
   - Document all changes

3. **Prepare Mainnet Genesis**
   - New chain spec
   - New genesis validators
   - Token distribution plan
   - Initial parameters

4. **Final Security Review**
   - Re-audit if major changes
   - Update threat model
   - Final pen-test

5. **Mainnet Launch Planning**
   - Pick launch date
   - Coordinate with exchanges
   - Marketing materials
   - Press release

---

## Budget Breakdown

### Infrastructure ($10k-$20k)
- Bootnodes: $2k/month × 3 months = $6k
- RPC nodes: $3k/month × 3 months = $9k
- Telemetry/Explorer: $2k/month × 3 months = $6k

### Incentives ($50k-$200k)
- Validator rewards: $30k-$100k
- Bug bounties: $10k-$50k
- Community rewards: $10k-$50k

### Total: $60k-$220k

**Recommended Budget:** $100k-$150k for comprehensive testnet

---

## Timeline Summary

```
Week 1-4:   Pre-launch (infrastructure, docs, marketing)
Week 5:     Genesis + early validators (0-20 validators)
Week 6:     Network growth (20-50+ validators)
Week 7-8:   Stress testing (high TPS, bridge, atomic)
Week 9-10:  Chaos engineering (crashes, partitions, byzantine)
Week 11-12: Economic testing (rewards, slashing, governance)
Week 13:    Analysis + retrospective
Week 14:    Bug fixes
Week 15:    Final report + incentive distribution
```

**Total Duration:** 15 weeks (3.5 months)

After testnet success → **6-8 weeks to mainnet** (audit fixes, genesis prep, marketing)

---

## Contact & Support

**Discord:** https://discord.gg/x3network
**Telegram:** https://t.me/x3validators
**Email:** validators@x3.network
**Docs:** https://docs.x3.network/testnet

**Validator Support Hours:** 24/7 (team on-call during testnet)
