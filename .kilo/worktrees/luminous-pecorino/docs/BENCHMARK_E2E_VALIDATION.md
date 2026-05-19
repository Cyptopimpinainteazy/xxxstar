# X3 Benchmark E2E Validation Guide

## Overview

This guide describes the end-to-end (E2E) real-service validation for the benchmark result flow. The validation proves that the complete pipeline works as expected:

```
POST /benchmarks/jobs (sidecar RPC)
  ↓
Benchmark execution
  ↓
Report generation (BenchmarkReport)
  ↓
GatewayClient::submit_benchmark_result()
  ↓
HTTP POST to gateway:/api/v1/benchmarks/results
  ↓
Gateway auth validation (Bearer token)
  ↓
PostgreSQL: INSERT INTO benchmark_reports
```

## Components

### Docker Services

The E2E environment consists of three services defined in `docker-compose.benchmark-e2e.yml`:

1. **PostgreSQL** (localhost:5432)
   - Database: `x3_indexer`
   - User: `gateway`/`gateway`
   - Schema: Auto-migrated from `crates/x3-gateway/migrations/`
   - Contains table: `benchmark_reports`

2. **X3 Gateway** (localhost:8080)
   - REST API with endpoints:
     - `POST /api/v1/benchmarks/results` - Publish benchmark report
     - `GET /health` - Health check
   - Auth: Bearer token in `Authorization` header
   - Token: `benchmark-secret-token-e2e` (for E2E testing)
   - Database: PostgreSQL at `postgres:5432`

3. **X3 Sidecar** (localhost:9955 RPC, localhost:9956 metrics)
   - RPC endpoint: `POST /rpc` for benchmark job submission
   - Metrics endpoint: `GET /metrics`
   - Gateway integration: Publishes reports to `http://x3-gateway:8080`
   - Auth token: `benchmark-secret-token-e2e`
   - Config file: `/etc/x3/sidecar.toml`

### Configuration Files

- **docker-compose.benchmark-e2e.yml**: Service definitions with env vars
- **docker-config/sidecar-e2e.toml**: Sidecar configuration for E2E testing
- **crates/x3-gateway/Dockerfile**: Gateway image build
- **crates/x3-sidecar/Dockerfile**: Sidecar image build

### Test Infrastructure

- **crates/x3-sidecar/tests/e2e_gateway_integration.rs**: Comprehensive test suite
- **scripts/benchmark-e2e.sh**: CLI tool for managing E2E validation

## Quick Start

### 1. Full Validation (One Command)

```bash
./scripts/benchmark-e2e.sh full
```

This will:
- Start all services
- Run tests
- Test negative paths
- Query the database
- Stop all services

### 2. Manual Step-by-Step

```bash
# Start services
./scripts/benchmark-e2e.sh up

# Check health
./scripts/benchmark-e2e.sh health

# Run tests
./scripts/benchmark-e2e.sh test

# Submit a benchmark job
./scripts/benchmark-e2e.sh submit

# Query database for stored reports
./scripts/benchmark-e2e.sh query-db

# View logs
./scripts/benchmark-e2e.sh logs x3-gateway

# Test negative scenarios
./scripts/benchmark-e2e.sh invalid-token

# Stop services
./scripts/benchmark-e2e.sh down
```

## Expected Output

### Service Startup

```
[INFO] Starting E2E benchmark services...
[✓] Docker is available
[✓] docker-compose is available
[✓] Services started (containers may still be initializing)
[INFO] Waiting for PostgreSQL on port 5432 (timeout: 60s)...
[✓] PostgreSQL is ready
[INFO] Waiting for Gateway on port 8080 (timeout: 60s)...
[✓] Gateway is ready
[INFO] Waiting for Sidecar on port 9955 (timeout: 60s)...
[✓] Sidecar is ready
[✓] All services are healthy
```

### Health Check

```
[INFO] Checking service health...
[✓] PostgreSQL is running on port 5432
[✓] Gateway is running on port 8080
[✓] Sidecar metrics are running on port 9955
```

### Test Results

```
[INFO] Running E2E tests...
[INFO] Running E2E benchmark integration tests...
running 5 tests
test benchmark::tests::benchmark_store_persists_job_and_report ... ok
test benchmark::tests::benchmark_store_executes_job_and_generates_report ... ok
test rpc::tests::benchmark_routes_submit_and_fetch_report ... ok
test rpc::tests::benchmark_publish_requires_gateway_configuration ... ok
test benchmark::tests::benchmark_store_submits_result_to_gateway_with_retry ... ok

test result: ok. 5 passed; 0 failed; 0 ignored

[✓] Tests passed
```

### Database Query

```
[INFO] Querying PostgreSQL for stored benchmark reports...
             report_id              |       tenant_id       | chain_name |           generated_at           
------------------------------------+---------------------+------------+----------------------------------
 e2e-test-001                       | e2e-test-tenant      | ethereum   | 2024-01-01 12:00:00+00
 e2e-test-002                       | e2e-test-tenant      | ethereum   | 2024-01-01 12:05:00+00
(2 rows)
```

## Testing Scenarios

### Positive Path: Successful Benchmark Submission

**Flow:**
1. Submit benchmark job via sidecar RPC
2. Job executes and generates report
3. Report published to gateway with valid token
4. Gateway stores report in PostgreSQL
5. Query confirms report was persisted

**Validation Steps:**
```bash
./scripts/benchmark-e2e.sh up
./scripts/benchmark-e2e.sh submit
./scripts/benchmark-e2e.sh query-db
./scripts/benchmark-e2e.sh down
```

**Expected Result:**
```
✓ Benchmark job submitted
✓ Report published to gateway
✓ Report persisted in PostgreSQL
```

### Negative Path 1: Invalid Auth Token

**Test:** Submit report with invalid token, expect 401/403 rejection

**Command:**
```bash
./scripts/benchmark-e2e.sh invalid-token
```

**Expected Behavior:**
```
[INFO] Testing invalid auth token scenario...
[✓] Invalid token correctly rejected (as expected)
```

**Validation:**
- Gateway returns 401 Unauthorized or 403 Forbidden
- Report is NOT persisted in database
- Error is logged in gateway logs

### Negative Path 2: Gateway Down During Submission

**Test:** Sidecar should retry with exponential backoff when gateway is unavailable

**Manual Steps:**
```bash
# Terminal 1: Start services
./scripts/benchmark-e2e.sh up

# Terminal 2: Stop gateway
docker-compose -f docker-compose.benchmark-e2e.yml stop x3-gateway

# Terminal 3: Watch sidecar logs
./scripts/benchmark-e2e.sh logs x3-sidecar

# Terminal 2: Restart gateway after a few seconds
docker-compose -f docker-compose.benchmark-e2e.yml start x3-gateway

# Terminal 3: Verify eventual success in logs
```

**Expected Log Output (Sidecar):**
```
[WARN] Failed to reach gateway (attempt 1/3, backing off 100ms)
[WARN] Failed to reach gateway (attempt 2/3, backing off 200ms)
[INFO] Successfully published report to gateway (attempt 3/3)
```

**Validation:**
- Sidecar attempts retries with increasing delays
- After gateway recovery, report publishes successfully
- Report appears in database after recovery

### Negative Path 3: Gateway Permanently Unavailable

**Test:** Sidecar should exhaust retries and report failure

**Manual Steps:**
```bash
# Terminal 1: Start services
./scripts/benchmark-e2e.sh up

# Terminal 2: Stop gateway permanently
docker-compose -f docker-compose.benchmark-e2e.yml stop x3-gateway

# Terminal 3: Watch sidecar logs and try to submit
./scripts/benchmark-e2e.sh logs x3-sidecar &
./scripts/benchmark-e2e.sh submit

# Terminal 3: Observe final error
```

**Expected Behavior:**
- Sidecar retries 3 times with exponential backoff
- Final error is logged clearly with all retry attempts shown
- Report is NOT persisted (gateway never received it)

### Negative Path 4: Database Connection Failure

**Test:** Gateway should fail gracefully if PostgreSQL is unavailable

**Manual Steps:**
```bash
# Terminal 1: Start services
./scripts/benchmark-e2e.sh up

# Terminal 2: Stop PostgreSQL
docker-compose -f docker-compose.benchmark-e2e.yml stop postgres

# Terminal 3: Try to submit
./scripts/benchmark-e2e.sh submit

# Terminal 3: Observe gateway error response
```

**Expected Behavior:**
- Gateway returns 500 Internal Server Error
- Sidecar sees error and logs it
- Report is not persisted

## Key Files and Locations

### Source Files
```
crates/x3-sidecar/src/gateway_client.rs         - Benchmark publishing logic
crates/x3-sidecar/src/benchmark.rs              - Benchmark execution
crates/x3-sidecar/src/rpc.rs                    - RPC server (/benchmarks/jobs)
crates/x3-gateway/src/rest.rs                   - REST endpoints
crates/x3-gateway/src/db.rs                     - Database operations
crates/x3-gateway/migrations/                   - Database schema
crates/x3-rpc/src/benchmark.rs                  - RPC types (BenchmarkReport)
```

### Test Files
```
crates/x3-sidecar/tests/e2e_gateway_integration.rs  - Comprehensive E2E tests
```

### Configuration Files
```
docker-compose.benchmark-e2e.yml                - Service definitions
docker-config/sidecar-e2e.toml                  - Sidecar config
crates/x3-gateway/Dockerfile                    - Gateway container
crates/x3-sidecar/Dockerfile                    - Sidecar container
scripts/benchmark-e2e.sh                        - CLI validation tool
```

## Database Schema

### benchmark_reports Table

```sql
CREATE TABLE benchmark_reports (
    report_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    chain_name TEXT NOT NULL,
    chain_type TEXT NOT NULL,
    recommendation TEXT NOT NULL,
    signer TEXT NOT NULL,
    generated_at TIMESTAMPTZ NOT NULL,
    baseline_avg_tps DOUBLE PRECISION NOT NULL,
    baseline_p50_latency_ms BIGINT NOT NULL,
    baseline_p95_latency_ms BIGINT NOT NULL,
    baseline_p99_latency_ms BIGINT NOT NULL,
    baseline_failure_rate DOUBLE PRECISION NOT NULL,
    x3_avg_tps DOUBLE PRECISION NOT NULL,
    x3_p50_latency_ms BIGINT NOT NULL,
    x3_p95_latency_ms BIGINT NOT NULL,
    x3_p99_latency_ms BIGINT NOT NULL,
    x3_failure_rate DOUBLE PRECISION NOT NULL,
    projected_soft_confirmation_improvement TEXT NOT NULL,
    projected_app_throughput_improvement TEXT NOT NULL,
    projected_route_latency_delta TEXT NOT NULL,
    projected_bridge_latency_delta TEXT NOT NULL,
    workload_profile JSONB NOT NULL DEFAULT '{}',
    artifacts JSONB NOT NULL DEFAULT '[]'
);

CREATE INDEX benchmark_reports_tenant_generated_at_idx
    ON benchmark_reports (tenant_id, generated_at DESC);

CREATE INDEX benchmark_reports_chain_generated_at_idx
    ON benchmark_reports (chain_name, generated_at DESC);
```

### Query Examples

**Find all reports from a tenant:**
```sql
SELECT report_id, chain_name, x3_avg_tps, baseline_avg_tps, generated_at
FROM benchmark_reports
WHERE tenant_id = 'e2e-test-tenant'
ORDER BY generated_at DESC;
```

**Find reports with high TPS improvement:**
```sql
SELECT report_id, chain_name, baseline_avg_tps, x3_avg_tps,
       ROUND((x3_avg_tps - baseline_avg_tps) / baseline_avg_tps * 100, 2) as improvement_pct
FROM benchmark_reports
WHERE x3_avg_tps > baseline_avg_tps * 1.2
ORDER BY generated_at DESC;
```

**Check stored workload profiles:**
```sql
SELECT report_id, chain_name, workload_profile
FROM benchmark_reports
WHERE chain_name = 'ethereum'
LIMIT 5;
```

## Port Reference

| Service | Port | Purpose |
|---------|------|---------|
| PostgreSQL | 5432 | Database connection |
| Gateway REST API | 8080 | Benchmark report publishing, health checks |
| Sidecar RPC | 9955 | Benchmark job submission |
| Sidecar Metrics | 9956 | Prometheus metrics, health checks |

## Environment Variables

### Gateway (docker-compose.benchmark-e2e.yml)
- `DATABASE_URL=postgres://gateway:gateway@postgres:5432/x3_indexer`
- `GATEWAY_HOST=0.0.0.0`
- `GATEWAY_PORT=8080`
- `RUST_LOG=debug`
- `X3_GATEWAY_BENCHMARK_TOKEN=benchmark-secret-token-e2e`

### Sidecar (docker-compose.benchmark-e2e.yml)
- `RUST_LOG=debug`
- `X3_SIDECAR_RPC_PORT=9955`
- `X3_SIDECAR_METRICS_PORT=9956`

## Troubleshooting

### Services won't start
```bash
# Check Docker status
docker --version
docker-compose --version

# Check for port conflicts
lsof -i :5432
lsof -i :8080
lsof -i :9955

# Clean up and retry
./scripts/benchmark-e2e.sh clean
./scripts/benchmark-e2e.sh up
```

### Database connection fails
```bash
# Check PostgreSQL is running
docker ps | grep postgres

# Check logs
./scripts/benchmark-e2e.sh logs postgres

# Verify credentials
docker exec $(docker-compose -f docker-compose.benchmark-e2e.yml ps -q postgres) \
    psql -U gateway -d x3_indexer -c "SELECT 1"
```

### Gateway health check fails
```bash
# Check gateway logs
./scripts/benchmark-e2e.sh logs x3-gateway

# Test manually
curl http://localhost:8080/health

# Check if gateway can reach PostgreSQL
./scripts/benchmark-e2e.sh logs x3-gateway | grep -i postgres
```

### Tests fail
```bash
# Run with verbose output
cargo test -p x3-sidecar --lib benchmark -- --nocapture

# Check if services are healthy
./scripts/benchmark-e2e.sh health

# Check gateway token configuration
docker-compose -f docker-compose.benchmark-e2e.yml exec x3-gateway env | grep TOKEN
```

## Next Steps

This E2E validation covers **Step 1** of the 5-step Task 4 plan:

1. ✅ **Real-service E2E validation harness** (THIS STEP)
   - Docker-compose for services
   - E2E test suite
   - Negative path scenarios
   - Validation script

2. 🔜 **Deployment validation script** (Next)
   - Production-like configuration
   - Health monitoring
   - Operational readiness checks

3. 🔜 **Observability instrumentation**
   - Prometheus metrics
   - Structured logging
   - OpenTelemetry tracing

4. 🔜 **Concurrency/load testing**
   - 10x concurrent submissions
   - Sustained load scenarios
   - Resource utilization analysis

5. 🔜 **Operator documentation**
   - Runbooks
   - Troubleshooting guides
   - Production SOP

## References

- **Sidecar Config**: `docs/SIDECAR_CONFIGURATION.md`
- **Gateway REST API**: `crates/x3-gateway/src/rest.rs` (lines 79-81)
- **Benchmark Types**: `crates/x3-rpc/src/benchmark.rs`
- **Database Migrations**: `crates/x3-gateway/migrations/`
