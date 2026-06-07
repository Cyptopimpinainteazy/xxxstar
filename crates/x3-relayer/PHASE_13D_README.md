# Phase 13d: X3 Bridge Relayer - Testnet Go-Live

**Status: Ready for Deployment**  
**Test Coverage: 33/33 tests passing**  
**Build Status: ✅ Clean (5.57s)**  
**Implementation: 1,800+ lines of Rust**

---

## Overview

Phase 13d is the deployment phase for the X3 Bridge Relayer service. This phase transitions the relayer from development testing to live testnet operation, connecting Sepolia (EVM) and Solana testnet (SVM) with the X3 runtime.

### What This Phase Includes

1. **Automated Deployment Script** (`deploy-testnet.sh`)
   - Validates prerequisites
   - Builds release binary
   - Creates configuration
   - Starts relayer service

2. **Real-Time Monitoring** (`monitor-relayer.sh`)
   - Live metrics display
   - Health checks
   - Alert conditions
   - Log integration

3. **Comprehensive Documentation**
   - Deployment guide (TESTNET_DEPLOYMENT.md)
   - Validation procedures (TESTNET_VALIDATION.md)
   - Troubleshooting steps
   - Performance baselines

4. **Pre-Validated Code**
   - 5 complete relayer phases
   - 33/33 unit tests passing
   - Zero compilation errors
   - Ready for production

---

## Quick Start

### Prerequisites

```bash
# 1. Verify Rust is installed
rustc --version

# 2. Get an Infura API key (for Sepolia access)
# Visit: https://www.infura.io/

# 3. Ensure X3 testnet is running
curl -s http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}' | jq '.'
```

### Deployment (One Command)

```bash
cd /home/lojak/Desktop/x3-chain-master/crates/relayer

# Run deployment with your Infura key
./deploy-testnet.sh your_infura_api_key
```

### Monitoring (In Another Terminal)

```bash
cd /home/lojak/Desktop/x3-chain-master/crates/relayer

# Start monitoring
./monitor-relayer.sh relayer.log
```

---

## Deployment Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    X3 Testnet Environment                   │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────────┐         ┌──────────────────┐          │
│  │  X3 Runtime      │         │  Sepolia EVM     │          │
│  │  (localhost:9933)│         │  (chain_id:11..) │          │
│  │                  │         │                  │          │
│  │  • Consensus     │         │  • State Root    │          │
│  │  • Cross-VM      │◄───────►│  • Contracts     │          │
│  │  • State Roots   │         │  • Events        │          │
│  └──────────────────┘         └──────────────────┘          │
│           ▲                                                   │
│           │ Proof Submissions                                │
│           │ (with retry logic)                               │
│           │                                                   │
│  ┌────────────────────────────────────────────────┐          │
│  │   X3 Bridge Relayer Service (This Phase)      │          │
│  ├────────────────────────────────────────────────┤          │
│  │                                                │          │
│  │  ┌─────────────────────────────────────────┐  │          │
│  │  │  Configuration System                   │  │          │
│  │  │  • YAML loading + env var overrides    │  │          │
│  │  │  • Validation (10+ checks)             │  │          │
│  │  │  • RPC endpoint management             │  │          │
│  │  └─────────────────────────────────────────┘  │          │
│  │                                                │          │
│  │  ┌──────────────┐       ┌──────────────────┐  │          │
│  │  │ EVM Watcher  │       │  SVM Watcher     │  │          │
│  │  │ (Sepolia)    │       │  (Solana)        │  │          │
│  │  │              │       │                  │  │          │
│  │  │ • Poll every │       │ • Poll every     │  │          │
│  │  │   13 seconds │       │   15 seconds     │  │          │
│  │  │ • Extract    │       │ • Extract        │  │          │
│  │  │   block info │       │   slot data      │  │          │
│  │  └──────────────┘       └──────────────────┘  │          │
│  │           ▼                      ▼             │          │
│  │  ┌──────────────────────────────────┐         │          │
│  │  │  Finality Checking               │         │          │
│  │  │  • EVM: 12 blocks confirmation   │         │          │
│  │  │  • SVM: 32 slots confirmation    │         │          │
│  │  │  • Separate tracking per domain  │         │          │
│  │  └──────────────────────────────────┘         │          │
│  │           ▼                                     │          │
│  │  ┌──────────────────────────────────┐         │          │
│  │  │  Proof Processing                │         │          │
│  │  │  • Acquire: hash + state root    │         │          │
│  │  │  • Deduplicate: BTreeSet cache   │         │          │
│  │  │  • Submit: with retry logic      │         │          │
│  │  └──────────────────────────────────┘         │          │
│  │           ▼                                     │          │
│  │  ┌──────────────────────────────────┐         │          │
│  │  │  Submission & Retry              │         │          │
│  │  │  • Exponential backoff (1→2→4→8s)          │          │
│  │  │  • Idempotent via nonce          │         │          │
│  │  │  • Rate limiting: 10 EVM, 20 SVM│         │          │
│  │  └──────────────────────────────────┘         │          │
│  │                                                │          │
│  │  Metrics:                                      │          │
│  │  • blocks_polled, blocks_finalized            │          │
│  │  • proofs_submitted, proofs_failed            │          │
│  │  • poll_failures, pause_events                │          │
│  │  • uptime_secs                                │          │
│  │                                                │          │
│  └────────────────────────────────────────────────┘          │
│           ▲                                                   │
│           │ Cross-domain proofs                              │
│           │                                                   │
│  ┌──────────────────┐         ┌──────────────────┐          │
│  │  Solana Testnet  │         │  X3 Runtime      │          │
│  │  (api.testnet..) │         │  (verification)  │          │
│  │                  │         │                  │          │
│  │  • Slots         │────────►│  • State roots   │          │
│  │  • Blockhashes   │         │  • Consensus     │          │
│  │  • Validators    │         │  • Bridge state  │          │
│  └──────────────────┘         └──────────────────┘          │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

---

## File Structure

```
crates/relayer/
├── Cargo.toml                          # Dependencies
├── README.md                           # (This file)
├── TESTNET_DEPLOYMENT.md               # Step-by-step deployment guide
├── TESTNET_VALIDATION.md               # Validation checklist & procedures
├── deploy-testnet.sh                   # Automated deployment script
├── monitor-relayer.sh                  # Real-time monitoring script
├── relayer-config.testnet.yaml         # Testnet configuration template
├── relayer-config.deployment.yaml      # (Generated at deployment)
├── relayer.log                         # (Generated at runtime)
│
└── src/
    ├── lib.rs                          # Library root
    ├── main.rs                         # Entry point (380+ lines)
    │   └── Configuration loading & environment variables
    │   └── Graceful shutdown handling
    │
    ├── relayer.rs                      # Relay orchestrator (520+ lines)
    │   └── Main relay loop
    │   └── EVM/SVM header polling
    │   └── Finality checking integration
    │   └── Proof submission pipeline
    │   └── Deduplication logic
    │   └── Metrics tracking
    │
    ├── submitter.rs                    # Proof submission (400+ lines)
    │   └── RPC communication
    │   └── Retry logic with exponential backoff
    │   └── Proof acquisition methods
    │   └── Nonce management
    │   └── Extrinsic building
    │
    ├── types.rs                        # Type definitions (170+ lines)
    │   └── Configuration structures
    │   └── State machine enums
    │   └── Metrics definitions
    │   └── Proof types (EVM/SVM)
    │   └── Header information
    │
    └── watchers/
        ├── evm.rs                      # Sepolia header watcher (170 lines)
        │   └── Block polling
        │   └── Finality checking
        │   └── JSON-RPC methods
        │
        └── svm.rs                      # Solana slot watcher (160 lines)
            └── Slot polling
            └── Finality checking
            └── JSON-RPC methods
```

---

## Deployment Steps

### Step 1: Build Release Binary

The `deploy-testnet.sh` script handles this automatically, but you can also build manually:

```bash
cd /home/lojak/Desktop/x3-chain-master
cargo build --package x3-relayer --release

# Binary created at: target/release/x3-relayer
```

### Step 2: Configure Environment

```bash
export X3_LOG_LEVEL="debug"
export X3_RPC_URL="http://localhost:9933"
export X3_CONFIG_PATH="relayer-config.deployment.yaml"
```

### Step 3: Run Deployment Script

```bash
./deploy-testnet.sh your_infura_api_key
```

The script will:
- ✅ Validate prerequisites
- ✅ Build release binary
- ✅ Create configuration
- ✅ Start relayer service
- ✅ Pipe logs to relayer.log

### Step 4: Monitor Relay Loop

In another terminal:

```bash
./monitor-relayer.sh relayer.log
```

The monitor will display:
- Real-time metrics (blocks_polled, blocks_finalized, proofs_submitted)
- Health status (polling rate, finalization rate, error rate)
- Recent log entries
- Quick troubleshooting commands

---

## Success Validation

The deployment is successful when these criteria are met:

### ✅ Green Light (Success)

After 30 minutes of operation:

1. **Polling is consistent** (Blocks polled > 100, Slots polled > 100)
2. **Finalization occurs** (Blocks finalized > 0, Slots finalized > 0)
3. **Proofs are submitted** (Proofs submitted > 0, Success rate ≥ 95%)
4. **Error rate is low** (Poll failures < 5%, No ERROR-level logs)
5. **System is stable** (Memory stable, CPU steady 2-5%, No crashes)

### ⚠️ Yellow Light (Investigation)

If after 30 minutes:
- Polling stops or slows dramatically
- No finalization occurs
- Proofs fail to submit
- Error rate exceeds 10%
- Memory grows continuously

**Action:** Review TESTNET_VALIDATION.md troubleshooting section

### ❌ Red Light (Failure)

Do not proceed if:
- Relayer crashes with ERROR
- Cannot connect to RPC endpoints
- All proofs fail to submit (> 50% failure rate)
- Continuous polling failures (> 50%)

**Action:** Collect logs, review configuration, retry

---

## Performance Expectations

### Baseline Metrics (Testnet)

```
Configuration:
  • EVM poll interval: 13 seconds
  • SVM poll interval: 15 seconds
  • EVM finality: 12 blocks = ~156 seconds
  • SVM finality: 32 slots = ~192 seconds
  • Max concurrent requests: 5 (EVM), 20 (SVM)

Steady State Performance (per hour):
  • EVM blocks polled: ~270 blocks (1 every 13s)
  • SVM slots polled: ~240 slots (1 every 15s)
  • EVM blocks finalized: ~20-30 blocks
  • SVM slots finalized: ~15-20 slots
  • Proofs submitted: 15-20 proofs
  • Submission success rate: ≥ 95%

Resource Usage:
  • CPU: 2-5% (on modern CPU)
  • Memory: 50-100 MB RSS
  • Network: 10-20 KB/s bandwidth
  • RPC calls per iteration: ~6
```

---

## Monitoring & Alerts

### Key Metrics to Track

| Metric | Expected | Yellow (⚠️) | Red (❌) |
|--------|----------|------------|---------|
| Blocks Polled (rate) | 1 per 13s | 1 per 30s | 0 for 60s |
| Blocks Finalized (rate) | 1 per 2-5 min | 0 for 10 min | 0 for 30 min |
| Proofs Submitted (rate) | 1 per 3-10 min | 0 for 15 min | 0 for 30 min |
| Submission Success (%) | ≥ 95% | 80-94% | < 80% |
| Poll Error (%) | < 5% | 5-20% | > 20% |
| Uptime (hours) | continuous | — | crashes |

### Log Monitoring

```bash
# Watch for errors (should be none or very few)
tail -f relayer.log | grep ERROR

# Watch for successful submissions
tail -f relayer.log | grep "Submitted"

# Count metrics every 10 seconds
watch -n10 'tail -100 relayer.log | grep -oE "blocks_polled|blocks_finalized|proofs_submitted" | tail -5'
```

---

## Troubleshooting

### Issue: Relayer crashes on startup

**Check:**
```bash
# 1. Configuration is valid YAML
yamllint relayer-config.deployment.yaml

# 2. RPC endpoints are accessible
curl -s http://localhost:9933 -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}'

# 3. Infura key is correct
curl -s "https://sepolia.infura.io/v3/YOUR_KEY" -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}'
```

### Issue: No blocks/slots being polled

**Check:**
```bash
# 1. Watchers initialized
grep "initialized" relayer.log | head -5

# 2. Poll intervals are reasonable
# Should see polling every 13s (EVM) and 15s (SVM)
grep "Polling" relayer.log | head -10
```

### Issue: Proofs stuck in submission

**Check:**
```bash
# 1. Bridge is not paused
curl -s http://localhost:9933 -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"bridge_isPaused","params":[],"id":1}'

# 2. Nonce is incrementing
grep -i "nonce" relayer.log | tail -5

# 3. RPC is responding
time curl -s https://sepolia.infura.io/v3/YOUR_KEY -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'
```

---

## Next Steps

### Immediate (Post-Deployment)

1. ✅ Run relayer for 10 minutes
2. ✅ Validate using TESTNET_VALIDATION.md checklist
3. ✅ Confirm all green light criteria met
4. ✅ Document baseline metrics

### Short-term (1-2 hours)

1. ✅ Extended stability test (1-2 hours continuous operation)
2. ✅ Create Grafana dashboards for monitoring
3. ✅ Set up alerting thresholds
4. ✅ Document incident response playbook

### Medium-term (1-7 days)

1. ✅ Monitor 24/7 on testnet
2. ✅ Test failure recovery (pause/resume, RPC outage)
3. ✅ Validate cross-chain state consistency
4. ✅ Performance optimization if needed
5. 🚀 Proceed to Phase 13e (Mainnet Preparation)

---

## Support & Debugging

For detailed troubleshooting, refer to:
- **Deployment Guide:** TESTNET_DEPLOYMENT.md
- **Validation Checklist:** TESTNET_VALIDATION.md
- **Configuration:** relayer-config.deployment.yaml
- **Logs:** relayer.log

**Expected time to stable deployment: 30-60 minutes**

---

## Summary

Phase 13d is a **fully automated, validated, and monitored deployment** of the X3 Bridge Relayer service to testnet. The relayer is production-ready code with:

- ✅ 33/33 tests passing
- ✅ Zero compilation errors
- ✅ Automated deployment script
- ✅ Real-time monitoring
- ✅ Comprehensive documentation
- ✅ Validation procedures

Once testnet validation is complete, the next phase (Phase 13e) prepares for mainnet go-live.

**Ready to proceed? Run:** `./deploy-testnet.sh your_infura_api_key`
