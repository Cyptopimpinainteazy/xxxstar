# GPU Swarm Operational Runbooks

This document provides step-by-step procedures for responding to alerts and common operational issues.

---

## Table of Contents

1. [Availability Alerts](#availability-alerts)
2. [Performance Alerts](#performance-alerts)
3. [Resource Alerts](#resource-alerts)
4. [Network Alerts](#network-alerts)
5. [Economic Alerts](#economic-alerts)
6. [X3 Execution Alerts](#x3-execution-alerts)
7. [System Alerts](#system-alerts)

---

## Availability Alerts

### CoordinatorQuorumLoss

**Severity**: CRITICAL  
**Impact**: Complete system unavailability - no new tasks accepted  
**Threshold**: < 2 of 3 coordinator replicas healthy

**Immediate Actions (0-5 minutes)**:

```bash
# 1. Assess coordinator health
kubectl -n gpu-swarm get pods -l app=swarm-coordinator
kubectl -n gpu-swarm get pods -l app=swarm-coordinator -o wide

# Expected output:
# NAME                    READY   STATUS    RESTARTS
# swarm-coordinator-0     1/1     Running   0
# swarm-coordinator-1     1/1     Running   0
# swarm-coordinator-2     1/1     Running   2  ← Problem here

# 2. Check pod logs for errors
kubectl -n gpu-swarm logs swarm-coordinator-2 --tail=50
kubectl -n gpu-swarm logs swarm-coordinator-2 --previous  # If crashed

# 3. Determine root cause
# Look for:
# - "permission denied" → RBAC issue
# - "connection refused" → network/DNS issue
# - "out of memory" → resource limits too low
# - "disk full" → PVC storage issue
```

**Diagnosis Matrix**:

| Symptom | Cause | Fix |
|---------|-------|-----|
| Pod CrashedLoopBackOff | Code panic | `kubectl logs ... --previous` to see error |
| Pod Pending | Node affinity | Check node selectors, add labels |
| Pod ImagePullBackOff | Image not found | Verify image tag, check registry credentials |
| Pod FailedScheduling | Resource request >available | Increase node resources or reduce request |

**Recovery Procedure (5-15 minutes)**:

```bash
# Option 1: Force pod restart (if transient error)
kubectl -n gpu-swarm delete pod swarm-coordinator-2
kubectl -n gpu-swarm rollout status statefulset/swarm-coordinator

# Wait for recovery
sleep 30
kubectl -n gpu-swarm get pods -l app=swarm-coordinator

# Option 2: Recover from persistent volume corruption
# Check PVC status
kubectl -n gpu-swarm get pvc

# If volume is full, clean old files
kubectl -n gpu-swarm exec swarm-coordinator-0 -- df -h
kubectl -n gpu-swarm exec swarm-coordinator-0 -- \
  find /var/lib/swarm -type f -mtime +30 -delete

# Option 3: Full coordinator replacement (if corruption suspected)
kubectl -n gpu-swarm scale statefulset swarm-coordinator --replicas=2
sleep 60  # Wait for existing node to take over
kubectl -n gpu-swarm scale statefulset swarm-coordinator --replicas=3
kubectl -n gpu-swarm rollout status statefulset/swarm-coordinator
```

**Verification**:

```bash
# Verify quorum is restored
curl http://coordinator:9000/metrics | grep coordinator_quorum_size

# Expected output:
# coordinator_quorum_size 3
# coordinator_quorum_healthy 3

# Verify tasks can be submitted
curl -X POST http://coordinator:9000/submit_task \
  -d '{"code": "test", "backend": "cuda"}'

# Should return task_id (not error)
```

**Prevention**:

```yaml
# Set resource requests high enough
resources:
  requests:
    cpu: 2
    memory: 4Gi
  limits:
    cpu: 4
    memory: 8Gi

# Enable pod disruption budgets
kubectl apply -f - << EOF
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: swarm-coordinator-pdb
  namespace: gpu-swarm
spec:
  minAvailable: 2
  selector:
    matchLabels:
      app: swarm-coordinator
EOF

# Set up persistent volume replica
# Use StorageClass with replication enabled
replication.storage.cnpg.io/enabled: "true"
```

---

### GPUNodeDown

**Severity**: WARNING (OK if temporary), CRITICAL if >30% down  
**Impact**: Reduced capacity, tasks may timeout  
**Threshold**: Up status == 0 for 5 minutes

**Immediate Actions**:

```bash
# 1. Identify which node is down
curl http://prometheus:9090/api/v1/query?query=up{job="gpu-node"}

# 2. SSH to node and check status
ssh gpu-node-2.example.com

# Check GPU status
nvidia-smi
nvidia-smi --query-gpu=index,name,driver_version,memory.total \
  --format=csv

# 3. Check network connectivity
ping -c 3 coordinator.gpu-swarm.svc.cluster.local
curl -v http://coordinator:9000/health

# 4. Check agent process
ps aux | grep gpu-node
systemctl status gpu-swarm-node  # or journalctl
```

**Common Causes & Fixes**:

```bash
# Cause 1: GPU driver crashed
# Fix: Reinstall driver
sudo apt-get install --reinstall nvidia-driver-525
sudo reboot

# Cause 2: Out of memory on host
# Fix: Free up RAM
free -h
ps aux --sort=-%mem | head -20
kill -9 <problematic_pid>

# Cause 3: Network connectivity lost
# Fix: Check networking
ip route show
systemctl restart networking
ping 8.8.8.8

# Cause 4: Agent service stopped
# Fix: Restart service
sudo systemctl start gpu-swarm-node
sudo systemctl enable gpu-swarm-node
journalctl -u gpu-swarm-node -f
```

**Remote Recovery**:

```bash
# If node is in K8s DaemonSet
kubectl -n gpu-swarm delete pod <gpu-node-pod>
# K8s will reschedule automatically

# Monitor recovery
kubectl -n gpu-swarm get pods -w

# If pod gets stuck, drain node and uncordon
kubectl drain gpu-node-2 --ignore-daemonsets
kubectl uncordon gpu-node-2
```

---

## Performance Alerts

### HighTaskExecutionLatency

**Severity**: WARNING  
**Impact**: Users see slow task results  
**Threshold**: P99 latency > 5 seconds for 10 minutes

**Investigation**:

```bash
# 1. Check if system is CPU or GPU bound
kubectl -n gpu-swarm top pods
# High CPU → Need more cores
# High GPU utilization → Contention

# 2. Analyze latency percentiles
curl 'http://prometheus:9090/api/v1/query?query=histogram_quantile(0.99, rate(gpu_execution_time_seconds_bucket[5m]))'

# Get full distribution
curl 'http://prometheus:9090/api/v1/query?query=gpu_execution_time_seconds{quantile=~"0.5|0.95|0.99"}'

# 3. Check for specific slow queries
curl 'http://prometheus:9090/api/v1/query?query=gpu_execution_time_seconds_count{job="gpu-node"}' \
  | jq -r '.data.result[] | select(.value[1] | tonumber > 5000) | .metric'

# 4. Look at Kibana logs for errors
# Navigate to Kibana → Logs
# Query: level: ERROR AND component: task_executor
# Find patterns in errors
```

**Root Cause Analysis**:

```
If latency spike correlates with:
├─ High GPU utilization (>90%)
│  └─ Cause: GPU queue congestion
│     └─ Fix: Add more GPU nodes OR reduce priority of background tasks
│
├─ High memory fragmentation (>50%)
│  └─ Cause: Memory pool fragmentation
│     └─ Fix: Trigger defragmentation OR restart node
│
├─ High network latency (>100ms)
│  └─ Cause: Network congestion
│     └─ Fix: Enable packet compression OR increase network bandwidth
│
└─ Specific service slow (e.g., X3 compiler)
   └─ Cause: Service overloaded
   └─ Fix: Scale service horizontally OR optimize algorithm
```

**Remediation**:

```bash
# Option 1: Scale GPU nodes (temporary)
kubectl -n gpu-swarm scale daemonset/swarm-gpu-node --replicas=<new_count>

# Option 2: Force memory defragmentation
kubectl -n gpu-swarm exec <gpu-node-pod> -- \
  swarm-cli gpu defragment --aggressive

# Option 3: Increase resource limits
kubectl -n gpu-swarm set resources daemonset/swarm-gpu-node \
  --limits=cpu=8,memory=16Gi

# Option 4: Enable network compression
kubectl -n gpu-swarm set env statefulset/swarm-coordinator \
  NETWORK_COMPRESSION=gzip
```

---

### HighTaskFailureRate

**Severity**: CRITICAL  
**Impact**: User tasks failing, revenue loss  
**Threshold**: > 5% failure rate for 5 minutes

**Immediate Investigation**:

```bash
# 1. Get real-time failure rate
curl 'http://prometheus:9090/api/v1/query?query=rate(gpu_tasks_failed_total[5m]) / rate(gpu_tasks_total[5m])'

# 2. Categorize failures
curl 'http://prometheus:9090/api/v1/query?query=increase(gpu_tasks_failed_total[5m]) by (reason)'

# Expected output:
# {reason="out_of_memory"}: 45
# {reason="timeout"}: 23
# {reason="verification_failed"}: 12

# 3. Check Elasticsearch for detailed error logs
curl -X POST "localhost:9200/swarm-logs/_search" \
  -H 'Content-Type: application/json' \
  -d '{
    "query": {
      "bool": {
        "must": [
          {"match": {"level": "ERROR"}},
          {"match": {"event": "task_failed"}}
        ]
      }
    },
    "aggs": {
      "failure_reasons": {
        "terms": {"field": "reason", "size": 20}
      }
    }
  }'
```

**Failure Type Mitigation Matrix**:

| Reason | Fix | Priority |
|--------|-----|----------|
| out_of_memory | Increase memory pool OR scale nodes | P0 |
| timeout | Increase timeout OR optimize task | P1 |
| compilation_error | Check X3 compiler logs | P1 |
| network_error | Check network connectivity | P0 |
| node_timeout | Check GPU node health | P0 |
| verification_failed | Audit jury system | P1 |

**Troubleshooting**:

```bash
# For out_of_memory failures
# 1. Check GPU memory availability
kubectl -n gpu-swarm exec <gpu-node-pod> -- nvidia-smi
# Look for: Used / Total memory

# 2. Check memory fragmentation
kubectl -n gpu-swarm exec <gpu-node-pod> -- \
  curl http://localhost:9000/metrics | grep fragmentation

# 3. Trigger defragmentation
kubectl -n gpu-swarm exec <gpu-node-pod> -- \
  swarm-cli gpu defragment

# For timeout failures
# 1. Check task execution time
# Navigate to Grafana → GPU Swarm Overview
# Look for tasks taking >timeout

# 2. Check if X3 compilation is slow
curl 'http://prometheus:9090/api/v1/query?query=rate(gpu_x3_compilation_time_seconds[5m])'

# 3. Increase timeout (in coordinator config)
kubectl -n gpu-swarm set env statefulset/swarm-coordinator \
  TASK_TIMEOUT=600s  # Increase from 300s to 600s
```

---

## Resource Alerts

### HighGPUMemoryUtilization

**Severity**: WARNING (at 90%), CRITICAL (at 95%)  
**Impact**: OOM errors, task failures  
**Threshold**: > 90% for 10 minutes

**Diagnosis**:

```bash
# 1. Check memory allocation breakdown
kubectl -n gpu-swarm exec <gpu-node-pod> -- \
  curl http://localhost:9000/memory_stats

# Expected response:
# {
#   "total_memory_mb": 40960,
#   "allocated_mb": 38400,
#   "fragmented_mb": 1200,
#   "free_mb": 360,
#   "allocations": [
#     {"task_id": "task-1", "size_mb": 8192},
#     {"task_id": "task-2", "size_mb": 12288}
#   ]
# }

# 2. Identify memory hogs
kubectl -n gpu-swarm exec <gpu-node-pod> -- \
  ps aux --sort=-%mem | head -10

# 3. Check GPU memory directly
nvidia-smi --query-compute-apps=pid,used_memory --format=csv
```

**Immediate Mitigation**:

```bash
# Option 1: Kill individual large tasks
kill -9 <pid>

# Option 2: Trigger aggressive memory defragmentation
kubectl -n gpu-swarm exec <gpu-node-pod> -- \
  swarm-cli gpu defragment --aggressive

# Option 3: Temporarily disable low-priority tasks
kubectl -n gpu-swarm set env statefulset/swarm-coordinator \
  MIN_TASK_PRIORITY=50  # Only accept priority >= 50

# Option 4: Drain node and force reschedule
kubectl -n gpu-swarm cordon <gpu-node>
# Wait for active tasks to complete
kubectl -n gpu-swarm uncordon <gpu-node>
```

**Long-term Fixes**:

```bash
# Add GPU memory pooling limits
# In GPU node config:
MEMORY_POOL_SIZE=35GB  # Leave 5GB buffer

# Implement memory pressure monitoring
curl 'http://prometheus:9090/api/v1/query?query=gpu_memory_utilization > 0.8'
# → Auto-scale or kill low-priority tasks

# Optimize memory allocation
# Review task memory requirements
# Consider task size hints from users
```

---

### HighGPUMemoryFragmentation

**Severity**: WARNING  
**Impact**: Reduced effective capacity, allocation failures  
**Threshold**: > 50% fragmented for 10 minutes

**Analysis**:

```bash
# 1. View fragmentation breakdown
curl 'http://prometheus:9090/api/v1/query?query=gpu_memory_fragmentation_ratio'

# 2. Analyze block allocation pattern
kubectl -n gpu-swarm exec <gpu-node-pod> -- \
  curl http://localhost:9000/memory_blocks

# Response:
# {
#   "blocks": [
#     {"id": 1, "size_mb": 8192, "allocated": true, "task_id": "task-1"},
#     {"id": 2, "size_mb": 256, "allocated": false},  ← Fragment
#     {"id": 3, "size_mb": 12288, "allocated": true, "task_id": "task-2"},
#     ...
#   ]
# }

# 3. Correlate with allocation patterns
# High fragmentation typically happens with:
# - Many small tasks followed by few large tasks
# - Long-running background task not releasing memory
```

**Defragmentation Procedure**:

```bash
# Method 1: Automatic defragmentation
# Already configured to trigger at >30% fragmentation
# Monitor in logs:
kubectl -n gpu-swarm logs <gpu-node-pod> | grep -i defrag

# Method 2: Manual defragmentation
kubectl -n gpu-swarm exec <gpu-node-pod> -- \
  curl -X POST http://localhost:9000/defragment

# Monitor progress
kubectl -n gpu-swarm exec <gpu-node-pod> -- \
  watch -n 1 'curl -s http://localhost:9000/memory_stats | grep fragmented_mb'

# Method 3: Node restart (last resort)
# Saves & restores all in-flight tasks
kubectl -n gpu-swarm delete pod <gpu-node-pod>
# K8s will restart it with clean memory state
```

---

## Network Alerts

### HighNetworkLatency

**Severity**: WARNING  
**Impact**: Slow task propagation, verification delays  
**Threshold**: P95 latency > 100ms for 5 minutes

**Network Diagnostics**:

```bash
# 1. Measure peer-to-peer latency
kubectl -n gpu-swarm exec <gpu-node-pod> -- \
  curl http://localhost:9000/peer_latencies

# Response:
# {
#   "peers": [
#     {"node_id": "gpu-1", "latency_ms": 5},
#     {"node_id": "gpu-2", "latency_ms": 45},  ← High
#     {"node_id": "gpu-3", "latency_ms": 120}  ← Very high
#   ]
# }

# 2. Check network path
traceroute gpu-3.example.com

# 3. Monitor bandwidth
iftop -i eth0  # Real-time bandwidth usage
# Or:
kubectl top nodes

# 4. Check for dropped packets
netstat -i
# Look for RX-ERR, TX-ERR columns
```

**Root Cause Matrix**:

| Pattern | Cause | Fix |
|---------|-------|-----|
| Latency to single node | Node network issue | SSH to node, check NIC |
| Latency to all nodes | Network path issue | Check switches, routing |
| Latency spike periodic | Network congestion | Shape traffic, add bandwidth |
| Latency spike correlated with tasks | CPU saturation | Reduce workers per node |

**Remediation**:

```bash
# For network congestion
# 1. Enable compression
kubectl -n gpu-swarm set env statefulset/swarm-coordinator \
  NETWORK_COMPRESSION=gzip

# Measure improvement
# Wait 5 minutes for metric update
curl 'http://prometheus:9090/api/v1/query?query=gpu_network_latency_seconds'

# 2. Reduce message batch frequency
kubectl -n gpu-swarm set env daemonset/swarm-gpu-node \
  BATCH_TIMEOUT=200ms  # From 100ms

# 3. For severe congestion, segment network
# Group nodes into regions, local consensus
```

---

## Economic Alerts

### HighSlashingRate

**Severity**: CRITICAL  
**Impact**: Node reputation loss, potential ejection  
**Threshold**: > 100 tokens/hour

**Incident Response**:

```bash
# 1. Identify which nodes are being slashed
curl 'http://prometheus:9090/api/v1/query?query=increase(gpu_slashing_total[1h]) by (node_id)'

# 2. Review audit logs for evidence
curl -X POST "localhost:9200/swarm-audits/_search" \
  -H 'Content-Type: application/json' \
  -d '{
    "query": {
      "match": {"event": "slashing"}
    },
    "sort": [{"@timestamp": {"order": "desc"}}],
    "size": 50
  }' | jq '.hits.hits[].source | {node_id, reason, amount}'

# 3. Analyze root cause
# Typical reasons:
# - "wrong_result": Node executed incorrectly
# - "timeout": Node timed out (infrastructure issue?)
# - "refusal": Node refused to execute (Byzantine?)
```

**Investigation Protocol**:

```bash
# For wrong_result slashing
# 1. Check if issue is reproducible
task_id=<slashed_task_id>
curl "http://coordinator:9000/task/$task_id/results"

# Compare results from multiple execution attempts
# If consistently wrong → potential hardware issue

# For timeout slashing
# 1. Check if node is under resource pressure
kubectl top nodes
kubectl top pods -n gpu-swarm

# If idle → potential network latency
# If busy → node needs scaling

# For refusal slashing
# 1. Check if node was intentionally offline
kubectl -n gpu-swarm logs <node-pod> | tail -50
journalctl -u gpu-swarm-node | tail -50
```

**Corrective Actions**:

```bash
# If hardware fault confirmed
# Flag node for replacement
kubectl -n gpu-swarm label node <gpu-node> maintenance=required

# Cordon the node
kubectl cordon <gpu-node>

# Migrate tasks to other nodes
kubectl drain <gpu-node> --ignore-daemonsets

# Schedule hardware replacement

# If network/infra issue
# Increase timeout for affected node
kubectl -n gpu-swarm set env statefulset/swarm-coordinator \
  NODE_TIMEOUT_<node_id>=600s

# Monitor for improvement
```

---

## X3 Execution Alerts

### X3CompilationFailure

**Severity**: WARNING → CRITICAL (if affecting >10% of tasks)  
**Impact**: X3 tasks cannot execute  
**Threshold**: Compilation failures > 5% of attempts

**Debugging**:

```bash
# 1. Get compilation error details
curl 'http://prometheus:9090/api/v1/query?query=increase(gpu_x3_compilation_failed_total[5m])'

# 2. Check X3 compiler logs
kubectl -n gpu-swarm exec <gpu-node-pod> -- \
  tail -100 /var/log/x3-compiler.log | head -50

# Look for patterns:
# - "syntax_error" → Issue in user code
# - "type_error" → Type mismatch
# - "optimization_error" → Optimization pass failed
# - "out_of_memory" → Compiler ran out of memory

# 3. Reproduce compilation failure locally
x3-cli compile --verbose <task_code>
```

**Root Cause Analysis**:

```bash
# Type 1: User code error
# Query for failing task code
curl -X POST "localhost:9200/swarm-logs/_search" \
  -H 'Content-Type: application/json' \
  -d '{
    "query": {"match": {"event": "compilation_failed"}},
    "_source": ["task_code", "reason"]
  }' | jq '.hits.hits[].source'

# Compare with working tasks to identify syntax differences

# Type 2: X3 compiler bug
# Isolate minimal failing example
# Report to X3 development team

# Type 3: Resource exhaustion
# Check compiler memory usage
kubectl top pods -l app=swarm-gpu-node
# Increase compiler memory limit
```

**Fixes**:

```bash
# For user code errors
# Alert users to check syntax
# Provide helpful error messages

# For X3 compiler bugs
# Downgrade compiler version or upgrade (depending on fix)
kubectl -n gpu-swarm set image statefulset/swarm-coordinator \
  coordinator=x3-chain/gpu-swarm-coordinator:v1.2.3

# For resource issues
# Increase compiler resource limits
kubectl -n gpu-swarm patch configmap swarm-config \
  -p '{"data":{"x3_compiler_memory": "2GB"}}'

# Restart nodes to pick up new config
kubectl -n gpu-swarm delete pods -l app=swarm-gpu-node
```

---

## System Alerts

### ProcessCrashLoop

**Severity**: CRITICAL  
**Impact**: Service unavailable  
**Threshold**: Restarts > 5 in 5 minutes

**Investigation**:

```bash
# 1. Check restart count
kubectl -n gpu-swarm describe pod <pod-name> | grep -A5 "State"

# Output shows:
# State: Waiting (CrashLoopBackOff)
# Reason: BackOff
# Last State: Terminated
# Exit Code: 1
# Reason: OOMKilled

# 2. Get crash logs
kubectl -n gpu-swarm logs <pod-name>
kubectl -n gpu-swarm logs <pod-name> --previous
kubectl -n gpu-swarm logs <pod-name> -c <container-name>

# 3. Check resource exhaustion
kubectl -n gpu-swarm top pod <pod-name>

# 4. Get detailed events
kubectl -n gpu-swarm describe pod <pod-name> | grep -A10 "Events:"
```

**Common Crash Scenarios**:

```bash
# Scenario 1: OOMKilled
# Solution: Increase memory limits
kubectl -n gpu-swarm set resources pod/<pod-name> \
  --limits=memory=8Gi

# Or in StatefulSet:
kubectl -n gpu-swarm patch statefulset swarm-coordinator \
  -p '{
    "spec": {
      "template": {
        "spec": {
          "containers": [{
            "name": "coordinator",
            "resources": {
              "limits": {
                "memory": "8Gi"
              }
            }
          }]
        }
      }
    }
  }'

# Scenario 2: Permission denied
# Solution: Fix RBAC
kubectl -n gpu-swarm get rolebindings
kubectl -n gpu-swarm get roles

# Scenario 3: Failed to pull image
# Solution: Check image registry
kubectl -n gpu-swarm set image statefulset/swarm-coordinator \
  coordinator=x3-chain/gpu-swarm-coordinator:v1.2.3

# Scenario 4: Dependency service down
# Check if upstream services are healthy
kubectl -n gpu-swarm get svc
curl http://elasticsearch:9200/_cluster/health
```

---

## On-Call Escalation

### Escalation Path

```
Severity | Time to Respond | Escalations
---------|-----------------|------------------
P0 (Critical) | 5 min | On-call engineer → Team lead → CTO
P1 (High) | 15 min | On-call engineer → Team lead
P2 (Medium) | 1 hour | Assign to queue
P3 (Low) | Next business day | Backlog
```

### Contact Information

- **Primary On-Call**: Page via PagerDuty
- **Slack Channel**: #gpu-swarm-alerts
- **War Room**: Zoom link in PagerDuty incidents
- **Post-Incident**: Create issue in Jira with label `postmortem`

---

## Reference Commands

### Quick Health Check

```bash
#!/bin/bash
echo "=== Cluster Health ==="
kubectl -n gpu-swarm get pods

echo "=== Coordinator Status ==="
curl http://coordinator:9000/status

echo "=== Metrics Summary ==="
curl 'http://prometheus:9090/api/v1/query?query=up'

echo "=== Recent Errors ==="
kubectl -n gpu-swarm logs -l app=swarm-coordinator \
  --tail=20 | grep -i error
```

### Emergency Shutdown

```bash
# If system misbehavior suspected, safe shutdown:
kubectl -n gpu-swarm scale statefulset/swarm-coordinator --replicas=0
kubectl -n gpu-swarm delete daemonset/swarm-gpu-node
# Wait 5 minutes for in-flight tasks to complete
# Then restart
```

---

**Document Version**: 1.0
**Last Updated**: January 2024
**Next Review**: 30 days
