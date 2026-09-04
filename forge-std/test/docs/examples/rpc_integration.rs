//! Practical Integration Examples for RPC Configuration
//!
//! This file demonstrates how to integrate the RPC and environment configuration
//! modules with the X3 Settlement Engine and external chain adapters.

use external_chains::{
    rpc::{arbitrum_mainnet_config, create_default_registry, ChainRpcConfig, RpcRegistry},
    env_config::{EnvConfig, NetworkEnv, WalletConfig},
    ChainType,
};

/// Example 1: Initialize RPC configuration for settlement engine
pub fn example_settlement_engine_init() {
    // Load environment variables
    let env_config = EnvConfig::from_env();
    println!("Network: {}", env_config.network.as_str());
    println!("Chain ID: {}", env_config.network.chain_id());

    // Get primary RPC endpoint for the configured network
    if let Some(primary_rpc) = env_config.primary_rpc() {
        println!("Primary RPC: {}", primary_rpc);
    }

    // Get all fallback endpoints
    let fallbacks = env_config.fallback_rpcs();
    println!("Fallback RPCs: {:?}", fallbacks);

    // Load wallet configuration if available
    if let Some(wallet) = &env_config.wallet {
        println!("Wallet address: {}", wallet.address);
        // NEVER log private keys!
    }
}

/// Example 2: Get Arbitrum mainnet configuration
pub fn example_arbitrum_config() {
    let arb_config = arbitrum_mainnet_config();

    println!("=== {} ===", arb_config.chain_name);
    println!("Chain ID: {}", arb_config.chain_id);
    println!("Available RPC endpoints: {}", arb_config.rpc_endpoints.len());
    println!("WebSocket endpoints: {}", arb_config.ws_endpoints.len());
    println!("Block explorer: {}", arb_config.block_explorer_url);
    println!("Average block time: {}ms", arb_config.average_block_time_ms);
    println!("Finality depth: {} blocks", arb_config.finality_depth);

    // Show primary RPC endpoint
    if let Some(primary) = arb_config.primary_rpc() {
        println!("\nPrimary RPC endpoint:");
        println!("  URL: {}", primary.url);
        println!("  Priority: {}", primary.priority);
        println!("  Timeout: {}ms", primary.timeout_ms);
        println!("  Max retries: {}", primary.max_retries);
    }
}

/// Example 3: Select best flashloan provider
pub fn example_select_flashloan_provider() {
    let arb_config = arbitrum_mainnet_config();
    let amount_needed = 1_000_000_000_000_000_000u128; // 1 ETH in wei

    // Get all available providers
    let providers = arb_config.enabled_flashloans();
    println!("Available flash loan providers: {}", providers.len());

    // Find cheapest provider
    let cheapest = providers.iter().min_by_key(|p| p.fee_bps);
    if let Some(provider) = cheapest {
        if provider.max_liquidity >= amount_needed {
            println!(
                "Best provider: {} ({}bps fee)",
                provider.name, provider.fee_bps
            );
            println!("Contract: {}", provider.contract_address);
            println!("Max liquidity: {}", provider.max_liquidity);
        } else {
            println!(
                "{} doesn't have sufficient liquidity",
                provider.name
            );
        }
    }

    // List all providers with fees
    println!("\nAll providers:");
    for provider in providers {
        println!(
            "  {} - {}bps, Liquidity: {}",
            provider.name, provider.fee_bps, provider.max_liquidity
        );
    }
}

/// Example 4: Get DEX routers for routing
pub fn example_dex_routers() {
    let arb_config = arbitrum_mainnet_config();

    println!("Available DEX routers for swaps:");
    for dex in arb_config.enabled_dexes() {
        println!("\n  {}", dex.name);
        println!("  Protocol: {}", dex.protocol);
        println!("  Router: {}", dex.router_address);
        if let Some(factory) = &dex.factory_address {
            println!("  Factory: {}", factory);
        }
    }
}

/// Example 5: Multi-chain registry access
pub fn example_multi_chain_registry() {
    // Create registry with all pre-configured chains
    let registry = create_default_registry();

    println!("Supported chain IDs: {:?}", registry.supported_chain_ids());

    // Access each chain's configuration
    for chain_id in registry.supported_chain_ids() {
        if let Some(config) = registry.get(chain_id) {
            println!("\nChain: {} ({})", config.chain_name, config.chain_id);
            println!("  RPCs: {}", config.rpc_endpoints.len());
            println!("  Flash Loans: {}", config.flashloan_providers.len());
            println!("  DEXs: {}", config.dex_routers.len());
            println!("  Block time: {}ms", config.average_block_time_ms);
        }
    }
}

/// Example 6: Settlement engine RPC initialization
pub fn example_settlement_engine_rpc_init() {
    // Load environment
    let env = EnvConfig::from_env();

    // Get RPC config for active network
    let rpc_config = match env.network {
        NetworkEnv::Arbitrum => arbitrum_mainnet_config(),
        _ => arbitrum_mainnet_config(), // default to arbitrum
    };

    // Validate RPC connectivity (would be async in real code)
    println!("Settlement Engine RPC Configuration:");
    println!("Network: {}", env.network.as_str());
    println!("Chain: {}", rpc_config.chain_name);

    // Primary RPC for proof verification
    if let Some(primary) = rpc_config.primary_rpc() {
        println!("Proof verification RPC: {}", primary.url);
    }

    // Secondary RPCs for failover
    let all_rpcs: Vec<_> = rpc_config
        .rpc_endpoints
        .iter()
        .take(3)
        .map(|r| &r.url)
        .collect();
    println!("Fallback RPCs: {:?}", all_rpcs);

    // Flash loan config for atomic swaps
    let flashloans = rpc_config.enabled_flashloans();
    println!("Flash loan providers available: {}", flashloans.len());
}

/// Example 7: Build settlement proof URL
pub fn example_settlement_proof_verification() {
    let arb_config = arbitrum_mainnet_config();

    // For verifying settlement proofs on Arbitrum
    let block_hash = "0xabc123...";
    let tx_hash = "0xdef456...";

    // Use block explorer URL
    let explorer_url = format!(
        "{}/tx/{}",
        arb_config.block_explorer_url, tx_hash
    );
    println!("View settlement TX: {}", explorer_url);

    // RPC endpoint for state verification
    if let Some(rpc) = arb_config.primary_rpc() {
        println!("State verification RPC: {}", rpc.url);
    }
}

/// Example 8: Configure gasPrice multipliers for L2
pub fn example_gas_price_multipliers() {
    // Different L2s have different gas characteristics
    let multipliers = vec![
        ("Arbitrum", 1.1),  // Slightly higher than reported
        ("Base", 1.2),      // More conservative
        ("Optimism", 1.1),
        ("zkSync Era", 1.05), // Very stable
    ];

    for (chain, multiplier) in multipliers {
        println!("{}: {} x reported gas price", chain, multiplier);
    }

    // In real code, load from env or config
    // ARBITRUM_GAS_MULTIPLIER=1.1
    // BASE_GAS_MULTIPLIER=1.2
    // etc.
}

/// Example 9: Arbitrage detection using multiple DEX routers
pub fn example_arbitrage_opportunity_detection() {
    let arb_config = arbitrum_mainnet_config();

    println!("Arbitrage opportunity parameters:");
    println!("Supported swap routes:");

    for dex1 in arb_config.enabled_dexes() {
        for dex2 in arb_config.enabled_dexes() {
            if dex1.router_address != dex2.router_address {
                println!("  {:<15} -> {}", dex1.name, dex2.name);
            }
        }
    }

    // Flash loan for capital
    let flashloans = arb_config.enabled_flashloans();
    println!("\nFlash loan providers for MEV sandwiching:");
    for provider in flashloans {
        println!(
            "  {} - {}bps (max: {})",
            provider.name, provider.fee_bps, provider.max_liquidity
        );
    }
}

/// Example 10: Fallback RPC coordination
pub fn example_rpc_failover_strategy() {
    let env = EnvConfig::from_env();
    let arb_config = arbitrum_mainnet_config();

    println!("RPC Selection Strategy:");
    println!("1. Try primary (highest priority)");
    if let Some(primary) = arb_config.primary_rpc() {
        println!("   {}", primary.url);
    }

    println!("2. Fallback to secondary (if primary fails)");
    for rpc in arb_config.rpc_endpoints.iter().skip(1).take(2) {
        println!("   {} (priority: {})", rpc.url, rpc.priority);
    }

    println!("3. Timeout and retry configuration");
    if let Some(rpc) = arb_config.primary_rpc() {
        println!("   Timeout: {}ms", rpc.timeout_ms);
        println!("   Max retries: {}", rpc.max_retries);
    }

    println!("4. Circuit breaker after max failures");
    println!("   Disable failing RPC for 5 minutes");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arbitrum_config_integrity() {
        let config = arbitrum_mainnet_config();
        assert_eq!(config.chain_id, 42161);
        assert!(!config.rpc_endpoints.is_empty());
        assert!(!config.flashloan_providers.is_empty());
        assert!(!config.dex_routers.is_empty());
    }

    #[test]
    fn test_env_config_loading() {
        let env = EnvConfig::from_env();
        assert_eq!(env.network, NetworkEnv::Arbitrum);
        assert!(env.primary_rpc().is_some());
    }

    #[test]
    fn test_registry_contains_all_chains() {
        let registry = create_default_registry();
        let chain_ids = registry.supported_chain_ids();
        assert!(chain_ids.contains(&42161)); // Arbitrum
        assert!(chain_ids.contains(&8453));  // Base
        assert!(chain_ids.contains(&137));   // Polygon
    }

    #[test]
    fn test_flashloan_provider_selection() {
        let config = arbitrum_mainnet_config();
        let providers = config.enabled_flashloans();
        
        // Should have multiple providers configured
        assert!(providers.len() >= 3);
        
        // All providers should have contracts and fees
        for provider in providers {
            assert!(!provider.contract_address.is_empty());
            assert!(provider.fee_bps > 0);
            assert!(provider.max_liquidity > 0);
        }
    }

    #[test]
    fn test_dex_router_configuration() {
        let config = arbitrum_mainnet_config();
        let dexes = config.enabled_dexes();
        
        // Should have multiple DEX routers
        assert!(dexes.len() >= 4);
        
        // Verify Uniswap V3 is configured
        let uniswap = dexes.iter().find(|d| d.name == "Uniswap V3");
        assert!(uniswap.is_some());
    }
}

// Run examples with: cargo test --example rpc_integration -- --nocapture
fn main() {
    println!("=== RPC Configuration Examples ===\n");

    println!("1. Settlement Engine Init:");
    example_settlement_engine_init();

    println!("\n2. Arbitrum Config Overview:");
    example_arbitrum_config();

    println!("\n3. Flash Loan Provider Selection:");
    example_select_flashloan_provider();

    println!("\n4. Available DEX Routers:");
    example_dex_routers();

    println!("\n5. Multi-Chain Registry:");
    example_multi_chain_registry();

    println!("\n10. RPC Failover Strategy:");
    example_rpc_failover_strategy();
}
