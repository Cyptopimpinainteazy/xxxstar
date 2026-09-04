# Inferstructor 300× Solana Test Suite

**Complete testing infrastructure** for proving that external validators can achieve 300× Solana throughput using Inferstructor's GPU-accelerated cross-chain validator superhighway.

## 🚀 NEW: External Validator Onboarding

**Inferstructor is now open for external validators!** Any chain can plug in and get 300× speed with authenticated API access.

### ⚡ Quick Start for External Validators

```bash
# 1. Start all services
./start_inferstructor.sh

# 2. Register your validator
./register_validator.sh solana your-email@example.com pro

# 3. Start testing
source .env.validator.*
curl -H "X-API-Key: $INFRA_API_KEY" http://localhost:9999/accelerate -d '...'
```

**📚 Complete Guides:**
- **[ONBOARDING_COMPLETE.md](ONBOARDING_COMPLETE.md)** ← Start here for full setup
- **[VALIDATOR_QUICKSTART.md](VALIDATOR_QUICKSTART.md)** ← 3-minute quick start
- **[INTEGRATION_GUIDE.md](INTEGRATION_GUIDE.md)** ← Integration examples

### 🔐 Authentication System

Integrated with JWT-based authentication from [docs/runbooks/getting-started/AUTHENTICATION_SETUP.md](../../../docs/runbooks/getting-started/AUTHENTICATION_SETUP.md):

- **Validator Registry** (port 7001): Register validators, issue API keys
- **API Key Validation**: All requests authenticated via X-API-Key header  
- **SLA Tiers**: Basic (100K TPS), Pro (1M TPS), Enterprise (unlimited)
- **Usage Tracking**: Real-time monitoring per validator
- **JWT Tokens**: 24hr tokens for dashboard access

---

## 🎯 What This Is

This is **production-grade failover testing** for your cross-chain GPU validator plugin. When other chains plug into X3 for speed, they need  to know:

1. **Can you really deliver 300× Solana speed?** (19.5M TPS)
2. **What happens when your nodes fail?**
3. **Can they safely fallback to native validation?**

This suite proves all three - **and now external validators can test it themselves!**

## 📁 Structure

```
inferstructor/
├── configs/
│   ├── primary_lane.yaml      # Primary GPU lane config
│   ├── shadow_lane.yaml       # Hot standby config
│   ├── tertiary_lane.yaml     # Regional failover config
│   ├── toll_booth.yaml        # Access control & SLA config
│   └── orchestrator.yaml      # Failover orchestration config
│
├── 🔐 AUTHENTICATION & REGISTRY
│   ├── validator_registry.py      # NEW: API key management & JWT auth
│   ├── register_validator.sh      # NEW: One-command registration
│   ├── start_inferstructor.sh    # NEW: Start all services
│   ├── stop_inferstructor.sh     # NEW: Stop all services
│
├── 🌉 ACCELERATION SERVICES
│   ├── lane_orchestrator.py       # Lane health monitoring & failover engine
│   ├── tps_bridge.py              # Go ↔ Python bridge (now with auth!)
│   ├── failover_triggers.py       # Controlled failure injection
│   ├── metrics_dashboard.py       # Real-time monitoring dashboard
│
├── 🧪 TESTING
│   ├── tps_inferstructor_adapter.go  # Go adapter for existing TPS tester
│   ├── run_300x_test.sh           # 🚀 Master test harness (ONE COMMAND)
│
├── 📚 DOCUMENTATION
│   ├── ONBOARDING_COMPLETE.md     # 🆕 Complete setup guide for validators
│   ├── VALIDATOR_QUICKSTART.md    # 🆕 3-minute quick start
│   ├── INTEGRATION_GUIDE.md       # 🆕 Integration examples & best practices
│   ├── QUICKREF.md                # Quick reference
│   └── docs/root/README.md                  # You are here
│
└── logs/                          # Service logs
```

## 🚀 Quick Start

### Prerequisites

```bash
# Python dependencies
pip install aiohttp pyyaml prometheus-client psutil

# Go (for TPS adapter)
go version  # Should be 1.20+

# GPU (for actual acceleration)
nvidia-smi  # Check GPU availability
```

### Run Complete Test Suite

```bash
cd cross-chain-gpu-validator/tests/inferstructor

# Full 8-hour test with all phases
./run_300x_test.sh --duration 8h --export-proof --enable-all-phases

# Quick 10-minute test
./run_300x_test.sh --duration 10m

# Specific phase only
./run_300x_test.sh --phase acceleration --duration 1h
```

### Monitor Progress

While test is running:
- **Dashboard:** http://localhost:8080
- **Real-time TPS, success rate, lane status**
- **Failover event log**

## 🏗 Architecture

### The "Superhighway" Model

```
┌─────────────────┐
│ External        │
│ Validators      │
│ (Any Chain)     │
└────────┬────────┘
         │
    ┌────▼─────┐
    │ Toll     │  ← SLA enforcement, traffic shaping
    │ Booth    │
    └────┬─────┘
         │
    ┌────▼──────────────────────────────┐
    │  Inferstructor Superhighway      │
    │                                   │
    │  ┌──────────┐  ┌──────────┐     │
    │  │ Primary  │  │ Shadow   │     │
    │  │ GPU Lane │←→│ GPU Lane │     │
    │  │ (Active) │  │ (Standby)│     │
    │  └──────────┘  └────┬─────┘     │
    │                      │            │
    │               ┌──────▼──────┐    │
    │               │ Tertiary    │    │
    │               │ (CPU Fallback)│  │
    │               └─────────────┘    │
    └───────────────────────────────────┘
              │
         GPU Bridge
    (bypasses CUDA roadblocks)
              │
         ┌────▼────┐
         │ Result  │
         │ (300× ⚡)│
         └─────────┘
```

### Failover Behavior

1. **Primary fails** → Shadow promotes instantly (<3ms)
2. **Shadow fails** → Tertiary activates (degraded CPU mode)
3. **All fail** → External validator uses native fallback

**Critical:** No split-brain. Only ONE lane holds signing authority at any time.

## 🧪 Test Phases

### Phase 1: Baseline Validation
- Measure native Solana/Ethereum TPS
- Establish control metrics

### Phase 2: GPU Acceleration
- Ramp to 19.5M TPS (300× Solana)
- Sustain for test duration
- Verify deterministic results

### Phase 3: Failover Testing
```bash
# Trigger failures during load:
python3 failover_triggers.py --trigger kill_primary_gpu
python3 failover_triggers.py --trigger cascade_failure
python3 failover_triggers.py --trigger inject_latency_spike
```

### Phase 4: Metrics Collection
- TPS time-series
- Latency distribution (p50, p95, p99)
- Failover events
- GPU utilization

### Phase 5: Proof Generation
- Export all metrics
- Generate reproducible proof document
- Archive results

## 🔧 Configuration

### Lane Configuration

Each lane (primary, shadow, tertiary) has its own config:

```yaml
# configs/primary_lane.yaml
lane:
  id: primary
  role: primary
  priority: 1
  
hardware:
  gpus:
    count: 4
    model: "NVIDIA A100 80GB"
    
performance:
  target_tps: 20_000_000
  max_latency_ms: 1.0
```

### SLA Tiers (Toll Booth)

```yaml
# configs/toll_booth.yaml
sla_tiers:
  basic:
    max_tps: 100_000
    cost_per_million_tx: 10  # USD
    
  pro:
    max_tps: 1_000_000
    cost_per_million_tx: 50
    
  enterprise:
    max_tps: unlimited
    cost_per_million_tx: 200
```

## 📊 Metrics

### Success Criteria

| Metric | Threshold | Priority |
|--------|-----------|----------|
| **TPS Sustained** | ≥19.5M | P0 |
| **Response Latency** | <1ms (p99) | P0 |
| **Hash Correctness** | 100% | P0 |
| **Failover Recovery** | <3ms | P1 |
| **Success Rate** | ≥99.9% | P1 |

### Monitoring

#### Prometheus Endpoints
- Orchestrator: `http://localhost:8000/metrics`
- TPS Bridge: `http://localhost:8002/metrics`
- Primary Lane: `http://10.0.1.10:9091/metrics`
- Shadow Lane: `http://10.0.2.10:9092/metrics`
- Tertiary Lane: `http://10.1.1.10:9093/metrics`

#### Dashboard
- Real-time: `http://localhost:8080`
- Stats API: `http://localhost:8080/api/current`
- History: `http://localhost:8080/api/history?count=1000`

## 🧬 Integration with Existing TPS Tester

The **Go TPS adapter** bridges your existing `Blockchain-TPS-Test-GO` tool with the Inferstructor lanes:

```bash
cd "TPS TESTING/inferstructor"

# Build adapter
go build -o tps_inferstructor_adapter tps_inferstructor_adapter.go

# Run with custom settings
./tps_inferstructor_adapter \
  --target-tps 19500000 \
  --duration 600 \
  --bridge http://localhost:9999 \
  --batch-size 1000 \
  --workers 1000
```

### Why It Works

Your existing TPS code → Go adapter → Python bridge → Toll booth → GPU lanes

- **No rewrites needed** - uses your existing TPS testing logic
- **Batch optimization** - 1000 tx/batch for efficiency
- **Parallel workers** - 1000 concurrent goroutines

## ⚠️ Failover Triggers

Controlled failure injection for testing:

```bash
# GPU crash
python3 failover_triggers.py --trigger kill_primary_gpu

# Node failure
python3 failover_triggers.py --trigger kill_primary_node

# Network issues
python3 failover_triggers.py --trigger inject_latency_spike --duration 60 --intensity 0.8

# Split-brain prevention test
python3 failover_triggers.py --trigger partition_primary_shadow

# Worst case: cascade failure
python3 failover_triggers.py --trigger cascade_failure
```

### Available Triggers

- `kill_primary_gpu` - Simulate GPU crash
- `kill_primary_node` - Kill entire node
- `kill_shadow_lane` - Kill shadow for cascade test
- `inject_latency_spike` - Network degradation
- `fill_gpu_memory` - VRAM exhaustion
- `partition_primary_shadow` - Network partition
- `cascade_failure` - Kill primary + shadow simultaneously
- `corrupt_state` - State corruption test
- `exhaust_cpu` - CPU overload
- `network_disconnect` - Full network loss

## 📈 Expected Results

### Baseline Comparison

| Chain | Native TPS | Inferstructor TPS | Speedup |
|-------|------------|-------------------|---------|
| Solana | 65,000 | 19,500,000 | **300×** ✅ |
| Arbitrum | 4,000 | 1,200,000 | 300× |
| zkSync | 2,000 | 600,000 | 300× |

### Failover Performance

| Scenario | Recovery Time | State Correctness | Validator Impact |
|----------|---------------|-------------------|------------------|
| GPU crash | <3ms | ✓ | None (transparent) |
| Node crash | <3ms | ✓ | None |
| Region outage | <50ms | ✓ | Degraded mode |
| Cascade failure | <100ms | ✓ | CPU fallback |

## 🔒 Safety Guarantees

### For External Validators

1. **Always have native fallback** - If Inferstructor fails, validator continues natively
2. **Deterministic results** - Every acceleration includes hash verification
3. **Timeout protection** - 50ms timeout → automatic fallback
4. **No consensus dependency** - You are never forced to use acceleration

### For X3

1. **No split-brain** - Only one lane signs at a time
2. **Distributed signer lock** - etcd-based coordination
3. **State verification** - Hash correctness checked per request
4. **Graceful degradation** - GPU → CPU → external native

## 🚦 Usage Patterns

### Development Testing
```bash
# Quick smoke test (1 minute)
./run_300x_test.sh --duration 1m

# Watch dashboard
open http://localhost:8080
```

### CI/CD Integration
```bash
# Automated test (10 minutes, no failover)
./run_300x_test.sh --phase acceleration --duration 10m --export-proof
```

### Production Validation
```bash
# Full 8-hour soak test with all failure scenarios
./run_300x_test.sh --duration 8h --enable-all-phases --export-proof

# Results in tests/inferstructor/results/<timestamp>/
```

## 📝 Proof Document

After test completion with `--export-proof`:

```
results/<timestamp>/
├── PROOF_OF_300X_SOLANA.md    # Human-readable proof
├── final_stats.json            # Complete statistics
├── metrics_history.json        # Time-series data
├── failover_events.json        # Failover log
├── tps_load.log                # Load generator output
├── orchestrator.log            # Orchestrator events
└── tps_bridge.log              # Bridge metrics
```

Share this directory for **reproducible proof** of 300× speed.

## 🐛 Troubleshooting

### Bridge won't start
```bash
# Check if port 9999 is available
lsof -i :9999

# Check Python dependencies
pip install -r requirements.txt
```

### GPU not detected
```bash
# Verify CUDA
nvidia-smi

# Check lane config
cat configs/primary_lane.yaml | grep gpu
```

### TPS not reaching target
- Check GPU utilization: `nvidia-smi -l 1`
- Verify network latency: `ping 10.0.1.10`
- Check batch size: increase `--batch-size`
- Increase workers: `--workers 2000`

### Failover not triggering
```bash
# Check orchestrator logs
tail -f results/<timestamp>/orchestrator.log

# Verify health endpoints
curl http://localhost:8000/metrics
```

## 🔗 Related Documentation

- **Full Test Plan:** [docs/INFERSTRUCTOR_300X_TEST_PLAN.md](../../docs/INFERSTRUCTOR_300X_TEST_PLAN.md)
- **Root Repo README:** [../../../docs/root/README.md](../../../docs/root/README.md)
- **GPU Validator README:** [../../docs/root/README.md](../../docs/root/README.md)
- **TPS Testing (Go):** [../../../docs/TPS TESTING/README.md](../../../TPS%20TESTING/docs/root/README.md)

## 💡 Key Insights

### Why 3 Lanes?

- **1 lane** = single point of failure
- **2 lanes** = risk of split-brain
- **3 lanes** = quorum logic, Byzantine-safe

### Why Go + Python?

- **Go** = existing TPS tester (high throughput)
- **Python** = lane orchestration (async/await, easy GPU libs)
- **Bridge** = best of both worlds

### Why Toll Booth?

- **SLA enforcement** = monetization layer
- **Traffic shaping** = prevent overload
- **Metering** = usage tracking
- **Access control** = security

### Why External Fallback?

- **Decentralization** - X3 is optional, not required
- **Safety** - validators never dependent on single provider
- **Optics** - avoids centralization criticism

## 🚀 Next Steps

After proving 300× speed:

1. **Multi-region deployment** - 3 clusters globally
2. **Chain-specific optimizations** - Solana vs EVM tuning
3. **Advanced features:**
   - Threshold signing (2-of-3 consensus)
   - Adaptive lane allocation
   - Cross-chain atomic acceleration
4. **Production launch** - Public testnet integration

## 📞 Support

- **Issues:** File at `cross-chain-gpu-validator/issues/`
- **Questions:** See [docs/](../../docs/)
- **Logs:** Check `results/<timestamp>/` directory

---

**Remember:** This is a **superhighway**, not a roadblock. External validators can always take the native road. You're just offering them a 300× faster route. 🏎️💨
