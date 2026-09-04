# X3 Chain Mainnet RPC Endpoints

This document describes the configured mainnet RPC endpoints for cross-chain operations.

## Solana Mainnet Configuration

### Provider: DRPC (Decentralized RPC Network)

**WebSocket Endpoint:**
```
wss://lb.drpc.live/solana/ArgUBy0RzURpos-Jlz1TqLRxbgscV2AR8JXZrqRhf0fE
```

**HTTPS Endpoint:**
```
https://lb.drpc.live/solana/ArgUBy0RzURpos-Jlz1TqLRxbgscV2AR8JXZrqRhf0fE
```

### Configuration Details

- **Network:** Solana mainnet-beta
- **Cluster ID:** 1 (configured in `crates/svm-integration/src/lib.rs`)
- **Provider:** DRPC load-balanced endpoint
- **Max Retries:** 3
- **Timeout:** 30 seconds
- **Rate Limit:** 100 requests/second

## File Locations

### Configuration Files

1. **`infra/mainnet-rpc-endpoints.toml`**
   - Primary RPC endpoint configuration
   - Network settings and connection parameters
   - Monitoring and health check settings

2. **`infra/SECRETS.example.env`**
   - Environment variable template
   - Includes SOLANA_MAINNET_WSS and SOLANA_MAINNET_HTTPS
   - Copy to `.env` for local development

3. **`crates/svm-integration/src/lib.rs`**
   - SvmConfig default cluster_id set to 1 (mainnet)
   - Integration code for SVM execution

## Usage

### Development Environment

1. Copy the example environment file:
   ```bash
   cp infra/SECRETS.example.env .env
   ```

2. The endpoints are already configured in `.env`:
   ```bash
   SOLANA_MAINNET_WSS=wss://lb.drpc.live/solana/ArgUBy0RzURpos-Jlz1TqLRxbgscV2AR8JXZrqRhf0fE
   SOLANA_MAINNET_HTTPS=https://lb.drpc.live/solana/ArgUBy0RzURpos-Jlz1TqLRxbgscV2AR8JXZrqRhf0fE
   ```

### Runtime Configuration

The SVM integration crate (`crates/svm-integration`) uses cluster_id = 1 for mainnet operations:

```rust
impl Default for SvmConfig {
    fn default() -> Self {
        Self {
            compute_unit_limit: 200_000,
            compute_unit_price: 1,
            block_height: 0,
            block_timestamp: 0,
            cluster_id: 1,  // Solana mainnet-beta
        }
    }
}
```

### Cross-Chain Bridge Operations

The RPC endpoints are used for:

1. **SVM Transaction Execution**: Submit Solana transactions through DRPC
2. **State Verification**: Query Solana state for dual-VM verification
3. **Account Updates**: Monitor Solana account changes
4. **Block Finality**: Wait for 32 confirmations before bridging

## Security Considerations

- ✅ Endpoints use HTTPS/WSS for encrypted communication
- ✅ Load-balanced infrastructure for high availability
- ✅ Rate limiting configured to prevent abuse
- ⚠️ Store API keys in `.env`, never commit to repository
- ⚠️ Use fallback endpoints for production redundancy

## Testing

Test the endpoints with curl:

```bash
# Test HTTPS endpoint
curl -X POST https://lb.drpc.live/solana/ArgUBy0RzURpos-Jlz1TqLRxbgscV2AR8JXZrqRhf0fE \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'
```

Expected response:
```json
{"jsonrpc":"2.0","result":"ok","id":1}
```

## Monitoring

Monitor endpoint health through:

- **Health checks**: Every 60 seconds (configured in `mainnet-rpc-endpoints.toml`)
- **Metrics**: Enabled for request/response tracking
- **Alerts**: Configured to notify on endpoint failures

## Future Enhancements

- [ ] Add Ethereum mainnet endpoints (Infura/Alchemy)
- [ ] Configure fallback Solana endpoints
- [ ] Implement automatic failover logic
- [ ] Add metrics dashboard for RPC monitoring
- [ ] Set up alerting for endpoint downtime

## Support

For DRPC support and documentation:
- Website: https://drpc.org
- Documentation: https://docs.drpc.org
- Status: https://status.drpc.org
