# Monitoring and Dashboard Guide

## Operator Dashboard

The operator dashboard provides real-time visibility into validator operations.

### Access

```bash
# Local dashboard
open http://localhost:8080/dashboard

# Remote dashboard
open http://<validator-host>:8080/dashboard
```

## Metrics

### Core Metrics

| Metric | Description | Target |
|--------|-------------|--------|
| TPS | Transactions per second | 2M-4M combined |
| Atomic Success Rate | % of swaps atomically committed | >99% |
| Rollback Count | Number of failed swaps | <1% |
| Timeout Count | Number of timeout swaps | <0.1% |
| GPU Health | GPU functional and accurate | Healthy |
| RPC Latency | Average RPC response time | <100ms |
| Active Swaps | Concurrent in-flight swaps | <10k |

### Dashboard JSON Schema

```json
{
  "total_swaps": 1000,
  "successful_commits": 990,
  "rollbacks": 8,
  "timeouts": 2,
  "total_txs_processed": 50000,
  "avg_tps": 1500.5,
  "peak_tps": 2100.0,
  "gpu_enabled": true,
  "gpu_healthy": true,
  "avg_rpc_latency_ms": 45,
  "snapshots": [
    {
      "timestamp": "2024-01-01T12:00:00Z",
      "tps": 1500.5,
      "atomic_success_rate": 0.99,
      "rollback_count": 8,
      "timeout_count": 2,
      "gpu_health": true,
      "rpc_latency_ms": 45,
      "active_swaps": 125
    }
  ]
}
```

## Observability

### Logging Levels

```bash
# Info (default)
RUST_LOG=info ./validator

# Debug (detailed)
RUST_LOG=debug ./validator

# Trace (very verbose)
RUST_LOG=trace ./validator

# Component-specific
RUST_LOG=cross_chain_gpu_validator::orchestrator=debug ./validator
```

### Key Log Patterns

#### Healthy Operation
```
INFO: Registered swap swap-001 with timeout 60 secs
INFO: EVM validation succeeded for block 1000
INFO: SVM validation succeeded for slot 500
INFO: Swap swap-001 atomically committed
```

#### Rollback
```
WARN: EVM validation failed for swap-001
INFO: Swap swap-001 rolled back: EVM validation failed
```

#### Timeout
```
ERROR: EVM validation timeout for swap swap-001
INFO: Swap swap-001 timed out
```

#### Failover
```
WARN: GPU validation failed: ..., falling back to CPU
INFO: CPU fallback signature verification succeeded
```

## Health Checks

### GPU Health Check

```bash
curl http://localhost:8080/health/gpu
```

Response (healthy):
```json
{
  "status": "healthy",
  "gpu_enabled": true,
  "parity_check_passed": true
}
```

### Registry Health Check

```bash
curl http://localhost:8080/health/registry
```

Response:
```json
{
  "status": "connected",
  "redis_latency_ms": 2,
  "active_swaps": 125
}
```

## Alerting

### Alert Conditions

1. **GPU Unhealthy**
   - Trigger: GPU parity check fails
   - Action: Fallback to CPU, notify ops

2. **Registry Unavailable**
   - Trigger: Redis connection fails
   - Action: Fail-closed, reject new swaps, page oncall

3. **High Rollback Rate**
   - Trigger: Rollback % > 5%
   - Action: Investigate RPC/network issues

4. **High Timeout Rate**
   - Trigger: Timeout % > 1%
   - Action: Increase timeout or check RPC latency

5. **RPC Latency High**
   - Trigger: Latency > 500ms
   - Action: Check RPC endpoint health, consider redundancy

## Performance Benchmarking

### Run Benchmark Suite

```bash
cargo bench --bench gpu_kernels --release
```

### Expected Results

- Secp256k1 batch verification: 100k+ sig/sec
- Keccak256 batch hashing: 1M+ hashes/sec
- End-to-end swap latency: <5s (P99)

### Benchmark Report

```bash
./deployment/run_benchmark.sh 2>&1 | tee benchmark_report.txt
```

Report includes:
- Throughput metrics
- Latency percentiles (P50, P95, P99)
- GPU vs CPU comparison
- Failover performance

## Trend Analysis

### Weekly Review

1. Check average TPS trend (should be stable)
2. Review rollback rate trend (should be <1%)
3. Verify GPU health trend
4. Assess RPC latency trend

### Monthly Review

1. Capacity planning: Are we approaching limits?
2. Cost analysis: GPU vs CPU usage
3. Reliability: MTBF, incident postmortems
4. Optimization opportunities

## Troubleshooting Guide

### Problem: Low TPS

**Diagnosis**:
```bash
# Check if GPU is being used
grep "GPU batch" logs.txt | wc -l

# Check RPC latency
curl http://localhost:8080/metrics | grep rpc_latency
```

**Solution**:
- Increase batch size (GPU bottleneck)
- Add RPC replicas (RPC bottleneck)
- Scale horizontally with load balancer

### Problem: High Rollback Rate

**Diagnosis**:
```bash
# Check which validations are failing
grep "validation failed" logs.txt

# Check timeouts
grep "timeout" logs.txt
```

**Solution**:
- Check EVM/SVM RPC health
- Increase timeout if network latency is high
- Verify network connectivity

### Problem: GPU Failover Triggered

**Diagnosis**:
```bash
# Check GPU error logs
RUST_LOG=debug ./validator 2>&1 | grep -i gpu

# Check CUDA status
nvidia-smi
```

**Solution**:
- Update GPU drivers
- Check for memory pressure
- Verify CUDA compatibility
- Fall back to CPU only if persistent
