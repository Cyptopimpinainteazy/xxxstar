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

        // Integer arithmetic, not `f64` (PHASE 43): a share of a supply is a ratio of two
        // integers. The float form was lossy above 2^53 and — the reason it matters here —
        // its value depended on the platform's floating-point behaviour, so a score built
        // from it was a decision that varied with where it ran. A supply too large to scale
        // is scored at the ceiling rather than wrapped, because an unrepresentable share is
        // not a small one.
        let lock_percentage = locked_lp
            .checked_mul(10_000)
            .map(|scaled| scaled / total_lp_supply)
            .map_or(10_000, |basis_points| {
                u16::try_from(basis_points).unwrap_or(10_000).min(10_000)
            });

        // Estimate lock duration (use the longest lock for this pool)
        let lock_duration = self
            .locks
            .values()
            .filter(|lock| lock.pool_id == pool_id)
            .map(|lock| lock.unlock_at_block.saturating_sub(0)) // Would need current block
            .max()
            .unwrap_or(0);

        // Team wallet concentration (percentage of total supply held by team)
        // The same, and the ceiling here is 100: a concentration that cannot be expressed is
        // scored as total, which is the fail-closed direction for a rug heuristic.
        let team_concentration = team_wallet_balance
            .checked_mul(100)
            .map(|scaled| scaled / total_lp_supply)
            .map_or(100, |percent| u8::try_from(percent).unwrap_or(100).min(100));

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

        // Lock duration (20% weight) - longer locks = higher score, 30 days max.
        //
        // `ln` rather than integer arithmetic for two reasons, and the second forced the
        // change: a float logarithm is not deterministic across platforms — its value comes
        // from the platform's `libm` — so a score computed from it was a decision that
        // depended on where it ran (PHASE 43's prohibition), and `ln` is not in `core`, so
        // this crate could not be built without `std` while it used it (TICKET-093).
        //
        // `ln(a)/ln(b) == log2(a)/log2(b)`, so the ratio is the same function of the two
        // arguments and integer `ilog2` computes it without a float anywhere. Each
        // logarithm is floored, so this sub-score can differ from the float form by at most
        // one point of twenty — and it no longer depends on the platform, which is the
        // property that matters.
        const THIRTY_DAYS_SECONDS: u64 = 30 * 24 * 3600;
        let duration_score = if lock_duration > 0 {
            let ratio = (u128::from(lock_duration).ilog2() * 20) / u128::from(THIRTY_DAYS_SECONDS).ilog2();
            ratio.min(20)
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
            days_since_launch * 10 / 30
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A pool that is fully locked, holds no team allocation, has a flat distribution and is
    /// older than a month. Every other term is at its maximum — 40 for the lock, 20 for team
    /// concentration, 10 for distribution and 10 for age — so the total isolates the
    /// duration term, which is 20 at most: `score == 80 + duration`.
    fn fully_locked_for(unlock_at_block: u64) -> u8 {
        let mut registry = LpLockRegistry::new();
        registry
            .lock([1u8; 32], 7, 1_000_000, unlock_at_block)
            .expect("a first lock is accepted");
        let day = 86_400u64;
        registry
            .compute_rug_score(7, 1_000_000, 0, 0, 0, 31 * day)
            .expect("the pool is scoreable")
            .score
    }

    /// The duration term is integer arithmetic, and this pins what it produces.
    ///
    /// It used to be `ln(lock_duration) * 20 / ln(30 days)` in `f64` — which is PHASE 43's
    /// prohibition for a consensus-sensitive decision, because the value of a platform's
    /// `ln` is the platform's, and because `ln` is not in `core`, so the crate could not be
    /// built without `std` while it used it (TICKET-093). `ln(a)/ln(b) == log2(a)/log2(b)`,
    /// so the integer form computes the same ratio; flooring each logarithm means the
    /// sub-score can differ from the old one by at most one point of twenty.
    #[test]
    fn the_duration_term_is_integer_arithmetic_with_a_reachable_maximum() {
        // 30 days is the documented ceiling, and it has to be reachable — a maximum the
        // heuristic can never award is a bug in the heuristic.
        assert_eq!(fully_locked_for(30 * 86_400), 100, "a 30-day lock is the maximum");
        assert_eq!(fully_locked_for(1), 80, "a one-second lock contributes nothing");
        // And it is monotonic in the duration: more lock is never worse.
        let scores: Vec<u8> = [1u64, 3_600, 86_400, 604_800, 2_592_000]
            .iter()
            .map(|seconds| fully_locked_for(*seconds))
            .collect();
        assert!(
            scores.windows(2).all(|pair| pair[0] <= pair[1]),
            "the duration term must not decrease as the lock lengthens: {scores:?}"
        );
        // The four data points the ladder passes through, so a change to the scaling is a
        // failing test rather than a quiet one.
        assert_eq!(scores, vec![80, 90, 95, 98, 100]);
    }

    /// A share of the supply is a ratio of two integers, and the conversion is exact where
    /// the float form was not.
    #[test]
    fn supply_shares_are_exact_integers() {
        let mut registry = LpLockRegistry::new();
        // Half the supply locked: 5000 basis points, exactly.
        registry
            .lock([1u8; 32], 7, 500_000, 30 * 86_400)
            .expect("lock");
        let result = registry
            .compute_rug_score(7, 1_000_000, 0, 0, 0, 31 * 86_400)
            .expect("the pool is scoreable");
        assert_eq!(result.factors.lock_percentage, 5_000);

        // A third, where the float form would have carried a fraction: 3_333, truncated.
        let mut registry = LpLockRegistry::new();
        registry.lock([1u8; 32], 7, 1, 30 * 86_400).expect("lock");
        let result = registry
            .compute_rug_score(7, 3, 0, 0, 0, 31 * 86_400)
            .expect("the pool is scoreable");
        assert_eq!(result.factors.lock_percentage, 3_333);

        // And a supply whose scaling overflows is scored at the ceiling rather than wrapped:
        // an unrepresentable share is not a small one.
        let mut registry = LpLockRegistry::new();
        registry.lock([1u8; 32], 7, u128::MAX / 2, 30 * 86_400).expect("lock");
        let result = registry
            .compute_rug_score(7, u128::MAX, 0, 0, 0, 31 * 86_400)
            .expect("the pool is scoreable");
        assert_eq!(result.factors.lock_percentage, 10_000);
    }
}
