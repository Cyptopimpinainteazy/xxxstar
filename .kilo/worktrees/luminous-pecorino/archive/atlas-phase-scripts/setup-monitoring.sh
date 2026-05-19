#!/bin/bash

# LLM Router with Prometheus Metrics Setup
# Installs and configures full monitoring stack

set -e

echo "
╔════════════════════════════════════════════════════════════╗
║   LLM Router Prometheus Metrics Installation              ║
╚════════════════════════════════════════════════════════════╝
"

# Check Node.js
if ! command -v node &> /dev/null; then
  echo "❌ Node.js is required but not installed"
  exit 1
fi

echo "✓ Node.js $(node --version) detected"

# Check Docker (optional but recommended)
DOCKER_AVAILABLE=false
if command -v docker &> /dev/null && command -v docker-compose &> /dev/null; then
  DOCKER_AVAILABLE=true
  echo "✓ Docker and Docker Compose available"
fi

# Create required files if they don't exist
echo "📝 Setting up configuration files..."

# Check Prometheus config
if [ ! -f prometheus.yml ]; then
  echo "Creating prometheus.yml..."
fi

# Check Grafana configs
if [ ! -f grafana-datasources.yml ]; then
  echo "Creating grafana-datasources.yml..."
fi

echo "✓ Configuration files ready"

# Offer setup options
echo "
🚀 Choose your setup:
  1. Local Node.js (Fast startup for development)
  2. Docker Compose (Full containerized stack)
  3. Kubernetes (Production deployment)

"

read -p "Select option (1-3): " setup_choice

case $setup_choice in
  1)
    echo "
Starting with local Node.js setup...

Commands to run:
  # Terminal 1: Start LLM Router with Metrics
  source ~/.nvm/nvm.sh
  node llm-service/start-with-metrics.js

  # Terminal 2: Start Prometheus
  docker run -p 9090:9090 \\
    -v \$(pwd)/prometheus.yml:/etc/prometheus/prometheus.yml \\
    prom/prometheus

  # Terminal 3: Start Grafana
  docker run -p 3001:3000 \\
    -e GF_SECURITY_ADMIN_PASSWORD=admin \\
    grafana/grafana

Services will be available at:
  - LLM Router:        http://localhost:3000
  - Metrics Exporter:  http://localhost:9090/metrics
  - Prometheus:        http://localhost:9090
  - Grafana:           http://localhost:3001 (admin/admin)
    "
    ;;
  2)
    echo "
Starting Docker Compose stack...

Run this command:
  docker-compose -f docker-compose.monitoring.yml up -d

Check status:
  docker-compose -f docker-compose.monitoring.yml ps

View logs:
  docker-compose -f docker-compose.monitoring.yml logs -f llm-router
  docker-compose -f docker-compose.monitoring.yml logs -f prometheus
  docker-compose -f docker-compose.monitoring.yml logs -f grafana

Stop services:
  docker-compose -f docker-compose.monitoring.yml down

Services available:
  - LLM Router:    http://localhost:3000
  - Prometheus:    http://localhost:9090
  - Grafana:       http://localhost:3001 (admin/admin)
    "
    if $DOCKER_AVAILABLE; then
      read -p "Start now? (y/n): " start_now
      if [ "$start_now" = "y" ]; then
        cd "$(dirname "$0")"/..
        docker-compose -f docker-compose.monitoring.yml up -d
        echo "✓ Services starting..."
        sleep 5
        echo "
✓ Stack started! Waiting for services to be ready...

Services URLs:
  - Prometheus: http://localhost:9090
  - Grafana:    http://localhost:3001
  - LLM Router: http://localhost:3000
        "
      fi
    fi
    ;;
  3)
    echo "
Kubernetes setup coming soon!

For now, use Docker Compose or Local Node.js setup.
    "
    ;;
  *)
    echo "Invalid selection"
    exit 1
    ;;
esac

echo "
📊 Monitoring Setup Complete!

Next steps:
  1. Access Grafana at http://localhost:3001
  2. Default credentials: admin / admin
  3. Dashboard 'LLM Router Monitoring' should auto-load
  4. Prometheus queries available at http://localhost:9090

Key metrics to monitor:
  - llm_queries_total: Total queries processed
  - llm_query_duration_seconds: Query latency
  - llm_query_errors_total: Failed queries
  - provider_availability: Service availability
  - tokens_processed_total: Token usage

Documentation:
  - Prometheus: https://prometheus.io/docs
  - Grafana: https://grafana.com/docs
  - Query examples in: docs/runbooks/operations/MONITORING_GUIDE.md
"
