//! Configuration for the Cross-VM coordinator.

use crate::types::*;
use serde::{Deserialize, Serialize};

/// Coordinator configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorConfig {
    /// Confirmation requirements per VM.
    pub confirmations: ConfirmationConfig,
    /// Timelock configuration.
    pub timelocks: TimelockConfig,
    /// Flash leg gas/compute limits.
    pub gas_limits: GasLimitConfig,
    /// Flashloan provider preferences (ordered by priority).
    pub evm_providers: Vec<FlashloanProvider>,
    pub svm_providers: Vec<FlashloanProvider>,
    pub x3_providers: Vec<FlashloanProvider>,
    /// Maximum slippage tolerance (basis points).
    pub max_slippage_bps: u32,
    /// Maximum retry count for each operation.
    pub max_retries: u32,
    /// Retry delay in milliseconds.
    pub retry_delay_ms: u64,
}

/// Block confirmations required per chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmationConfig {
    /// Ethereum confirmations (default: 12).
    pub evm: u32,
    /// Solana confirmations (default: 50 for "finalized").
    pub svm: u32,
    /// X3 Chain confirmations (default: 1, Flash Finality).
    pub x3: u32,
}

/// Timelock configuration (in seconds).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelockConfig {
    /// Base timelock for the fast chain (seconds).
    pub fast_chain_secs: u64,
    /// Delta added to slow chain timelock (seconds).
    pub slow_chain_delta_secs: u64,
    /// Safety margin before expiry to trigger abort (seconds).
    pub safety_margin_secs: u64,
}

/// Gas and compute limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasLimitConfig {
    /// EVM gas limit for flashloan legs.
    pub evm_flash_gas: u64,
    /// EVM gas limit for HTLC operations.
    pub evm_htlc_gas: u64,
    /// Solana compute unit limit for flash legs.
    pub svm_flash_compute: u64,
    /// Solana compute unit limit for HTLC operations.
    pub svm_htlc_compute: u64,
    /// X3 gas limit for operations.
    pub x3_gas: u64,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            confirmations: ConfirmationConfig {
                evm: 12,
                svm: 50,
                x3: 1,
            },
            timelocks: TimelockConfig {
                fast_chain_secs: 3600,       // 1 hour
                slow_chain_delta_secs: 3600, // +1 hour for slow chain
                safety_margin_secs: 300,     // 5 min safety window
            },
            gas_limits: GasLimitConfig {
                evm_flash_gas: 500_000,
                evm_htlc_gas: 100_000,
                svm_flash_compute: 400_000,
                svm_htlc_compute: 200_000,
                x3_gas: 100_000,
            },
            evm_providers: vec![
                FlashloanProvider::BalancerV2, // 0% fee first
                FlashloanProvider::AaveV3,     // 0.05% fallback
                FlashloanProvider::Euler,      // 0% alternative
            ],
            svm_providers: vec![
                FlashloanProvider::MarginFi, // 0% fee first
                FlashloanProvider::Kamino,   // 0% fee
                FlashloanProvider::Solend,   // 0.3% fallback
            ],
            x3_providers: vec![
                FlashloanProvider::X3Native, // X3's own pool
            ],
            max_slippage_bps: 50, // 0.5% max slippage
            max_retries: 3,
            retry_delay_ms: 1000,
        }
    }
}

impl CoordinatorConfig {
    /// Get confirmation requirement for a VM.
    pub fn confirmations_for(&self, vm: &VmTarget) -> u32 {
        match vm {
            VmTarget::Evm { .. } => self.confirmations.evm,
            VmTarget::Svm => self.confirmations.svm,
            VmTarget::X3Vm => self.confirmations.x3,
        }
    }

    /// Select the best flashloan provider for a given VM.
    pub fn best_provider(&self, vm: &VmTarget) -> Option<&FlashloanProvider> {
        let providers = match vm {
            VmTarget::Evm { .. } => &self.evm_providers,
            VmTarget::Svm => &self.svm_providers,
            VmTarget::X3Vm => &self.x3_providers,
        };
        providers.first()
    }

    /// Calculate timelocks for a swap pair.
    /// Returns (fast_timelock, slow_timelock) as unix timestamps.
    pub fn compute_timelocks(&self, now_unix: u64, _fast_vm: &VmTarget) -> (u64, u64) {
        let t_fast = now_unix + self.timelocks.fast_chain_secs;
        let t_slow = t_fast + self.timelocks.slow_chain_delta_secs;
        (t_fast, t_slow)
    }

    /// Check if we're within the safety margin before a timelock.
    pub fn is_near_expiry(&self, timelock: u64, now_unix: u64) -> bool {
        now_unix + self.timelocks.safety_margin_secs >= timelock
    }
}
