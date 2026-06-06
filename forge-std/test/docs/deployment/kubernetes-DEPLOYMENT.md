# GPU Swarm Deployment Guide

## Quick Start

### Using Kubernetes YAML

```bash
# Create namespace
kubectl create namespace x3-chain

# Deploy swarm
kubectl apply -f deployment/kubernetes/swarm-deployment.yaml

# Check status
kubectl get pods -n x3-chain
kubectl get svc -n x3-chain
```

### Using Helm

```bash
# Add Helm repository (if configured)
# helm repo add x3-chain https://charts.x3-chain.io

# Install swarm
helm install gpu-swarm deployment/helm/gpu-swarm/ \
  -n x3-chain \
  --create-namespace

# Check status
helm status gpu-swarm -n x3-chain

# Upgrade
helm upgrade gpu-swarm deployment/helm/gpu-swarm/ \
  -n x3-chain
```

## Components

### Coordinator
- **StatefulSet**: 3 replicas for high availability
- **Service**: Headless service for peer discovery
- **Persistence**: 10Gi per pod for state
- **Resources**: 2CPU/2Gi requests, 4CPU/4Gi limits

### GPU Nodes
- **DaemonSet**: One node per GPU-enabled machine
- **Node Selection**: Requires `gpu: true` label
- **Resources**: 4CPU/8Gi requests with 1 GPU per node
- **Privileges**: Requires privileged mode for GPU access

## Configuration

### Coordinator Configuration

```toml
[coordinator]
listen_addresses = ["/ip4/0.0.0.0/tcp/9000"]
max_peers = 500
enable_mdns = true

[scheduler]
strategy = "reputationweighted"
max_queue_size = 10000

[verification]
min_verifiers = 2
verification_timeout_secs = 60

[reputation]
initial_score = 100
success_reward = 1
failure_penalty = 5
```

### Node Configuration

```toml
[network]
listen_addresses = ["/ip4/0.0.0.0/tcp/9000"]
max_peers = 100

[gpu]
enable_cuda = true
enable_vulkan = false
max_concurrent_tasks = 6
task_timeout = 300
sandbox_enabled = true

[rewards]
min_stake = 1000
claim_threshold = 100
```

## Monitoring & Observability

### Prometheus Metrics

Both coordinator and nodes expose Prometheus metrics on `:9090/metrics`:

```
swarm_tasks_submitted_total
swarm_tasks_completed_total
swarm_tasks_failed_total
swarm_task_execution_time_seconds
swarm_task_queue_size
gpu_utilization_percent
gpu_memory_used_bytes
gpu_temperature_celsius
swarm_network_peers_connected
swarm_network_peer_latency_ms
swarm_reputation_score
```

### Health Checks

```bash
# Coordinator health
curl http://swarm-coordinator:3000/health

# Coordinator readiness
curl http://swarm-coordinator:3000/ready

# Node metrics
curl http://swarm-node:9090/metrics
```

### Grafana Dashboards

Deploy Grafana and import dashboards from `deployment/grafana/dashboards/`:

```bash
kubectl apply -f deployment/grafana/prometheus-datasource.yaml
kubectl apply -f deployment/grafana/dashboards/
```

## Scaling

### Automatic Scaling

HPA is configured to scale coordinator based on:
- CPU utilization > 70%
- Memory utilization > 80%
- Task queue size > 1000

```bash
kubectl get hpa -n x3-chain
```

### Manual Scaling

```bash
# Scale coordinator
kubectl scale statefulset swarm-coordinator -n x3-chain --replicas=5

# Check node daemonset status
kubectl get daemonset swarm-node -n x3-chain
```

## GPU Support

### NVIDIA GPUs

Requires:
- nvidia-docker
- nvidia-device-plugin on each GPU node

Install device plugin:
```bash
kubectl apply -f https://raw.githubusercontent.com/NVIDIA/k8s-device-plugin/v0.13.0/nvidia-device-plugin.yml
```

### AMD GPUs

Requires ROCm and appropriate drivers

### Auto-Detection

Nodes with GPUs will be auto-labeled based on driver detection.

## Networking

### Port Requirements

| Service | Port | Protocol | Purpose |
|---------|------|----------|---------|
| P2P | 9000 | TCP | Peer communication |
| RPC | 9100 | TCP | JSON-RPC for API |
| Admin | 3000 | TCP | Health checks, admin UI |
| Metrics | 9090 | TCP | Prometheus metrics |

### Service Discovery

P2P nodes discover each other via:
1. **mDNS**: Local cluster discovery
2. **Kademlia DHT**: Global peer discovery
3. **Bootstrap Peers**: Hardcoded peers for cold start

## Security

### RBAC

Only coordinators have permissions to list pods/services.

### Network Policies

Recommended (not included):
```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: swarm-network-policy
spec:
  podSelector:
    matchLabels:
      app: swarm-coordinator
  policyTypes:
    - Ingress
    - Egress
  ingress:
    - from:
      - namespaceSelector:
          matchLabels:
            name: x3-chain
```

### Sandbox Execution

Task execution runs in seccomp/AppArmor sandbox within containers.

## Troubleshooting

### Check Coordinator Status

```bash
kubectl logs -f swarm-coordinator-0 -n x3-chain
kubectl describe pod swarm-coordinator-0 -n x3-chain
```

### Check Node Status

```bash
kubectl logs -f swarm-node-xxxxx -n x3-chain --tail=100
kubectl top pod swarm-node-xxxxx -n x3-chain
```

### Peer Connection Issues

```bash
# Check connected peers
kubectl exec swarm-coordinator-0 -n x3-chain -- curl localhost:3000/peers

# Check network
kubectl exec swarm-coordinator-0 -n x3-chain -- ip addr
```

### GPU Detection Issues

```bash
# Check GPU availability
kubectl exec swarm-node-xxxxx -n x3-chain -- nvidia-smi

# Check GPU plugin status
kubectl get nodes -o json | grep nvidia
```

## Backup & Recovery

### Coordinator State

Persistent volumes store coordinator state. Regular backups recommended:

```bash
# Backup
kubectl exec swarm-coordinator-0 -n x3-chain -- tar czf /backup.tar.gz /var/lib/swarm

# Restore
kubectl cp swarm-coordinator-0:/backup.tar.gz ./backup.tar.gz -n x3-chain
```

### ConfigMaps

```bash
# Backup configmaps
kubectl get configmap -n x3-chain -o yaml > configmaps-backup.yaml

# Restore
kubectl apply -f configmaps-backup.yaml
```

## Performance Tuning

### Coordinator

Adjust scheduler queue size based on workload:
```yaml
max_queue_size: 20000  # For high-throughput
max_queue_size: 100    # For low-latency
```

### Nodes

Adjust concurrent tasks based on GPU:
```yaml
max_concurrent_tasks: 1   # Single task per GPU
max_concurrent_tasks: 10  # Multiple concurrent tasks
```

### Network

Adjust peer limits:
```yaml
max_peers: 1000   # Large swarms
max_peers: 50     # Small clusters
```

## Upgrades

```bash
# Rolling update
kubectl set image statefulset/swarm-coordinator \
  swarm-coordinator=x3-chain/swarm-coordinator:v0.2.0 \
  -n x3-chain

# DaemonSet auto-updates nodes
kubectl set image daemonset/swarm-node \
  node=x3-chain/swarm-node:v0.2.0 \
  -n x3-chain
```

## Support & Monitoring

- Check logs: `kubectl logs -f <pod> -n x3-chain`
- Check events: `kubectl get events -n x3-chain --sort-by='.lastTimestamp'`
- Check metrics: Access Prometheus UI
- Check dashboards: Access Grafana

For any issues, provide:
1. Pod logs
2. Events
3. Metrics snapshot
4. Node/Pod descriptions
