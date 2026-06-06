# Deployment Guide

## Prerequisites

- Rust 1.70+
- Redis 6.0+
- CUDA Toolkit 11.0+ (for GPU support)
- Solana CLI (for testnet interaction)
- Ethereum JSON-RPC endpoint

## Installation

### 1. Build from source

```bash
cd crates/cross-chain-gpu-validator
cargo build --release
```

### 2. Configure environment

```bash
export REDIS_URL="redis://localhost:6379"
export SOLANA_RPC="https://api.devnet.solana.com"
export ETH_RPC="https://goerli.infura.io/v3/YOUR_KEY"
export VALIDATOR_PORT="8080"
export RUST_LOG="info"
```

### 3. Start Redis

```bash
redis-server
```

### 4. Run validator

```bash
./target/release/cross-chain-gpu-validator
```

## Testnet Deployment

Run the automated deployment script:

```bash
bash deployment/deploy_testnet.sh
```

This script:
1. Verifies dependencies
2. Builds the validator
3. Checks Redis availability
4. Starts the validator service
5. Outputs metrics endpoint URL

## Configuration

### Redis

- Connection: `redis://hostname:port/db`
- TTL: 3600 seconds (swaps auto-expire)
- Persistence: Enabled for swap record durability

### RPC Endpoints

- **Solana Devnet**: https://api.devnet.solana.com
- **Ethereum Goerli**: https://goerli.infura.io/v3/YOUR_PROJECT_ID
- **Ethereum Sepolia**: https://sepolia.infura.io/v3/YOUR_PROJECT_ID

### Validator Options

- `--batch-size` (32): GPU batch size for signature verification
- `--timeout-secs` (60): Swap timeout in seconds
- `--max-snapshots` (1000): Dashboard metric history size

## Monitoring

### Metrics Endpoint

```bash
curl http://localhost:8080/metrics
```

### Dashboard

Open browser: `http://localhost:8080/dashboard`

Displays:
- Live TPS (transactions per second)
- Atomic success rate
- Rollback and timeout counts
- GPU health status
- RPC latency

### Logs

Logs are directed to stderr with RUST_LOG level control:

```bash
RUST_LOG=debug ./target/release/cross-chain-gpu-validator
```

## Troubleshooting

### Redis Connection Failed

```
ValidatorError: Redis connection failed: Connection refused
```

**Solution**: Ensure Redis is running on the configured host/port:
```bash
redis-cli ping  # Should respond PONG
```

### GPU Initialization Failed

```
ValidatorError: GPU initialization failed: CUDA not available
```

**Solution**: Validator falls back to CPU only. This is expected if no GPU is available.

### Swap Timeout Exceeded

**Symptom**: Multiple `SwapStatus::TimedOut` in metrics

**Solution**: Increase `--timeout-secs` or check RPC latency with:
```bash
time curl -X POST $ETH_RPC -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'
```

## Performance Tuning

### Maximize GPU Throughput

- Increase `--batch-size` to 256+ if GPU memory permits
- Ensure CUDA driver is up-to-date
- Monitor GPU utilization: `nvidia-smi`

### Reduce Latency

- Use local Redis instance
- Deploy validator close to RPC endpoints
- Increase worker thread count if available

## Rollout Strategy

### Phase 1: Testnet Validation
1. Deploy on Solana Devnet + Ethereum Goerli
2. Run benchmark suite (2-4M TPS target)
3. Monitor metrics for 24+ hours
4. Verify zero atomic violations

### Phase 2: Staging
1. Deploy on Solana Devnet + Ethereum Sepolia
2. Simulate high-frequency swaps
3. Test failover scenarios
4. Verify timeout handling

### Phase 3: Mainnet (future)
1. Deploy on Solana Mainnet + Ethereum Mainnet
2. Gradual traffic ramp-up
3. 24/7 monitoring
4. Incident response procedures in place
