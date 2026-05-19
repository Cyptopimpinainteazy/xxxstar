# X3 Chain TPS Testing Infrastructure

Real-time **Transactions Per Second (TPS)** tracking and visualization for the X3 Chain blockchain platform.

This implementation adapts the [Solana TPS tracking project](https://github.com/amil13/solana_project.git) for X3 Chain, providing:

- **Rust Backend** - Efficiently polls blockchain for transaction metrics
- **InfluxDB** - High-performance time-series data storage
- **Streamlit Dashboard** - Interactive real-time visualization
- **Integration** - Aligns with x3-chain test invariants

## Quick Start

### Prerequisites

- Docker & Docker Compose
- Rust toolchain (for building TPS tracker)
- Python 3.10+
- X3 Chain node running on `127.0.0.1:9944` (or set `RPC_URL`)

### Run TPS Testing

```bash
cd /home/lojak/Desktop/x3-chain-master

# Start the complete TPS testing infrastructure
python3 tests/perf/run_tps_testing.py

# Or with options
python3 tests/perf/run_tps_testing.py --no-browser --no-logs
```

This will:
1. Build the TPS tracker crate
2. Start InfluxDB, TPS tracker, and Streamlit dashboard in Docker
3. Open the dashboard at `http://localhost:8501`
4. Stream logs in your terminal

### Stop Services

```bash
python3 tests/perf/run_tps_testing.py --stop

# Or manually
docker-compose -f tests/perf/docker-compose.tps.yml down
```

## Architecture

### Components

#### 1. TPS Tracker (Rust Crate)
**Location:** `crates/tps-tracker/`

Polls the X3 Chain RPC endpoint and collects:
- Block height
- Transaction count per block
- Calculated TPS
- Block timestamps

**Key files:**
- `src/lib.rs` - Core tracking logic
- `src/main.rs` - Binary entry point
- `Dockerfile` - Container build spec

**Configuration (env vars):**
```bash
RPC_URL=http://127.0.0.1:9944          # X3 Chain RPC endpoint
INFLUX_URL=http://localhost:8086       # InfluxDB connection
INFLUX_DB=x3_chain_tps             # Database name
INFLUX_TOKEN=x3-chain-key          # Auth token
POLL_INTERVAL=1                        # Poll interval in seconds
BUFFER_SIZE=100                        # Batch write size to InfluxDB
```

#### 2. InfluxDB
Time-series database for storing TPS metrics.

**Container:** `x3-chain-influxdb`
**Port:** 8086
**Database:** `x3_chain_tps`
**Retention:** 30 days

Data stored includes:
- `transaction_stats` measurement with fields: `tps`, `transaction_count`, `block_height`, `block_time_seconds`

#### 3. Streamlit Dashboard
**Location:** `tests/perf/tps-dashboard/`

Interactive web interface for visualizing TPS metrics.

**Container:** `x3-chain-tps-dashboard`
**Port:** 8501
**URL:** http://localhost:8501

**Features:**
- Real-time TPS trend chart
- Moving averages
- Block metrics visualization
- Statistical summaries
- Auto-refresh capability

**Key files:**
- `dashboard.py` - Main application
- `requirements.txt` - Python dependencies
- `Dockerfile` - Container build spec

## Docker Compose Stack

See `tests/perf/docker-compose.tps.yml` for complete infrastructure definition.

Services:
- `influxdb` - Time-series database
- `tps-tracker` - TPS collection service
- `dashboard` - Streamlit web interface

Network: `x3-chain-perf` (isolated)
Storage: `influxdb-storage` (persistent volume)

## Testing

### Unit Tests (TPS Tracker)

```bash
cd crates/tps-tracker
cargo test
```

Tests verify:
- TPS calculation logic
- Configuration handling
- Basic tracker initialization

### Integration Tests

Run the dashboard and monitor metrics:

```bash
# Start infrastructure
python3 tests/perf/run_tps_testing.py --no-logs

# In another terminal, verify metrics are flowing
curl http://localhost:8086/query?db=x3_chain_tps&q="SELECT * FROM transaction_stats LIMIT 10"
```

### Test Invariants

TPS testing integrates with x3-chain invariants (see `tests/invariants/registry.toml`):

- **TPS-001**: Blockchain maintains consistent block production
- **TPS-002**: Transaction metrics are accurately recorded
- **TPS-003**: No data loss in InfluxDB writes during normal operation
- **NET-001**: Network remains stable during high-throughput testing

Reference these in test code:
```rust
#[test]
#[invariant("TPS-001")]
fn test_block_production_rate() {
    // Test implementation
}
```

## Usage Scenarios

### Scenario 1: Baseline Performance Testing

Establish baseline TPS under normal conditions:

```bash
# Start monitoring
python3 tests/perf/run_tps_testing.py

# Let it run for 30 minutes (data retention is 30 days)
# View dashboard at http://localhost:8501

# Analyze statistics for:
# - Average TPS
# - Peak TPS
# - Transaction throughput consistency
# - Block production rate
```

### Scenario 2: Load Testing Integration

Combine with load testing tools like k6:

```bash
# In terminal 1: Start TPS tracking
python3 tests/perf/run_tps_testing.py --no-logs

# In terminal 2: Run k6 load test
k6 run tests/perf/k6/1k_tps_test.js --env TARGET_URL=http://127.0.0.1:9944

# Monitor TPS metrics in dashboard as load increases
```

### Scenario 3: Network Upgrade Validation

Compare TPS before/after network upgrades:

```bash
# Before upgrade
python3 tests/perf/run_tps_testing.py --no-logs
# Let run for 5 minutes, grab baseline stats

# Perform upgrade...

# After upgrade
# Dashboard will show new metrics for comparison
```

## Performance Metrics

### Dashboard Metrics

The dashboard tracks:

| Metric | Description | Unit |
|--------|-------------|------|
| Current TPS | Latest transaction rate | tx/sec |
| Average TPS | Mean TPS over time window | tx/sec |
| Peak TPS | Maximum TPS observed | tx/sec |
| Latest Block | Current block height | # |
| Total Blocks | Blocks in time window | # |
| Avg Txs/Block | Mean transactions per block | tx |
| Total Txs | Total transactions in window | # |

### Data Points Collected

Per block:
- Block height
- Transaction count
- Block timestamp
- Calculated TPS
- Block time duration

## Troubleshooting

### Dashboard shows "No data available"

1. Verify RPC connection:
```bash
curl -X POST http://127.0.0.1:9944 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"system_health","params":[]}'
```

2. Check TPS tracker logs:
```bash
docker logs x3-chain-tps-tracker
```

3. Verify InfluxDB is running:
```bash
docker logs x3-chain-influxdb
```

### InfluxDB connection refused

Ensure InfluxDB is healthy:
```bash
docker ps | grep influxdb
curl http://localhost:8086/health
```

If not running:
```bash
docker-compose -f tests/perf/docker-compose.tps.yml up -d influxdb
```

### High memory usage

InfluxDB memory can be tuned:
```bash
# In docker-compose.tps.yml, adjust under influxdb.environment:
INFLUXDB_MEMORY_MAX: 2g
```

### RPC timeout errors

If `Cannot connect to RPC endpoint`:
1. Verify node is running: `ps aux | grep node`
2. Check RPC port is exposed: `netstat -tulpn | grep 9944`
3. Update RPC_URL environment variable for Docker

## Development

### Adding New Metrics

To track additional metrics in the TPS tracker:

1. Add field to `TransactionStats` struct in `src/lib.rs`
2. Update `get_block_info()` method to fetch the data
3. Store in InfluxDB write
4. Update dashboard query in `dashboard.py`

Example:
```rust
#[derive(InfluxDbWriteable)]
struct TransactionStats {
    // ... existing fields
    finality_time_ms: f64,  // New field
}
```

### Building Docker Images Manually

```bash
# TPS Tracker
cd crates/tps-tracker
docker build -t x3-chain-tps-tracker:latest .

# Dashboard
cd tests/perf/tps-dashboard
docker build -t x3-chain-tps-dashboard:latest .
```

## References

- **Original Project:** [Solana TPS Dashboard](https://github.com/amil13/solana_project.git) by Amil Shrivastava
- **InfluxDB Documentation:** https://docs.influxdata.com/
- **Streamlit Documentation:** https://docs.streamlit.io/
- **X3 Chain Tests:** See `docs/tests/README.md` and `tests/invariants/registry.toml`

## License

This TPS tracking implementation follows the X3 Chain project license terms.
