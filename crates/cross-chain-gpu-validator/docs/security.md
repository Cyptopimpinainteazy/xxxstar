# Security and Rollback Procedures

## Security Principles

### 1. Atomic Invariants

**ATOMIC-COMMIT-001**: Both chains commit or both rollback.
- **Enforcement**: Swap state machine enforces strict phase progression
- **Verification**: Registry tracks which chains have validated

**ATOMIC-TIMEOUT-002**: Timeout causes automatic rollback.
- **Enforcement**: Timeout triggers phase transition to `TimedOut` and rollback
- **Timeout Values**: Configurable per-swap, default 60 seconds

**GPU-CPU-PARITY-001**: GPU and CPU produce identical results.
- **Verification**: Parity tests run on startup and periodically
- **Fallback**: GPU errors trigger CPU validation, result verified before commit

**REGISTRY-CONSISTENCY-001**: Redis registry is source of truth.
- **Synchronization**: All phase transitions go through registry
- **TTL**: Swap records expire after 1 hour (configurable)
- **Fail-Closed**: Registry unavailability rejects new swaps

### 2. Cryptographic Assumptions

- **secp256k1**: NIST-approved elliptic curve, FIPS 186-4 compliant
- **keccak256**: SHA-3 winner, EVM standard
- **No Key Material**: Validator does not hold private keys

### 3. Input Validation

- **Signature Format**: Must be exactly 64 bytes (r,s)
- **Public Key Format**: Must be 33 (compressed) or 65 (uncompressed) bytes
- **Block/Slot Numbers**: Must be > 0 and within acceptable range
- **Data Size Limits**: Max 1MB per transaction (configurable)

## Threat Model

### Network Adversary

**Threat**: Byzantine RPC endpoints return false validation results.

**Mitigation**:
- Use multiple RPC endpoints with quorum voting
- Signature verification confirms actual blockchain state
- Timeout + rollback prevents hanging on malicious RPC

### GPU Corruption

**Threat**: GPU produces incorrect results silently.

**Mitigation**:
- Parity checks compare GPU and CPU results
- Failover to CPU on discrepancy
- Health checks run periodically
- All results verified in validator logic

### Registry Compromise

**Threat**: Redis is compromised, swap state corrupted.

**Mitigation**:
- Redis access restricted to local network
- Swap records are immutable (only phase transitions allowed)
- Fail-closed behavior: if registry unreachable, reject new swaps
- Regular backups and audit logs

### Timeout Abuse

**Threat**: Attacker exploits timeouts to cause rollbacks.

**Mitigation**:
- Configurable timeout values (default 60s, reasonable for testnets)
- Timeout is safety mechanism, not performance optimization
- Metrics track timeout frequency for anomaly detection

## Anomaly Detection

### Triggers for Investigation

1. **Success Rate Drop Below 95%**
   ```bash
   grep "rolled back\|timeout" logs.txt | wc -l
   # If > 5% of swaps, investigate RPC health
   ```

2. **GPU/CPU Parity Failure**
   ```bash
   grep "parity check failed" logs.txt
   # Update GPU drivers or disable GPU
   ```

3. **Registry Latency Spike**
   ```bash
   grep "Redis latency" logs.txt | tail -20
   # If > 1s, check Redis health or network connectivity
   ```

4. **Repeated Timeouts on Specific Chain**
   ```bash
   grep "EVM.*timeout\|SVM.*timeout" logs.txt
   # Switch RPC endpoints or increase timeout
   ```

## Rollback Procedures

### 1. Manual Swap Rollback

If a swap is stuck in `Pending` or `ValidatingEvm`:

```bash
# Get swap status
redis-cli GET "swap:SWAP_ID"

# Force rollback
redis-cli SET "swap:SWAP_ID" '{"swap_id":"SWAP_ID","phase":"RolledBack",...}'

# Notify operator
echo "Swap SWAP_ID manually rolled back" | mail -s "Alert" ops@example.com
```

### 2. Registry Rollback (Emergency)

If Redis is corrupted:

1. **Restore from backup**:
   ```bash
   redis-cli BGREWRITEAOF
   # Wait for completion
   redis-cli CONFIG GET appendonly
   ```

2. **Replay from logs**:
   ```bash
   # Extract swap records from validator logs
   grep "Registered swap\|update_phase" logs.txt | \
     awk '{print $NF}' | sort -u > affected_swaps.txt
   ```

3. **Manual verification**:
   ```bash
   # For each swap in affected_swaps.txt:
   redis-cli GET "swap:ID"  # Verify consistency
   ```

### 3. Cascading Failure Recovery

If multiple chains fail simultaneously:

1. **Stop validator**:
   ```bash
   pkill -TERM cross-chain-gpu-validator
   ```

2. **Check RPC endpoints**:
   ```bash
   # Ping both chains
   curl $ETH_RPC -X POST -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'
   curl $SOLANA_RPC -X POST -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","method":"getBlockHeight","params":[],"id":1}'
   ```

3. **Wait for stability** (usually 5-10 minutes):
   ```bash
   # Monitor RPC endpoint health
   while true; do
     echo "$(date): $(curl -s $ETH_RPC ... | jq -r '.result')"
     sleep 5
   done
   ```

4. **Restart validator**:
   ```bash
   RUST_LOG=info ./target/release/cross-chain-gpu-validator
   ```

5. **Monitor recovery**:
   ```bash
   curl http://localhost:8080/metrics | grep -E "total_swaps|rollback"
   ```

## Audit Trail

### Logging Retention

- **Local**: 7 days, rotated daily
- **Archive**: 90 days, compressed

### Compliance

- **SOC 2 Type II**: Audit logs for access and modifications
- **GDPR**: No personal data in logs or swap records
- **Immutability**: Logs are append-only, no deletion

### Review Checklist

- [ ] Check rollback frequency (< 1% is normal)
- [ ] Verify timeout handling works correctly
- [ ] Confirm GPU parity checks pass
- [ ] Review registry consistency
- [ ] Validate RPC endpoint health
- [ ] Check for security log anomalies

## Incident Response

### Major Incident (>10% rollback rate)

**Severity**: P1 (Critical)

**Steps**:
1. Page oncall immediately
2. Stop accepting new swaps (set registry to fail-closed)
3. Investigate RPC endpoint issues
4. Collect diagnostic logs: `curl http://localhost:8080/diagnostic`
5. Engage infrastructure team for RPC support
6. Restart with increased timeout once issue resolved

### Minor Incident (1-10% rollback rate)

**Severity**: P2 (High)

**Steps**:
1. Notify on-call (non-urgent page)
2. Investigate root cause (RPC latency, GPU issues, etc.)
3. Make targeted fix (increase timeout, switch RPC, disable GPU)
4. Monitor metrics for 1 hour
5. Document incident in postmortem

### Non-incident (< 1% rollback rate)

**Severity**: Informational

**Steps**:
1. Log in monitoring system
2. No action required (normal operation)
3. Include in weekly metrics review
