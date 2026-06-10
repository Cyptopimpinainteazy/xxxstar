# Validator Fallback Packages — Tiered Offering

## Overview

X3 offers four tiers of validator fallback protection. Each tier adds
additional layers of resilience, from basic process supervision to
multi-region cluster failover. External validators always retain native
fallback — X3 is a turbocharger, never a consensus dependency.

---

## Tier 1 — Bronze (Process Supervision)

**Best for:** Solo validators, testnet nodes, low-stake operators.

### Included
- **Watchdog process supervisor** — auto-restart on crash with exponential
  backoff (1s → 2s → 4s → ... → 60s max)
- **Health check** — periodic PID liveness check every 10s
- **Memory limit enforcement** — RSS monitoring via `/proc/PID/status`,
  SIGKILL on limit exceeded
- **PID file management** — systemd integration
- **Restart event logging** — structured log of all restart events

### Architecture
```
Validator Process
      │
      ▼
  Watchdog ──▶ Health Check (10s interval)
      │            │
      │            ▼
      │       Memory Monitor (5s interval)
      │
      ▼
  Restart on crash (exponential backoff)
```

### SLA
- Recovery time: < 60s (max backoff)
- Max restarts: configurable (default unlimited)
- Memory limit: configurable (default none)

### Pricing
- Free for all validators
- Included with Base access tier

---

## Tier 2 — Silver (Hot Standby)

**Best for:** Production validators, medium-stake operators, single-region
deployments.

### Everything in Bronze, plus:
- **Hot standby instance** — secondary validator running in standby mode
- **Automatic failover** — standby promotes to primary on health degradation
- **State sync tracking** — monitors block height lag between primary/standby
- **Signer lock integration** — prevents double-signing during failover
- **Fencing tokens** — stale primitives can't sign after failover

### Architecture
```
Primary (Active)              Standby (Hot)
┌──────────────────────┐      ┌──────────────────────┐
│ GPU acceleration     │      │ GPU-warmed, synced   │
│ Signing authority    │      │ NO signing authority │
│ Serving requests     │      │ Mirroring workload   │
└──────────┬───────────┘      └──────────┬───────────┘
           │                             │
           └─────────── Redis ───────────┘
                  (SignerLock + health state)

On primary failure:
  1. Standby detects health score drop via Redis
  2. Standby acquires SignerLock (fencing token)
  3. Standby promotes to active
  4. Old primary is drained and demoted
```

### Failover Timeline
```
t=0s    Primary fails
t=5s    Standby detects missing heartbeat (first check)
t=10s   Standby confirms primary is dead (second check)
t=10.5s Standby acquires SignerLock
t=11s   Standby restarts validator in primary mode
t=12s   Standby is now primary — serving requests
```

### SLA
- Recovery time: < 15s (typical)
- Max sync lag: 3 blocks before promotion allowed
- Double-sign prevention: guaranteed (fencing tokens)

### Pricing
- 0.1 X3 per day per validator
- Included with Pro access tier

---

## Tier 3 — Gold (Multi-Region Cluster)

**Best for:** Enterprise validators, multi-region deployments, high-stake
operators.

### Everything in Silver, plus:
- **Multi-machine cluster** — up to 7 nodes across regions
- **Raft-style leader election** — quorum-based (51% majority)
- **Automatic leader promotion** — no manual intervention
- **Split-brain detection & resolution** — highest term wins
- **Heartbeat system** — Redis-based, 5s interval, 30s timeout
- **Dynamic peer registration** — nodes can join/leave cluster

### Architecture
```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  Region A    │     │  Region B    │     │  Region C    │
│  (Primary)   │     │  (Warm)      │     │  (Cold)      │
│              │     │              │     │              │
│  GPU Node 1  │◀───▶│  GPU Node 2  │◀───▶│  CPU Node 3  │
│  Signing     │     │  No Signing  │     │  No Signing  │
└──────┬───────┘     └──────┬───────┘     └──────┬───────┘
       │                    │                    │
       └────────────────────┴────────────────────┘
                      Redis
            (SignerLock + ClusterState)

Failover Chain:
  Region A (Primary) → Region B (Warm) → Region C (Cold)
```

### Failover Timeline
```
t=0s    Primary (Region A) fails
t=5s    Followers detect missing heartbeat
t=10s   Election timeout triggers
t=10.5s Candidate requests votes
t=11s   Quorum reached — new leader elected
t=11.5s New leader acquires SignerLock
t=12s   New leader starts serving
```

### SLA
- Recovery time: < 20s (typical)
- Max nodes: 7
- Quorum: 51% majority
- Split-brain resolution: automatic within 30s

### Pricing
- 0.5 X3 per day per validator
- Included with Enterprise access tier

---

## Tier 4 — Platinum (Global Superhighway)

**Best for:** Institutional validators, global deployments, maximum uptime
requirements.

### Everything in Gold, plus:
- **Global multi-region topology** — up to 21 nodes across 7 regions
- **Deterministic GPU kernel execution** — verified by remote attestation
- **300x cost curve model** — predictive scaling based on workload
- **Validator simulator** — pre-deployment validation of fallback behavior
- **24/7 monitoring** — dedicated health dashboard
- **Priority support** — 15-minute response SLA
- **Custom SLA negotiation** — tailored to your requirements

### Architecture
```
Global Superhighway Topology

  NA-East (Primary)    EU-Central (Warm)    AP-Southeast (Cold)
  ┌────────────────┐   ┌────────────────┐   ┌────────────────┐
  │ GPU Cluster    │   │ GPU Cluster    │   │ GPU Cluster    │
  │ 3 nodes        │   │ 3 nodes        │   │ 2 nodes        │
  │ Signing        │   │ No Signing     │   │ No Signing     │
  └───────┬────────┘   └───────┬────────┘   └───────┬────────┘
          │                    │                    │
          └────────────────────┼────────────────────┘
                               │
                         Global Redis
                    (Cross-region consensus)
```

### SLA
- Recovery time: < 10s (typical)
- Max nodes: 21
- Global uptime: 99.999%
- Double-sign prevention: guaranteed (hardware-backed fencing)

### Pricing
- Custom pricing
- Contact sales@x3protocol.io

---

## Comparison Matrix

| Feature                          | Bronze | Silver | Gold   | Platinum |
|----------------------------------|--------|--------|--------|----------|
| Process supervision              | ✓      | ✓      | ✓      | ✓        |
| Health checks                    | ✓      | ✓      | ✓      | ✓        |
| Memory limits                    | ✓      | ✓      | ✓      | ✓        |
| Hot standby                      | —      | ✓      | ✓      | ✓        |
| Automatic failover               | —      | ✓      | ✓      | ✓        |
| Fencing tokens                   | —      | ✓      | ✓      | ✓        |
| Multi-region cluster             | —      | —      | ✓      | ✓        |
| Leader election                  | —      | —      | ✓      | ✓        |
| Split-brain detection            | —      | —      | ✓      | ✓        |
| Global topology                  | —      | —      | —      | ✓        |
| GPU kernel attestation           | —      | —      | —      | ✓        |
| Cost curve modeling              | —      | —      | —      | ✓        |
| Validator simulator              | —      | —      | —      | ✓        |
| 24/7 monitoring                  | —      | —      | —      | ✓        |
| Priority support                 | —      | —      | —      | ✓        |
| **Recovery time**                | <60s   | <15s   | <20s   | <10s     |
| **Max nodes**                    | 1      | 2      | 7      | 21       |
| **Uptime SLA**                   | 99%    | 99.9%  | 99.99% | 99.999%  |

## Quick Start

### Bronze
```bash
python -m cross_chain_gpu_validator.resilience.watchdog \
    --cmd "python -m cross_chain_gpu_validator.cli start" \
    --pid-file /var/run/x3-validator.pid \
    --memory-limit-mb 8192
```

### Silver
```bash
# On primary machine:
python -m cross_chain_gpu_validator.resilience.standby \
    --mode primary --port 9933 --standby-port 9944 \
    --validator-cmd x3-chain-node --dev

# On standby machine:
python -m cross_chain_gpu_validator.resilience.standby \
    --mode standby --port 9944 --primary-port 9933 \
    --validator-cmd x3-chain-node --dev
```

### Gold
```bash
# On each node:
python -m cross_chain_gpu_validator.resilience.cluster \
    --cluster-id x3-mainnet \
    --node-id node-us-east-1 \
    --region us-east-1 \
    --role follower \
    --peers node-us-west-2,node-eu-frankfurt
```

### Platinum
Contact sales@x3protocol.io for custom deployment.
