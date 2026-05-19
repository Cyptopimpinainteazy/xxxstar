# GPU Swarm Production Deployment Guide

## Table of Contents
1. [Prerequisites](#prerequisites)
2. [Local Development Setup](#local-development-setup)
3. [Docker Compose Deployment](#docker-compose-deployment)
4. [Kubernetes Deployment](#kubernetes-deployment)
5. [Monitoring & Observability](#monitoring--observability)
6. [Production Checklist](#production-checklist)
7. [Troubleshooting](#troubleshooting)

---

## Prerequisites

### System Requirements
- Docker and Docker Compose >= 20.10
- Kubernetes >= 1.24 (for K8s deployments)
- 32GB RAM minimum (16GB per GPU node)
- NVIDIA GPU with CUDA Compute Capability >= 7.0
- 500GB free disk space

### Software Stack
- Python 3.9+
- Rust 1.70+
- Node.js 18+
- kubectl >= 1.24
- Helm >= 3.12 (optional)

### Network Requirements
- Port 9000: Coordinator API
- Port 9090: Prometheus
- Port 3000: Grafana
- Port 5601: Kibana
- Port 9200: Elasticsearch
- Port 16686: Jaeger UI

---

## Local Development Setup

### 1. Clone and Setup

```bash
cd X3_ATOMIC_STAR
python3 -m venv venv
source venv/bin/activate
pip install -r swarm/requirements.txt
```

### 2. Install Monitoring Dependencies

```bash
pip install prometheus-client opentelemetry-api opentelemetry-sdk
pip install opentelemetry-exporter-jaeger opentelemetry-exporter-prometheus
pip install elasticsearch logstash-formatter python-json-logger
```

### 3. Install Social Agents Dependencies

```bash
pip install tweepy python-telegram-bot discord.py
```

### 4. Run Unit Tests

```bash
pytest crates/gpu-swarm/src/ -v
pytest swarm/ -v
```

---

## Docker Compose Deployment

### Quick Start

```bash
cd deployment/monitoring
docker-compose up -d
```

### Verify Services

```bash
# Check all services are running
docker-compose ps

# View logs for specific service
docker-compose logs -f prometheus
docker-compose logs -f grafana
```

### Service Access

| Service | URL | Default Credentials |
|---------|-----|-------------------|
| Prometheus | http://localhost:9090 | N/A |
| Grafana | http://localhost:3000 | admin/admin |
| Kibana | http://localhost:5601 | elastic/changeme |
| Jaeger | http://localhost:16686 | N/A |
| Elasticsearch | http://localhost:9200 | elastic/changeme |

### Monitoring the Stack

```bash
# Check Prometheus targets
curl http://localhost:9090/api/v1/targets

# Query metrics
curl 'http://localhost:9090/api/v1/query?query=gpu_utilization'

# View active alerts
curl http://localhost:9090/api/v1/alerts
```

### Stopping Services

```bash
docker-compose down -v  # -v removes volumes
```

---

## Kubernetes Deployment

### Prerequisites

```bash
# Create GPU node pool (GKE example)
gcloud container node-pools create gpu-pool \
  --cluster=<cluster-name> \
  --num-nodes=3 \
  --machine-type=a100-80gb \
  --accelerator=type=nvidia-tesla-a100,count=2

# Verify GPU support
kubectl get nodes -L nvidia.com/gpu
```

### 1. Create Namespace and Secrets

```bash
# Create namespace
kubectl create namespace gpu-swarm

# Create secrets
kubectl apply -f deployment/kubernetes/secrets.yaml

# Update secrets with real values
kubectl -n gpu-swarm edit secret swarm-secrets
```

### 2. Deploy Core Infrastructure

```bash
# Apply production manifests
kubectl apply -f deployment/kubernetes/gpu-swarm-production.yaml

# Verify StatefulSet is ready
kubectl -n gpu-swarm rollout status statefulset/swarm-coordinator
kubectl -n gpu-swarm rollout status daemonset/swarm-gpu-node

# Check pod status
kubectl -n gpu-swarm get pods -o wide
```

### 3. Helm Deployment (Alternative)

```bash
# Create Helm chart
helm create charts/gpu-swarm

# Copy values
cp deployment/kubernetes/values.yaml charts/gpu-swarm/values.yaml

# Deploy
helm install gpu-swarm charts/gpu-swarm -n gpu-swarm

# Upgrade
helm upgrade gpu-swarm charts/gpu-swarm -n gpu-swarm
```

### 4. Verify Deployment

```bash
# Check all resources
kubectl -n gpu-swarm get all

# Check service endpoints
kubectl -n gpu-swarm get endpoints

# Verify persistent volumes
kubectl -n gpu-swarm get pvc

# Check resource usage
kubectl -n gpu-swarm top nodes
kubectl -n gpu-swarm top pods
```

### 5. Access Services

```bash
# Port forward to Grafana
kubectl -n gpu-swarm port-forward svc/monitoring-stack 3000:3000

# Port forward to Jaeger
kubectl -n gpu-swarm port-forward svc/monitoring-stack 16686:16686

# Port forward to Kibana
kubectl -n gpu-swarm port-forward svc/logging-stack 5601:5601

# Access Coordinator API
kubectl -n gpu-swarm port-forward svc/swarm-coordinator 9000:9000
```

---

## Monitoring & Observability

### 1. Prometheus Scrape Targets

Automatically discovered targets:
- Swarm Coordinator (9090)
- GPU Node Agents (9100)
- Prometheus self-metrics (9090)
- Alertmanager (9093)
- Node Exporter (9100)
- cAdvisor (8080)

### 2. Key Metrics to Monitor

```promql
# GPU Utilization Rate
gpu_utilization:5m

# Task Execution Rate
rate(gpu_tasks_total[5m])

# Task Failure Rate
rate(gpu_tasks_failed_total[5m]) / rate(gpu_tasks_total[5m])

# Queue Depth
gpu_queue_depth

# Network Latency
histogram_quantile(0.95, gpu_network_latency_seconds)

# Memory Fragmentation
gpu_memory_fragmentation_ratio
```

### 3. Alert Evaluation

```bash
# View alert rules
kubectl -n gpu-swarm exec -it <prometheus-pod> \
  cat /etc/prometheus/rules/prometheus-alerts.yml

# Check alert firing
curl http://prometheus:9090/api/v1/alerts

# Check alert history
curl http://prometheus:9090/api/v1/query_range \
  -G -d 'query=ALERTS{alertstate="firing"}' \
  -d 'start=2024-01-01T00:00:00Z' \
  -d 'end=2024-01-02T00:00:00Z' \
  -d 'step=1h'
```

### 4. Log Aggregation

```bash
# Search logs in Kibana
# Navigate to http://localhost:5601 -> Discover

# Query syntax example
# app: gpu-swarm AND level: ERROR
# timestamp: [2024-01-01 TO 2024-01-02]
# component: performance_optimizer AND event: optimization_failed

# Export logs
curl -X POST "localhost:9200/_sql?format=csv" \
  -H 'Content-Type: application/json' \
  -d '{"query": "SELECT * FROM swarm-logs WHERE level = \"ERROR\""}' \
  > error_logs.csv
```

### 5. Distributed Tracing

```bash
# Access Jaeger UI
# Navigate to http://localhost:16686

# View traces for service
# Service: gpu-swarm-coordinator
# Operation: submit_task

# Analyze spans
# Look for high-latency spans
# Check error rates per operation
```

### 6. Dashboard Navigation

**Grafana Dashboards** (http://localhost:3000):

1. **GPU Swarm Overview** - System health and task rates
2. **GPU Resources** - Utilization and thermal metrics
3. **Task Execution** - Queue depth and latency percentiles
4. **Network Performance** - Peer connectivity and bandwidth
5. **Economics** - Rewards and slashing rates
6. **X3 Execution** - Compilation and gas metrics

---

## Configuration Management

### Prometheus Configuration

```yaml
# deployment/monitoring/prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s
  external_labels:
    cluster: production
    region: us-east-1

alerting:
  alertmanagers:
    - static_configs:
        - targets: ['localhost:9093']

rule_files:
  - '/etc/prometheus/rules/*.yml'

scrape_configs:
  - job_name: 'swarm-coordinator'
    static_configs:
      - targets: ['coordinator-0:9000', 'coordinator-1:9000', 'coordinator-2:9000']
```

### Logstash Configuration

```conf
# deployment/monitoring/logstash.conf
input {
  tcp {
    port => 5000
    codec => json
  }
  udp {
    port => 5000
    codec => json
  }
}

filter {
  json {
    source => "message"
  }
  
  if [app] == "gpu-swarm" {
    grok {
      match => { "message" => "%{LOGLEVEL:level} \[%{DATA:component}\] %{GREEDYDATA:msg}" }
    }
  }
}

output {
  elasticsearch {
    hosts => ["elasticsearch:9200"]
    index => "swarm-%{+YYYY.MM.dd}"
  }
}
```

---

## Production Checklist

### Pre-Deployment Validation

- [ ] All configuration files reviewed and updated for production
- [ ] Database migrations tested and validated
- [ ] SSL/TLS certificates installed
- [ ] Secrets stored in secure vault (AWS Secrets Manager, HashiCorp Vault)
- [ ] Resource limits and quotas configured
- [ ] Network policies applied
- [ ] RBAC roles and bindings configured
- [ ] Storage provisioning tested
- [ ] Backup and disaster recovery procedures documented
- [ ] Load testing completed with 1000+ concurrent tasks
- [ ] Failover testing completed with coordinator node failures
- [ ] Alert routing tested (Slack, PagerDuty, etc.)

### Deployment Steps

```bash
# 1. Backup existing state
kubectl -n gpu-swarm get all -o yaml > backup-$(date +%s).yaml

# 2. Deploy with blue-green strategy
kubectl set image deployment/swarm-coordinator \
  coordinator=x3-chain/gpu-swarm-coordinator:v1.2.3

# 3. Monitor rollout
kubectl -n gpu-swarm rollout status deployment/swarm-coordinator

# 4. Verify metrics
# - Check Prometheus targets are healthy
# - Verify alert rules are loaded
# - Monitor dashboard for anomalies

# 5. Smoke tests
./scripts/smoke-test.sh

# 6. Load tests
./scripts/load-test.sh --concurrent=100 --duration=300s

# 7. Cutover
# - Update DNS/load balancer to point to new deployment
# - Monitor error rates for 24 hours
```

### Post-Deployment Validation

- [ ] All pods are running and healthy (kubectl get pods)
- [ ] Services are responsive (kubectl exec -it... curl)
- [ ] Metrics are being collected (Prometheus UI)
- [ ] Logs are being aggregated (Kibana)
- [ ] Traces are being recorded (Jaeger)
- [ ] Alerts are firing correctly for test events
- [ ] Performance is within SLO bounds
- [ ] No errors in application logs
- [ ] Backup jobs completed successfully

---

## Troubleshooting

### Common Issues

#### 1. Coordinator Quorum Loss

**Symptom**: Alert "CoordinatorQuorumLoss" firing

**Diagnosis**:
```bash
# Check coordinator status
kubectl -n gpu-swarm get pods -l app=swarm-coordinator

# Check logs
kubectl -n gpu-swarm logs -f swarm-coordinator-0

# Check peer connectivity
kubectl -n gpu-swarm exec swarm-coordinator-0 -- \
  curl http://localhost:9000/status
```

**Resolution**:
```bash
# If corruption: replace affected pod
kubectl -n gpu-swarm delete pod swarm-coordinator-2

# If network issue: restart all coordinators
kubectl -n gpu-swarm rollout restart statefulset/swarm-coordinator

# Wait for recovery
kubectl -n gpu-swarm rollout status statefulset/swarm-coordinator
```

#### 2. GPU Memory Exhaustion

**Symptom**: Tasks failing with "Out of memory"

**Diagnosis**:
```bash
# Check GPU memory usage
kubectl -n gpu-swarm exec <gpu-node-pod> -- nvidia-smi

# Check fragmentation
kubectl -n gpu-swarm logs <gpu-node-pod> | grep fragmentation

# Check task queue
curl http://coordinator:9000/metrics | grep gpu_queue_depth
```

**Resolution**:
```bash
# Increase memory pool size
kubectl -n gpu-swarm set env daemonset/swarm-gpu-node \
  GPU_POOL_SIZE=48GB

# Force defragmentation
kubectl -n gpu-swarm exec <gpu-node-pod> -- \
  swarm-cli gpu defragment

# Monitor recovery
watch "kubectl top nodes"
```

#### 3. High Network Latency

**Symptom**: Alert "HighNetworkLatency" firing

**Diagnosis**:
```bash
# Check network metrics
curl http://prometheus:9090/api/v1/query?query=gpu_network_latency_seconds

# Check peer connections
kubectl -n gpu-swarm exec swarm-coordinator-0 -- \
  curl http://localhost:9000/peers

# Check network policies
kubectl -n gpu-swarm get networkpolicies
```

**Resolution**:
```bash
# Check node connectivity
kubectl get nodes -o wide
kubectl describe node <node>

# Check pod networking
kubectl -n gpu-swarm logs <gpu-node-pod> | grep -i network

# Increase batch timeout
kubectl -n gpu-swarm set env daemonset/swarm-gpu-node \
  BATCH_TIMEOUT=2s
```

#### 4. Elasticsearch Cluster Issues

**Symptom**: Logs not appearing in Kibana

**Diagnosis**:
```bash
# Check Elasticsearch status
curl -u elastic:changeme http://elasticsearch:9200/_cluster/health

# Check indices
curl -u elastic:changeme http://elasticsearch:9200/_cat/indices

# Check Logstash connectivity
docker-compose logs logstash | grep -i error
```

**Resolution**:
```bash
# Restart Elasticsearch
docker-compose restart elasticsearch

# Check Logstash configuration
docker-compose exec logstash cat /usr/share/logstash/config/logstash.conf

# Force index creation
curl -u elastic:changeme -X PUT \
  http://elasticsearch:9200/swarm-logs-2024.01.01
```

#### 5. Alert Manager Not Sending Notifications

**Symptom**: Alerts firing but no Slack messages

**Diagnosis**:
```bash
# Check Alertmanager status
curl http://alertmanager:9093/api/v1/status

# Check alert groups
curl http://alertmanager:9093/api/v1/alerts

# Check configuration
kubectl -n gpu-swarm exec <alertmanager-pod> -- \
  cat /etc/alertmanager/config.yml
```

**Resolution**:
```bash
# Verify webhook URL
kubectl -n gpu-swarm get secret swarm-secrets -o \
  jsonpath='{.data.slack-webhook-url}' | base64 -d

# Test webhook
curl -X POST <webhook-url> \
  -d '{"text":"Test message"}'

# Update configuration
kubectl -n gpu-swarm edit configmap alertmanager-config
```

### Performance Tuning

#### Prometheus Optimization
```yaml
# Reduce scrape interval for less critical jobs
scrape_interval: 30s  # for non-critical services

# Increase retention for important metrics
--storage.tsdb.retention.time=90d

# Enable compression
--storage.tsdb.max-block-duration=2h
```

#### Logstash Optimization
```conf
pipeline.batch.size: 1000
pipeline.batch.delay: 50
pipeline.workers: 4
```

#### Elasticsearch Optimization
```yaml
indices.memory.index_buffer_size: "40%"
thread_pool.search.queue_size: 1000
thread_pool.write.queue_size: 1000
```

---

## Maintenance

### Regular Tasks

**Daily**:
- Monitor for alert firing
- Review error logs in Kibana
- Check resource utilization

**Weekly**:
- Verify backup completion
- Rotate logs
- Review performance trends

**Monthly**:
- Update dependencies
- Review and tune alerts
- Capacity planning review

### Scaling

#### Horizontal Scaling

```bash
# Scale coordinator replicas
kubectl -n gpu-swarm scale statefulset swarm-coordinator --replicas=5

# Add GPU node pool
gcloud container node-pools create gpu-pool-2 --cluster=production
```

#### Vertical Scaling

```bash
# Increase resource limits
kubectl -n gpu-swarm set resources deployment swarm-coordinator \
  --requests=cpu=4,memory=8Gi \
  --limits=cpu=8,memory=16Gi
```

---

## Support & Documentation

- **Issues**: Create ticket in project tracking system
- **Runbooks**: `/deployment/runbooks/` directory
- **Architecture**: `/docs/architecture/` directory
- **API Documentation**: Swagger UI at `http://coordinator:9000/swagger`
- **CLI Help**: `swarm-cli --help`

---

**Last Updated**: January 2024
**Version**: 1.0
**Maintainer**: GPU Swarm Team
