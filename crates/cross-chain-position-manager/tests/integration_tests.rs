//! Integration tests for Cross-Chain Position Manager facade API.

use cross_chain_position_manager::{
    AllocationTarget, ChainConfig, ChainSpecifics, CrossChainPositionManager, PositionId,
    PositionManagerConfig,
};
use sp_core::{H160, U256};

const BASE_CHAIN_ID: u64 = 8453;
const ARBITRUM_CHAIN_ID: u64 = 42161;
const POLYGON_CHAIN_ID: u64 = 137;

fn create_test_config() -> PositionManagerConfig {
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

fn create_test_rebalance_targets() -> Vec<AllocationTarget> {
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

#[tokio::test]
async fn test_full_position_lifecycle() {
    let config = create_test_config();
    let mut manager = CrossChainPositionManager::new_with_config(config).unwrap();

    manager.start().await.unwrap();

    let positions = manager.track_positions().await.unwrap();
    assert!(positions.is_empty());

    let summary = manager.get_portfolio_summary().await.unwrap();
    assert_eq!(summary.total_value_usd, U256::zero());

    manager.stop().await.unwrap();
}

#[tokio::test]
async fn test_position_migration() {
    let config = create_test_config();
    let manager = CrossChainPositionManager::new_with_config(config).unwrap();

    let position_id = PositionId::new();
    let result = manager
        .migrate_position(BASE_CHAIN_ID, ARBITRUM_CHAIN_ID, &position_id)
        .await
        .unwrap();

    assert!(result.success);
    assert!(result.estimated_duration_ms > 0);
    assert!(result.gas_cost_estimate > U256::zero());
    assert!(result.slippage_estimate >= 0.0);
}

#[tokio::test]
async fn test_position_migration_invalid_input() {
    let config = create_test_config();
    let manager = CrossChainPositionManager::new_with_config(config).unwrap();

    let result = manager
        .migrate_position(BASE_CHAIN_ID, BASE_CHAIN_ID, &PositionId::new())
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_portfolio_rebalancing() {
    let config = create_test_config();
    let manager = CrossChainPositionManager::new_with_config(config).unwrap();

    let targets = create_test_rebalance_targets();
    let result = manager.rebalance(&targets).await.unwrap();

    assert!(result.success);
    assert_eq!(result.actions_executed, targets.len());
}

#[tokio::test]
async fn test_empty_rebalance_fails() {
    let config = create_test_config();
    let manager = CrossChainPositionManager::new_with_config(config).unwrap();

    let result = manager.rebalance(&[]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_arbitrage_evaluation() {
    let config = create_test_config();
    let manager = CrossChainPositionManager::new_with_config(config).unwrap();

    let opportunities = manager.evaluate_arbitrage().await.unwrap();
    assert!(!opportunities.is_empty());

    for opportunity in opportunities {
        assert!(opportunity.profit_estimate_usd >= U256::zero());
        assert!(opportunity.confidence >= 0.0 && opportunity.confidence <= 1.0);
    }
}

#[tokio::test]
async fn test_kill_switch_check() {
    let config = create_test_config();
    let manager = CrossChainPositionManager::new_with_config(config).unwrap();

    let kill_switches = manager.check_kill_switches().await.unwrap();
    assert!(kill_switches.is_empty());
}

#[tokio::test]
async fn test_simulation_capabilities() {
    let config = create_test_config();
    let manager = CrossChainPositionManager::new_with_config(config).unwrap();

    let result = manager
        .simulate_cross_chain_move(
            BASE_CHAIN_ID,
            ARBITRUM_CHAIN_ID,
            H160::zero(),
            U256::from(1_000_000_000_000_000_000u128),
        )
        .await
        .unwrap();

    assert!(result.feasible);
    assert!(result.estimated_cost >= U256::zero());
    assert!(result.estimated_duration > 0);
    assert!(!result.alternatives.is_empty());
}
