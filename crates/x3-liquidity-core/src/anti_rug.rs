//! LP lock registry — basic rug-pull mitigation.
//!
//! Operators who seed a pool via [`crate::launchpad::Launchpad`] can
//! voluntarily lock their LP tokens until a future block height.  This
//! provides on-chain proof of commitment and prevents immediate liquidity
//! withdrawal after listing.
//!
//! This is an in-memory registry used by the CLI and devnet harness.  The
//! production on-chain variant lives in a pallet `StorageMap`.

use alloc::collections::BTreeMap;

/// Key: (owner, pool_id).
type LockKey = ([u8; 32], u64);

/// A single LP lock record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LpLock {
    pub owner: [u8; 32],
    pub pool_id: u64,
    pub lp_amount: u128,
    /// Block number at or after which the LP can be withdrawn.
    pub unlock_at_block: u64,
}

/// Anti-rug scoring factors for a pool
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AntiRugScore {
    /// Percentage of total LP tokens that are locked (0-10000, representing 0.00%-100.00%)
    pub lock_percentage: u16,
    /// Lock duration in blocks
    pub lock_duration_blocks: u32,
    /// Team wallet concentration score (0-100, lower is better)
    pub team_wallet_concentration: u8,
    /// Holder distribution Gini coefficient (0-100, lower is more equal)
    pub holder_distribution_gini: u8,
    /// Days since pool launch
    pub days_since_launch: u32,
}

/// Computed anti-rug score result
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RugScoreResult {
    /// Overall score (0-100, higher is better/safer)
    pub score: u8,
    /// Risk level
    pub risk_level: RugRiskLevel,
    /// Individual factor scores
    pub factors: AntiRugScore,
}

/// Risk levels based on score
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RugRiskLevel {
    /// Very High Risk (>80% chance of rug)
    VeryHigh,
    /// High Risk (60-80% chance)
    High,
    /// Medium Risk (40-60% chance)
    Medium,
    /// Low Risk (20-40% chance)
    Low,
    /// Very Low Risk (<20% chance)
    VeryLow,
}

/// Errors from the anti-rug module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AntiRugError {
    /// Lock amount is zero.
    ZeroAmount,
    /// No lock exists for the given (owner, pool_id).
    NotFound,
    /// The lock has not yet expired.
    LockNotExpired,
    /// A lock already exists; use `extend` to update.
    AlreadyLocked,
    /// Invalid score parameters
    InvalidScoreParameters,
}

/// In-memory LP lock registry.
#[derive(Default)]
pub struct LpLockRegistry {
    locks: BTreeMap<LockKey, LpLock>,
}

impl LpLockRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new LP lock.
    pub fn lock(
        &mut self,
        owner: [u8; 32],
        pool_id: u64,
        lp_amount: u128,
        unlock_at_block: u64,
    ) -> Result<(), AntiRugError> {
        if lp_amount == 0 {
            return Err(AntiRugError::ZeroAmount);
        }
        let key = (owner, pool_id);
        if self.locks.contains_key(&key) {
            return Err(AntiRugError::AlreadyLocked);
        }
        self.locks.insert(
            key,
            LpLock {
                owner,
                pool_id,
                lp_amount,
                unlock_at_block,
            },
        );
        Ok(())
    }

    /// Retrieve an existing lock.
    pub fn get(&self, owner: &[u8; 32], pool_id: u64) -> Option<&LpLock> {
        self.locks.get(&(*owner, pool_id))
    }

    /// Withdraw (remove) a lock once the unlock block has passed.
    ///
    /// `current_block` must be >= `lock.unlock_at_block`.
    pub fn withdraw(
        &mut self,
        owner: &[u8; 32],
        pool_id: u64,
        current_block: u64,
    ) -> Result<LpLock, AntiRugError> {
        let key = (*owner, pool_id);
        let lock = self.locks.get(&key).ok_or(AntiRugError::NotFound)?;
        if current_block < lock.unlock_at_block {
            return Err(AntiRugError::LockNotExpired);
        }
        Ok(self.locks.remove(&key).unwrap())
    }

    pub fn len(&self) -> usize {
        self.locks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.locks.is_empty()
    }

    /// Compute anti-rug score for a pool
    ///
    /// This is a simplified scoring model. In production, this would analyze:
    /// - LP token lock percentages and durations
    /// - Team wallet holdings vs public distribution
    /// - Holder concentration (Gini coefficient)
    /// - Time since launch
    /// - Trading volume patterns
    /// - Social metrics and audits
    pub fn compute_rug_score(
        &self,
        pool_id: u64,
        total_lp_supply: u128,
        team_wallet_balance: u128,
        holder_gini: u8,
        launch_timestamp: u64,
        current_timestamp: u64,
    ) -> Result<RugScoreResult, AntiRugError> {
        if total_lp_supply == 0 {
            return Err(AntiRugError::InvalidScoreParameters);
        }

        // Calculate lock percentage
        let locked_lp: u128 = self
            .locks
            .values()
            .filter(|lock| lock.pool_id == pool_id)
            .map(|lock| lock.lp_amount)
            .sum();

        let lock_percentage = if total_lp_supply > 0 {
            ((locked_lp as f64) * 10000.0 / (total_lp_supply as f64)) as u16
        } else {
            0
        };
        let lock_percentage = lock_percentage.min(10000);

        // Estimate lock duration (use the longest lock for this pool)
        let lock_duration = self
            .locks
            .values()
            .filter(|lock| lock.pool_id == pool_id)
            .map(|lock| lock.unlock_at_block.saturating_sub(0)) // Would need current block
            .max()
            .unwrap_or(0);

        // Team wallet concentration (percentage of total supply held by team)
        let team_concentration = if total_lp_supply > 0 {
            ((team_wallet_balance as f64) * 100.0 / (total_lp_supply as f64)) as u8
        } else {
            100
        };

        // Days since launch (simplified)
        let days_since_launch =
            ((current_timestamp.saturating_sub(launch_timestamp)) / 86400) as u32;

        let factors = AntiRugScore {
            lock_percentage,
            lock_duration_blocks: lock_duration as u32,
            team_wallet_concentration: team_concentration.min(100),
            holder_distribution_gini: holder_gini.min(100),
            days_since_launch,
        };

        // Compute weighted score (0-100, higher = safer)
        let mut score = 0u32;

        // Lock percentage (40% weight) - higher locks = higher score
        score += (lock_percentage as u32) * 40 / 10000;

        // Lock duration (20% weight) - longer locks = higher score
        let duration_score = if lock_duration > 0 {
            ((lock_duration as f64).ln() * 20.0 / (30.0_f64 * 24.0 * 3600.0).ln()) as u32
        // 30 days max
        } else {
            0
        };
        score += duration_score.min(20);

        // Team concentration (20% weight) - lower concentration = higher score
        let team_score = 20 - ((team_concentration as u32) * 20 / 100);
        score += team_score;

        // Holder distribution (10% weight) - lower Gini = higher score
        let distribution_score = 10 - ((holder_gini as u32) * 10 / 100);
        score += distribution_score;

        // Time since launch (10% weight) - older pools = higher score
        let time_score = if days_since_launch > 30 {
            10
        } else {
            (days_since_launch as u32) * 10 / 30
        };
        score += time_score;

        let final_score = (score as u8).min(100);
        let risk_level = match final_score {
            0..=20 => RugRiskLevel::VeryHigh,
            21..=40 => RugRiskLevel::High,
            41..=60 => RugRiskLevel::Medium,
            61..=80 => RugRiskLevel::Low,
            _ => RugRiskLevel::VeryLow,
        };

        Ok(RugScoreResult {
            score: final_score,
            risk_level,
            factors,
        })
    }
}
