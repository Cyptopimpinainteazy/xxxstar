# RPC Configuration & Chain Connector Integration Guide

## Overview

This guide explains how to use the chain connector framework with the new RPC modules (`rpc.rs` and `env_config.rs`) to manage external chain connections for the X3 Chain settlement engine.

## Quick Start (Arbitrum Mainnet)

### 1. Configuration Files

The RPC setup is split into two modules:

- **`rpc.rs`**: Defines RPC endpoints, WebSocket connections, flash loan providers, and DEX routers
- **`env_config.rs`**: Loads environment variables and provides wallet/network configuration

### 2. Basic Usage in Rust Code

```rust
use external_chains::{
    rpc::{arbitrum_mainnet_config, create_default_registry},
    env_config::{EnvConfig, NetworkEnv},
    ChainAdapter, ChainType,
};

// Get Arbitrum configuration
let arbitrum_config = arbitrum_mainnet_config();
println!("Arbitrum RPC endpoints: {}", arbitrum_config.rpc_endpoints.len());
println!("Flash loan providers: {}", arbitrum_config.flashloan_providers.len());

// Get environment configuration
let env = EnvConfig::from_env();
println!("Network: {}", env.network.as_str());
println!("Primary RPC: {:?}", env.primary_rpc());

// Create registry with all chains
let registry = create_default_registry();
let arbitrum = registry.get(42161).expect("Arbitrum config");
println!("Chain: {}", arbitrum.chain_name);
```

### 3. Environment Setup

Copy `.env.example` to `.env` and update credentials:

```bash
cp .env.example .env
```

Key configurations:
```env
NETWORK=arbitrum
ALCHEMY_API_KEY=Fe5T2pGsX76ml9kDCwVRZhtmkdixfrDQ
DRPC_API_KEY=ArgUBy0RzURpos-Jlz1TqLRxbgscV2AR8JXZrqRhf0fE
ANKR_API_KEY=648269110992d35fb12b490f3e9d00e18141ad9212081909344f15ec1c342a3c
PRIVATE_KEY=480c2f0730a4b305123b759f2a20ceb701643116671b232ffd5cdcbb90d4431a
```

## Architecture

### Module Hierarchy

```
external-chains (crate)
├── adapter.rs          (ChainAdapter trait - base interface)
├── chains/             (Chain-specific implementations)
│   ├── base.rs
│   ├── arbitrum.rs
│   └── ...
├── rpc.rs              (RPC configuration data structures)
│   ├── RpcEndpoint     (HTTP RPC with priority/timeout/retries)
│   ├── WsEndpoint      (WebSocket subscriptions)
│   ├── FlashLoanProvider
│   ├── DexRouter
│   └── ChainRpcConfig  (Complete chain setup)
├── env_config.rs       (Environment variable loading)
│   ├── NetworkEnv      (arbitrum/base/polygon/etc)
│   ├── EnvConfig       (All loaded settings)
│   └── WalletConfig    (Private key + address)
└── lib.rs              (Public API exports)
```

### RPC Endpoint Selection Strategy

When connecting to a chain:

1. **Primary RPC**: Highest priority (set via `.with_priority(100)`)
2. **Fallback RPCs**: Lower priority, used if primary fails
3. **Load Balancing**: Requests are distributed across available endpoints
4. **Retry Logic**: Configurable max retries (default: 3) with exponential backoff

Example from code:
```rust
// Alchemy (priority 100) is primary
.add_rpc(RpcEndpoint::new("https://arb-mainnet.g.alchemy.com/v2/ALCH_KEY")
    .with_priority(100))
// DRPC is secondary fallback
.add_rpc(RpcEndpoint::new("https://lb.drpc.org/arbitrum/DRPC_KEY")
    .with_priority(90))
// Ankr is tertiary fallback
.add_rpc(RpcEndpoint::new("https://rpc.ankr.com/arbitrum/ANKR_KEY")
    .with_priority(80))
```

## Flash Loan Provider Integration

### Supported Providers (Arbitrum)

| Provider | Fee | Max Liquidity | Contract Address |
|----------|-----|---------------|------------------|
| Aave V3 | 0.03% | $100M+ | 0x794a61358... |
| Balancer V2 | 0.05% | $250M+ | 0xBA12222... |
| Radiant Capital | 0.04% | $50M+ | 0xF4B1486... |
| dForce | 0.05% | $40M+ | 0x0988f3C... |

### Accessing Flash Loan Configuration

```rust
let arbitrum = arbitrum_mainnet_config();

// Get all enabled providers
let providers = arbitrum.enabled_flashloans();
for provider in providers {
    println!("{}: {}bps fee, liquidity: {}", 
        provider.name, 
        provider.fee_bps, 
        provider.max_liquidity
    );
}

// Total liquidity available
let total_liquidity: u128 = providers
    .iter()
    .map(|p| p.max_liquidity)
    .sum();
println!("Total liquidity: {}", total_liquidity);
```

## DEX Router Integration

### Supported DEX Routers (Arbitrum)

```rust
let arbitrum = arbitrum_mainnet_config();

// Get all enabled DEX routers
for dex in arbitrum.enabled_dexes() {
    println!("DEX: {} ({})", dex.name, dex.protocol);
    if let Some(factory) = &dex.factory_address {
        println!("  Factory: {}", factory);
    }
    println!("  Router: {}", dex.router_address);
}
```

Routers include:
- **Uniswap V3** (AMM) - Factory: 0x1F98431...
- **SushiSwap V2** - Router: 0x1b02dA8...
- **Camelot V2** - Factory: 0x6EcCab4...
- **Trader Joe V2** - Factory: 0x8e42f2F...
- **Ramses V2** - Factory: 0xAAA20D0...
- **Chronos** - Factory: 0xCEFb89f...

## Integration with Settlement Engine

### In `x3-settlement-engine` Pallet

```rust
use external_chains::{
    rpc::arbitrum_mainnet_config,
    env_config::EnvConfig,
};

pub fn initialize_chain_rpc() -> Result<(), Error> {
    let env = EnvConfig::from_env();
    let rpc_config = arbitrum_mainnet_config();
    
    // Store in pallet storage
    <ChainRpcConfig<T>>::put(rpc_config);
    <EnvironmentConfig<T>>::put(env);
    
    // Verify connectivity (can be async in a pallet task)
    if let Some(primary) = rpc_config.primary_rpc() {
        log::info!("Chain RPC initialized: {}", primary.url);
    }
    
    Ok(())
}
```

### Event Integration

```rust
#[pallet::event]
pub enum Event<T: Config> {
    RpcConnected { chain_id: u64, endpoint: Vec<u8> },
    RpcDisconnected { chain_id: u64 },
    FlashloanProviderStatusUpdated { provider: Vec<u8>, available: bool },
}

// Emit when RPC connects
Self::deposit_event(Event::RpcConnected {
    chain_id: 42161,
    endpoint: b"https://arb-mainnet...".to_vec(),
});
```

## Advanced: Custom Network Configuration

### Add a New Chain to Registry

```rust
use external_chains::rpc::{ChainRpcConfig, RpcEndpoint, WsEndpoint, FlashLoanProvider};

let custom_polygon = ChainRpcConfig::new(137, "Polygon PoS")
    .add_rpc(RpcEndpoint::new("https://polygon.llamarpc.com")
        .with_priority(100)
        .with_timeout(20000))
    .add_ws(WsEndpoint::new("wss://polygon.llamarpc.com"))
    .add_flashloan(FlashLoanProvider::new(
        "Aave V3 Polygon",
        "0x794a61358D6845594F94dc1DB02A252b5b4814aD",
        3,
    ).with_liquidity(50_000_000 * 10u128.pow(18)))
    .with_explorer("https://polygonscan.com")
    .with_block_time(2000)
    .with_finality(128);
```

### Easy Partner Onboarding (Validated)

For partner chains that need a fast, safe onboarding path, use
`ChainConfig::onboard_external_chain(...)` and `validate()` from
`crates/external-chains/src/adapter.rs`.

See `docs/EXTERNAL_CHAIN_PARTNER_ONBOARDING.md` for the complete integration checklist and fee policy.

### Multi-Network Arbitrage Configuration

```rust
use external_chains::rpc::create_default_registry;

let registry = create_default_registry();

// Iterate across all chains
for chain_config in registry.all_chains() {
    println!("Chain: {}", chain_config.chain_name);
    println!("  RPCs: {}", chain_config.rpc_endpoints.len());
    println!("  Flash Loans: {}", chain_config.flashloan_providers.len());
    println!("  DEXs: {}", chain_config.dex_routers.len());
}
```

## Performance and Reliability

### RPC Endpoint Selection Criteria

When choosing which RPC to use:

1. **Priority Ordering**: Higher priority endpoints tried first
2. **Response Time**: Fastest responding endpoint is preferred
3. **Failure Rate**: Track which endpoints fail most and deprioritize
4. **Availability**: Skip endpoints that are consistently down

### Timeout Configuration

```rust
// For settlement verification (slower needed)
.add_rpc(RpcEndpoint::new("https://arb-mainnet.g.alchemy.com/v2/KEY")
    .with_timeout(30000)      // 30 seconds for proof verification
    .with_retries(5))

// For fast price checks (snappy)
.add_rpc(RpcEndpoint::new("https://lb.drpc.org/arbitrum/KEY")
    .with_timeout(5000)       // 5 seconds for price feeds
    .with_retries(2))
```

### Handling Rate Limits

Each provider has different rate limits:

- **Alchemy**: 300 req/s (standard plan)
- **DRPC**: 100 req/s (free tier)
- **Ankr**: 50 req/s (free tier)
- **Public RPC**: 5-10 req/s (shared)

Configure retries and backoff:

```rust
RpcEndpoint::new(url)
    .with_retries(3)          // Retry 3 times
    .with_timeout(30000)      // 30s total timeout
```

## Testing

### Unit Tests

```bash
# Run RPC module tests
cargo test -p external-chains rpc --lib

# Test environment configuration
cargo test -p external-chains env_config --lib
```

### Example Test

```rust
#[test]
fn test_arbitrum_flashloan_config() {
    let config = arbitrum_mainnet_config();
    
    assert_eq!(config.chain_id, 42161);
    assert!(!config.flashloan_providers.is_empty());
    
    // Verify all providers are configured correctly
    for provider in &config.flashloan_providers {
        assert!(!provider.contract_address.is_empty());
        assert!(provider.fee_bps > 0);
    }
}
```

## Troubleshooting

### RPC Connection Issues

**Problem**: "Connection refused" on startup
```rust
// Check if primary RPC is available
if let Some(rpc) = arbitrum_config.primary_rpc() {
    println!("Attempting to connect to: {}", rpc.url);
    // Add connectivity check before using
}
```

**Solution**: Verify API keys in `.env` and test endpoints manually:
```bash
curl https://arb-mainnet.g.alchemy.com/v2/Fe5T2pGsX76ml9kDCwVRZhtmkdixfrDQ \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'
```

### Flash Loan Provider Not Available

```rust
// Check if provider is enabled and has liquidity
let providers = arbitrum.enabled_flashloans();
let aave = providers.iter().find(|p| p.name == "Aave V3");

if let Some(aave) = aave {
    if aave.max_liquidity > amount_needed {
        println!("Can use Aave V3");
    } else {
        println!("Insufficient liquidity, try another provider");
    }
}
```

### Wallet Configuration Not Loaded

Ensure `.env` file exists and is readable:
```bash
ls -la .env
grep PRIVATE_KEY .env
```

## Related Documentation

- [X3 Settlement Engine](/pallets/x3-settlement-engine/docs/root/README.md)
- [External Chain Adapters](/crates/external-chains/docs/root/README.md)
- [Arbitrage Bot Setup](/docs/arbitrage-bot-setup.md)
- [Environment Configuration Reference](#environment-reference)

## Support

For issues or questions:
1. Check `.env.example` for correct format
2. Test RPC endpoints manually with curl
3. Review logs: `RUST_LOG=external_chains=debug cargo run`
4. See `/crates/external-chains/tests/` for example configurations
