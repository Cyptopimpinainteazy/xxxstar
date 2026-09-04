use super::*;
use crate::mev_protection::MEVProtectionError;
use sp_core::{H160, U256};

fn sample_params() -> SwapParams {
    SwapParams {
        token_in: H160::from_low_u64_be(1),
        token_out: H160::from_low_u64_be(2),
        amount_in: U256::from(1_000_000u64),
        min_amount_out: U256::from(900_000u64),
        chain_in: 1,
        chain_out: 1,
        deadline: 1000,
        recipient: H160::from_low_u64_be(3),
        slippage_tolerance_bps: 50,
        gas_price_limit: Some(U256::from(2_000_000_000u64)),
        source_vm: VmType::Evm,
        destination_vm: VmType::Evm,
    }
}

fn hop(amount: u64) -> Hop {
    Hop {
        dex_address: H160::from_low_u64_be(10),
        amount: U256::from(amount),
        estimated_block: 100,
        slippage: 0.01,
    }
}

#[tokio::test]
async fn optimizer_picks_highest_output_lowest_gas() {
    let opt = RouteOptimizer::new().unwrap();
    let p = sample_params();

    let quotes = vec![
        QuoteResult {
            route: SwapRoute {
                hops: vec![hop(500)],
                amount_in: p.amount_in,
                estimated_output: U256::from(950_000u64),
                gas_estimate: U256::from(25_000u64),
            },
            estimated_output: U256::from(950_000u64),
            gas_cost: U256::from(25_000u64),
        },
        QuoteResult {
            route: SwapRoute {
                hops: vec![hop(700)],
                amount_in: p.amount_in,
                estimated_output: U256::from(950_000u64),
                gas_estimate: U256::from(30_000u64),
            },
            estimated_output: U256::from(950_000u64),
            gas_cost: U256::from(30_000u64),
        },
        QuoteResult {
            route: SwapRoute {
                hops: vec![hop(400)],
                amount_in: p.amount_in,
                estimated_output: U256::from(940_000u64),
                gas_estimate: U256::from(20_000u64),
            },
            estimated_output: U256::from(940_000u64),
            gas_cost: U256::from(20_000u64),
        },
    ];

    let route = opt.optimize_route(&quotes, &p).await.unwrap();
    assert_eq!(route.estimated_output, U256::from(950_000u64));
    assert_eq!(route.gas_estimate, U256::from(25_000u64));
}

#[tokio::test]
async fn optimizer_errors_when_no_route_meets_min_out() {
    let opt = RouteOptimizer::new().unwrap();
    let mut p = sample_params();
    p.min_amount_out = U256::from(1_000_000u64);

    let quotes = vec![QuoteResult {
        route: SwapRoute {
            hops: vec![hop(300)],
            amount_in: p.amount_in,
            estimated_output: U256::from(900_000u64),
            gas_estimate: U256::from(21_000u64),
        },
        estimated_output: U256::from(900_000u64),
        gas_cost: U256::from(21_000u64),
    }];

    let err = opt.optimize_route(&quotes, &p).await.unwrap_err();
    assert!(matches!(err, SwapRouterError::RouteNotFound));
}

#[tokio::test]
async fn gas_optimizer_returns_nonzero_defaults() {
    let gas = GasOptimizer::new().unwrap();
    let route = SwapRoute {
        hops: vec![hop(100)],
        amount_in: U256::from(1),
        estimated_output: U256::from(1),
        gas_estimate: U256::zero(),
    };
    let params = gas.calculate_gas(&route).await.unwrap();
    assert!(params.gas_limit > U256::zero());
    assert!(params.gas_price > U256::zero());
}

#[tokio::test]
async fn slippage_controller_rejects_zero_tolerance() {
    let ctrl = SlippageController::new().unwrap();
    let mut p = sample_params();
    p.slippage_tolerance_bps = 0;
    let route = SwapRoute {
        hops: vec![hop(100)],
        amount_in: p.amount_in,
        estimated_output: p.min_amount_out,
        gas_estimate: U256::zero(),
    };
    let err = ctrl.apply_protection(&p, &route).await.unwrap_err();
    assert!(matches!(err, SwapRouterError::HighSlippage));
}

#[tokio::test]
async fn fee_calculator_sums_total() {
    let calc = FeeCalculator::new().unwrap();
    let mut params = sample_params();
    params.chain_in = 8453;
    params.chain_out = 42161;
    let route = SwapRoute {
        hops: vec![hop(100)],
        amount_in: U256::from(1_000_000u64),
        estimated_output: U256::from(1_000_000u64),
        gas_estimate: U256::from(10_000u64),
    };
    let fees = calc.calculate_swap_fees(&route, &params).await.unwrap();
    assert!(fees.protocol_fee > U256::zero());
    assert_eq!(fees.total_fee, fees.protocol_fee + fees.gas_fee);
}

#[tokio::test]
async fn fee_calculator_applies_four_percent_for_cross_vm() {
    let calc = FeeCalculator::new().unwrap();
    let mut params = sample_params();
    params.source_vm = VmType::Evm;
    params.destination_vm = VmType::Svm;

    let route = SwapRoute {
        hops: vec![hop(250)],
        amount_in: U256::from(1_000_000u64),
        estimated_output: U256::from(1_000_000u64),
        gas_estimate: U256::from(5_000u64),
    };

    let fees = calc.calculate_swap_fees(&route, &params).await.unwrap();
    assert_eq!(fees.protocol_fee, U256::from(40_000u64));
    assert_eq!(fees.total_fee, U256::from(45_000u64));
}

#[tokio::test]
async fn fee_calculator_applies_two_percent_for_same_vm_cross_chain() {
    let calc = FeeCalculator::new().unwrap();
    let mut params = sample_params();
    params.chain_in = 8453;
    params.chain_out = 42161;
    params.source_vm = VmType::Evm;
    params.destination_vm = VmType::Evm;

    let route = SwapRoute {
        hops: vec![hop(150)],
        amount_in: U256::from(1_000_000u64),
        estimated_output: U256::from(1_000_000u64),
        gas_estimate: U256::from(7_000u64),
    };

    let fees = calc.calculate_swap_fees(&route, &params).await.unwrap();
    assert_eq!(fees.protocol_fee, U256::from(20_000u64));
    assert_eq!(fees.total_fee, U256::from(27_000u64));
}

#[tokio::test]
async fn executor_marks_success_with_nonempty_inputs() {
    let exec = AtomicSwapExecutor::new().unwrap();
    let route = SwapRoute {
        hops: vec![hop(100)],
        amount_in: U256::from(1),
        estimated_output: U256::from(1),
        gas_estimate: U256::from(10_000u64),
    };
    let gas = ChainGasParams {
        gas_price: U256::from(1_000_000_000u64),
        gas_limit: U256::from(25_000u64),
    };
    let params = SlippageProtectedParams {
        params: sample_params(),
        slippage_bps: 50,
    };
    let res = exec
        .execute_swap_bundle(&route, &gas, &params)
        .await
        .unwrap();
    assert!(res.success);
    assert!(res.gas_used > U256::zero());
}

#[tokio::test]
async fn mev_overhead_error_propagates() {
    let protector = MEVProtector::new().unwrap().with_overhead_limit(0); // force error

    let route = SwapRoute {
        hops: vec![hop(100)],
        amount_in: U256::from(1),
        estimated_output: U256::from(1),
        gas_estimate: U256::from(1),
    };
    let err = protector.protect_route(&route).await.unwrap_err();
    assert!(matches!(err, MEVProtectionError::OverheadExceeded { .. }));
}
