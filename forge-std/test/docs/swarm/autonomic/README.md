# X3 Autonomic Control Plane

Self-monitoring, self-healing, self-optimizing control layer for the X3 swarm.

## Architecture

```
┌────────────────────────────────────────────────────┐
│              Orchestrator (Brain)                   │
│  Consumes health scores → matches playbooks →       │
│  dispatches operators → enforces guardrails          │
├────────────────────────────────────────────────────┤
│              Health Engine                          │
│  Aggregates sentinel scores → weighted system       │
│  score → drives state machine transitions           │
├────────────────────────────────────────────────────┤
│   GPU Guard  │  Resource Monitor  │  Log Watcher   │
│   (Sentinel) │    (Sentinel)      │   (Sentinel)   │
│   nvidia-smi │    /proc, statvfs  │   journalctl   │
│   dmesg Xid  │    thermal, FD     │   pattern match │
├────────────────────────────────────────────────────┤
│              Operators (Hands)                      │
│  Service │  GPU  │  Process  │  Swarm              │
│  restart │ power │  kill/sig │  pause/scale/gc     │
│  whitelisted, rate-limited, audited                │
├────────────────────────────────────────────────────┤
│         Cross-cutting Concerns                      │
│  MetricsBus · CircuitBreaker · StateMachine         │
│  AuditLog (JSONL) · SafeMode · Guardrails          │
└────────────────────────────────────────────────────┘
```

## State Machine

```
NORMAL ─── score<75 ──→ DEGRADED ─── score<60 ──→ CONTAINMENT
  ↑                       ↑                           │
  │ score≥75              │ score≥60                   │
  │ (5 consecutive)       │ (5 consecutive)           ↓
  │                       │                      score<40
  │                       │                           │
  │                       └──────────────────── SAFE_MODE
  │                                                   │
  │                                              score<20
  │                                                   ↓
  └──────── manual reset only ───────── MANUAL_REQUIRED
```

## Quick Start

### Standalone
```bash
# With defaults
python -m swarm.autonomic

# With custom config
python -m swarm.autonomic swarm/config/autonomic_config.json
```

### Embedded in Swarm API Server
Automatic — the control plane boots with the swarm API server.
Set `AUTONOMIC_DISABLED=1` to skip.

### API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/autonomic/health` | Compact health (score, state, ok) |
| GET | `/api/autonomic/status` | Full system snapshot |
| GET | `/api/autonomic/gpu` | GPU Guard details |
| GET | `/api/autonomic/resources` | System resources |
| GET | `/api/autonomic/logs` | Log watcher events |
| GET | `/api/autonomic/circuit-breakers` | Circuit breaker states |
| GET | `/api/autonomic/state` | State machine status |
| GET | `/api/autonomic/operators` | Operator status & recent actions |
| GET | `/api/autonomic/health/history` | Health score history |
| GET | `/api/autonomic/metrics` | Raw metrics snapshot |
| GET | `/api/autonomic/audit` | Audit trail |
| POST | `/api/autonomic/override/state` | Force system state |
| POST | `/api/autonomic/override/playbook` | Manually trigger playbook |
| POST | `/api/autonomic/override/circuit-breaker` | Reset a circuit breaker |

### Manual Overrides

```bash
# Force state to normal
curl -X POST http://localhost:8080/api/autonomic/override/state \
  -H 'Content-Type: application/json' \
  -d '{"state": "normal", "reason": "manual reset after maintenance"}'

# Trigger a playbook
curl -X POST http://localhost:8080/api/autonomic/override/playbook \
  -H 'Content-Type: application/json' \
  -d '{"name": "gpu_overheat", "reason": "testing"}'

# Reset a circuit breaker
curl -X POST http://localhost:8080/api/autonomic/override/circuit-breaker \
  -H 'Content-Type: application/json' \
  -d '{"name": "service"}'
```

## Configuration

Edit `swarm/config/autonomic_config.json`. Key settings:

| Setting | Default | Purpose |
|---------|---------|---------|
| `health.normal_min` | 75 | Score threshold for NORMAL state |
| `gpu.temp_crit_c` | 88 | GPU temp (°C) that triggers critical |
| `resource.ram_crit_pct` | 95 | RAM usage % that triggers critical |
| `intervention.max_per_hour` | 20 | Max operator actions per hour |
| `circuit_breaker.failure_threshold` | 5 | Failures before breaker opens |

## Built-in Playbooks

| Name | Trigger | Actions |
|------|---------|---------|
| `gpu_overheat` | GPU score < 40 | Reduce power limit to 100W on all GPUs |
| `ollama_restart` | Swarm score < 50 | Restart ollama_server service |
| `oom_response` | Log score < 40 (OOM) | GC agents + pause job queue |
| `enter_safe_mode` | State = SAFE_MODE | Min power + pause queues |

## File Layout

```
swarm/autonomic/
├── __init__.py           # Package entry, exports AutonomicControlPlane
├── __main__.py           # python -m swarm.autonomic
├── bootstrap.py          # Wires everything together
├── config.py             # Dataclass config schema
├── metrics_bus.py        # In-memory pub/sub telemetry
├── health_engine.py      # Weighted health aggregation
├── orchestrator.py       # Decision engine + playbooks
├── operators.py          # Constrained executors (service, GPU, process, swarm)
├── circuit_breaker.py    # Failure isolation per module
├── state_machine.py      # NORMAL→DEGRADED→...→MANUAL_REQUIRED
├── audit.py              # Immutable JSONL audit trail
├── api_routes.py         # aiohttp route registration
└── sentinels/
    ├── __init__.py
    ├── gpu_guard.py      # NVIDIA GPU monitoring
    ├── resource_monitor.py  # RAM/Disk/CPU/Load/FD
    └── log_watcher.py    # journalctl pattern matching
```

## Design Principles

1. **Observe, don't guess** — Sentinels only read real system state (nvidia-smi, /proc, journalctl)
2. **Score, don't boolean** — Health is 0-100, not up/down
3. **Constrain, don't automate** — Operators have whitelists, rate limits, cooldowns
4. **Escalate, don't loop** — State machine only allows one-step recovery; fast escalation
5. **Audit everything** — Every action gets a JSONL audit entry
6. **Human override always wins** — `force_state()`, `force_playbook()`, `reset_circuit_breaker()`
7. **Safe mode is the floor** — When things are bad, reduce blast radius, don't try heroics

## Dashboard UI

Real-time monitoring dashboard at `swarm/autonomic/dashboard.html`.

### Features
- 🎯 System health score (real-time, color-coded)
- 📊 Component status (GPU, RAM, disk, logs)
- 🔴 Circuit breaker states
- 📜 Recent interventions & playbook triggers
- 🧾 Audit trail (last 10 entries)
- ⚙️ Playbook cooldown timers
- ⚡ Auto-refresh every 5 seconds

### Usage
```bash
# Method 1: Direct file open
firefox swarm/autonomic/dashboard.html

# Method 2: Serve via Python
cd swarm/autonomic
python3 -m http.server 8888 &
firefox http://localhost:8888/dashboard.html
```

No external dependencies - pure HTML/CSS/JS with inline styles.

## Testing

### Chaos Test Suite

Comprehensive integration tests at `tests/chaos/autonomic_chaos_test.sh`.

```bash
# Run full suite (17 tests)
./tests/chaos/autonomic_chaos_test.sh

# Expected output:
# ==========================================
# CHAOS TEST SUMMARY
# ==========================================
# Total Tests:  17
# Passed:       17
# Failed:       0
# Pass Rate:    100%
```

**Tests Cover**:
- API connectivity & health retrieval
- Component health reporting
- State machine transitions (all states)
- Circuit breaker lifecycle (CLOSED→OPEN→HALF_OPEN→CLOSED)
- Sentinel status (GPU, resources, logs)
- Operator registry
- Playbook listing
- Recovery path from SAFE_MODE
- Rapid state changes (stress test)
- Audit trail validation

### Manual Testing

```bash
# Quick smoke test
curl http://127.0.0.1:8080/api/autonomic/health | jq

# Force state transition
curl -X POST http://127.0.0.1:8080/api/autonomic/override/state \
  -d '{"state": "degraded"}' -H 'Content-Type: application/json'

# Verify state changed
curl http://127.0.0.1:8080/api/autonomic/health | jq '.state'
# Should show: "degraded"

# Check audit trail for the manual override
curl http://127.0.0.1:8080/api/autonomic/audit | jq '.[-1]'

# Restore to normal
curl -X POST http://127.0.0.1:8080/api/autonomic/override/state \
  -d '{"state": "normal"}' -H 'Content-Type: application/json'
```

## Production Deployment

See comprehensive guides:
- **Enterprise Readiness Checklist**: [docs/X3_ENTERPRISE_READINESS.md](../../X3_ENTERPRISE_READINESS.md)
- **Deployment Guide**: [docs/X3_AUTONOMIC_DEPLOYMENT.md](../../X3_AUTONOMIC_DEPLOYMENT.md)

Quick deploy:
```bash
# 1. Install dependencies
pip install -r swarm/requirements.txt

# 2. Configure (optional)
$EDITOR swarm/config/autonomic_config.json

# 3. Start swarm server (includes autonomic)
python3 -m swarm.api_server

# 4. Verify
curl http://127.0.0.1:8080/api/autonomic/health

# 5. Open dashboard
firefox swarm/autonomic/dashboard.html
```

**Status**: ✅ Production-ready with monitoring and chaos testing
