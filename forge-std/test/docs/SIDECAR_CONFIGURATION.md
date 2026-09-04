# X3 Sidecar Configuration Guide

This guide documents all configuration options for the X3 Sidecar daemon, an off-chain execution node for X3 Chain benchmarking.

## Table of Contents

- [Environment Variables](#environment-variables)
- [Configuration File](#configuration-file)
- [Runtime Configuration](#runtime-configuration)
- [Validation](#validation)

## Environment Variables

The sidecar reads configuration from environment variables following the standard pattern: `COMPONENT_SETTING`. All environment variables are optional unless marked as **Required**.

### Core Settings

| Variable | Type | Default | Description | Required |
|----------|------|---------|-------------|----------|
| `RUST_LOG` | string | `info` | Logging level (error, warn, info, debug, trace) | No |
| `SIDECAR_PORT` | u16 | `8080` | HTTP server listen port | No |
| `SIDECAR_HOST` | string | `0.0.0.0` | HTTP server listen address | No |

### Gateway Configuration

| Variable | Type | Default | Description | Required |
|----------|------|---------|-------------|----------|
| `GATEWAY_URL` | string | `http://localhost:3001` | Gateway endpoint (base URL) | **Yes** |
| `GATEWAY_AUTH_TOKEN` | string | (none) | Bearer token for gateway authentication | No |
| `GATEWAY_MAX_RETRIES` | u32 | `3` | Maximum retry attempts for failed submissions | No |
| `GATEWAY_BACKOFF_MS` | u64 | `100` | Initial backoff duration in milliseconds (exponential) | No |

### Database Configuration

| Variable | Type | Default | Description | Required |
|----------|------|---------|-------------|----------|
| `DATABASE_URL` | string | (none) | PostgreSQL connection string (format: `postgres://user:pass@host:port/db`) | **Yes** |
| `DATABASE_POOL_SIZE` | u32 | `10` | Connection pool size | No |
| `DATABASE_TIMEOUT_SECS` | u64 | `30` | Connection timeout in seconds | No |

### Benchmarking Configuration

| Variable | Type | Default | Description | Required |
|----------|------|---------|-------------|----------|
| `BENCHMARK_CHAIN_NAME` | string | `ethereum` | Target chain name (e.g., `ethereum`, `polygon`, `arbitrum`) | No |
| `BENCHMARK_TIMEOUT_SECS` | u64 | `300` | Benchmark execution timeout in seconds | No |
| `BENCHMARK_WORKLOAD_TYPE` | string | `mixed` | Workload profile (e.g., `transfer`, `swap`, `mixed`) | No |

## Configuration File

The sidecar can also read configuration from a TOML file at `./config/sidecar.toml` or via `CONFIG_PATH` environment variable:

```toml
[logging]
level = "info"
format = "json"

[server]
host = "0.0.0.0"
port = 8080

[gateway]
url = "http://gateway:3001"
auth_token = "your-bearer-token"
max_retries = 3
initial_backoff_ms = 100

[database]
url = "postgres://user:password@localhost:5432/x3_benchmark"
pool_size = 10
timeout_secs = 30

[benchmark]
chain_name = "ethereum"
timeout_secs = 300
workload_type = "mixed"
```

**Priority**: Environment variables override configuration file settings.

## Runtime Configuration

### Startup Sequence

1. **Load Configuration**
   - Read `CONFIG_PATH` if set, else use `./config/sidecar.toml`
   - Override with environment variables
   - Validate all required fields

2. **Initialize Logging**
   - Set up structured logging with configured level
   - Enable JSON formatting if `LOG_FORMAT=json`

3. **Connect to Database**
   - Establish PostgreSQL connection pool
   - Run migrations automatically if enabled
   - Verify schema is up-to-date

4. **Connect to Gateway**
   - Perform health check: `GET /health`
   - Verify authentication if token is provided
   - Store gateway configuration for submission

5. **Start Benchmark Loop**
   - Begin polling for new benchmarks (or wait for API requests)
   - Initialize result submission queue
   - Start metrics collection

### Gateway Integration

The sidecar communicates with the gateway via HTTP:

```
POST /api/v1/benchmarks/results

Content-Type: application/json
Authorization: Bearer {GATEWAY_AUTH_TOKEN}

{
  "tenant_id": "tenant-123",
  "report": {
    "report_id": "uuid",
    "chain_name": "ethereum",
    "chain_type": "evm",
    "recommendation": "sidecar_mode",
    "baseline": {
      "avg_tps": 100.0,
      "p50_latency_ms": 200,
      ...
    },
    ...
  }
}
```

Response:
```json
{
  "report_id": "uuid",
  "status": "accepted"
}
```

### Database Schema

The sidecar requires these tables (automatically created via migrations):

- `benchmark_reports` - Stores submitted benchmark results
- `benchmark_metrics` - Time-series metrics data
- `tenants` - Tenant configuration and quotas
- `submission_queue` - Pending submissions awaiting retry

## Validation

### Startup Validation

The sidecar validates configuration on startup:

1. **Required Fields**
   - `GATEWAY_URL` must be a valid HTTP URL
   - `DATABASE_URL` must be a valid PostgreSQL connection string
   - All numeric values must be within acceptable ranges

2. **Range Checks**
   - `SIDECAR_PORT`: 1–65535
   - `GATEWAY_MAX_RETRIES`: 1–10
   - `GATEWAY_BACKOFF_MS`: 10–10000
   - `DATABASE_POOL_SIZE`: 1–100
   - `DATABASE_TIMEOUT_SECS`: 1–300
   - `BENCHMARK_TIMEOUT_SECS`: 10–3600

3. **Connectivity Checks**
   - Database connection pool verified
   - Gateway health check (`GET /health`) succeeded

### Configuration Errors

If validation fails during startup, the sidecar logs detailed errors and exits with code `1`:

```
ERROR: Missing required configuration: GATEWAY_URL not set
ERROR: Validation failed for configuration:
  - SIDECAR_PORT: invalid value 99999 (must be 1-65535)
  - GATEWAY_MAX_RETRIES: invalid value 15 (must be 1-10)
```

## Example Deployment

### Minimal Configuration

```bash
export GATEWAY_URL="http://gateway.example.com:3001"
export DATABASE_URL="postgres://user:pass@db.example.com:5432/x3_benchmark"
export RUST_LOG="info"

./x3-sidecar
```

### Production Configuration

```bash
# Gateway
export GATEWAY_URL="https://gateway.prod.example.com"
export GATEWAY_AUTH_TOKEN="$(cat /run/secrets/gateway_token)"
export GATEWAY_MAX_RETRIES="5"
export GATEWAY_BACKOFF_MS="200"

# Database
export DATABASE_URL="$(cat /run/secrets/db_url)"
export DATABASE_POOL_SIZE="20"
export DATABASE_TIMEOUT_SECS="60"

# Logging
export RUST_LOG="warn"
export LOG_FORMAT="json"

# Benchmarking
export BENCHMARK_CHAIN_NAME="ethereum"
export BENCHMARK_TIMEOUT_SECS="600"

./x3-sidecar
```

### Docker Deployment

```dockerfile
FROM rust:1.75 as builder
WORKDIR /build
COPY . .
RUN cargo build --release --bin x3-sidecar

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/x3-sidecar /usr/local/bin/
EXPOSE 8080
CMD ["x3-sidecar"]
```

Run with:
```bash
docker run \
  -e GATEWAY_URL=http://gateway:3001 \
  -e DATABASE_URL=postgres://db:5432/x3 \
  -p 8080:8080 \
  x3-sidecar:latest
```

## Next Steps

- See [SIDECAR_DEPLOYMENT.md](SIDECAR_DEPLOYMENT.md) for deployment procedures
- See [SIDECAR_TROUBLESHOOTING.md](SIDECAR_TROUBLESHOOTING.md) for common issues
