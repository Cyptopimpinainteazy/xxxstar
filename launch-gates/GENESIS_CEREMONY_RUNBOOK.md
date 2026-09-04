# Genesis Ceremony Runbook

**Document Version:** 1.0  
**Date:** April 26, 2026  
**Status:** ⚠️ HISTORICAL / INTERNAL PRE-LAUNCH PLAN — Not current public readiness status  
**Target Audience:** Core team, founding validators, operations lead

---

## ⚠️ CRITICAL PRE-REQUISITES

**DO NOT PROCEED** unless ALL of the following are true:

- ✅ All 9 security blockers (6 S0 + 3 S1) are RESOLVED
- ✅ ProofForge re-run shows 0 S0/S1 findings
- ✅ External security audit(s) complete with PASS rating
- ✅ Bug bounty program ran 4+ weeks with no Critical/High findings
- ✅ Public testnet ran 8+ weeks successfully (50+ validators, no chain halts)
- ✅ Legal compliance review complete
- ✅ All founding validators passed KYC/background checks
- ✅ Incident response team on standby

**Current Status:** ⚠️ MAINNET READINESS BLOCKED / UNDER REVIEW (historical planning document only)  
**Canonical Status:** [docs/CURRENT_MAINNET_STATUS.md](../docs/CURRENT_MAINNET_STATUS.md)

---

## Executive Summary

The genesis ceremony is a **one-time, irreversible event** that launches the X3 ATOMIC STAR mainnet. This runbook provides step-by-step procedures for:

1. **Validator Selection** (Week -4 to -2)
2. **Key Generation Ceremony** (Week -2)
3. **Genesis Spec Creation** (Week -1)
4. **Launch Coordination** (Launch Day)
5. **Post-Launch Verification** (Day 1-7)
6. **Emergency Rollback** (if needed)

**Timeline:** 4 weeks preparation + Launch Day + 1 week verification  
**Team Size:** 8-12 people (3 core devs, 5-10 founding validators, 1 ops lead, 1 comms lead)  
**Budget:** $50k-$80k (validator incentives, infrastructure, coordination)

---

## Phase 1: Validator Selection (Week -4 to -2)

### Objectives
- Select 5-10 founding validators
- Ensure geographic distribution
- Verify technical capability and security practices
- Establish communication channels

### Selection Criteria

**Technical Requirements:**
```yaml
hardware:
  cpu: "8+ cores (AMD EPYC or Intel Xeon recommended)"
  ram: "32 GB minimum, 64 GB recommended"
  disk: "1 TB NVMe SSD minimum"
  network: "1 Gbps symmetric, <50ms latency to peers"
  
datacenter:
  uptime_sla: "99.9%+"
  ddos_protection: "Required"
  backup_power: "Required (UPS + generator)"
  monitoring: "24/7 NOC"

security:
  key_management: "HSM or secure enclave required"
  access_control: "MFA + audit logging"
  incident_response: "Documented procedures"
  background_check: "Completed and verified"
```

**Geographic Distribution:**
- North America: 2-3 validators
- Europe: 2-3 validators
- Asia-Pacific: 1-2 validators
- Other regions: 1-2 validators

**Reputation Requirements:**
- Active in blockchain community (GitHub, forums, social)
- No history of slashing on other networks
- Reference checks from 2+ other networks

### Selection Process

**Week -4:**
1. Publish validator application form:
   ```markdown
   # X3 ATOMIC STAR Founding Validator Application
   
   **Organization:** _________________
   **Primary Contact:** _________________
   **Email:** _________________
   **Telegram/Discord:** _________________
   
   **Technical Infrastructure:**
   - [ ] Dedicated servers (not shared/VPS)
   - [ ] 1 Gbps+ network
   - [ ] 24/7 monitoring
   - [ ] DDoS protection
   - [ ] HSM or secure key storage
   
   **Experience:**
   - [ ] Validator experience on other networks (list):
   - [ ] Substrate/Polkadot ecosystem experience
   - [ ] Slashing history: Y/N (explain if yes)
   
   **Datacenter Location:**
   - City: _________________
   - Country: _________________
   - Provider: _________________
   
   **References:**
   1. Network + Contact: _________________
   2. Network + Contact: _________________
   ```

2. Distribute application via:
   - X3 Discord/Telegram announcement
   - Twitter/X post
   - Direct outreach to known validators
   - Email to testnet participants

3. **Deadline:** Applications due Week -3 Friday 5pm UTC

**Week -3:**
1. Review all applications
2. Technical screening calls (30min each)
3. Reference checks
4. Background verification (if required by legal)

**Week -2:**
1. Select final 5-10 founding validators
2. Send acceptance notifications
3. Send rejection notifications (with waitlist option)
4. Create private coordination channel (Telegram/Discord)

**Validator Acceptance Template:**
```
Subject: X3 ATOMIC STAR Founding Validator - ACCEPTED

Dear [Validator Name],

Congratulations! You have been selected as a founding validator for X3 ATOMIC STAR mainnet.

NEXT STEPS:
1. Join coordination channel: [Telegram/Discord invite]
2. Attend key generation ceremony: [Date] [Time] UTC
3. Review technical runbook: [Link]
4. Test infrastructure: [Testnet endpoint]

GENESIS TIMELINE:
- Week -2: Key generation ceremony (MANDATORY attendance)
- Week -1: Genesis spec review + signing
- Launch Day: [Date] [Time] UTC
- Week +1: Stability monitoring

COMPENSATION:
- Genesis allocation: [Amount] X3 tokens
- First year rewards: Estimated [Amount] X3 tokens
- Monthly infrastructure stipend: $[Amount] (first 6 months)

REQUIREMENTS:
- Attend all coordination meetings
- Maintain 99.9%+ uptime
- Respond to security incidents within 1 hour
- Sign NDA (if applicable)

POINT OF CONTACT:
- Technical Lead: [Name] [Email] [Telegram]
- Operations Lead: [Name] [Email] [Telegram]

Welcome to the founding validator set!

[Core Team Signature]
```

---

## Phase 2: Key Generation Ceremony (Week -2)

### Objectives
- Generate validator keys securely
- Generate session keys (BABE, GRANDPA, ImOnline, Authority Discovery)
- Establish chain spec signing keys
- Document all public keys for genesis spec

### Pre-Ceremony Checklist

**All Validators:**
- [ ] Fresh Ubuntu 22.04 LTS server deployed
- [ ] Firewall configured (ports 30333, 9944, 9933 only)
- [ ] X3 node binary built from audited source (or verified Docker image)
- [ ] HSM or secure enclave configured (if using)
- [ ] Backup procedures tested
- [ ] Attended pre-ceremony technical briefing

**Core Team:**
- [ ] Zoom/Google Meet room ready (with recording)
- [ ] Screen sharing enabled
- [ ] Technical support standing by
- [ ] Documentation templates prepared
- [ ] Emergency rollback plan ready

### Key Generation Process

**Duration:** 2-3 hours  
**Format:** Live video call (all validators + core team)  
**Recording:** YES (for audit trail)

#### Step 1: Install X3 Node (All Validators)

```bash
# Download audited release
wget https://releases.x3.network/mainnet/x3-chain-node-v1.0.0.tar.gz
wget https://releases.x3.network/mainnet/x3-chain-node-v1.0.0.tar.gz.sha256

# Verify checksum
sha256sum -c x3-chain-node-v1.0.0.tar.gz.sha256
# Expected: x3-chain-node-v1.0.0.tar.gz: OK

# Extract and install
tar -xzf x3-chain-node-v1.0.0.tar.gz
sudo mv x3-chain-node /usr/local/bin/
sudo chmod +x /usr/local/bin/x3-chain-node

# Verify installation
x3-chain-node --version
# Expected: x3-chain-node 1.0.0-dc9d1bd
```

#### Step 2: Generate Validator Stash Key (All Validators)

```bash
# Create secure directory
mkdir -p ~/x3-keys
chmod 700 ~/x3-keys
cd ~/x3-keys

# Generate stash account (SR25519)
x3-chain-node key generate --scheme sr25519 > stash-key.json

# Extract public key and address
cat stash-key.json | grep "SS58 Address"
# Example output: SS58 Address: 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY

# IMPORTANT: Print both outputs
echo "=== STASH KEY DETAILS ==="
cat stash-key.json
echo "========================"
```

**🔴 CRITICAL: Record in ceremony log:**
```
Validator: [Organization Name]
Stash Address: 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY
Timestamp: [ISO 8601]
Verified by: [Core Team Member Name]
```

#### Step 3: Generate Session Keys (All Validators)

```bash
# Start node in dev mode temporarily
x3-chain-node \
  --base-path /tmp/x3-genesis-test \
  --chain dev \
  --rpc-port 9944 \
  --unsafe-rpc-external \
  --rpc-methods Unsafe &

# Wait 10 seconds for node to start
sleep 10

# Generate session keys via RPC
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "author_rotateKeys"}' \
  http://localhost:9944

# Expected output:
# {"jsonrpc":"2.0","result":"0x[HEX_STRING_384_CHARS]","id":1}

# Store session keys
SESSION_KEYS=$(curl -s -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "author_rotateKeys"}' \
  http://localhost:9944 | jq -r .result)

echo "Session Keys: $SESSION_KEYS" >> session-keys.txt

# Stop dev node
pkill x3-chain-node

# Cleanup dev data
rm -rf /tmp/x3-genesis-test
```

**Session Keys Format:**
```
0x
[BABE public key - 32 bytes]
[GRANDPA public key - 32 bytes]  
[ImOnline public key - 32 bytes]
[Authority Discovery public key - 32 bytes]
[248 additional bytes for other session keys]
```

**🔴 CRITICAL: Record in ceremony log:**
```
Validator: [Organization Name]
Session Keys: 0x[384 hex chars]
Timestamp: [ISO 8601]
Verified by: [Core Team Member Name]
```

#### Step 4: Secure Key Storage (All Validators)

```bash
# Encrypt keys with strong passphrase
tar -czf x3-keys-backup.tar.gz ~/x3-keys/
gpg --symmetric --cipher-algo AES256 x3-keys-backup.tar.gz

# Move encrypted backup to multiple locations
# Location 1: USB drive (offline storage)
# Location 2: Password manager vault
# Location 3: Secure cloud backup (encrypted)

# Set restrictive permissions on live keys
chmod 400 ~/x3-keys/*

# Delete unencrypted backup
shred -u x3-keys-backup.tar.gz
```

#### Step 5: Ceremony Verification (Core Team)

Create ceremony attestation document:

```markdown
# X3 ATOMIC STAR Genesis Key Generation Ceremony

**Date:** [ISO 8601 timestamp]  
**Location:** Virtual (Zoom)  
**Recording:** [Link to recording]  
**Attestation:** All validators generated keys securely

## Validator Set

| # | Organization | Stash Address | Session Keys | Verified |
|---|--------------|---------------|--------------|----------|
| 1 | Validator A | 5GrwvaEF... | 0x[384 chars] | ✅ [Name] |
| 2 | Validator B | 5Dkw9XDH... | 0x[384 chars] | ✅ [Name] |
| 3 | Validator C | 5FHneW46... | 0x[384 chars] | ✅ [Name] |
| ... | ... | ... | ... | ... |

## Attestation Signatures

I certify that I witnessed all validators generate their keys securely and that no private keys were transmitted over insecure channels.

- Core Team Member 1: [Signature] [Date]
- Core Team Member 2: [Signature] [Date]
- Core Team Member 3: [Signature] [Date]
```

---

## Phase 3: Genesis Spec Creation (Week -1)

### Objectives
- Create chain specification with validator set
- Configure initial balances and allocations
- Set consensus parameters
- Generate genesis block
- Distribute spec to all validators for verification

### Genesis Configuration

#### Step 1: Create Base Chain Spec

```bash
# Generate default chain spec
x3-chain-node build-spec --chain mainnet > chain-spec-raw.json

# This creates a JSON file with all runtime parameters
```

#### Step 2: Customize Genesis Parameters

Edit `chain-spec-raw.json`:

```json
{
  "name": "X3 ATOMIC STAR",
  "id": "x3_mainnet",
  "chainType": "Live",
  "bootNodes": [
    "/dns/bootnode-1.x3.network/tcp/30333/p2p/12D3KooW...",
    "/dns/bootnode-2.x3.network/tcp/30333/p2p/12D3KooW...",
    "/dns/bootnode-3.x3.network/tcp/30333/p2p/12D3KooW..."
  ],
  "telemetryEndpoints": [
    ["wss://telemetry.x3.network/submit", 0]
  ],
  "protocolId": "x3",
  "properties": {
    "tokenSymbol": "X3",
    "tokenDecimals": 12,
    "ss58Format": 42
  },
  "genesis": {
    "runtime": {
      "system": {},
      "balances": {
        "balances": [
          ["5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY", 1000000000000000],
          ["5Dkw9XDH...", 1000000000000000],
          // ... all initial allocations
        ]
      },
      "session": {
        "keys": [
          [
            "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",  // Stash
            "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",  // Controller (same)
            {
              "babe": "5GrwvaEF...",
              "grandpa": "5Dkw9XDH...",
              "im_online": "5FHneW46...",
              "authority_discovery": "5CiPPseX..."
            }
          ],
          // ... all validator session keys
        ]
      },
      "staking": {
        "validatorCount": 10,
        "minimumValidatorCount": 5,
        "invulnerables": [],  // No invulnerable validators (fair game)
        "stakers": [
          [
            "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",  // Stash
            "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",  // Controller
            500000000000000,  // 500k X3 staked
            "Validator"
          ],
          // ... all validator stakes
        ]
      },
      "babe": {
        "authorities": [],  // Populated from session keys
        "epochDuration": 600  // 1 hour epochs (600 blocks * 6s)
      },
      "grandpa": {
        "authorities": []  // Populated from session keys
      },
      "sudo": {
        "key": null  // NO SUDO KEY FOR MAINNET
      }
    }
  }
}
```

**🔴 CRITICAL CONFIG DECISIONS:**

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| `validatorCount` | 10 | Start small, expand post-launch |
| `minimumValidatorCount` | 5 | Minimum for BFT (3f+1 with f=1) |
| `epochDuration` | 600 blocks | 1 hour epochs for stability |
| `sessionDuration` | 6 epochs | 6 hour sessions |
| `bondingDuration` | 28 days | Standard unbonding period |
| `slashDeferDuration` | 7 days | Time to appeal slashing |
| `sudo.key` | **null** | NO SUDO on mainnet |

#### Step 3: Generate Raw Chain Spec

```bash
# Convert human-readable spec to raw format
x3-chain-node build-spec \
  --chain chain-spec-raw.json \
  --raw \
  > x3-mainnet-raw.json

# Verify genesis hash
x3-chain-node build-spec \
  --chain x3-mainnet-raw.json \
  2>&1 | grep "Genesis"

# Expected output:
# Genesis Hash: 0x[64 hex chars]
```

**Store Genesis Hash:**
```
GENESIS_HASH=0x[64 hex chars]
GENESIS_DATE=[ISO 8601 launch time]
```

#### Step 4: Validator Review & Signing

**Send to all validators:**
```
Subject: REVIEW REQUIRED: X3 ATOMIC STAR Genesis Spec

Attached: x3-mainnet-raw.json

INSTRUCTIONS:
1. Download attached genesis spec
2. Verify your validator is included:
   - Search for your stash address
   - Verify initial balance
   - Verify session keys
   - Verify stake amount
3. Verify genesis hash matches: 0x[64 hex chars]
4. Sign approval message (below)
5. Reply by [Deadline]

APPROVAL MESSAGE TO SIGN:
"I, [Your Name], representing [Organization], approve the X3 ATOMIC STAR genesis specification with hash 0x[64 hex chars] for mainnet launch on [Date] [Time] UTC."

Sign with your validator stash key:
$ echo -n "I, [Your Name]..." | x3-chain-node key sign --suri "//YourSeed"

DEADLINE: [48 hours before launch]
```

**Collect Signatures:**
- Minimum 66% (2/3) of validators must approve
- Store all signatures in `genesis-approvals.txt`
- If any validator objects, HALT and investigate

---

## Phase 4: Launch Coordination (Launch Day)

### Pre-Launch Timeline

**L-24 hours:**
- [ ] All validator signatures collected
- [ ] Final chain spec published to GitHub
- [ ] All validators confirmed node installation
- [ ] Bootnodes deployed and tested
- [ ] Telemetry endpoints ready
- [ ] RPC endpoints ready
- [ ] Block explorer ready
- [ ] Monitoring dashboards ready
- [ ] Incident response team on standby
- [ ] Communications plan activated

**L-12 hours:**
- [ ] Final validator readiness check
- [ ] Launch announcement posted (Twitter, Discord, website)
- [ ] Media outreach (if applicable)
- [ ] Emergency contact list verified

**L-6 hours:**
- [ ] Launch rehearsal (dry run on testnet)
- [ ] Final go/no-go poll

**L-1 hour:**
- [ ] All validators in coordination channel
- [ ] Screen sharing enabled
- [ ] Core team monitoring ready

### Launch Sequence

**L-0 (Genesis Block Time)**

All validators execute simultaneously:

```bash
# Start validator node
x3-chain-node \
  --base-path /var/lib/x3-data \
  --chain x3-mainnet-raw.json \
  --name "Validator-[YourName]" \
  --validator \
  --rpc-cors all \
  --unsafe-rpc-external \
  --rpc-methods Safe \
  --port 30333 \
  --rpc-port 9944 \
  --ws-port 9945 \
  --telemetry-url 'wss://telemetry.x3.network/submit 0' \
  --bootnodes /dns/bootnode-1.x3.network/tcp/30333/p2p/12D3KooW... \
  --bootnodes /dns/bootnode-2.x3.network/tcp/30333/p2p/12D3KooW... \
  --bootnodes /dns/bootnode-3.x3.network/tcp/30333/p2p/12D3KooW... \
  >> /var/log/x3-validator.log 2>&1 &

# Save PID
echo $! > /var/run/x3-validator.pid

# Tail logs
tail -f /var/log/x3-validator.log
```

**Expected Log Output:**
```
2026-04-26 22:00:00 X3 ATOMIC STAR Node
2026-04-26 22:00:00   version 1.0.0-dc9d1bd
2026-04-26 22:00:00   by X3 Team, 2024-2026
2026-04-26 22:00:00 Chain specification: X3 ATOMIC STAR
2026-04-26 22:00:00 Node name: Validator-[YourName]
2026-04-26 22:00:00 Role: AUTHORITY
2026-04-26 22:00:00 Database: RocksDB
2026-04-26 22:00:00 Initializing Genesis block/state...
2026-04-26 22:00:05 Genesis Hash: 0x[64 hex chars]
2026-04-26 22:00:10 🏷  Local node identity is: 12D3KooW...
2026-04-26 22:00:15 🔍 Discovered new external address: /ip4/X.X.X.X/tcp/30333/p2p/12D3KooW...
2026-04-26 22:00:20 💤 Idle (0 peers), best: #0 (0x[hash]), finalized #0
```

**L+5 minutes:** Validators discover each other
```
2026-04-26 22:05:00 💤 Idle (5 peers), best: #0 (0x[hash]), finalized #0
```

**L+10 minutes:** First block produced
```
2026-04-26 22:10:00 🙌 Starting consensus session on top of parent 0x[hash]
2026-04-26 22:10:06 🎁 Prepared block for proposing at 1 [hash: 0x...; parent_hash: 0x...]
2026-04-26 22:10:06 🔖 Pre-sealed block for proposal at 1. Hash now 0x...
2026-04-26 22:10:12 ✨ Imported #1 (0x...)
2026-04-26 22:10:18 💤 Idle (7 peers), best: #1 (0x[hash]), finalized #0
```

**L+20 minutes:** First finality
```
2026-04-26 22:20:00 ✨ Imported #10 (0x...)
2026-04-26 22:20:00 ⚙️  Finalizing blocks up to #5 (0x...)
2026-04-26 22:20:00 💤 Idle (9 peers), best: #10 (0x[hash]), finalized #5 (0x[hash])
```

### Launch Checklist (Live Monitoring)

**Core Team Monitors:**

| Metric | Target | Status |
|--------|--------|--------|
| Validators online | 10/10 (100%) | ⏳ |
| Peer connections | 5+ per validator | ⏳ |
| Block production | 1 block every 6 seconds | ⏳ |
| Finality | Blocks finalizing | ⏳ |
| No errors in logs | Zero WARN/ERROR | ⏳ |
| Telemetry reporting | All validators visible | ⏳ |
| Block explorer syncing | Real-time updates | ⏳ |
| RPC endpoints responding | 200 OK | ⏳ |

**Red Flags (ABORT CRITERIA):**
- 🚨 Less than 5 validators online after 10 minutes
- 🚨 No blocks produced after 15 minutes
- 🚨 No finality after 30 minutes
- 🚨 Multiple validators reporting errors
- 🚨 Network partition detected
- 🚨 Consensus failures

**If Red Flag → Execute Emergency Rollback (see Phase 6)**

---

## Phase 5: Post-Launch Verification (Day 1-7)

### Day 1 Monitoring

**Hour 1-2:**
- [ ] All validators producing blocks
- [ ] Finality progressing normally
- [ ] No errors in validator logs
- [ ] Telemetry showing healthy metrics

**Hour 2-6:**
- [ ] First epoch transition successful (block 600)
- [ ] First session transition successful (block 3600)
- [ ] Staking rewards calculated correctly
- [ ] ImOnline heartbeats received from all validators

**Hour 6-24:**
- [ ] 24 hours of continuous operation
- [ ] No chain halts
- [ ] No finality stalls
- [ ] No slashing events (unless justified)

### Week 1 Checklist

**Daily Tasks:**
- [ ] Review validator logs for errors
- [ ] Check finality lag (should be <10 blocks)
- [ ] Monitor peer count (should be stable)
- [ ] Verify telemetry reporting
- [ ] Check for slashing events
- [ ] Review incident reports

**Key Milestones:**
- [ ] Day 1: 24 hours continuous operation ✅
- [ ] Day 3: First governance proposal (simple test) ✅
- [ ] Day 7: First week complete, no major incidents ✅

**Success Criteria:**
- ✅ 99.9%+ network uptime
- ✅ <2 second finality achieved
- ✅ All validators at 99%+ individual uptime
- ✅ Zero critical bugs discovered
- ✅ No emergency patches required
- ✅ Community feedback positive

---

## Phase 6: Emergency Rollback Procedures

### When to Rollback

**IMMEDIATE ROLLBACK if:**
- Chain halt lasting >1 hour with no recovery path
- Consensus failure (multiple competing forks)
- Critical security exploit discovered (S0 level)
- Canonical supply violation detected
- Bridge exploit in progress
- Majority validator collusion detected

**DO NOT ROLLBACK for:**
- Individual validator offline (<30% of set)
- Temporary network partition (resolves <30 min)
- Non-critical bugs (can be patched)
- FUD/social panic without technical issue

### Rollback Decision Authority

**Decision Maker:** Core team lead + 66% of validators must agree

**Decision Template:**
```
EMERGENCY ROLLBACK DECISION

Issue: [Brief description]
Severity: [S0/S1/S2]
Impact: [# users affected, $ at risk]
Discovery Time: [Timestamp]
Rollback Decision: [YES/NO]
Decided by: [Names + signatures]
Timestamp: [ISO 8601]

Rationale:
[Detailed explanation of why rollback is necessary]

Alternative Considered:
[Why not patching/forking instead]
```

### Rollback Procedure

**Step 1: Halt Network (All Validators)**

```bash
# Stop validator immediately
PID=$(cat /var/run/x3-validator.pid)
kill $PID

# Verify stopped
ps aux | grep x3-chain-node
```

**Step 2: Coordinate Communication**

Post to all channels simultaneously:
```
🚨 EMERGENCY NETWORK HALT 🚨

The X3 ATOMIC STAR network has been halted due to [ISSUE].

WHAT HAPPENED:
[Brief technical explanation]

WHAT WE'RE DOING:
[Rollback plan or fix plan]

TIMELINE:
- Current time: [Timestamp]
- Expected resolution: [Estimate]
- Updates every: 30 minutes

WHERE TO FOLLOW:
- Discord: [Link]
- Twitter: [Link]
- Status page: [Link]

USER IMPACT:
- Transactions: PAUSED
- Balances: SAFE (preserved at block #[NUM])
- Staking: PAUSED

WHAT YOU SHOULD DO:
- Do NOT send transactions
- Do NOT panic sell
- Wait for official updates
```

**Step 3: Create New Genesis (Core Team)**

```bash
# Export state at last good block
x3-chain-node export-state \
  --chain x3-mainnet-raw.json \
  --base-path /var/lib/x3-data \
  --block [LAST_GOOD_BLOCK_HASH] \
  > exported-state.json

# Create new genesis with exported state
# (Manual process - edit chain spec with exported state)

# Generate new chain spec
x3-chain-node build-spec \
  --chain chain-spec-rollback.json \
  --raw \
  > x3-mainnet-rollback-raw.json

# Distribute to all validators
```

**Step 4: Validator Data Wipe (All Validators)**

```bash
# DANGEROUS: This deletes all blockchain data
# Only execute after confirming with core team

# Backup old data (just in case)
sudo tar -czf /backup/x3-data-emergency-$(date +%s).tar.gz /var/lib/x3-data

# Wipe blockchain data
sudo rm -rf /var/lib/x3-data/*

# Verify clean
ls -la /var/lib/x3-data/
# Should be empty
```

**Step 5: Relaunch Network**

Follow Phase 4 launch sequence with new chain spec.

**Step 6: Post-Mortem**

Within 72 hours of rollback, publish post-mortem:
```markdown
# X3 ATOMIC STAR Emergency Rollback Post-Mortem

**Date:** [ISO 8601]  
**Duration:** [Start time] to [Resolution time]  
**Impact:** [# blocks rolled back, # users affected]

## What Happened
[Technical root cause analysis]

## Timeline
- [Time]: Issue detected
- [Time]: Decision to rollback made
- [Time]: Network halted
- [Time]: New genesis created
- [Time]: Network relaunched
- [Time]: Normal operation resumed

## Root Cause
[Detailed technical explanation]

## What Went Wrong
[Specific failures - code, process, monitoring]

## What Went Right
[Successful aspects - detection, communication, execution]

## Lessons Learned
1. [Lesson 1]
2. [Lesson 2]
3. [Lesson 3]

## Action Items
1. [ ] [Fix 1] - Owner: [Name] - Due: [Date]
2. [ ] [Fix 2] - Owner: [Name] - Due: [Date]
3. [ ] [Fix 3] - Owner: [Name] - Due: [Date]

## Preventative Measures
[Steps taken to prevent recurrence]

Signed: [Core Team] [Date]
```

---

## Appendix A: Communication Templates

### Launch Announcement (Twitter)

```
🚀 X3 ATOMIC STAR MAINNET IS LIVE! 🚀

After 8 weeks of public testnet, 2 external security audits, and rigorous testing, we're proud to announce the official launch of X3 ATOMIC STAR mainnet.

🔗 Genesis Hash: 0x[64 chars]
⏰ Launch Time: [ISO 8601]
🌐 Explorer: https://explorer.x3.network
📊 Telemetry: https://telemetry.x3.network

Features:
✅ Universal Asset Kernel
✅ Atomic Cross-VM Execution (EVM+SVM)
✅ Bridge Security with replay protection
✅ <2s finality
✅ 10 founding validators

Join us: https://x3.network

#X3 #Blockchain #Mainnet #Launch
```

### Launch Announcement (Discord)

```
@everyone 

# 🎉 MAINNET LAUNCH SUCCESSFUL 🎉

X3 ATOMIC STAR mainnet is now LIVE!

**Genesis Block:** #0 at [Timestamp]  
**Genesis Hash:** `0x[64 chars]`  
**Validators:** 10/10 online ✅  
**Block Production:** Active ✅  
**Finality:** Progressing ✅  

**Links:**
- 🌐 Explorer: https://explorer.x3.network
- 📊 Telemetry: https://telemetry.x3.network
- 📡 RPC Endpoint: wss://rpc.x3.network
- 📖 Docs: https://docs.x3.network

**What You Can Do Now:**
- Create wallet and receive tokens
- Stake with validators
- Use bridge (EVM ↔ X3)
- Build dApps
- Participate in governance

**Support:**
- Technical questions: #support
- Validator questions: #validators
- Development: #dev-chat

Thank you to our incredible community for making this possible! 🙏
```

---

## Appendix B: Monitoring Dashboard

### Key Metrics

**Grafana Dashboard Panels:**

1. **Network Health:**
   - Total peer count
   - Validator count (online vs configured)
   - Block height (should increase monotonically)
   - Finality lag (should be <10 blocks)

2. **Block Production:**
   - Blocks per minute (target: 10 blocks/min)
   - Block production time (target: 6s)
   - Empty block rate (should be <5%)
   - Uncle block rate (should be <1%)

3. **Consensus:**
   - BABE epoch progress
   - GRANDPA finality votes
   - Session progress
   - Authority set changes

4. **Validator Performance:**
   - Per-validator uptime
   - ImOnline heartbeats
   - Authored blocks per validator
   - Slash events (should be zero)

5. **Network:**
   - Inbound/outbound bandwidth
   - Peer connection churn
   - Network latency between validators
   - Transaction pool size

6. **Alerts:**
   - 🚨 Validator offline >5 minutes
   - 🚨 No blocks produced in 60 seconds
   - 🚨 Finality stalled >2 minutes
   - 🚨 Consensus errors
   - ⚠️ High memory usage (>80%)
   - ⚠️ High disk usage (>80%)

---

## Appendix C: Contact List

### Core Team

| Role | Name | Email | Phone | Telegram |
|------|------|-------|-------|----------|
| CEO | TBD | ceo@x3.network | +1-XXX-XXX-XXXX | @x3ceo |
| CTO | TBD | cto@x3.network | +1-XXX-XXX-XXXX | @x3cto |
| Lead Dev | TBD | dev@x3.network | +1-XXX-XXX-XXXX | @x3dev |
| Ops Lead | TBD | ops@x3.network | +1-XXX-XXX-XXXX | @x3ops |
| Comms Lead | TBD | comms@x3.network | +1-XXX-XXX-XXXX | @x3comms |

### External

| Role | Organization | Contact | Email | Phone |
|------|--------------|---------|-------|-------|
| Legal Counsel | TBD | TBD | legal@firm.com | +1-XXX |
| Security Auditor | Trail of Bits | TBD | audits@trailofbits.com | +1-XXX |
| Infrastructure | AWS | TBD | support@aws.com | Enterprise Support |

### Emergency Escalation

**Severity 1 (Chain Halt):**
1. Call CTO immediately (+1-XXX-XXX-XXXX)
2. Post in #emergency-response (Discord)
3. If no response in 5 min, call CEO

**Severity 2 (Degraded Performance):**
1. Post in #operations (Discord)
2. Tag @ops-team
3. Create incident report

---

**END OF RUNBOOK**

*This document is classified as: PUBLIC*  
*Last updated: April 26, 2026*  
*Version: 1.0*  
*Owner: X3 Core Team*
