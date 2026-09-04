# X3 Chain Owner Runbook

## Quick Reference

| Item           | Value                                     |
| -------------- | ----------------------------------------- |
| Rust Toolchain | `nightly-2024-12-01` (pinned)             |
| Substrate Rev  | `948fbd2`                                 |
| Build Command  | `SKIP_WASM_BUILD=1 cargo build --release` |
| Test Command   | `cargo test --all`                        |
| Dev Node       | `./run-dev-node.sh`                       |

---

## Pre-Launch Checklist

### Toolchain & Build
- [ ] Pin toolchain: `rust-toolchain.toml` → `nightly-2024-12-01`
- [ ] Document exact commit hashes for Substrate dependencies
- [ ] Verify reproducible builds with CI container
- [ ] Sign release artifacts with GPG keys
- [ ] Test WASM build: `cargo +nightly build -Z build-std`

### Secrets & Keys
- [ ] Store node keys in HSM or hardware wallet
- [ ] Never commit private keys to repository
- [ ] Set up key rotation schedule
- [ ] Document key recovery procedures
- [ ] Configure TREASURY_ADDRESS in genesis

### Backups
- [ ] Export keystore to secure storage
- [ ] Backup chain DB snapshots regularly
- [ ] Store config files offline
- [ ] Version control `Cargo.lock` and `rust-toolchain.toml`
- [ ] Document restore procedures

### Network & Security
- [ ] Open only required RPC ports (9933, 9944)
- [ ] Restrict admin RPC to localhost
- [ ] Configure firewall rules
- [ ] Enable TLS for public endpoints
- [ ] Set up DDoS protection

---

## Node Operations

### Starting a Development Node
```bash
# Quick start (development mode)
./run-dev-node.sh

# With external access
ENABLE_EXTERNAL=1 ./run-dev-node.sh

# Manual start
./target/release/x3-chain-node \
    --dev \
    --rpc-cors all \
    --rpc-external
```

### Public RPC (dev/local/staging/testnet)

Use the hardened, repeatable bundle in [/deployment/public-rpc/docs/root/README.md](/deployment/public-rpc/docs/root/README.md).


### Checking Node Health
```bash
# RPC health check
curl -s http://127.0.0.1:9933 \
    -H "Content-Type: application/json" \
    -d '{"id":1,"jsonrpc":"2.0","method":"system_health","params":[]}'

# Expected: {"jsonrpc":"2.0","result":{"peers":0,"isSyncing":false,"shouldHavePeers":false},"id":1}
```

### Purging Chain Data
```bash
./target/release/x3-chain-node purge-chain --dev -y
```

---

## Runtime Upgrades

### Pre-Upgrade Checklist
- [ ] Backup current chain state
- [ ] Test upgrade on local devnet
- [ ] Verify runtime version bump
- [ ] Check migration code paths
- [ ] Review governance proposal

### Upgrade Process
1. Build new runtime WASM
2. Submit governance proposal with WASM blob
3. Wait for voting period
4. Execute upgrade after approval
5. Monitor for `panic_impl` type mismatches
6. Verify new runtime version via RPC

### Rollback Procedure
If upgrade fails:
1. Stop all validator nodes
2. Restore from pre-upgrade snapshot
3. Start nodes with previous binary
4. Investigate failure cause

---

## Monitoring & Alerts

### Key Metrics
| Metric              | Warning Threshold | Critical Threshold |
| ------------------- | ----------------- | ------------------ |
| Mempool Size        | > 1000 pending    | > 5000 pending     |
| Block Production    | > 12s gap         | > 30s gap          |
| Atomic Failure Rate | > 5%              | > 15%              |
| Reserve Exhaustion  | > 10 incidents/hr | > 50 incidents/hr  |
| Rollback Rate       | > 1%              | > 5%               |

### Log Monitoring
```bash
# Watch for errors
journalctl -u x3-chain -f | grep -E "(ERROR|WARN|panic)"

# Watch for atomic operations
journalctl -u x3-chain -f | grep -E "Comit"
```

### Health Endpoints
- `/health` - Basic node health
- `/metrics` - Prometheus metrics
- `/system_health` - RPC health method

---

## Governance Configuration

### Fee Distribution (On-Chain Constants)
```rust
FeeDistribution = {
    validators: 60,
    treasury: 20,
    burn: 15,
    kernel: 5
}
```

### Treasury Setup
1. Set `TREASURY_ADDRESS` in genesis config
2. Store private key offline (cold wallet)
3. Configure multisig for treasury operations
4. Document spending procedures

### Parameter Governance
Governable parameters:
- Fee distribution percentages
- Maximum payload sizes (EVM: 16KB, SVM: 16KB, Combined: 32KB)
- Gas/compute limits
- Authorized account lists
- Minimum fee thresholds

---

## Emergency Procedures

### Node Crash Recovery
1. Check logs for crash cause
2. Verify disk space and memory
3. Restart node service
4. Monitor for repeated crashes
5. Escalate if pattern detected

### Consensus Failure
1. Stop affected validator
2. Compare state roots with other validators
3. Identify divergence point
4. Restore from known-good snapshot
5. Resync from canonical chain

### Security Incident Response
1. Isolate affected nodes
2. Preserve logs and evidence
3. Notify security team
4. Assess scope of compromise
5. Execute containment procedures
6. Post-incident review

---

## Production Readiness Checklist

### Minimum Requirements (Do NOT go live without)
- [ ] On-chain governance for critical constants
- [ ] Runtime upgrade protection (timelocks, multisig)
- [ ] Full node backup + cold wallet for treasury
- [ ] **Security audit of pallets (kernel + adapters)**
- [ ] Monitoring & alerts configured
- [ ] Incident response procedures documented
- [ ] Disaster recovery tested

### Optional (Recommended)
- [ ] Bug bounty program (HackerOne/Immunefi)
- [ ] Formal verification of atomic paths
- [ ] Chaos engineering tests
- [ ] Geographic distribution of validators
- [ ] Rate limiting on public RPC

---

## Contact & Escalation

| Role             | Contact                  |
| ---------------- | ------------------------ |
| On-Call Engineer | TBD                      |
| Security Team    | security@x3-chain.io |
| Core Dev Lead    | TBD                      |

---

## References

- [Architecture Document](./ARCHITECTURE.md)
- [Deployment Guide](./DEPLOYMENT.md)
- [Comit Specification](./COMIT_SPEC.md)
- [RPC Integration Guide](./RPC_INTEGRATION_GUIDE.md)
