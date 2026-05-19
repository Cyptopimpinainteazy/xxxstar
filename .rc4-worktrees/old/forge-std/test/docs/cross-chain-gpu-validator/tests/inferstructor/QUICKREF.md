# Inferstructor Quick Reference

## One-Line Test Commands

```bash
# Full test (8 hours)
./run_300x_test.sh --duration 8h --export-proof --enable-all-phases

# Quick test (10 minutes)  
./run_300x_test.sh --duration 10m

# Just acceleration (no failover)
./run_300x_test.sh --phase acceleration --duration 1h

# Custom TPS target
./run_300x_test.sh --target-tps 25000000 --duration 30m
```

## Service Endpoints

| Service | URL | Purpose |
|---------|-----|---------|
| Dashboard | http://localhost:8080 | Real-time monitoring |
| TPS Bridge | http://localhost:9999 | Go ↔ GPU bridge |
| Orchestrator | http://localhost:8000/metrics | Failover control |
| Primary Lane | http://10.0.1.10:9091/metrics | GPU lane metrics |
| Shadow Lane | http://10.0.2.10:9092/metrics | Standby metrics |
| Tertiary Lane | http://10.1.1.10:9093/metrics | Failover metrics |

## Failover Triggers

```bash
# Kill primary GPU
python3 failover_triggers.py --trigger kill_primary_gpu

# Inject 500ms latency for 60 seconds
python3 failover_triggers.py --trigger inject_latency_spike --duration 60 --intensity 0.5

# Cascade failure (primary + shadow)
python3 failover_triggers.py --trigger cascade_failure

# Network partition
python3 failover_triggers.py --trigger partition_primary_shadow --duration 30
```

## Success Metrics

| Metric | Target | Status |
|--------|--------|--------|
| TPS | ≥19,500,000 | 300× Solana |
| Latency (p99) | <1ms | Required |
| Success Rate | ≥99.9% | Required |
| Failover Time | <3ms | Required |
| Hash Correctness | 100% | Required |

## File Locations

```
tests/inferstructor/
├── run_300x_test.sh          ← Master harness
├── lane_orchestrator.py      ← Failover engine
├── tps_bridge.py             ← Go/Python bridge
├── failover_triggers.py      ← Failure injection
├── metrics_dashboard.py      ← Monitoring
├── configs/                  ← Lane configs
└── results/                  ← Test outputs
```

## Checking Results

```bash
# View latest results
ls -lt results/ | head -n 1

# Check final stats
cat results/<timestamp>/final_stats.json | jq

# View TPS graph
cat results/<timestamp>/metrics_history.json | jq -r '.[] | "\(.timestamp) \(.tps)"'

# Failover events
cat results/<timestamp>/failover_events.json | jq
```

## Troubleshooting

```bash
# Check services are running
ps aux | grep -E "lane_orchestrator|tps_bridge|metrics_dashboard"

# Check GPU
nvidia-smi

# Check logs
tail -f results/<timestamp>/orchestrator.log
tail -f results/<timestamp>/tps_bridge.log

# Test bridge health
curl http://localhost:9999/health

# Test dashboard
curl http://localhost:8080/api/current | jq
```

## Architecture Summary

```
External Validators
        ↓
    Toll Booth (SLA/metering)
        ↓
┌───────────────────┐
│ Primary GPU Lane  │ ← Active
│ Shadow GPU Lane   │ ← Hot standby
│ Tertiary CPU Lane │ ← Regional failover
└───────────────────┘
        ↓
  GPU Acceleration
        ↓
   300× Solana Speed
```

## Key Files

| File | Purpose |
|------|---------|
| `primary_lane.yaml` | Main GPU config |
| `shadow_lane.yaml` | Standby config |
| `tertiary_lane.yaml` | Failover config |
| `toll_booth.yaml` | SLA policies |
| `orchestrator.yaml` | Failover rules |
| `docs/INFERSTRUCTOR_300X_TEST_PLAN.md` | Full test spec |

## Common Issues

### Bridge Not Starting
```bash
lsof -i :9999        # Check port
pip install aiohttp pyyaml prometheus-client
```

### Low TPS
```bash
# Increase workers and batch size
./tps_inferstructor_adapter --workers 2000 --batch-size 2000
```

### GPU Not Detected
```bash
nvidia-smi                    # Verify GPU
cat configs/primary_lane.yaml | grep gpu  # Check config
```

### Failover Not Working
```bash
# Check orchestrator
curl http://localhost:8000/metrics | grep lane_health
```

## Production Checklist

- [ ] All 3 lanes deployed
- [ ] GPU kernels precompiled
- [ ] Toll booth configured
- [ ] SLA tiers defined
- [ ] Monitoring enabled
- [ ] Backup lanes tested
- [ ] Split-brain prevention verified
- [ ] External fallback tested
- [ ] 300× speed proven
- [ ] Proof document exported

## Quick Math

```
Solana baseline:  65,000 TPS
Target:          300× faster
Target TPS:      19,500,000 TPS

If you sustain ≥19.5M TPS → ✅ Success
```

## Support

- Docs: `docs/INFERSTRUCTOR_300X_TEST_PLAN.md`
- README: `docs/root/README.md`
- Results: `results/<timestamp>/`
- Logs: `results/<timestamp>/*.log`
