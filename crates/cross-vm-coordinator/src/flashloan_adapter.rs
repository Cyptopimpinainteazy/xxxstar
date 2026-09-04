//! Flashloan adapter — polymorphic interface to multi-provider flash lending.
//!
//! Routes flashloan requests to the optimal provider per chain/asset,
//! selecting by: lowest fee → highest liquidity → fastest execution.

use crate::types::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Flashloan execution interface.
///
/// Each chain has one or more flashloan providers. The adapter
/// selects the best one and executes the borrow → swap → repay
/// cycle atomically within a single transaction.
#[async_trait]
pub trait FlashloanAdapter: Send + Sync {
    /// Execute a flashloan leg: borrow from provider, swap on DEX, repay.
    ///
    /// This MUST be atomic: if any step fails, the entire transaction reverts.
    async fn execute_flash_leg(&self, leg: &FlashLeg) -> Result<FlashLegOutcome, CoordinatorError>;

    /// Check available liquidity from a provider for an asset.
    async fn check_liquidity(
        &self,
        vm: &VmTarget,
        provider: &FlashloanProvider,
        asset: &[u8],
    ) -> Result<u128, CoordinatorError>;

    /// Calculate expected premium for a flashloan.
    fn calculate_premium(&self, provider: &FlashloanProvider, amount: u128) -> u128 {
        let fee_bps = provider.fee_bps() as u128;
        (amount * fee_bps) / 10_000
    }
}

/// Multi-provider flashloan router.
///
/// Maintains a registry of available liquidity across providers
/// and automatically routes to the best one.
pub struct FlashloanRouter {
    /// Available providers per VM, sorted by fee (ascending).
    evm_providers: Vec<ProviderEntry>,
    svm_providers: Vec<ProviderEntry>,
    x3_providers: Vec<ProviderEntry>,
}

/// Registry entry for a flashloan provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEntry {
    pub provider: FlashloanProvider,
    /// Maximum loan amount available.
    pub max_amount: u128,
    /// Whether this provider is currently active.
    pub active: bool,
}

impl FlashloanRouter {
    pub fn new() -> Self {
        Self {
            evm_providers: vec![
                ProviderEntry {
                    provider: FlashloanProvider::BalancerV2,
                    max_amount: u128::MAX,
                    active: true,
                },
                ProviderEntry {
                    provider: FlashloanProvider::AaveV3,
                    max_amount: u128::MAX,
                    active: true,
                },
                ProviderEntry {
                    provider: FlashloanProvider::Euler,
                    max_amount: u128::MAX,
                    active: true,
                },
            ],
            svm_providers: vec![
                ProviderEntry {
                    provider: FlashloanProvider::MarginFi,
                    max_amount: u128::MAX,
                    active: true,
                },
                ProviderEntry {
                    provider: FlashloanProvider::Kamino,
                    max_amount: u128::MAX,
                    active: true,
                },
                ProviderEntry {
                    provider: FlashloanProvider::Solend,
                    max_amount: u128::MAX,
                    active: true,
                },
            ],
            x3_providers: vec![ProviderEntry {
                provider: FlashloanProvider::X3Native,
                max_amount: u128::MAX,
                active: true,
            }],
        }
    }

    /// Select the best provider for a given VM and amount.
    ///
    /// Strategy: lowest fee first, then highest liquidity.
    pub fn select_provider(&self, vm: &VmTarget, amount: u128) -> Option<&FlashloanProvider> {
        let providers = match vm {
            VmTarget::Evm { .. } => &self.evm_providers,
            VmTarget::Svm => &self.svm_providers,
            VmTarget::X3Vm => &self.x3_providers,
        };

        providers
            .iter()
            .filter(|p| p.active && p.max_amount >= amount)
            .min_by_key(|p| p.provider.fee_bps())
            .map(|p| &p.provider)
    }

    /// Calculate total premium across all legs.
    pub fn total_premium(&self, legs: &[FlashLeg]) -> u128 {
        legs.iter()
            .map(|leg| {
                let fee_bps = leg.provider.fee_bps() as u128;
                (leg.borrow_amount * fee_bps) / 10_000
            })
            .sum()
    }

    /// Deactivate a provider (e.g., after it fails).
    pub fn deactivate_provider(&mut self, vm: &VmTarget, provider: &FlashloanProvider) {
        let providers = match vm {
            VmTarget::Evm { .. } => &mut self.evm_providers,
            VmTarget::Svm => &mut self.svm_providers,
            VmTarget::X3Vm => &mut self.x3_providers,
        };

        for entry in providers.iter_mut() {
            if std::mem::discriminant(&entry.provider) == std::mem::discriminant(provider) {
                entry.active = false;
                info!(
                    provider = ?provider,
                    vm = %vm,
                    "Deactivated flashloan provider"
                );
            }
        }
    }
}

impl Default for FlashloanRouter {
    fn default() -> Self {
        Self::new()
    }
}
