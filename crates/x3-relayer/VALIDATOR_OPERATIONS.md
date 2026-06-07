# Validator Operations

**Document Version:** 1.0  
**Last Updated:** 2026-04-21  
**Status:** Ready for Production Use  
**Target Audience:** Validators, DevOps Engineers, Network Operators

---

## Overview

This runbook provides **detailed procedures for managing X3 validators** on mainnet. It covers:
- Adding validators to the network
- Removing validators gracefully
- Key rotation and security management
- Slashing recovery procedures
- Rewards management and collection
- Validator health monitoring
- Lifecycle management

### Quick Reference

**Key Commands:**
```bash
# Check validator status
x3-cli validator status --validator $VALIDATOR_ADDRESS

# Add validator
x3-cli validator add --stake $STAKE_AMOUNT --key-file $KEY_FILE

# Remove validator
x3-cli validator remove --validator $VALIDATOR_ADDRESS

# Check slashing status
x3-cli validator slashing-status --validator $VALIDATOR_ADDRESS

# Claim rewards
x3-cli validator claim-rewards --validator $VALIDATOR_ADDRESS
```
### Related Documents

**For incidents affecting validators:** See **MAINNET_INCIDENT_RESPONSE.md** (validator recovery during crisis)

**For GPU validators:** See **GPU_VALIDATOR_TROUBLESHOOTING.md** (GPU initialization, CUDA errors, thermal issues)

**For performance expectations:** See **MAINNET_PERFORMANCE_BASELINE.md** (validator's impact on network metrics)

**For launch timeline:** See **PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md** (when validator operations should occur)
---

## Section 1: Validator Architecture

### X3 Validator Requirements

**Hardware Requirements:**
- CPU: 8+ cores (16+ recommended)
- RAM: 16GB minimum (32GB recommended)
- Storage: 1TB SSD (fast random access required)
- Network: 1Gbps uplink (dedicated preferred)
- Uptime: 99.5% minimum

**Software Requirements:**
- X3 node binary (latest stable version)
- Consensus client (if separate)
- Execution client (if separate)
- Monitoring agent (Prometheus)

**Staking Requirements:**
- Minimum stake: 1000000 X3 tokens
- Lock period: 300 blocks
- Unbond period: 150 blocks after removal

### Validator Economic Model

| Parameter | Value |
|-----------|-------|
| Validator commission | 5-15% (variable) |
| Annual inflation | ~5% |
| Slashing penalty | 0.1-1.0 (varies by severity) |
| Lock period | 300 blocks |
| Unbond period | 150 blocks |

---

## Section 2: Adding New Validators

### Pre-Addition Checklist

**Before adding a validator, verify:**

```bash
#!/bin/bash

echo "=== Pre-Addition Validator Checklist ==="

# 1. Hardware check
echo "Checking hardware..."
CORES=$(nproc)
echo "  CPU cores: $CORES (need 8+)"
[ $CORES -ge 8 ] && echo "  ✓ CPU OK" || echo "  ✗ CPU insufficient"

RAM=$(free -g | awk 'NR==2 {print $2}')
echo "  RAM: ${RAM}GB (need 16+)"
[ $RAM -ge 16 ] && echo "  ✓ RAM OK" || echo "  ✗ RAM insufficient"

DISK=$(df -BG /var/lib/x3 | awk 'NR==2 {print $4}' | sed 's/G//')
echo "  Disk: ${DISK}GB (need 1000+)"
[ $DISK -ge 1000 ] && echo "  ✓ Disk OK" || echo "  ✗ Disk insufficient"

# 2. Network check
echo
echo "Checking network..."
PING=$(ping -c 3 x3-mainnet-api.example.com | grep -oP '\d+\.\d+' | tail -1)
echo "  Latency to network: ${PING}ms"

# 3. Software check
echo
echo "Checking software..."
x3-cli --version
echo "  X3 CLI: OK"

# 4. Wallet check
echo
echo "Checking wallet..."
x3-cli account balance --account $VALIDATOR_ACCOUNT
echo "  Account balance: OK"

# 5. Stake check
echo
echo "Checking available stake..."
BALANCE=$(x3-cli account balance --account $VALIDATOR_ACCOUNT --json | jq '.balance')
REQUIRED_STAKE=1000000  # Example: 1M tokens
echo "  Available: $BALANCE tokens"
echo "  Required: $REQUIRED_STAKE tokens"
[ $BALANCE -ge $REQUIRED_STAKE ] && echo "  ✓ Sufficient stake" || echo "  ✗ Insufficient stake"

echo
echo "=== Pre-Addition Checklist Complete ==="
```

### Key Generation

**Generate secure validator keys:**

```bash
# Option 1: Using X3 CLI (Recommended)
x3-cli keys generate --validator-key \
  --output /secure/path/validator_key.json \
  --password-file /secure/path/password.txt

# Option 2: Using HSM (Hardware Security Module) - Most Secure
x3-cli keys generate --validator-key \
  --hsm-slot 0 \
  --hsm-pin 1234

# Verify key generation
x3-cli keys import --key-file /secure/path/validator_key.json --verify

# Expected output:
# ✓ Key generated successfully
# Public key: 0x123abc...
# Key type: validator-ed25519
```

**Key Security Best Practices:**

```bash
# Store key with restricted permissions
chmod 600 /secure/path/validator_key.json

# Create backup (encrypted)
gpg -e -r "backup@example.com" /secure/path/validator_key.json
mv /secure/path/validator_key.json.gpg /backup/location/

# Verify backup integrity
gpg -d /backup/location/validator_key.json.gpg | md5sum

# Document recovery procedure
cat > /secure/path/KEY_RECOVERY.md << 'EOF'
# Key Recovery Procedure
1. Decrypt backup: gpg -d validator_key.json.gpg > validator_key.json
2. Verify checksum matches recovery document
3. Place in secure location
4. Restart validator service
EOF

# Encrypt recovery document
gpg -e -r "backup@example.com" /secure/path/KEY_RECOVERY.md
```

### Staking and Adding Validator

**Submit stake and add validator to network:**

```bash
# Step 1: Verify stake amount
STAKE_AMOUNT=1000000  # 1M X3 tokens
echo "Adding validator with stake: $STAKE_AMOUNT"

# Step 2: Submit stake transaction
x3-cli validator add \
  --stake $STAKE_AMOUNT \
  --key-file /secure/path/validator_key.json \
  --commission 10 \
  --moniker "my-validator-1" \
  --website "https://myvalidator.example.com" \
  --details "Reliable validator operator" \
  --from $VALIDATOR_ACCOUNT \
  --gas-limit 500000 \
  --broadcast

# Expected output:
# Transaction: 0x123abc...
# Block height: 12345
# Validator added at: height 12346 (after block confirmation)

# Step 3: Wait for confirmation
sleep 30

# Step 4: Verify addition
x3-cli validator status --validator $VALIDATOR_ADDRESS

# Expected output:
# Status: active
# Stake: 1000000 X3
# Commission: 10%
# Voting power: 0.001% (approx)
```

### Post-Addition Configuration

```bash
# Step 1: Create validator config file
cat > /etc/x3-validator/mainnet.yaml << 'EOF'
validator:
  address: "0x1234..."  # Replace with actual address
  key_file: "/secure/path/validator_key.json"
  monitoring:
    enabled: true
    metrics_port: 9091
  rewards:
    auto_claim: true
    claim_interval_blocks: 100000
  slashing:
    auto_monitor: true
    alert_on_risk: true
EOF

# Step 2: Start validator service
sudo systemctl enable x3-validator
sudo systemctl start x3-validator

# Step 3: Verify validator is running
sudo systemctl status x3-validator

# Expected output:
# ● x3-validator.service - X3 Validator
#    Loaded: loaded (/etc/systemd/system/x3-validator.service)
#    Active: active (running)

# Step 4: Monitor logs
sudo journalctl -u x3-validator -f
```

---

## Section 3: Removing Validators

### Graceful Validator Exit

**Remove validator while preserving stake (bonds):**

```bash
# Step 1: Announce intent to exit (optional but recommended)
x3-cli validator announce-exit \
  --validator $VALIDATOR_ADDRESS \
  --exit-height 123500  # Announce will exit at this height

# Step 2: Stop accepting new delegations (if applicable)
x3-cli validator set-commission \
  --validator $VALIDATOR_ADDRESS \
  --commission 100  # Set to 100% to stop accepting delegations

# Step 3: Wait for current consensus round to complete
# (Usually < 5 minutes)
sleep 300

# Step 4: Remove validator
x3-cli validator remove \
  --validator $VALIDATOR_ADDRESS \
  --from $VALIDATOR_ACCOUNT \
  --gas-limit 500000 \
  --broadcast

# Expected output:
# Transaction: 0x456def...
# Validator removed at block: 12346
# Bonds released: 1000000 X3
# Unbond period: 150 blocks (~30 minutes)

# Step 5: Wait for unbond period
sleep 1800  # 30 minutes

# Step 6: Claim unbonded stake
x3-cli validator claim-unbonded \
  --validator $VALIDATOR_ADDRESS \
  --from $VALIDATOR_ACCOUNT \
  --broadcast

# Expected output:
# Transaction: 0x789ghi...
# Stake returned: 1000000 X3
```

### Emergency Validator Shutdown

**If validator must stop immediately (e.g., security issue):**

```bash
# Step 1: Immediately stop validator service
sudo systemctl stop x3-validator

# Step 2: Secure the keys
sudo systemctl stop x3-validator-key-manager  # If using key manager

# Step 3: Mark validator as offline in monitoring
echo "EMERGENCY_STOP: $(date)" >> /var/log/validator_emergency.log

# Step 4: Notify network operators
# (Send alert to operations team)

# Step 5: Initiate graceful removal (see previous section)
# Once situation is handled and keys are secured

# Step 6: Post-mortem and recovery
# See Slashing Recovery section below
```

---

## Section 4: Key Rotation

### Security Best Practice: Regular Key Rotation

**Rotate validator keys every 6 months:**

```bash
#!/bin/bash

echo "=== Validator Key Rotation Procedure ==="

# Step 1: Generate new key
echo "Generating new validator key..."
x3-cli keys generate --validator-key \
  --output /secure/path/validator_key_new.json \
  --password-file /secure/path/password.txt

NEW_PUBLIC_KEY=$(cat /secure/path/validator_key_new.json | jq '.public_key')
echo "New public key: $NEW_PUBLIC_KEY"

# Step 2: Submit key rotation transaction
echo "Submitting key rotation transaction..."
x3-cli validator rotate-key \
  --validator $VALIDATOR_ADDRESS \
  --new-key $NEW_PUBLIC_KEY \
  --from $VALIDATOR_ACCOUNT \
  --broadcast

# Expected output:
# Transaction: 0xkey_rotation...
# Key rotation initiated at block: 12345
# Effective at: block 12355 (10 blocks delay for safety)

# Step 3: Wait for rotation to be effective
sleep 100  # ~10 blocks

# Step 4: Verify key rotation
x3-cli validator status --validator $VALIDATOR_ADDRESS | grep "public_key"

# Should show new public key

# Step 5: Backup old key (encrypted)
gpg -e -r "backup@example.com" /secure/path/validator_key_old.json

# Step 6: Update validator configuration
sudo nano /etc/x3-validator/mainnet.yaml
# Change: key_file: "/secure/path/validator_key_new.json"

# Step 7: Restart validator service
sudo systemctl restart x3-validator

# Step 8: Verify new key is active
sleep 30
x3-cli validator status --validator $VALIDATOR_ADDRESS

# Expected output:
# Key rotation: successful
# Current key: [new key]
# Status: active

echo "=== Key Rotation Complete ==="
```

### Key Compromise Recovery

**If private key is compromised:**

```bash
# EMERGENCY PROCEDURE

# Step 1: Immediately stop validator
sudo systemctl stop x3-validator
echo "COMPROMISED KEY - $(date)" >> /var/log/validator_emergency.log

# Step 2: Notify chain security team immediately
# This is CRITICAL - notify VP Engineering and Security team

# Step 3: Rotate key immediately (do not wait for scheduled rotation)
x3-cli validator rotate-key --validator $VALIDATOR_ADDRESS --emergency

# Step 4: Wait for rotation effectiveness
sleep 300  # Wait longer for emergency rotation

# Step 5: Verify old key no longer controls validator
# Try to sign transaction with old key - should fail

# Step 6: Recover from backup
# Get clean key backup (stored securely away)

# Step 7: Restart with new key
sudo systemctl restart x3-validator

# Step 8: Post-incident review
# Document: When key was compromised, how it was discovered, recovery steps taken
```

---

## Section 5: Slashing Recovery

### Slashing Overview

**Slashing is the penalty for validator misbehavior:**

| Violation Type | Penalty | Recovery |
|---|---|---|
| Downtime (< 95% uptime) | 0.1% of stake | Automatic after uptime recovers |
| Double proposal | 1% of stake | Must reactivate validator |
| Double voting | 2% of stake | Must reactivate validator |

### Detecting Slashing

```bash
# Check if validator has been slashed
x3-cli validator slashing-status --validator $VALIDATOR_ADDRESS

# Expected output (if slashed):
# Slashing events: 1
# Total slashed: 10000 X3 (0.1% of 10M stake)
# Slashing reason: downtime_violation
# Recovery possible: true
# Expected recovery: 3 days

# Monitor slashing risk
watch -n 300 'x3-cli validator slashing-status --validator $VALIDATOR_ADDRESS | grep -E "risk|downtime|proposal"'
```

### Recovering from Downtime Slashing

```bash
# Downtime slashing is most common
# Validator automatically recovers once uptime improves

# Step 1: Verify validator uptime
x3-cli validator uptime --validator $VALIDATOR_ADDRESS

# Step 2: If downtime < 95%, fix the issue
# (Usually: fix network, fix hardware, fix software)

# Step 3: Restart validator service
sudo systemctl restart x3-validator

# Step 4: Monitor recovery
for i in {1..10}; do
  echo "Check $i (Block: $(x3-cli status | jq '.block_height')):"
  x3-cli validator uptime --validator $VALIDATOR_ADDRESS
  sleep 60
done

# Once uptime > 95%, slashing penalty removed automatically
```

### Recovering from Double Proposal/Voting

**More serious - requires manual reactivation:**

```bash
# Step 1: Determine severity
x3-cli validator slashing-status --validator $VALIDATOR_ADDRESS | grep "reason"

# Step 2: If double proposal/voting occurred
# This usually means validator was running on multiple machines
# or forked inappropriately

# Step 3: Fix root cause
# - Ensure validator only runs on one machine
# - Verify clock synchronization
# - Verify validator key is not duplicated elsewhere

# Step 4: Reactivate validator
x3-cli validator reactivate \
  --validator $VALIDATOR_ADDRESS \
  --from $VALIDATOR_ACCOUNT \
  --broadcast

# Expected output:
# Reactivation submitted
# Status: jailed (waiting for recovery period)
# Reactivation available at: block 12500

# Step 5: Wait for reactivation period
sleep 300  # Usually 5 minutes

# Step 6: Complete reactivation
x3-cli validator unjail \
  --validator $VALIDATOR_ADDRESS \
  --from $VALIDATOR_ACCOUNT \
  --broadcast

# Step 7: Verify validator active again
x3-cli validator status --validator $VALIDATOR_ADDRESS | grep "status"
# Should show: active
```

---

## Section 6: Rewards Management

### Automatic Rewards Collection

**Configure automatic reward claiming:**

```bash
# Edit validator configuration
sudo nano /etc/x3-validator/mainnet.yaml

# Add rewards configuration:
rewards:
  auto_claim: true           # Enable automatic claiming
  claim_interval_blocks: 100000  # Claim every ~3 weeks
  minimum_balance_required: 0    # Always claim, regardless of balance
  account_to_send: "0x..."   # Account to receive rewards

# Restart validator service
sudo systemctl restart x3-validator

# Verify auto-claim is working
sudo journalctl -u x3-validator | grep "claiming rewards"
# Should see periodic claim transactions
```

### Manual Rewards Collection

```bash
# Check pending rewards
x3-cli validator pending-rewards --validator $VALIDATOR_ADDRESS

# Expected output:
# Pending rewards: 1500 X3
# Last claim: block 123400
# Claim available at: block 124400

# Claim rewards manually
x3-cli validator claim-rewards \
  --validator $VALIDATOR_ADDRESS \
  --from $VALIDATOR_ACCOUNT \
  --broadcast

# Expected output:
# Transaction: 0xreward_claim...
# Rewards claimed: 1500 X3
# Sent to: 0x...
```

### Rewards Delegation

```bash
# Automatically re-stake rewards to increase stake
x3-cli validator update-config \
  --validator $VALIDATOR_ADDRESS \
  --restake-rewards true \
  --broadcast

# This will automatically:
# - Claim rewards regularly
# - Add them to validator stake
# - Increase voting power over time

# Monitor restaking
watch -n 3600 'x3-cli validator status --validator $VALIDATOR_ADDRESS | jq ".stake"'
# Stake should gradually increase
```

---

## Section 7: Validator Health Monitoring

### Monitoring Metrics

**Key metrics to monitor:**

```bash
# Setup Prometheus to scrape validator metrics
cat > /etc/prometheus/x3-validator-rules.yml << 'EOF'
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'x3-validator'
    static_configs:
      - targets: ['localhost:9091']

# Alerts
rule_files:
  - '/etc/prometheus/x3-validator-alerts.yml'
EOF

# Restart Prometheus
sudo systemctl restart prometheus
```

**Metrics to track:**

| Metric | Target | Alert Threshold |
|--------|--------|-----------------|
| Validator uptime | > 99.5% | < 95% |
| Block proposals | > 0 per epoch | 0 per epoch |
| Attestations | > 90% | < 85% |
| Rewards claimed | > 0 per week | 0 per week |
| Slashing events | 0 | > 0 |

### Custom Monitoring Script

```bash
#!/bin/bash

VALIDATOR="0x1234..."
MONITORING_INTERVAL=300  # Check every 5 minutes

while true; do
  # Get metrics
  UPTIME=$(x3-cli validator uptime --validator $VALIDATOR | jq '.percentage')
  STATUS=$(x3-cli validator status --validator $VALIDATOR | jq '.status')
  PROPOSALS=$(x3-cli validator proposals --validator $VALIDATOR | jq '.count')
  
  echo "$(date): Uptime=$UPTIME% Status=$STATUS Proposals=$PROPOSALS"
  
  # Alert conditions
  if (( $(echo "$UPTIME < 95" | bc -l) )); then
    echo "ALERT: Uptime below threshold!" | mail -s "Validator Alert" admin@example.com
  fi
  
  if [ "$STATUS" != "active" ]; then
    echo "ALERT: Validator not active: $STATUS" | mail -s "Validator Alert" admin@example.com
  fi
  
  sleep $MONITORING_INTERVAL
done
```

---

## Section 8: Validator Lifecycle Management

### Regular Maintenance Schedule

```bash
# Daily checks (automated)
0 */4 * * * /usr/local/bin/check-validator-health.sh

# Weekly reviews (manual)
Every Monday 10:00 AM: Review validator performance

# Monthly maintenance (scheduled)
1st of each month: Full validator audit
- Hardware checks
- Security review
- Performance analysis
- Key backup verification

# Quarterly validation (major review)
Every 3 months:
- Commission review and adjustment (if needed)
- Delegation analysis
- Rewards optimization
- Slashing risk assessment

# Bi-annual key rotation (security)
Every 6 months: Rotate validator keys
```

### Validator Upgrade Procedure

**When new X3 version is released:**

```bash
# Step 1: Download and verify new version
wget https://releases.x3.network/x3-node-v1.2.0-linux-x86_64.tar.gz
shasum -a 256 x3-node-v1.2.0-linux-x86_64.tar.gz > /tmp/checksum.txt
cat > /tmp/expected-checksum.txt << 'EOF'
abcd1234... x3-node-v1.2.0-linux-x86_64.tar.gz
EOF
diff /tmp/checksum.txt /tmp/expected-checksum.txt

# Step 2: Test upgrade on staging first
# (See MAINNET_VALIDATION.md for staging procedures)

# Step 3: Schedule upgrade (ideally during low-activity period)
# Notify delegators if applicable

# Step 4: Backup current binary and data
cp -r /opt/x3-validator /backup/x3-validator.v1.1.5.$(date +%s)

# Step 5: Extract and install new binary
tar -xzf x3-node-v1.2.0-linux-x86_64.tar.gz -C /tmp/
sudo cp /tmp/x3-node-v1.2.0/x3-validator /opt/x3-validator/x3-validator

# Step 6: Verify binary
/opt/x3-validator/x3-validator --version
# Should show: v1.2.0

# Step 7: Restart validator service
sudo systemctl restart x3-validator

# Step 8: Monitor for issues
sudo journalctl -u x3-validator -f &
MONITOR_PID=$!
sleep 300  # Monitor for 5 minutes

# Step 9: Verify validator healthy
x3-cli validator status --validator $VALIDATOR_ADDRESS
# Should show: active

kill $MONITOR_PID
```

---

## Appendix: Common Issues and Solutions

| Issue | Cause | Solution |
|-------|-------|----------|
| High latency blocks | Network congestion | Check network setup, upgrade bandwidth |
| Frequent slashing | Poor uptime | Fix underlying infrastructure issue |
| Low rewards | Insufficient stake | Redelegate more funds or accept lower rewards |
| Key access errors | Permissions issue | Check file permissions on key file |
| Commission not updating | Wrong account used | Verify you're using the correct account |

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-04-21 | Initial validator operations procedures |

---

**Questions?** Contact: [validator-support-email]
