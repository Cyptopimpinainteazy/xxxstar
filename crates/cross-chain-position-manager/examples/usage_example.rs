//! Cross-Chain Position Manager Usage Example
//!
//! This example demonstrates the currently supported public API surface.

use cross_chain_position_manager::{
    AllocationTarget, ChainConfig, ChainSpecifics, CrossChainPositionManager, PositionManagerConfig,
};
use sp_core::{H160, U256};

const BASE_CHAIN_ID: u64 = 8453;
const ARBITRUM_CHAIN_ID: u64 = 42161;
const POLYGON_CHAIN_ID: u64 = 137;
const AVALANCHE_CHAIN_ID: u64 = 43114;

#[tokio::main]
async fn main() -> cross_chain_position_manager::Result<()> {
    println!("🚀 Starting Cross-Chain Position Manager Example");

    let config = create_example_config();
    let mut manager = CrossChainPositionManager::new_with_config(config)?;

    manager.start().await?;
    println!("✅ Position Manager started successfully");

    let positions = manager.track_positions().await?;
    println!("Tracked positions: {}", positions.len());

    let summary = manager.get_portfolio_summary().await?;
    println!("Total Value: {}", summary.total_value_usd);
    println!("Risk Score: {:.2}/1.0", summary.risk_score);

    let dummy_position_id = cross_chain_position_manager::PositionId::new();
    let migration_result = manager
        .migrate_position(BASE_CHAIN_ID, ARBITRUM_CHAIN_ID, &dummy_position_id)
        .await?;
    println!(
        "Migration success: {}, id: {:?}",
        migration_result.success, migration_result.migration_id
    );

    let rebalance_result = manager.rebalance(&create_rebalance_targets()).await?;
    println!(
        "Rebalance success: {}, actions: {}",
        rebalance_result.success, rebalance_result.actions_executed
    );

    let opportunities = manager.evaluate_arbitrage().await?;
    println!("Arbitrage opportunities: {}", opportunities.len());

    let kill_switches = manager.check_kill_switches().await?;
    for trigger in kill_switches {
        println!(
            "Kill switch: chain={}, type={:?}, severity={:?}",
            trigger.chain_id, trigger.trigger_type, trigger.severity
        );
    }

    manager.stop().await?;
    println!("🛑 Position Manager stopped");

    Ok(())
}

fn create_example_config() -> PositionManagerConfig {
    let mut config = PositionManagerConfig::default();

    config
        .chain_configs
        .insert(BASE_CHAIN_ID, create_chain_config(BASE_CHAIN_ID, 1.2));
    config.chain_configs.insert(
        ARBITRUM_CHAIN_ID,
        create_chain_config(ARBITRUM_CHAIN_ID, 1.5),
    );
    config
        .chain_configs
        .insert(POLYGON_CHAIN_ID, create_chain_config(POLYGON_CHAIN_ID, 0.8));
    config.chain_configs.insert(
        AVALANCHE_CHAIN_ID,
        create_chain_config(AVALANCHE_CHAIN_ID, 1.0),
    );

    config.risk_config.max_position_size_usd = U256::from(100_000_000_000_000_000_000u128);
    config.risk_config.max_exposure_per_chain = 0.3;
    config.risk_config.max_correlation = 0.7;
    config.risk_config.liquidation_threshold = 0.8;
    config.risk_config.stop_loss_percentage = 0.1;

    config
}

fn create_chain_config(chain_id: u64, gas_multiplier: f64) -> ChainConfig {
    ChainConfig {
        chain_id,
        enabled: true,
        priority: 1,
        chain_specifics: ChainSpecifics {
            chain_id,
            gas_price_multiplier: gas_multiplier,
            min_gas_price: U256::from(1_000_000_000u64),
            max_gas_price: U256::from(100_000_000_000u64),
            bridge_timeout_ms: 300_000,
            confirmations_required: 12,
            native_token_decimals: 18,
            supports_eip1559: true,
        },
        assets: vec![],
    }
}

fn create_rebalance_targets() -> Vec<AllocationTarget> {
    vec![
        AllocationTarget {
            chain_id: BASE_CHAIN_ID,
            asset: H160::from_low_u64_be(0),
            target_percentage: 0.4,
            min_amount: U256::from(100_000_000_000_000_000u128),
            max_amount: U256::from(10_000_000_000_000_000_000u128),
        },
        AllocationTarget {
            chain_id: ARBITRUM_CHAIN_ID,
            asset: H160::from_low_u64_be(1),
            target_percentage: 0.3,
            min_amount: U256::from(1_000_000_000_000u128),
            max_amount: U256::from(100_000_000_000_000u128),
        },
        AllocationTarget {
            chain_id: POLYGON_CHAIN_ID,
            asset: H160::from_low_u64_be(2),
            target_percentage: 0.3,
            min_amount: U256::from(1_000_000_000_000u128),
            max_amount: U256::from(100_000_000_000_000u128),
        },
    ]
}
