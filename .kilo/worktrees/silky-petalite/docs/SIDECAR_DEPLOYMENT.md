# X3 Sidecar Deployment Guide

This guide provides step-by-step procedures for deploying and operating the X3 sidecar in production environments.

## Table of Contents

1. [Pre-Deployment Checklist](#pre-deployment-checklist)
2. [Local Development Deployment](#local-development-deployment)
3. [Docker Deployment](#docker-deployment)
4. [Kubernetes Deployment](#kubernetes-deployment)
5. [Health Checks and Verification](#health-checks-and-verification)
6. [Monitoring and Metrics](#monitoring-and-metrics)
7. [Rollback Procedures](#rollback-procedures)
8. [Security Considerations](#security-considerations)

## Pre-Deployment Checklist

Before deploying the X3 sidecar, verify the following prerequisites:

### Gateway Connectivity
- [ ] X3 Gateway service is running and accessible
- [ ] Gateway URL is correctly configured: `X3_GATEWAY_URL=http://gateway-service:8080`
- [ ] Network connectivity verified: `curl -I {X3_GATEWAY_URL}/health`
- [ ] Firewall rules allow sidecar → gateway communication
- [ ] TLS certificates installed (if using HTTPS)

### Database Readiness
- [ ] PostgreSQL 12+ is running and accessible
- [ ] Database credentials are secure and available
- [ ] Connection URL format verified: `postgresql://user:password@host:5432/x3_sidecar`
- [ ] Database user has CREATE TABLE permissions
- [ ] Database schema is empty (sidecar will initialize on first run)
- [ ] Connection pool size configured appropriately for workload
- [ ] Backup strategy is in place

### Secrets and Configuration
- [ ] API keys/tokens are generated and stored securely
- [ ] Environment variables prepared in `.env` or secrets manager
- [ ] Configuration file (`sidecar.toml`) validated against schema
- [ ] Log level set appropriately (INFO for production, DEBUG for troubleshooting)
- [ ] All required environment variables are set

### System Resources
- [ ] Server has minimum 512MB RAM available
- [ ] 2+ CPU cores recommended for production
- [ ] 100GB+ disk space available for sidecar data
- [ ] Network connectivity: 1+ Mbps outbound to gateway
- [ ] System clock is synchronized (NTP enabled)

## Local Development Deployment

For development and testing purposes:

### 1. Start PostgreSQL
```bash
docker run -d \
  --name x3-postgres \
  -e POSTGRES_DB=x3_sidecar \
  -e POSTGRES_USER=x3_user \
  -e POSTGRES_PASSWORD=x3_password \
  -p 5432:5432 \
  postgres:15
```

### 2. Start X3 Gateway (if not already running)
```bash
# Assuming gateway runs on port 8080
# Refer to X3 Gateway documentation for startup
```

### 3. Configure Environment
```bash
export X3_GATEWAY_URL=http://localhost:8080
export X3_DATABASE_URL=postgresql://x3_user:x3_password@localhost:5432/x3_sidecar
export X3_LOG_LEVEL=DEBUG
export X3_BATCH_SIZE=100
export X3_BATCH_TIMEOUT_MS=5000
```

### 4. Build and Run Sidecar
```bash
cd crates/x3-sidecar
cargo build --release
./target/release/x3-sidecar
```

### 5. Verify Startup
```bash
# Check sidecar health
curl http://localhost:9090/health

# Expected response:
# {"status":"healthy","uptime_seconds":5,"database_connected":true}
```

## Docker Deployment

For containerized deployments:

### 1. Build Docker Image
```bash
docker build -f crates/x3-sidecar/Dockerfile -t x3-sidecar:latest .
```

If Dockerfile does not exist, create one:
```dockerfile
FROM rust:1.75 as builder
WORKDIR /workspace
COPY . .
RUN cd crates/x3-sidecar && cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /workspace/crates/x3-sidecar/target/release/x3-sidecar /usr/local/bin/
EXPOSE 9090
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:9090/health || exit 1
ENTRYPOINT ["x3-sidecar"]
```

### 2. Create Environment File
```bash
# .env.production
X3_GATEWAY_URL=http://x3-gateway:8080
X3_DATABASE_URL=postgresql://x3_user:x3_password@postgres:5432/x3_sidecar
X3_LOG_LEVEL=INFO
X3_BATCH_SIZE=500
X3_BATCH_TIMEOUT_MS=10000
X3_REQUEST_TIMEOUT_MS=30000
X3_MAX_RETRIES=3
X3_RETRY_BACKOFF_MS=1000
```

### 3. Run Container
```bash
docker run -d \
  --name x3-sidecar \
  --env-file .env.production \
  -p 9090:9090 \
  --network x3-network \
  x3-sidecar:latest
```

### 4. Verify Container is Running
```bash
docker logs x3-sidecar
docker exec x3-sidecar curl http://localhost:9090/health
```

## Kubernetes Deployment

For Kubernetes clusters:

### 1. Create ConfigMap for Configuration
```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: x3-sidecar-config
  namespace: x3
data:
  sidecar.toml: |
    [gateway]
    url = "http://x3-gateway:8080"
    timeout_ms = 30000
    
    [database]
    max_connections = 20
    connection_timeout_ms = 5000
    
    [submission]
    batch_size = 500
    batch_timeout_ms = 10000
    
    [logging]
    level = "info"
```

### 2. Create Secret for Credentials
```bash
kubectl create secret generic x3-sidecar-secrets \
  --from-literal=database-url='postgresql://x3_user:x3_password@postgres:5432/x3_sidecar' \
  -n x3
```

### 3. Create Deployment
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: x3-sidecar
  namespace: x3
spec:
  replicas: 2
  selector:
    matchLabels:
      app: x3-sidecar
  template:
    metadata:
      labels:
        app: x3-sidecar
    spec:
      containers:
      - name: sidecar
        image: x3-sidecar:latest
        imagePullPolicy: Always
        ports:
        - containerPort: 9090
          name: health
        env:
        - name: X3_GATEWAY_URL
          value: "http://x3-gateway:8080"
        - name: X3_DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: x3-sidecar-secrets
              key: database-url
        - name: X3_LOG_LEVEL
          value: "INFO"
        livenessProbe:
          httpGet:
            path: /health
            port: 9090
          initialDelaySeconds: 15
          periodSeconds: 30
          timeoutSeconds: 5
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /health
            port: 9090
          initialDelaySeconds: 5
          periodSeconds: 10
          timeoutSeconds: 3
          failureThreshold: 2
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        volumeMounts:
        - name: config
          mountPath: /etc/x3-sidecar
          readOnly: true
      volumes:
      - name: config
        configMap:
          name: x3-sidecar-config
```

### 4. Create Service
```yaml
apiVersion: v1
kind: Service
metadata:
  name: x3-sidecar
  namespace: x3
spec:
  selector:
    app: x3-sidecar
  ports:
  - protocol: TCP
    port: 9090
    targetPort: 9090
  type: ClusterIP
```

### 5. Deploy to Kubernetes
```bash
kubectl apply -f x3-sidecar-configmap.yaml
kubectl create secret generic x3-sidecar-secrets \
  --from-literal=database-url='postgresql://...' \
  -n x3
kubectl apply -f x3-sidecar-deployment.yaml
kubectl apply -f x3-sidecar-service.yaml
```

### 6. Verify Deployment
```bash
kubectl get pods -n x3 -l app=x3-sidecar
kubectl logs -n x3 -l app=x3-sidecar --tail=50
kubectl exec -n x3 -it <pod-name> -- curl http://localhost:9090/health
```

## Health Checks and Verification

### Local Health Check
```bash
curl -X GET http://localhost:9090/health
```

Expected response:
```json
{
  "status": "healthy",
  "uptime_seconds": 1234,
  "database_connected": true,
  "gateway_reachable": true
}
```

### Database Connectivity Verification
```bash
# From sidecar container/host
psql -U x3_user -d x3_sidecar -h localhost -c "SELECT version();"
```

### Gateway Connectivity Verification
```bash
curl -X GET http://{X3_GATEWAY_URL}/health
curl -X POST http://{X3_GATEWAY_URL}/api/v1/benchmarks/results \
  -H "Content-Type: application/json" \
  -d '{"test": "connectivity"}'
```

### Log Verification
```bash
# Docker
docker logs x3-sidecar | grep -E "ERROR|WARN|startup"

# Kubernetes
kubectl logs -n x3 -l app=x3-sidecar | grep -E "ERROR|WARN|startup"

# Local
tail -f /var/log/x3-sidecar.log | grep -E "ERROR|WARN|startup"
```

## Monitoring and Metrics

### Available Metrics Endpoints

The sidecar exposes metrics at `/metrics` (Prometheus format):

```bash
curl http://localhost:9090/metrics
```

Key metrics to monitor:
- `x3_sidecar_submissions_total` - Total submissions processed
- `x3_sidecar_submission_latency_seconds` - Latency histogram
- `x3_sidecar_retries_total` - Total retry attempts
- `x3_sidecar_gateway_errors_total` - Gateway communication failures
- `x3_sidecar_database_connections_active` - Active DB connections
- `x3_sidecar_uptime_seconds` - Sidecar uptime

### Prometheus Configuration

Add to `prometheus.yml`:
```yaml
scrape_configs:
  - job_name: 'x3-sidecar'
    static_configs:
      - targets: ['localhost:9090']
    scrape_interval: 30s
    metrics_path: '/metrics'
```

### Alerting Rules

Create `x3-sidecar-alerts.yml`:
```yaml
groups:
  - name: x3_sidecar
    rules:
    - alert: SidecarDown
      expr: up{job="x3-sidecar"} == 0
      for: 2m
      annotations:
        summary: "X3 Sidecar is down"
    
    - alert: HighErrorRate
      expr: rate(x3_sidecar_gateway_errors_total[5m]) > 0.1
      for: 5m
      annotations:
        summary: "X3 Sidecar error rate > 10%"
    
    - alert: HighLatency
      expr: histogram_quantile(0.95, x3_sidecar_submission_latency_seconds) > 5
      for: 5m
      annotations:
        summary: "X3 Sidecar p95 latency > 5s"
    
    - alert: DatabaseConnectionPoolExhausted
      expr: x3_sidecar_database_connections_active > 18
      for: 2m
      annotations:
        summary: "X3 Sidecar database connection pool nearly exhausted"
```

## Rollback Procedures

### Docker Rollback
```bash
# Identify previous image version
docker images x3-sidecar

# Stop current container
docker stop x3-sidecar

# Run previous version
docker run -d \
  --name x3-sidecar \
  --env-file .env.production \
  -p 9090:9090 \
  x3-sidecar:v1.0.0  # Previous version tag

# Verify health
curl http://localhost:9090/health
```

### Kubernetes Rollback
```bash
# Check rollout history
kubectl rollout history deployment/x3-sidecar -n x3

# Rollback to previous version
kubectl rollout undo deployment/x3-sidecar -n x3

# Verify rollback
kubectl get pods -n x3 -l app=x3-sidecar
kubectl rollout status deployment/x3-sidecar -n x3
```

### Data Consistency During Rollback
- Sidecar uses write-ahead logging - safe to stop/restart
- Pending submissions are stored in database and will be retried
- No data loss expected during rollback
- Verify database integrity post-rollback:
  ```bash
  psql -U x3_user -d x3_sidecar -c "SELECT COUNT(*) FROM submissions WHERE status='pending';"
  ```

## Security Considerations

### Database Security
- Use strong passwords (minimum 20 characters, mixed character types)
- Store credentials in secrets manager, not environment files
- Use SSL/TLS for database connections when possible
- Restrict database user permissions to minimum required
- Enable database audit logging
- Regular database backups and testing of restore procedures

### Network Security
- Deploy sidecar in private network/VPC
- Use network policies/security groups to restrict traffic
- Only allow traffic from authorized sources
- Use TLS for gateway communication
- Implement rate limiting at gateway level

### Application Security
- Keep dependencies updated: `cargo update`
- Use security scanning tools: `cargo audit`
- Implement request signing/authentication with gateway
- Validate all gateway responses
- Log security events (auth failures, errors)
- Regular security audits of configuration

### Operational Security
- Rotate API keys and passwords regularly
- Implement least privilege for service accounts
- Monitor and log all deployments and configuration changes
- Implement role-based access control (RBAC)
- Use immutable infrastructure principles
- Regular disaster recovery drills

### Compliance
- Verify data residency requirements are met
- Enable encryption at rest (database and backups)
- Implement data retention policies
- Document security controls for audit
- Regular compliance testing
