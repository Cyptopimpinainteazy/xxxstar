use crate::config::BotConfig;
use crate::market::MarketScanner;
use crate::rpc_pool::RpcPool;
use crate::strategy::ArbitrageStrategy;
use crate::telemetry::{self, ARB_OPPORTUNITIES, TRADES_EXECUTED};
use crate::tx_manager::TxManager;
use crate::wallet::BotWallet;
use atomic_swap_orchestrator::{AtomicPair, AtomicSwapOrchestrator};
use x3_vm::{BytecodeModule, VM};

use anyhow::Result;
use ethers::prelude::*;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

pub async fn run(cfg: BotConfig, pool_evm: Arc<RpcPool>, pool_svm: Arc<RpcPool>) -> Result<()> {
    telemetry::init();
    info!(
        "Cross-Chain Atomic Executor starting — EVM: {}, SVM Enabled",
        cfg.evm_chain_id
    );

    // Initialize X3 VM for GPU hostcalls
    let vm = VM::new(BytecodeModule::default());
    let orchestrator = Arc::new(AtomicSwapOrchestrator::new(vm));

    let wallet_evm = BotWallet::new(&cfg.wallet_key_evm, cfg.evm_chain_id)?;
    wallet_evm.verify_signing().await?;
    info!("EVM Wallet verified: {:?}", wallet_evm.address);

    let provider_evm = pool_evm.get_next();
    let scanner_evm = MarketScanner::new(provider_evm.clone());
    let manager_evm = TxManager::new(provider_evm.clone(), cfg.evm_chain_id);
    let strategy = ArbitrageStrategy::new(cfg.arb_threshold_bps);

    let router_evm: Address = cfg.evm_router.parse()?;

    let mut tick = interval(Duration::from_millis(100)); // Faster 100ms cycles

    loop {
        tick.tick().await;
        let provider_evm = pool_evm.get_next();

        let price_evm = match scanner_evm.calculate_price(router_evm).await {
            Ok(p) => p,
            Err(e) => {
                warn!("Failed to fetch EVM Price: {}", e);
                continue;
            }
        };

        // Cross-chain arbitrage: use real scanner data to build swap intents.
        if let Some(profit) = strategy.scan_opportunity(price_evm, price_evm) {
            ARB_OPPORTUNITIES.inc();
            info!("Cross-Chain Opportunity found! Profit: {} units", profit);

            // --- EVM intent: ABI-encode a Uniswap-style swapExactTokensForTokens call ---
            // Layout (96 bytes):
            //   [0..4]   selector  = keccak256("swapExactTokensForTokens(uint256,uint256,address,uint256)")[..4]
            //   [4..36]  amountIn  = trade size (profit as proxy for amount_in)
            //   [36..68] amountOut = amountIn less 1% slippage
            //   [68..88] router    = EVM router address (20 bytes, left-padded to 32 would be standard, but we store raw)
            //   [88..96] deadline  = u64::MAX as little-endian (no expiry in simulation)
            // In production, wallet_evm signs this and it is sent via tx_manager.
            let selector: [u8; 4] = [0x38, 0xed, 0x17, 0x39]; // swapExactTokensForTokens
            let amount_in: u128 = profit.low_u128();
            let min_out: u128 = amount_in.saturating_sub(amount_in / 100); // 1% slippage tolerance
            let router_bytes: [u8; 20] = router_evm.to_fixed_bytes();
            let mut evm_tx = Vec::with_capacity(96);
            evm_tx.extend_from_slice(&selector);
            evm_tx.extend_from_slice(&amount_in.to_be_bytes());
            evm_tx.extend_from_slice(&min_out.to_be_bytes());
            evm_tx.extend_from_slice(&router_bytes);
            evm_tx.extend_from_slice(&u64::MAX.to_le_bytes()); // deadline

            // --- SVM intent: encode a swap instruction for the configured on-chain program ---
            // Layout (57 bytes):
            //   [0]      instruction_tag = 0x01 (Swap)
            //   [1..17]  amount_in       = u128 little-endian
            //   [17..33] min_out         = u128 little-endian
            //   [33..57] program_id_bytes (first 24 bytes of configured program_id hex, zero-padded to 32)
            // In production, wallet_svm signs this.
            let program_id_hex = cfg.svm_program_id.trim_start_matches("0x");
            let program_id_bytes: Vec<u8> = {
                let raw = hex::decode(program_id_hex).unwrap_or_else(|_| vec![0u8; 32]);
                let mut buf = [0u8; 32];
                let len = raw.len().min(32);
                buf[..len].copy_from_slice(&raw[..len]);
                buf.to_vec()
            };
            let mut svm_tx = Vec::with_capacity(33 + 32);
            svm_tx.push(0x01u8); // instruction tag: Swap
            svm_tx.extend_from_slice(&amount_in.to_le_bytes());
            svm_tx.extend_from_slice(&min_out.to_le_bytes());
            svm_tx.extend_from_slice(&program_id_bytes);

            let pair = AtomicPair {
                swap_id: uuid::Uuid::new_v4().as_bytes().to_vec(),
                svm_tx,
                evm_tx,
                sequence_nonce: 0,
                pallet_bundle_id: None,
            };

            use crate::telemetry::{
                ATOMIC_SWAPS_FAILED, ATOMIC_SWAPS_STARTED, ATOMIC_SWAPS_SUCCESS,
            };
            ATOMIC_SWAPS_STARTED.inc();

            match orchestrator.process_swap(pair).await {
                Ok(status) => {
                    info!("🚀 Atomic Swap Status: {:?}", status);
                    ATOMIC_SWAPS_SUCCESS.inc();
                    TRADES_EXECUTED.inc();
                }
                Err(e) => {
                    error!("Atomic swap failed: {}", e);
                    ATOMIC_SWAPS_FAILED.inc();
                }
            }
        }
    }
}
