# Deployment Guide: Substreams Skills + LLM Integration

Production-ready deployment instructions for local, Docker, and Kubernetes environments.

## Pre-Deployment Checklist

- [ ] Node.js 18+ installed
- [ ] Ollama installed (optional but recommended)
- [ ] OpenRouter API key obtained (optional)
- [ ] Docker installed (for containerized deployment)
- [ ] Kubernetes cluster available (for K8s deployment)
- [ ] Configuration files created
- [ ] Dependencies installed
- [ ] Tests passing
- [ ] Performance benchmarks accepted

## Deployment Options

### Option 1: Local Development

Fastest for development and testing.

#### Setup

```bash
# 1. Install dependencies
npm install

# 2. Install Ollama (macOS/Linux)
curl -fsSL https://ollama.ai/install.sh | sh

# 3. Pull a model
ollama pull mistral

# 4. Set OpenRouter key (optional)
export OPENROUTER_API_KEY=sk-or-...

# 5. Start Ollama (in terminal 1)
ollama serve

# 6. Start LLM Router (in terminal 2)
npm start

# 7. Test (in terminal 3)
curl http://localhost:3000/health
```

#### Configuration

Edit `llm-config.json`:
- Set `default_provider` to `ollama` or `openrouter`
- Configure endpoints and models
- Adjust timeouts and retries

#### Monitoring

```bash
# Check health
curl http://localhost:3000/health

# View metrics
curl http://localhost:3000/metrics | jq

# View logs
tail -f logs/llm-service.log  # If logging enabled
```

#### Scaling

Not needed for local development. For production, use Docker or Kubernetes.

---

### Option 2: Docker Compose

Recommended for staging and small deployments.

#### Setup

```bash
# 1. Create .env file
cat > .env << EOF
DEFAULT_PROVIDER=ollama
OPENROUTER_API_KEY=sk-or-...  # Optional
EOF

# 2. Start services
docker-compose -f docker-compose.llm.yml up -d

# 3. Wait for services to be healthy
docker-compose -f docker-compose.llm.yml ps

# 4. Test
curl http://localhost:3000/health
```

#### Configuration

Environment variables in `docker-compose.llm.yml`:

```yaml
environment:
  - PORT=3000
  - OLLAMA_ENDPOINT=http://ollama:11434
  - OPENROUTER_API_KEY=${OPENROUTER_API_KEY}
  - DEFAULT_PROVIDER=${DEFAULT_PROVIDER:-ollama}
```

#### Scaling

```bash
# Scale LLM Router
docker-compose -f docker-compose.llm.yml up -d --scale llm-router=3

# Monitor
docker-compose -f docker-compose.llm.yml logs -f llm-router
```

#### Updates

```bash
# Pull latest images
docker-compose -f docker-compose.llm.yml pull

# Restart services
docker-compose -f docker-compose.llm.yml restart

# Full redeploy
docker-compose -f docker-compose.llm.yml up -d --force-recreate
```

#### Cleanup

```bash
# Stop services
docker-compose -f docker-compose.llm.yml down

# Remove volumes (WARNING: data loss)
docker-compose -f docker-compose.llm.yml down -v

# Remove all images
docker rmi ollama/ollama node:20-alpine
```

---

### Option 3: Kubernetes

Recommended for production deployments.

#### Prerequisites

- Kubernetes cluster (1.20+)
- kubectl configured
- Persistent storage available
- LoadBalancer service support (or Ingress)

#### Setup

```bash
# 1. Create namespace
kubectl create namespace substreams-llm

# 2. Create OpenRouter secret (optional)
kubectl create secret generic openrouter-secret \
  --from-literal=api-key=sk-or-... \
  -n substreams-llm

# 3. Deploy
kubectl apply -f k8s-deployment.yaml

# 4. Wait for deployments
kubectl -n substreams-llm wait --for=condition=available \
  --timeout=300s deployment/llm-router

# 5. Check status
kubectl -n substreams-llm get all
```

#### Accessing the Service

```bash
# Port forward (for testing)
kubectl -n substreams-llm port-forward svc/llm-router 3000:3000

# Or get LoadBalancer URL
kubectl -n substreams-llm get svc llm-router

# Test
curl http://localhost:3000/health
```

#### Configuration

Update `llm-config.json` in the ConfigMap:

```bash
# Edit config
kubectl -n substreams-llm edit configmap llm-config

# Rollout restart to apply
kubectl -n substreams-llm rollout restart deployment/llm-router
```

#### Scaling

Auto-scaling is configured via HorizontalPodAutoscaler:

```bash
# Check HPA status
kubectl -n substreams-llm get hpa

# View detailed HPA
kubectl -n substreams-llm describe hpa llm-router-hpa

# Manual scaling
kubectl -n substreams-llm scale deployment llm-router --replicas=5
```

#### Monitoring

```bash
# Pod status
kubectl -n substreams-llm get pods -w

# Logs
kubectl -n substreams-llm logs -f deployment/llm-router --all-containers=true

# Described resource
kubectl -n substreams-llm describe deployment llm-router

# Metrics
kubectl -n substreams-llm top pods --containers
```

#### Updates

```bash
# Update image
kubectl -n substreams-llm set image \
  deployment/llm-router llm-router=newimage:tag

# Rollback
kubectl -n substreams-llm rollout undo deployment/llm-router

# View history
kubectl -n substreams-llm rollout history deployment/llm-router
```

#### Cleanup

```bash
# Delete deployment
kubectl delete -f k8s-deployment.yaml

# Or delete namespace (removes everything)
kubectl delete namespace substreams-llm
```

---

## Health Checks & Monitoring

### Health Check Endpoints

```bash
# Router health
curl http://localhost:3000/health

# Detailed health
curl http://localhost:3000/models | jq '.[] | select(.available)'

# Metrics and statistics
curl http://localhost:3000/metrics | jq
```

### Expected HTTP Status Codes

| Endpoint | Status | Meaning |
|----------|--------|---------|
| `/health` | 200 | Router is healthy |
| `/query` (POST) | 200 | Query successful |
| `/query` (POST) | 400 | Invalid request |
| `/query` (POST) | 500 | Server error |
| `/models` | 200 | Models retrieved |
| `/metrics` | 200 | Metrics retrieved |

### Logging

The router logs important events:

```bash
# Development (console)
npm start

# Production (file-based, if configured)
tail -f logs/llm-service.log
```

Key log events:
- Service startup/shutdown
- Provider initialization
- Query errors
- Failover events
- Health check failures

### Performance Monitoring

```bash
# Get performance metrics
curl http://localhost:3000/metrics | jq '.providers'

# Expected metrics
{
  "ollama": {
    "queries": 1243,
    "failures": 5,
    "avgTime": 2156,
    "totalTokens": 245680
  }
}
```

---

## Configuration For Different Environments

### Development

```json
{
  "default_provider": "ollama",
  "caching": { "enabled": true, "ttl_seconds": 600 },
  "retry_config": { "max_retries": 2 }
}
```

### Staging

```json
{
  "default_provider": "ollama",
  "failover_chain": [
    { "provider": "ollama", "priority": 1 },
    { "provider": "openrouter", "priority": 2 }
  ],
  "caching": { "enabled": true, "ttl_seconds": 3600 },
  "retry_config": { "max_retries": 3 }
}
```

### Production

```json
{
  "default_provider": "ollama",
  "failover_chain": [
    { "provider": "ollama", "priority": 1 },
    { "provider": "openrouter", "priority": 2 },
    { "provider": "openrouter", "model": "gpt-4", "priority": 3 }
  ],
  "caching": { "enabled": true, "ttl_seconds": 7200, "max_size_mb": 500 },
  "retry_config": {
    "max_retries": 5,
    "backoff_multiplier": 2.0,
    "timeout_ms": 60000
  },
  "telemetry": {
    "enabled": true,
    "track_latency": true,
    "track_costs": true
  }
}
```

---

## Troubleshooting Deployment Issues

### Service Won't Start

```bash
# Check port is available
lsof -i :3000

# Check dependencies installed
npm ls

# Check configuration file
cat llm-config.json | jq

# Try starting with verbose errors
node --trace-uncaught llm-service/router.js
```

### Ollama Connection Failed

```bash
# Verify Ollama is running
ps aux | grep ollama

# Check endpoint is accessible
curl http://localhost:11434/api/tags

# Restart Ollama
killall ollama
ollama serve
```

### OpenRouter Errors

```bash
# Verify API key is set
echo $OPENROUTER_API_KEY

# Test API key
curl -H "Authorization: Bearer $OPENROUTER_API_KEY" \
  https://openrouter.dev/api/v1/auth/key

# Check API key validity
curl -X POST https://openrouter.dev/api/v1/chat/completions \
  -H "Authorization: Bearer $OPENROUTER_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model":"mistralai/mistral-7b-instruct","messages":[{"role":"user","content":"test"}]}'
```

### Memory Usage Too High

```bash
# Check process memory
ps aux | grep node
ps aux | grep ollama

# Try smaller model
ollama pull neural-chat  # Smaller than mistral

# Update config to use smaller model
{
  "providers": {
    "ollama": { "model": "neural-chat" }
  }
}

# Restart router
pkill node && npm start
```

### Slow Responses

```bash
# Check system resources
top -b -n 1 | head -15

# Monitor network latency (for OpenRouter)
ping openrouter.dev

# Check response times in metrics
curl http://localhost:3000/metrics | jq '.providers'

# Try faster model
ollama pull phi  # Ultra-fast, smaller

# Use local provider instead of API
# Update llm-config.json to prefer ollama
```

---

## Performance Benchmarks

### Baseline (Should Achieve)

| Metric | Expected | Acceptable | Threshold |
|--------|----------|------------|-----------|
| Health check | < 100ms | < 200ms | > 500ms ⚠️ |
| Query (ollama) | 2-5s | < 10s | > 30s ⚠️ |
| Query (OpenRouter) | 5-15s | < 30s | > 60s ⚠️ |
| Requests/sec | 2-5 | 1+ | < 1 ⚠️ |
| Error rate | < 1% | < 5% | > 10% ⚠️ |

### Run Benchmarks

```bash
# Simple load test
for i in {1..10}; do
  curl -X POST http://localhost:3000/query \
    -H "Content-Type: application/json" \
    -d '{"query":"test","provider":"ollama"}' \
    -w "Response time: %{time_total}s\n"
done

# More comprehensive testing
# Use Apache Bench, wrk, or k6
wrk -t4 -c100 -d30s --script=load-test.lua http://localhost:3000/query
```

---

## Disaster Recovery

### Backup

```bash
# Backup Ollama models (Docker)
docker cp substreams-ollama:/root/.ollama ./ollama-backup

# Backup config
cp llm-config.json llm-config.json.backup
```

### Restore

```bash
# Restore Ollama models
docker cp ./ollama-backup/. substreams-ollama:/root/.ollama

# Restore config
cp llm-config.json.backup llm-config.json
```

### High Availability (HA)

```bash
# Use multiple router instances with load balancer
docker-compose -f docker-compose.llm.yml up -d --scale llm-router=3

# Or in Kubernetes: adjust replicas in HPA
kubectl patch hpa llm-router-hpa \
  -p '{"spec":{"minReplicas":3,"maxReplicas":20}}'
```

---

## Cost Optimization

### Reduce API Calls

```bash
# Enable caching in config
"caching": {
  "enabled": true,
  "ttl_seconds": 7200,
  "max_size_mb": 1000
}
```

### Use Cheaper Models

```json
{
  "model_mappings": {
    "fast": {
      "openrouter": "mistralai/mistral-7b-instruct"
    }
  }
}
```

### Monitor Costs

```bash
# Check OpenRouter costs
curl -H "Authorization: Bearer $OPENROUTER_API_KEY" \
  https://openrouter.dev/api/v1/auth/key | jq '.data.limit'

# Check usage in metrics
curl http://localhost:3000/metrics | jq '.providers.openrouter'
```

---

## Security Considerations

### Environment Variables

Never commit API keys to version control:

```bash
# Use environment files
cat > .env.local << EOF
OPENROUTER_API_KEY=sk-or-...
EOF

# Add to .gitignore
echo ".env.local" >> .gitignore
```

### Network Security

For production Kubernetes:

```bash
# Enable network policies
kubectl apply -f - << EOF
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: llm-router-policy
  namespace: substreams-llm
spec:
  podSelector:
    matchLabels:
      app: llm-router
  policyTypes:
  - Ingress
  ingress:
  - from:
    - podSelector:
        matchLabels:
          allowed: "true"
EOF
```

### TLS/HTTPS

For production, use Ingress with TLS:

```bash
# Install cert-manager
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.13.0/cert-manager.yaml

# Update Ingress with TLS
kubectl patch ingress llm-router -p \
  '{"spec":{"tls":[{"hosts":["example.com"],"secretName":"llm-router-tls"}]}}'
```

---

## Support & Escalation

### Debug Information to Gather

```bash
# Collect system info
uname -a
node --version
npm --version
docker --version

# Check process status
ps aux | grep "node\|ollama"

# Get logs
npm start 2>&1 | head -100

# Check connectivity
curl -v http://localhost:3000/health
curl -v http://localhost:11434/api/tags

# Get metrics
curl http://localhost:3000/metrics | jq
```

### Common Issues & Solutions

Open an issue with:
1. Debug information (above)
2. Configuration (redacted API keys)
3. Expected vs actual behavior
4. Steps to reproduce

---

## Maintenance

### Regular Tasks

- **Daily**: Monitor metrics, check error rates
- **Weekly**: Review logs, update models if needed
- **Monthly**: Update dependencies, security patches
- **Quarterly**: Performance review, cost analysis

### Update Dependencies

```bash
npm outdated
npm update
npm audit fix
```

### Update Models

```bash
ollama pull mistral:latest
ollama delete mistral:old
```

---

**Deployment complete! 🚀**

For questions or issues, refer to the main documentation or open an issue.
