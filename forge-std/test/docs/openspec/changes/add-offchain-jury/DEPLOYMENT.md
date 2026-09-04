# Jury Service Infrastructure & Deployment Guide

## Overview

This guide covers deployment of the X3 Chain Off-Chain Jury Service across development, staging, and production environments.

**Table of Contents:**
- [Quick Start](#quick-start)
- [Docker Compose Deployment](#docker-compose-deployment)
- [Systemd Service Setup](#systemd-service-setup)
- [Production Deployment](#production-deployment)
- [Monitoring & Observability](#monitoring--observability)
- [Troubleshooting](#troubleshooting)
- [Security Considerations](#security-considerations)

---

## Quick Start

### Prerequisites
- Docker & Docker Compose 3.9+
- Python 3.11+ (for local development)
- PostgreSQL 15+ (or use Docker service)
- Git

### Start the Jury Service Locally

```bash
# 1. Clone and navigate to the directory
cd openspec/changes/add-offchain-jury

# 2. Create environment file
cp jury.env.example .env-local

# 3. Edit configuration (change passwords!)
nano .env-local

# 4. Start all services
docker-compose up -d

# 5. Verify services are running
docker-compose ps
docker-compose logs jury-service

# 6. Test the health endpoint
curl http://localhost:8000/health
```

**Expected Output:**
```json
{
  "status": "healthy",
  "timestamp": "2026-02-08T10:00:00Z",
  "version": "1.0.0"
}
```

---

## Docker Compose Deployment

### Services Included

| Service | Purpose | Port | CPU | Memory |
|---------|---------|------|-----|--------|
| `jury-db` | PostgreSQL 15 (audit logs) | 5432 | 1 | 1GB |
| `jury-cache` | Redis 7 (sessions/cache) | 6379 | 0.5 | 512MB |
| `jury-service` | Main API server | 8000 | 2 | 2GB |
| `jury-metrics` | Prometheus (optional) | 9090 | 0.5 | 512MB |

### Configuration Files

#### `docker-compose.yml`
Main orchestration file with service definitions, environment configuration, and volumes.

**Key Features:**
- Health checks for all services
- Volume persistence for PostgreSQL and Redis
- Network isolation via `jury-net` bridge
- Error recovery with restart policies
- GPU support stubs (commented for CPU-only)

#### `Dockerfile`
Multi-stage build supporting both CPU and GPU.

**Build Variants:**
```bash
# CPU-only (default)
docker build -t jury-service:latest .

# GPU with CUDA 12.1
docker build --build-arg WITH_GPU=true -t jury-service:gpu .

# Specific Python version
docker build --build-arg PYTHON_VERSION=3.10 -t jury-service:py310 .
```

### Environment Configuration

Create `.env-local` for local development:
```bash
cat > .env-local << 'EOF'
# Secrets (change these!)
JURY_DB_PASSWORD=my_secure_db_password
JURY_REDIS_PASSWORD=my_secure_redis_password

# Service configuration
JURY_LOG_LEVEL=DEBUG
JURY_COMMIT_DEADLINE_SECONDS=300
JURY_REVEAL_DEADLINE_SECONDS=600

# Optional GPU support
WITH_GPU=false
CUDA_VISIBLE_DEVICES=0
EOF
```

### Running Services

```bash
# Start in background
docker-compose up -d

# View logs (all services)
docker-compose logs -f

# View specific service logs
docker-compose logs -f jury-service

# Stop services
docker-compose down

# Stop and remove volumes (full cleanup)
docker-compose down -v

# Restart specific service
docker-compose restart jury-service

# Scale a service (if stateless)
docker-compose up -d --scale jury-service=2
```

### Database Initialization

The `sql-init/01-init-schema.sql` script automatically runs on first container start:

```bash
# Verify schema was created
docker exec x3-jury-db psql -U jury_admin -d jury_audit -c "\dt"

# Expected tables:
# - audit_logs
# - jury_sessions
# - jury_votes
# - audit_log_seals
```

### Accessing Services

```bash
# API Server
curl http://localhost:8000/health
curl -X POST http://localhost:8000/api/jury/session \
  -H "Content-Type: application/json" \
  -d '{...}'

# PostgreSQL
docker exec -it x3-jury-db psql -U jury_admin -d jury_audit

# Redis
docker exec -it x3-jury-cache redis-cli

# Prometheus (if enabled)
docker-compose --profile observability up -d jury-metrics
# Access at: http://localhost:9090
```

---

## Systemd Service Setup

### Installation Steps

#### 1. Create Service User
```bash
sudo useradd -m -s /bin/bash -d /opt/x3/jury jury
sudo mkdir -p /opt/x3/jury
sudo mkdir -p /var/log/jury /var/cache/jury/sessions
sudo chown -R jury:jury /opt/x3/jury /var/log/jury /var/cache/jury
```

#### 2. Install Application
```bash
# Copy application files
sudo cp -r swarm /opt/x3/jury/
cd /opt/x3/jury

# Create and activate venv
sudo -u jury python3 -m venv venv
sudo -u jury venv/bin/pip install -r swarm/requirements.txt
sudo -u jury venv/bin/pip install aiohttp aiohttp-cors aiofiles psycopg2-binary redis
```

#### 3. Configure Environment
```bash
# Create config directory
sudo mkdir -p /etc/x3/jury

# Copy and edit configuration
sudo cp jury.env.example /etc/x3/jury/jury.env
sudo nano /etc/x3/jury/jury.env  # Edit passwords and URLs

# Create local overrides (not in git)
sudo touch /etc/x3/jury/jury.local.env
sudo chmod 600 /etc/x3/jury/jury.local.env
```

#### 4. Install Systemd Service
```bash
# Copy service file
sudo cp jury.service /etc/systemd/system/

# Reload systemd daemon
sudo systemctl daemon-reload

# Enable service to start on boot
sudo systemctl enable jury

# Start service
sudo systemctl start jury

# Verify status
sudo systemctl status jury

# View logs
sudo journalctl -u jury -f
```

### Systemd Management

```bash
# Check status
sudo systemctl status jury

# Start/stop/restart
sudo systemctl start jury
sudo systemctl stop jury
sudo systemctl restart jury

# View live logs
sudo journalctl -u jury -f -n 100

# View logs from specific time
sudo journalctl -u jury --since "2026-02-08 10:00:00"

# View errors only
sudo journalctl -u jury -p err

# Test configuration without starting
sudo systemctl show jury -p ExecStart

# Get detailed failure info
sudo systemctl status jury --full --no-truncate
```

### Environment File Format (`/etc/x3/jury/jury.env`)

```bash
# Database
DATABASE_URL=postgresql://jury_admin:password@localhost:5432/jury_audit

# Redis
REDIS_URL=redis://:password@localhost:6379/0

# Service
JURY_LOG_LEVEL=INFO
JURY_API_PORT=8000

# On-chain (Phase 4)
ONCHAIN_RPC_URL=http://localhost:9944
```

### Health Checks

```bash
# Check API health
curl http://localhost:8000/health

# Check database connection
sudo systemctl exec jury psql $DATABASE_URL -c "SELECT 1"

# Monitor process
ps aux | grep jury
systemctl show jury --property=MainPID
```

---

## Production Deployment

### Architecture

```
┌─────────────────────────────────────────────────────┐
│ Load Balancer (nginx/haproxy)                       │
│ - SSL termination                                   │
│ - Health check routing                              │
│ - Rate limiting                                     │
└──────────────┬──────────────────────────────────────┘
               │
       ┌───────┴────────┬────────────┐
       │                │            │
   ┌─────────┐   ┌─────────┐   ┌─────────┐
   │ Jury 1  │   │ Jury 2  │   │ Jury 3  │
   │ :8000   │   │ :8001   │   │ :8002   │  (pool)
   └────┬────┘   └────┬────┘   └────┬────┘
        │             │             │
        └─────────────┼─────────────┘
                      │
         ┌────────────┴────────────┐
         │                         │
    ┌──────────┐           ┌──────────┐
    │PostgreSQL│           │  Redis   │
    │ Primary  │           │ Sentinel │
    │          │           │          │
    └──────────┘           └──────────┘
         │                      │
    ┌────────┐           (HA Replication)
    │   Hot  │
    │Standby │
    └────────┘
```

### 1. Pre-Deployment Checklist

```bash
# SSL certificates ready
test -f /etc/ssl/certs/jury-service.crt
test -f /etc/ssl/private/jury-service.key

# Firewall rules configured
sudo ufw allow 8000/tcp  # API
sudo ufw allow 5432/tcp  # PostgreSQL (internal only)
sudo ufw allow 6379/tcp  # Redis (internal only)

# Backups configured
crontab -e  # Add: 0 2 * * * /opt/x3/backup-jury.sh

# Monitoring configured
# - Prometheus scrape targets
# - AlertManager rules
# - Log aggregation (ELK/Datadog)
```

### 2. Database Setup

```bash
# On production database server:

# Create database
sudo -u postgres psql << EOF
CREATE USER jury_admin WITH PASSWORD 'production_secure_password';
CREATE DATABASE jury_audit OWNER jury_admin;
GRANT CONNECT ON DATABASE jury_audit TO jury_admin;
EOF

# Import schema
psql -U jury_admin -d jury_audit < sql-init/01-init-schema.sql

# Create readonly user
psql -U jury_admin -d jury_audit << EOF
CREATE USER jury_readonly WITH PASSWORD 'readonly_secure_password';
GRANT SELECT ON ALL TABLES IN SCHEMA public TO jury_readonly;
EOF

# Configure replication (for HA)
# See PostgreSQL documentation for streaming replication
```

### 3. Redis Setup

```bash
# Install Redis
sudo apt-get install redis-server

# Configure for production
sudo tee /etc/redis/redis.conf << EOF
requirepass production_secure_password
maxmemory 2gb
maxmemory-policy allkeys-lru
appendonly yes
appendfilename "appendonly.aof"
EOF

# Restart and verify
sudo systemctl restart redis-server
redis-cli ping
```

### 4. Service Deployment

```bash
# Deploy application
cd /opt/x3/jury
git clone https://github.com/Cyptopimpinainteazy/x3-chain.git .

# Install to required location
sudo cp -r swarm /opt/x3/jury/

# Create venv and install dependencies
cd /opt/x3/jury
python3 -m venv venv
venv/bin/pip install -r swarm/requirements.txt

# Copy and configure systemd service
sudo cp jury.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable jury

# Configure environment
sudo cp jury.env.example /etc/x3/jury/jury.env
sudo nano /etc/x3/jury/jury.env  # Edit for production

# Start service
sudo systemctl start jury
sudo systemctl status jury
```

### 5. Load Balancer Configuration (nginx example)

```nginx
# /etc/nginx/sites-available/jury-service

upstream jury_backend {
    least_conn;
    server localhost:8000 max_fails=3 fail_timeout=30s;
    server localhost:8001 max_fails=3 fail_timeout=30s;
    server localhost:8002 max_fails=3 fail_timeout=30s;
    keepalive 32;
}

server {
    listen 443 ssl http2;
    server_name jury.x3-chain.io;

    ssl_certificate /etc/ssl/certs/jury-service.crt;
    ssl_certificate_key /etc/ssl/private/jury-service.key;
    ssl_protocols TLSv1.3 TLSv1.2;
    ssl_ciphers HIGH:!aNULL:!MD5;

    # Rate limiting
    limit_req_zone $binary_remote_addr zone=jury_limit:10m rate=100r/s;
    limit_req zone=jury_limit burst=200 nodelay;

    location / {
        proxy_pass http://jury_backend;
        proxy_http_version 1.1;
        proxy_set_header Connection "";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # Timeouts
        proxy_connect_timeout 5s;
        proxy_send_timeout 30s;
        proxy_read_timeout 30s;
    }

    location /health {
        proxy_pass http://jury_backend/health;
        access_log off;
    }

    location /metrics {
        proxy_pass http://jury_backend/metrics;
        auth_basic "Metrics";
        auth_basic_user_file /etc/nginx/.htpasswd;
    }
}
```

---

## Monitoring & Observability

### Prometheus Metrics

Enable metrics collection:
```bash
docker-compose --profile observability up -d jury-metrics
```

Access Prometheus dashboard: `http://localhost:9090`

**Key Metrics to Monitor:**
- `jury_session_total` - Total sessions created
- `jury_votes_submitted` - Vote submissions
- `jury_quorum_checks` - Quorum verification results
- `jury_api_request_duration_seconds` - API latency
- `jury_db_query_duration_seconds` - Database query performance

### Logging

**JSON structured logging** to stdout:

```bash
# View formatted logs
docker-compose logs -f jury-service | jq .

# Filter by level
docker-compose logs jury-service | jq 'select(.level=="ERROR")'

# Filter by field
docker-compose logs jury-service | jq 'select(.session_id=="...")'
```

**Log Aggregation (ELK Stack):**
```yaml
# fluent-bit configuration
output:
  elasticsearch:
    host: elasticsearch.example.com
    port: 9200
    index: jury-service
    match: jury.*
```

### Alerting Rules

```yaml
# prometheus-rules.yml
groups:
  - name: jury-alerts
    rules:
      - alert: JuryServiceDown
        expr: up{job="jury-service"} == 0
        for: 5m
        annotations:
          summary: "Jury service is down"

      - alert: HighAPILatency
        expr: histogram_quantile(0.95, jury_api_request_duration_seconds) > 1
        for: 10m
        annotations:
          summary: "High API latency detected"

      - alert: LowQuorumRate
        expr: rate(jury_quorum_checks_failed[5m]) > 0.1
        for: 15m
        annotations:
          summary: "Quorum failures increasing"

      - alert: DatabaseConnectionPoolExhausted
        expr: jury_db_pool_available == 0
        for: 2m
        annotations:
          summary: "Database connection pool exhausted"
```

---

## Troubleshooting

### Common Issues

#### 1. Service won't start
```bash
# Check systemd logs
sudo journalctl -u jury -n 50

# Verify environment file
sudo cat /etc/x3/jury/jury.env | grep -E "DATABASE|REDIS"

# Test database connection
PGPASSWORD=... psql -h localhost -U jury_admin -d jury_audit -c "SELECT 1"

# Check file permissions
ls -la /opt/x3/jury /var/log/jury
```

#### 2. High database latency
```bash
# Check active connections
psql -U jury_admin -d jury_audit -c "\d+"

# Check slow queries
psql -U jury_admin -d jury_audit << EOF
SELECT query, calls, mean_time FROM pg_stat_statements 
ORDER BY mean_time DESC LIMIT 10;
EOF

# Analyze query plans
EXPLAIN ANALYZE SELECT * FROM jury_sessions WHERE state='COMPLETED';
```

#### 3. Redis connection failures
```bash
# Test connection
redis-cli -h localhost -p 6379 -a <password> ping

# Check memory usage
redis-cli INFO memory

# Monitor commands
redis-cli MONITOR

# Clear cache if needed
redis-cli FLUSHDB
```

#### 4. API response timeouts
```bash
# Check service CPU/memory
top -p $(pgrep -f "python -m swarm.api_server")

# Check concurrent sessions
curl http://localhost:8000/metrics | grep jury_session_active

# Increase timeouts in nginx
proxy_read_timeout 60s;
proxy_connect_timeout 10s;
```

---

## Security Considerations

### Passwords & Secrets

```bash
# Generate secure random passwords
openssl rand -base64 32

# Store in secure location
sudo chmod 600 /etc/x3/jury/jury.env
sudo chmod 600 /etc/x3/jury/jury.local.env

# Rotate credentials
# 1. Generate new password
# 2. Update database user: ALTER USER jury_admin WITH PASSWORD 'new_password'
# 3. Update systemd env file
# 4. Restart service
```

### Network Security

```bash
# Restrict database access to jury service only
sudo ufw default DENY INCOMING
sudo ufw allow from 127.0.0.1 to any port 5432
sudo ufw allow from 127.0.0.1 to any port 6379
sudo ufw allow 8000/tcp  # API endpoint

# Enable firewall
sudo ufw enable
```

### SSL/TLS

```bash
# Generate self-signed certificate (dev only)
sudo openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
  -keyout /etc/ssl/private/jury-service.key \
  -out /etc/ssl/certs/jury-service.crt

# Use Let's Encrypt for production
sudo apt-get install certbot python3-certbot-nginx
sudo certbot certonly --nginx -d jury.x3-chain.io
```

### Audit & Compliance

```bash
# Monitor audit logs
sudo journalctl -u jury --output json | jq '. | select(.MESSAGE | contains("ERROR"))'

# Export audit trail
SELECT * FROM audit_logs WHERE timestamp > now() - interval '7 days' ORDER BY timestamp DESC;

# Archive old audit logs
psql -U jury_admin -d jury_audit << EOF
CREATE TABLE audit_logs_archive_2026_01 AS 
  SELECT * FROM audit_logs WHERE timestamp < '2026-02-01'::date;
DELETE FROM audit_logs WHERE timestamp < '2026-02-01'::date;
EOF
```

---

## References

- [Docker Compose Documentation](https://docs.docker.com/compose/)
- [Systemd Manual](https://www.freedesktop.org/software/systemd/man/)
- [PostgreSQL Documentation](https://www.postgresql.org/docs/)
- [Redis Documentation](https://redis.io/documentation)
- [Prometheus Monitoring](https://prometheus.io/docs/)
- [nginx Reverse Proxy Guide](https://nginx.org/en/docs/)

---

**Last Updated:** 2026-02-08
**Status:** Production Ready ✅
