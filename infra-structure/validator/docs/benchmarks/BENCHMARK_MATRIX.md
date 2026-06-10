# Validator Fallback Benchmark Matrix

## Overview

This benchmark matrix measures validator fallback performance across six
domains. Each domain has standardized metrics, test scenarios, and
pass/fail criteria. Results are used to validate SLA compliance and
identify regression.

---

## Sheet 1: Throughput

### Metrics
| Metric | Unit | Bronze Target | Silver Target | Gold Target | Platinum Target |
|--------|------|---------------|---------------|-------------|-----------------|
| Primary lane TPS | tx/s | 1,000 | 5,000 | 10,000 | 50,000 |
| Shadow lane TPS | tx/s | — | 3,000 | 7,000 | 35,000 |
| Tertiary lane TPS | tx/s | 100 | 500 | 1,000 | 5,000 |
| Peak sustained TPS | tx/s | 1,200 | 6,000 | 12,000 | 60,000 |
| Degraded mode TPS | tx/s | 50 | 200 | 500 | 2,500 |

### Test Scenarios
1. **Steady state** — 10 min at 80% target TPS
2. **Burst** — 30s at 200% target TPS
3. **Lane failover** — measure TPS during primary→shadow transition
4. **Degraded mode** — measure TPS in CPU-only mode

### Pass Criteria
- Steady state: < 5% TPS variance
- Burst: no dropped transactions
- Failover: < 1s throughput dip
- Degraded: ≥ 50% of tertiary lane target

---

## Sheet 2: Latency

### Metrics
| Metric | Unit | Bronze Target | Silver Target | Gold Target | Platinum Target |
|--------|------|---------------|---------------|-------------|-----------------|
| Primary lane P50 | ms | 50 | 20 | 10 | 5 |
| Primary lane P99 | ms | 200 | 100 | 50 | 20 |
| Shadow lane P50 | ms | — | 50 | 25 | 10 |
| Shadow lane P99 | ms | — | 200 | 100 | 50 |
| Tertiary lane P50 | ms | 500 | 200 | 100 | 50 |
| Tertiary lane P99 | ms | 2,000 | 1,000 | 500 | 200 |
| Failover latency | ms | — | 15,000 | 20,000 | 10,000 |

### Test Scenarios
1. **P50/P99 under load** — measure at 80% target TPS
2. **Failover latency** — time from primary failure to standby serving
3. **Tail latency** — measure P99.9 under burst load

### Pass Criteria
- P50: within target
- P99: within 2x target
- Failover: within SLA
- Tail: < 5x P99

---

## Sheet 3: Determinism

### Metrics
| Metric | Unit | Bronze Target | Silver Target | Gold Target | Platinum Target |
|--------|------|---------------|---------------|-------------|-----------------|
| GPU kernel determinism | % | — | 99.9 | 99.99 | 99.999 |
| State hash consistency | % | 100 | 100 | 100 | 100 |
| Replay consistency | % | 100 | 100 | 100 | 100 |
| Cross-lane state match | % | — | 99.9 | 99.99 | 99.999 |

### Test Scenarios
1. **Same input, same output** — run identical workload 100x, verify state
2. **Cross-lane consistency** — compare primary/shadow/tertiary state hashes
3. **Replay test** — replay block history, verify identical results
4. **GPU vs CPU determinism** — compare GPU and CPU execution results

### Pass Criteria
- State hash: 100% across all runs
- Replay: 100% match
- Cross-lane: ≥ 99.9% match
- GPU determinism: ≥ target %

---

## Sheet 4: Failover

### Metrics
| Metric | Unit | Bronze Target | Silver Target | Gold Target | Platinum Target |
|--------|------|---------------|---------------|-------------|-----------------|
| Crash recovery time | s | < 60 | < 15 | < 20 | < 10 |
| Health detection time | s | 10 | 5 | 5 | 3 |
| Promotion time | s | — | < 10 | < 15 | < 7 |
| Split-brain resolution | s | — | — | < 30 | < 15 |
| Max consecutive failures | n | unlimited | 10 | 20 | 50 |

### Test Scenarios
1. **Process crash** — SIGKILL the validator, measure recovery
2. **Health check failure** — corrupt health check endpoint
3. **Network partition** — isolate node from Redis
4. **Split-brain** — simulate two leaders, measure resolution
5. **Cascading failure** — fail primary, then standby, measure tertiary

### Pass Criteria
- Recovery: within SLA
- No double-signing during any failover
- Split-brain: resolved within SLA
- Cascading: tertiary lane always available

---

## Sheet 5: Resource Efficiency

### Metrics
| Metric | Unit | Bronze Target | Silver Target | Gold Target | Platinum Target |
|--------|------|---------------|---------------|-------------|-----------------|
| Primary GPU utilization | % | 70 | 80 | 85 | 90 |
| Standby GPU utilization | % | — | 50 | 60 | 70 |
| Primary memory (RSS) | GB | 8 | 16 | 32 | 64 |
| Standby memory (RSS) | GB | — | 8 | 16 | 32 |
| CPU usage (degraded) | cores | 2 | 4 | 8 | 16 |
| Network bandwidth | Mbps | 100 | 500 | 1,000 | 10,000 |

### Test Scenarios
1. **Idle resource usage** — measure at 0 TPS
2. **Peak resource usage** — measure at 100% target TPS
3. **Standby overhead** — additional resources consumed by standby
4. **Degraded mode efficiency** — CPU-only resource usage

### Pass Criteria
- GPU utilization: ≥ target %
- Standby overhead: < 50% of primary
- Memory: within limits
- CPU: within allocated cores

---

## Sheet 6: Economic Viability

### Metrics
| Metric | Unit | Bronze Target | Silver Target | Gold Target | Platinum Target |
|--------|------|---------------|---------------|-------------|-----------------|
| Cost per tx (primary) | X3 | 0.0001 | 0.00005 | 0.00002 | 0.00001 |
| Cost per tx (degraded) | X3 | 0.001 | 0.0005 | 0.0002 | 0.0001 |
| Standby overhead cost | X3/day | 0 | 0.1 | 0.5 | 2.0 |
| Cluster overhead cost | X3/day | 0 | 0 | 1.0 | 5.0 |
| Break-even TPS | tx/s | 100 | 500 | 1,000 | 5,000 |

### Test Scenarios
1. **Cost per transaction** — total cost / total transactions
2. **Overhead analysis** — additional cost of fallback tiers
3. **Break-even analysis** — TPS needed to justify tier cost
4. **300x cost curve** — verify cost scales sub-linearly with TPS

### Pass Criteria
- Cost per tx: within target
- Overhead: < 10% of primary cost
- Break-even: achievable at moderate TPS
- Cost curve: sub-linear scaling

---

## Sheet 7: 300x Cost Curve Model

### Model Parameters

```
Cost(TPS) = Base_Cost + (TPS × Marginal_Cost)^0.85

Where:
  Base_Cost = Tier fixed cost (X3/day)
  Marginal_Cost = Cost per additional TPS
  0.85 = Scaling exponent (sub-linear)
```

### Tier Parameters
| Tier | Base Cost (X3/day) | Marginal Cost (X3/TPS) | Scaling Exponent |
|------|-------------------|----------------------|------------------|
| Bronze | 0 | 0.0001 | 0.90 |
| Silver | 0.1 | 0.00005 | 0.85 |
| Gold | 1.0 | 0.00002 | 0.80 |
| Platinum | 5.0 | 0.00001 | 0.75 |

### Validation Scenarios
1. **Low TPS (100)** — verify Bronze is cheapest
2. **Medium TPS (1,000)** — verify Silver is cheapest
3. **High TPS (10,000)** — verify Gold is cheapest
4. **Ultra TPS (100,000)** — verify Platinum is cheapest

### Pass Criteria
- Model fits empirical data within 10%
- Tier crossover points match economic analysis
- Sub-linear scaling confirmed (exponent < 1.0)

---

## Running Benchmarks

### Prerequisites
```bash
pip install locust psutil gpustat
```

### Throughput Benchmark
```bash
python tests/benchmarks/run_throughput.py \
    --target-tps 5000 \
    --duration 600 \
    --lanes primary,shadow,tertiary
```

### Latency Benchmark
```bash
python tests/benchmarks/run_latency.py \
    --target-tps 5000 \
    --percentiles 50,99,99.9
```

### Failover Benchmark
```bash
python tests/benchmarks/run_failover.py \
    --scenarios crash,health,partition,split-brain,cascade
```

### Full Benchmark Suite
```bash
python tests/benchmarks/run_all.py \
    --output-dir /var/log/x3-benchmarks/$(date +%Y-%m-%d)
```

## Results Format

Each benchmark run produces a JSON result file:

```json
{
  "benchmark": "throughput",
  "tier": "gold",
  "timestamp": "2026-06-10T00:00:00Z",
  "results": {
    "primary_tps": 10234,
    "shadow_tps": 7123,
    "tertiary_tps": 1056,
    "failover_dip_ms": 450
  },
  "pass": true,
  "violations": []
}
```

## Regression Detection

Benchmark results are compared against the baseline in
`benchmarks/baseline.json`. A regression is flagged if any metric
deviates by more than 10% from baseline.
