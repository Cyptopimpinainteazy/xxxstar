//! Flashloan liquidity pool — per-chain, per-asset capital reserves.
//!
//! Pools hold transient capital that is borrowed and repaid within a single
//! atomic execution context. Capital never crosses chains.

use std::collections::HashMap;

use crate::error::FlashloanError;
use crate::types::{AssetId, BorrowReceipt, BorrowRequest, ChainKind, FlashloanId};

/// Default flashloan premium in basis points (0.05% = 5 bps).
const DEFAULT_PREMIUM_BPS: u128 = 5;

/// Per-chain, per-asset liquidity pool.
#[derive(Debug)]
pub struct FlashloanPool {
    /// Available liquidity: (chain, asset) → amount.
    liquidity: HashMap<(ChainKind, AssetId), u128>,
    /// Outstanding borrows: flashloan_id → receipt.
    outstanding: HashMap<FlashloanId, BorrowReceipt>,
    /// Custom premium overrides per asset (basis points).
    premium_overrides: HashMap<AssetId, u128>,
    /// Total premium earned over lifetime.
    total_premium_earned: u128,
}

impl FlashloanPool {
    /// Create an empty pool.
    pub fn new() -> Self {
        Self {
            liquidity: HashMap::new(),
            outstanding: HashMap::new(),
            premium_overrides: HashMap::new(),
            total_premium_earned: 0,
        }
    }

    /// Deposit liquidity into the pool.
    pub fn deposit(&mut self, chain: ChainKind, asset: AssetId, amount: u128) {
        let entry = self.liquidity.entry((chain, asset)).or_insert(0);
        *entry += amount;
    }

    /// Withdraw liquidity from the pool (fails if insufficient).
    pub fn withdraw(
        &mut self,
        chain: ChainKind,
        asset: &AssetId,
        amount: u128,
    ) -> Result<(), FlashloanError> {
        let available = self.available(chain, asset);
        if available < amount {
            return Err(FlashloanError::InsufficientLiquidity {
                chain,
                asset: asset.clone(),
                requested: amount,
                available,
            });
        }
        let entry = self.liquidity.get_mut(&(chain, asset.clone())).unwrap();
        *entry -= amount;
        Ok(())
    }

    /// Query available liquidity for a chain+asset pair.
    pub fn available(&self, chain: ChainKind, asset: &AssetId) -> u128 {
        self.liquidity
            .get(&(chain, asset.clone()))
            .copied()
            .unwrap_or(0)
    }

    /// Borrow capital — issues a receipt that must be repaid atomically.
    pub fn borrow(&mut self, request: &BorrowRequest) -> Result<BorrowReceipt, FlashloanError> {
        if self
            .outstanding
            .values()
            .any(|receipt| receipt.chain == request.chain && receipt.asset == request.asset)
        {
            return Err(FlashloanError::ConcurrentBorrowRejected {
                chain: request.chain,
                asset: request.asset.clone(),
            });
        }

        let available = self.available(request.chain, &request.asset);
        if available < request.amount {
            return Err(FlashloanError::InsufficientLiquidity {
                chain: request.chain,
                asset: request.asset.clone(),
                requested: request.amount,
                available,
            });
        }

        // Deduct from pool
        let entry = self
            .liquidity
            .get_mut(&(request.chain, request.asset.clone()))
            .unwrap();
        *entry -= request.amount;

        // Calculate premium
        let premium_bps = self
            .premium_overrides
            .get(&request.asset)
            .copied()
            .unwrap_or(DEFAULT_PREMIUM_BPS);
        let premium = request.amount * premium_bps / 10_000;

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let receipt = BorrowReceipt {
            id: request.id.clone(),
            chain: request.chain,
            asset: request.asset.clone(),
            principal: request.amount,
            premium,
            borrowed_at: now_ms,
            must_repay_by: now_ms + 30_000, // 30 second hard deadline
        };

        self.outstanding.insert(request.id.clone(), receipt.clone());

        Ok(receipt)
    }

    /// Repay a flashloan. Returns `true` if fully settled.
    pub fn repay(
        &mut self,
        flashloan_id: &FlashloanId,
        amount: u128,
    ) -> Result<bool, FlashloanError> {
        let receipt = self
            .outstanding
            .get(flashloan_id)
            .ok_or_else(|| FlashloanError::UnknownFlashloan(flashloan_id.clone()))?;

        let total_owed = receipt.total_owed();
        if amount < total_owed {
            return Err(FlashloanError::InsufficientRepayment {
                owed: total_owed,
                paid: amount,
            });
        }

        let chain = receipt.chain;
        let asset = receipt.asset.clone();
        let premium = receipt.premium;

        // Return capital to pool (principal + premium goes back as new liquidity)
        let entry = self.liquidity.entry((chain, asset)).or_insert(0);
        *entry += amount;

        self.total_premium_earned += premium;
        self.outstanding.remove(flashloan_id);

        Ok(true)
    }

    /// Revert a borrow (return capital to pool without premium).
    /// Used when atomic execution context reverts.
    pub fn revert(&mut self, flashloan_id: &FlashloanId) -> Result<(), FlashloanError> {
        let receipt = self
            .outstanding
            .remove(flashloan_id)
            .ok_or_else(|| FlashloanError::UnknownFlashloan(flashloan_id.clone()))?;

        // Return only principal (no premium earned on revert)
        let entry = self
            .liquidity
            .entry((receipt.chain, receipt.asset))
            .or_insert(0);
        *entry += receipt.principal;

        Ok(())
    }

    /// Set a custom premium for an asset (basis points).
    pub fn set_premium(&mut self, asset: AssetId, bps: u128) {
        self.premium_overrides.insert(asset, bps);
    }

    /// Total premium earned over the pool's lifetime.
    pub fn total_earned(&self) -> u128 {
        self.total_premium_earned
    }

    /// Number of outstanding borrows.
    pub fn outstanding_count(&self) -> usize {
        self.outstanding.len()
    }

    /// Check if a flashloan is still outstanding.
    pub fn is_outstanding(&self, id: &FlashloanId) -> bool {
        self.outstanding.contains_key(id)
    }
}

impl Default for FlashloanPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BorrowPurpose;

    #[test]
    fn test_deposit_and_available() {
        let mut pool = FlashloanPool::new();
        let chain = ChainKind::Evm(1);
        let asset = AssetId::new("USDC");

        pool.deposit(chain, asset.clone(), 1_000_000);
        assert_eq!(pool.available(chain, &asset), 1_000_000);

        pool.deposit(chain, asset.clone(), 500_000);
        assert_eq!(pool.available(chain, &asset), 1_500_000);
    }

    #[test]
    fn test_borrow_reduces_liquidity() {
        let mut pool = FlashloanPool::new();
        let chain = ChainKind::Evm(1);
        let asset = AssetId::new("WETH");

        pool.deposit(chain, asset.clone(), 100_000);

        let request = BorrowRequest {
            id: FlashloanId::new(),
            chain,
            asset: asset.clone(),
            amount: 60_000,
            purpose: BorrowPurpose::ArbExecution,
        };

        let receipt = pool.borrow(&request).unwrap();
        assert_eq!(receipt.principal, 60_000);
        assert_eq!(pool.available(chain, &asset), 40_000);
    }

    #[test]
    fn test_borrow_insufficient_liquidity() {
        let mut pool = FlashloanPool::new();
        let chain = ChainKind::Evm(1);
        let asset = AssetId::new("USDC");

        pool.deposit(chain, asset.clone(), 100);

        let request = BorrowRequest {
            id: FlashloanId::new(),
            chain,
            asset,
            amount: 200,
            purpose: BorrowPurpose::ArbExecution,
        };

        let result = pool.borrow(&request);
        assert!(matches!(
            result,
            Err(FlashloanError::InsufficientLiquidity { .. })
        ));
    }

    #[test]
    fn test_repay_restores_plus_premium() {
        let mut pool = FlashloanPool::new();
        let chain = ChainKind::Evm(1);
        let asset = AssetId::new("USDC");

        pool.deposit(chain, asset.clone(), 1_000_000);

        let request = BorrowRequest {
            id: FlashloanId::new(),
            chain,
            asset: asset.clone(),
            amount: 500_000,
            purpose: BorrowPurpose::ArbExecution,
        };

        let receipt = pool.borrow(&request).unwrap();
        let repay_amount = receipt.total_owed();

        pool.repay(&receipt.id, repay_amount).unwrap();
        assert!(pool.available(chain, &asset) > 1_000_000);
    }

    #[test]
    fn test_revert_restores_exact_principal() {
        let mut pool = FlashloanPool::new();
        let chain = ChainKind::Svm;
        let asset = AssetId::new("SOL");

        pool.deposit(chain, asset.clone(), 1_000_000);

        let request = BorrowRequest {
            id: FlashloanId::new(),
            chain,
            asset: asset.clone(),
            amount: 500_000,
            purpose: BorrowPurpose::Liquidation,
        };

        let receipt = pool.borrow(&request).unwrap();
        pool.revert(&receipt.id).unwrap();

        // Exact principal restored, no premium
        assert_eq!(pool.available(chain, &asset), 1_000_000);
    }

    #[test]
    fn test_custom_premium() {
        let mut pool = FlashloanPool::new();
        let chain = ChainKind::Evm(1);
        let asset = AssetId::new("WETH");

        pool.deposit(chain, asset.clone(), 10_000_000);
        pool.set_premium(asset.clone(), 10); // 0.10%

        let request = BorrowRequest {
            id: FlashloanId::new(),
            chain,
            asset,
            amount: 1_000_000,
            purpose: BorrowPurpose::ArbExecution,
        };

        let receipt = pool.borrow(&request).unwrap();
        assert_eq!(receipt.premium, 1_000); // 0.10% of 1M = 1K
    }
}
