# LLM Router Prometheus Monitoring Guide

## Overview

The LLM Router includes integrated Prometheus metrics for production monitoring. Metrics are exposed in standard Prometheus format and can be visualized with Grafana.

## Quick Start

### Option 1: Local Node.js (Development)

```bash
# Start router with metrics exporter
NODE_ENV=development node llm-service/start-with-metrics.js

# In another terminal, start Prometheus
docker run -p 9090:9090 \
  -v $(pwd)/prometheus.yml:/etc/prometheus/prometheus.yml \
  prom/prometheus

# In another terminal, start Grafana
docker run -p 3001:3000 \
  -e GF_SECURITY_ADMIN_PASSWORD=admin \
  grafana/grafana
```

### Option 2: Docker Compose (Production)

```bash
docker-compose -f docker-compose.monitoring.yml up -d
```

Services will be available at:
- **LLM Router**: http://localhost:3000
- **Metrics Endpoint**: http://localhost:9090/metrics
- **Prometheus UI**: http://localhost:9090
- **Grafana**: http://localhost:3001

## Metrics Reference

### Query Metrics

#### `llm_queries_total`
**Type**: Counter  
**Labels**: `provider`, `model`  
**Description**: Total number of LLM queries processed  

```promql
# Queries per second per provider
rate(llm_queries_total[5m])

# Total queries for a specific provider
llm_queries_total{provider="ollama"}
```

#### `llm_query_errors_total`
**Type**: Counter  
**Labels**: `provider`, `model`  
**Description**: Total number of failed queries  

```promql
# Error rate per provider
rate(llm_query_errors_total[5m])

# Success rate
1 - (increase(llm_query_errors_total[5m]) / increase(llm_queries_total[5m]))
```

#### `llm_query_duration_seconds`
**Type**: Histogram  
**Labels**: `provider`, `model`, `le`  
**Description**: Query latency in seconds with histogram buckets  

```promql
# 95th percentile latency
histogram_quantile(0.95, rate(llm_query_duration_seconds_bucket[5m]))

# 99th percentile latency
histogram_quantile(0.99, rate(llm_query_duration_seconds_bucket[5m]))

# Average latency
rate(llm_query_duration_seconds_sum[5m]) / rate(llm_query_duration_seconds_count[5m])
```

#### `tokens_processed_total`
**Type**: Counter  
**Labels**: `provider`, `model`  
**Description**: Total tokens processed by each provider  

```promql
# Tokens per second
rate(tokens_processed_total[5m])

# Total tokens for a provider
tokens_processed_total{provider="ollama"}
```

### HTTP Metrics

#### `http_requests_total`
**Type**: Counter  
**Labels**: `endpoint`, `status_code`  
**Description**: Total HTTP requests to router endpoints  

```promql
# Requests per second
rate(http_requests_total[5m])

# Requests to /query endpoint
http_requests_total{endpoint="/query"}

# Error rate (4xx + 5xx)
rate(http_requests_total{status_code=~"[45].."}[5m])
```

#### `http_request_duration_seconds`
**Type**: Histogram  
**Labels**: `endpoint`, `le`  
**Description**: HTTP request latency  

```promql
# 95th percentile HTTP latency
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))
```

### Availability Metrics

#### `provider_availability`
**Type**: Gauge  
**Labels**: `provider`  
**Description**: Provider health status (1=available, 0=unavailable)  

```promql
# Ollama availability
provider_availability{provider="ollama"}

# OpenRouter availability
provider_availability{provider="openrouter"}
```

#### `process_uptime_seconds`
**Type**: Gauge  
**Description**: Router process uptime in seconds  

```promql
process_uptime_seconds
```

## Useful PromQL Queries

### Dashboard Queries

**Success Rate (Last 5 minutes)**
```promql
1 - (increase(llm_query_errors_total[5m]) / increase(llm_queries_total[5m]))
```

**Average Query Time**
```promql
rate(llm_query_duration_seconds_sum[5m]) / rate(llm_query_duration_seconds_count[5m])
```

**Queries per Provider**
```promql
sum by (provider) (rate(llm_queries_total[5m]))
```

**Error Rate per Provider**
```promql
sum by (provider) (rate(llm_query_errors_total[5m])) / sum by (provider) (rate(llm_queries_total[5m]))
```

**Model Performance Comparison**
```promql
histogram_quantile(0.95, sum by (model, le) (rate(llm_query_duration_seconds_bucket[5m])))
```

**Total Tokens Processed**
```promql
sum(tokens_processed_total)
```

**Token Throughput (tokens/sec)**
```promql
sum(rate(tokens_processed_total[5m]))
```

## Grafana Setup

### Import Dashboard

1. Open Grafana: http://localhost:3001
2. Login with `admin/admin`
3. Import the LLM dashboard:
   - Click **Dashboards** → **Import**
   - Paste content of `grafana-llm-dashboard.json`
   - Select Prometheus as data source
   - Click **Import**

### Create Custom Alerts

Example: Alert when error rate > 5%

```
Alert Name: High Error Rate
Condition: 
  1 - (increase(llm_query_errors_total[5m]) / increase(llm_queries_total[5m])) < 0.95
Threshold: 1
For duration: 5m
```

### Useful Grafana Panels

1. **Graph**: Query throughput over time
   ```promql
   rate(llm_queries_total[1m])
   ```

2. **Gauge**: Current success rate
   ```promql
   1 - (increase(llm_query_errors_total[5m]) / increase(llm_queries_total[5m]))
   ```

3. **Heatmap**: Query latency distribution
   ```promql
   llm_query_duration_seconds_bucket
   ```

4. **Table**: Top models by query count
   ```promql
   topk(10, sum by (model) (increase(llm_queries_total[1h])))
   ```

## Prometheus Configuration

The `prometheus.yml` file scrapes metrics from:

- **LLM Router Metrics**: http://127.0.0.1:9090/metrics (every 15s)
- **Ollama API**: http://127.0.0.1:11434/api/ps (every 30s)

To add more targets, edit `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'llm-router'
    scrape_interval: 15s
    static_configs:
      - targets: ['127.0.0.1:9090']
```

## Performance Tuning

### High Query Volume (>100 req/sec)

Increase Prometheus retention:
```bash
# In prometheus.yml or docker run:
--storage.tsdb.retention.time=7d
--storage.tsdb.max-block-duration=2h
```

### Memory Usage

Limit series cardinality:
```yaml
global:
  external_labels:
    # Reduce dimensional bloat
    scrape_interval: 30s  # Instead of 15s
```

### Dashboarding Performance

- Use recording rules to pre-compute common queries
- Limit Dashboard refresh rate to 30s minimum
- Archive old data to external storage

## Troubleshooting

### Metrics endpoint returns empty

```bash
# Check if exporter is running
curl http://localhost:9090/metrics

# Check router logs
tail -f /tmp/router.log
```

### Prometheus not scraping

```bash
# Check targets
curl http://localhost:9090/api/v1/targets

# Check for errors in Prometheus logs
docker logs substreams-prometheus
```

### Grafana can't connect to Prometheus

1. Check datasource URL: http://prometheus:9090 (in Docker) or http://127.0.0.1:9090 (local)
2. Verify Prometheus is running and accessible
3. Check network connectivity between containers

## Production Deployment

For production use:

1. **Use persistent storage**: Configure Prometheus volume mounts
2. **Enable authentication**: Use reverse proxy with auth
3. **Set up alerting**: Configure AlertManager
4. **Monitor monitor**: Set up separate monitoring for Prometheus/Grafana
5. **Backup metrics**: Export regularly to external storage

Example Docker Compose with volumes and networking:

```yaml
prometheus:
  volumes:
    - prometheus_data:/prometheus
  networks:
    - monitoring
networks:
  monitoring:
    driver: bridge
```

## API Reference

### Metrics Endpoint

```bash
# Get all metrics
curl http://localhost:9090/metrics

# Get specific metric
curl http://localhost:9090/metrics | grep llm_queries_total
```

### Health Check

```bash
curl http://localhost:9090/health
# Returns: {"status":"healthy","timestamp":"2026-02-12T10:15:30.123Z"}
```

## See Also

- [LLM Integration Guide](./substreams-skills-llm-integration.md)
- [Prometheus Documentation](https://prometheus.io/docs)
- [Grafana Documentation](https://grafana.com/docs)
- [Router Client Documentation](./QUICK_START_LLM.md)
