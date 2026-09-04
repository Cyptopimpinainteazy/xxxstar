# X3 Autonomic Control Plane - Deployment Guide 🚀

**Version**: 1.0  
**Target**: Production & Development Environments  
**System**: X3 Chain X3 Cross-Chain GPU Validation Network

---

## Table of Contents
1. [Quick Start](#quick-start)
2. [Architecture Overview](#architecture-overview)
3. [Prerequisites](#prerequisites)
4. [Installation](#installation)
5. [Configuration](#configuration)
6. [Starting the System](#starting-the-system)
7. [Monitoring](#monitoring)
8. [Manual Overrides](#manual-overrides)
9. [Troubleshooting](#troubleshooting)
10. [Production Hardening](#production-hardening)

---

## Quick Start

### 5-Minute Deploy

```bash
# 1. Navigate to project root
cd /home/lojak/Desktop/x3-chain-master

# 2. Verify Python dependencies
pip install -r swarm/requirements.txt

# 3. Check config
cat swarm/config/autonomic_config.json

# 4. Start the swarm API server (includes autonomic control plane)
python3 -m swarm.api_server &

# 5. Wait for startup
sleep 10

# 6. Check health
curl -s http://127.0.0.1:8080/api/autonomic/health | jq

# 7. Open dashboard
firefox swarm/autonomic/dashboard.html
```

**Done!** The autonomic control plane is now monitoring your system.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                  X3 AUTONOMIC CONTROL PLANE                 │
└─────────────────────────────────────────────────────────────┘

Layer 0: TELEMETRY BUS
┌──────────────────────────────────────────────────────────────┐
│  MetricsBus (pub/sub)  │  AuditLog (immutable JSONL)        │
└──────────────────────────────────────────────────────────────┘

Layer 1: SENTINELS (Eyes)
┌──────────────┬──────────────────┬─────────────────────┐
│  GPU Guard   │ Resource Monitor │   Log Watcher       │
│  (nvidia)    │  (CPU/RAM/disk)  │   (dmesg/journal)   │
└──────────────┴──────────────────┴─────────────────────┘
        │                │                    │
        └────────  Publish Metrics  ──────────┘
                         │
                         ▼
Layer 2: HEALTH ENGINE (Brain Stem)
┌──────────────────────────────────────────────────────────────┐
│  Component Scoring  →  System-Wide Score  →  State Machine  │
└──────────────────────────────────────────────────────────────┘
        │                                       │
        │                                CircuitBreakers
        ▼                                       │
Layer 3: ORCHESTRATOR (High Brain)                │
┌──────────────────────────────────────────────────────────────┐
│  Playbook Matching  →  Decision Engine  →  Authorization    │
└──────────────────────────────────────────────────────────────┘
        │
        ▼
Layer 4: OPERATORS (Hands)
┌──────────────┬──────────────┬──────────────┬──────────────┐
│   Service    │     GPU      │   Process    │    Swarm     │
│  (systemd)   │ (nvidia-smi) │  (kill/term) │  (API calls) │
└──────────────┴──────────────┴──────────────┴──────────────┘
        │
        └───────────  Apply Interventions  ─────────────►  SYSTEM

Guardrails (Skeleton):
├─ Rate Limiters (20 actions/hour max)
├─ Cooldowns (30s between same intervention)
├─ Whitelists (services, processes, GPUs)
├─ Safe Mode Filtering (restricted action set)
└─ Human Override Layer (manual control)
```

---

## Prerequisites

### System Requirements
- **OS**: Ubuntu 20.04+ / Debian 11+ (Linux required)
- **Python**: 3.9+
- **GPUs**: NVIDIA GPUs with proprietary driver (optional but recommended)
- **RAM**: 4GB minimum, 8GB+ recommended
- **Disk**: 20GB free space

### Required Packages
```bash
sudo apt-get update
sudo apt-get install -y \
    python3 python3-pip python3-venv \
    curl jq bc \
    nvidia-utils  # if using GPUs
```

### Python Dependencies
```bash
cd /home/lojak/Desktop/x3-chain-master
pip install -r swarm/requirements.txt

# Key dependencies:
# - aiohttp (async HTTP server)
# - asyncio (async framework)
```

### Permissions
Some operators require sudo:
```bash
# Allow service restarts (optional)
sudo visudo
# Add: yourusername ALL=(ALL) NOPASSWD: /usr/bin/systemctl restart x3-chain-*
# Add: yourusername ALL=(ALL) NOPASSWD: /usr/bin/nvidia-smi
```

---

## Installation

### Standard Installation
```bash
# 1. Clone or navigate to project
cd /home/lojak/Desktop/x3-chain-master

# 2. Install dependencies
pip install -r swarm/requirements.txt

# 3. Create log directory
mkdir -p logs/autonomic

# 4. Verify installation
python3 -c "from swarm.autonomic import AutonomicControlPlane; print('✅ OK')"
```

### Docker Installation (TODO)
```bash
# Build autonomic control plane image
docker build -t x3-x3-autonomic -f docker/Dockerfile.autonomic .

# Run container
docker run -d \
  --name x3-autonomic \
  --privileged \
  --network host \
  -v /var/log:/var/log:ro \
  -v /proc:/host/proc:ro \
  -v ./logs:/app/logs \
  x3-x3-autonomic
```

---

## Configuration

### Config File Location
`swarm/config/autonomic_config.json`

### Key Configuration Sections

#### Health Thresholds
```json
{
  "health": {
    "normal_min": 75,      // Below → DEGRADED
    "degraded_min": 60,    // Below → CONTAINMENT
    "containment_min": 40, // Below → SAFE_MODE
    "safe_mode_min": 20    // Below → MANUAL_REQUIRED
  }
}
```

#### GPU Guard
```json
{
  "gpu": {
    "poll_interval_s": 5.0,
    "xid_threshold": 3,       // Max Xid faults in window
    "temp_crit_c": 88,        // Critical temperature
    "vram_crit_pct": 96       // Critical VRAM usage
  }
}
```

#### Circuit Breakers
```json
{
  "circuit_breaker": {
    "failure_threshold": 5,      // Failures before OPEN
    "recovery_timeout_s": 60.0,  // Time before HALF_OPEN
    "half_open_max_calls": 2     // Successes to CLOSE
  }
}
```

#### Intervention Limits
```json
{
  "intervention": {
    "cooldown_s": 30.0,           // Min time between same action
    "max_per_hour": 20            // Rate limit
  }
}
```

### Environment Variables
```bash
# Override config path
export X3_AUTONOMIC_CONFIG=/path/to/custom_config.json

# Set log level
export PYTHONLOGLEVEL=INFO

# Swarm API URL (if not default)
export SWARM_API_URL=http://127.0.0.1:8080
```

---

## Starting the System

### Method 1: Via Swarm API Server (Recommended)
The autonomic control plane starts automatically when you start the swarm API server:

```bash
cd /home/lojak/Desktop/x3-chain-master
python3 -m swarm.api_server
```

Logs will show:
```
[INFO] Autonomic Control Plane initialized
[INFO] GPU Guard started (poll every 5.0s)
[INFO] Resource Monitor started (poll every 10.0s)
[INFO] Log Watcher started (poll every 15.0s)
[INFO] Health Engine started (eval every 5.0s)
[INFO] Orchestrator started (eval every 10.0s, 4 playbooks)
[INFO] Autonomic Control Plane fully operational
```

### Method 2: Standalone Mode
```bash
cd /home/lojak/Desktop/x3-chain-master
python3 -m swarm.autonomic
```

### Method 3: Via run-everything.sh
```bash
cd /home/lojak/Desktop/x3-chain-master
./run-everything.sh
```

### Verify Startup
```bash
# Check health endpoint
curl -s http://127.0.0.1:8080/api/autonomic/health | jq

# Expected output:
# {
#   "score": 95.2,
#   "state": "normal",
#   "ok": true
# }
```

---

## Monitoring

### Dashboard (Visual)
```bash
# Open in browser
firefox swarm/autonomic/dashboard.html
# or
xdg-open swarm/autonomic/dashboard.html
```

**Dashboard Features**:
- Real-time system health score
- Component status (GPU, RAM, disk, logs)
- Circuit breaker states
- Recent interventions
- Audit trail
- Auto-refresh every 5 seconds

### CLI Monitoring
```bash
# Health summary
curl -s http://127.0.0.1:8080/api/autonomic/health | jq

# Full status snapshot
curl -s http://127.0.0.1:8080/api/autonomic/status | jq | less

# Component details
curl -s http://127.0.0.1:8080/api/autonomic/status | jq '.components'

# GPU status
curl -s http://127.0.0.1:8080/api/autonomic/gpu | jq

# System resources
curl -s http://127.0.0.1:8080/api/autonomic/resources | jq

# Circuit breakers
curl -s http://127.0.0.1:8080/api/autonomic/circuit-breakers | jq

# Recent audit entries
curl -s http://127.0.0.1:8080/api/autonomic/audit | jq '.[-20:]'

# State machine history
curl -s http://127.0.0.1:8080/api/autonomic/state | jq
```

### Log Files
```bash
# Main swarm log (includes autonomic messages)
tail -f logs/swarm.log

# Audit trail (JSONL format)
tail -f logs/autonomic/audit.jsonl

# Filter audit by severity
jq 'select(.severity == "error")' < logs/autonomic/audit.jsonl
```

### Watch Script
```bash
# Continuous monitoring
watch -n 5 'curl -s http://127.0.0.1:8080/api/autonomic/health | jq'
```

---

## Manual Overrides

### Force System State
```bash
# Enter Safe Mode manually
curl -X POST http://127.0.0.1:8080/api/autonomic/override/state \
  -H 'Content-Type: application/json' \
  -d '{"state": "safe_mode", "reason": "manual intervention"}'

# Return to Normal
curl -X POST http://127.0.0.1:8080/api/autonomic/override/state \
  -H 'Content-Type: application/json' \
  -d '{"state": "normal", "reason": "manual recovery"}'
```

### Reset Circuit Breaker
```bash
curl -X POST http://127.0.0.1:8080/api/autonomic/override/circuit-breaker \
  -H 'Content-Type: application/json' \
  -d '{"name": "gpu", "action": "reset"}'
```

### Trigger Playbook
```bash
curl -X POST http://127.0.0.1:8080/api/autonomic/override/playbook \
  -H 'Content-Type: application/json' \
  -d '{"name": "gpu_overheat", "reason": "manual test"}'
```

---

## Troubleshooting

### Issue: API Returns 404 for /api/autonomic/*
**Cause**: Routes not registered  
**Fix**:
```bash
# Check swarm/api_server.py has autonomic route registration
grep -A 2 "register_autonomic_routes" swarm/api_server.py
# Should see: register_autonomic_routes(app, self._autonomic)

# Restart API server
pkill -f "python.*api_server"
python3 -m swarm.api_server
```

### Issue: GPU Guard Shows 0 GPUs
**Cause**: nvidia-smi not found or driver issue  
**Fix**:
```bash
# Check nvidia-smi
nvidia-smi

# If missing:
sudo apt-get install nvidia-utils

# If driver issue:
sudo dmesg | grep -i nvidia
# Look for Xid errors
```

### Issue: High False Positive Alerts
**Cause**: Thresholds too aggressive  
**Fix**:
Edit `swarm/config/autonomic_config.json`:
```json
{
  "gpu": {
    "temp_crit_c": 88,  // Increase to 92 if false positives
    "vram_crit_pct": 96 // Increase to 98 if false positives
  }
}
```
Restart swarm API server.

### Issue: Playbooks Not Triggering
**Cause**: Cooldowns or conditions not met  
**Debug**:
```bash
# Check playbook states
curl -s http://127.0.0.1:8080/api/autonomic/status | jq '.orchestrator.playbooks'

# Look for cooldown_remaining > 0
```

### Issue: Audit Log Growing Too Large
**Fix**:
```bash
# Rotate logs
cd logs/autonomic
mv audit.jsonl audit.jsonl.$(date +%Y%m%d)
gzip audit.jsonl.*

# Or configure log rotation
sudo tee /etc/logrotate.d/x3-autonomic <<EOF
/home/lojak/Desktop/x3-chain-master/logs/autonomic/*.jsonl {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
}
EOF
```

---

## Production Hardening

### 1. Persistent Metrics (Recommended)
```bash
# Install Prometheus
sudo apt-get install prometheus

# Configure autonomic to export metrics (TODO)
# Add prometheus_client to swarm/requirements.txt
```

### 2. Systemd Service
Create `/etc/systemd/system/x3-autonomic.service`:
```ini
[Unit]
Description=X3 Autonomic Control Plane
After=network.target

[Service]
Type=simple
User=lojak
WorkingDirectory=/home/lojak/Desktop/x3-chain-master
ExecStart=/usr/bin/python3 -m swarm.api_server
Restart=always
RestartSec=10
StandardOutput=append:/home/lojak/Desktop/x3-chain-master/logs/swarm.log
StandardError=append:/home/lojak/Desktop/x3-chain-master/logs/swarm.log

[Install]
WantedBy=multi-user.target
```

Enable:
```bash
sudo systemctl daemon-reload
sudo systemctl enable x3-autonomic
sudo systemctl start x3-autonomic
sudo systemctl status x3-autonomic
```

### 3. Firewall Rules
```bash
# Allow API access only from localhost
sudo ufw allow from 127.0.0.1 to any port 8080

# Or allow from specific network
sudo ufw allow from 192.168.1.0/24 to any port 8080
```

### 4. TLS Termination (Nginx Reverse Proxy)
```nginx
server {
    listen 443 ssl http2;
    server_name autonomic.x3-chain.local;

    ssl_certificate /etc/ssl/certs/autonomic.crt;
    ssl_certificate_key /etc/ssl/private/autonomic.key;

    location /api/autonomic {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

### 5. Secrets Management
```bash
# Use environment-based secrets
export X3_API_KEY=$(cat /etc/x3/api.key)
export POSTGRES_URL=$(cat /etc/x3/db.url)

# Launch with secrets
python3 -m swarm.api_server
```

---

## Testing

### Chaos Testing
```bash
# Run full chaos test suite
./tests/chaos/autonomic_chaos_test.sh

# Expected: 17 tests, all passing
```

### Manual Smoke Test
```bash
# 1. Check API
curl http://127.0.0.1:8080/api/autonomic/health

# 2. Force state change
curl -X POST http://127.0.0.1:8080/api/autonomic/override/state \
  -d '{"state": "degraded"}' -H 'Content-Type: application/json'

# 3. Check state changed
curl http://127.0.0.1:8080/api/autonomic/health | jq '.state'
# Should show: "degraded"

# 4. Return to normal
curl -X POST http://127.0.0.1:8080/api/autonomic/override/state \
  -d '{"state": "normal"}' -H 'Content-Type: application/json'
```

---

## Support & Feedback

- **GitHub Issues**: Report bugs or feature requests
- **Documentation**: `docs/X3_ENTERPRISE_READINESS.md`
- **Architecture**: `docs/swarm/autonomic/README.md`
- **API Reference**: `swarm/autonomic/api_routes.py`

---

## Quick Reference Card

```
┌────────────────────────────────────────────────────────────┐
│              X3 AUTONOMIC QUICK REFERENCE                  │
├────────────────────────────────────────────────────────────┤
│ Start System:     python3 -m swarm.api_server             │
│ Dashboard:        firefox swarm/autonomic/dashboard.html  │
│ Health Check:     curl localhost:8080/api/autonomic/health│
│ Force Safe Mode:  POST /api/autonomic/override/state      │
│ Reset Breaker:    POST /api/autonomic/override/cb         │
│ View Audit:       tail -f logs/autonomic/audit.jsonl      │
│ Run Chaos Test:   ./tests/chaos/autonomic_chaos_test.sh   │
└────────────────────────────────────────────────────────────┘
```

**Status**: ✅ Production-ready with monitoring
