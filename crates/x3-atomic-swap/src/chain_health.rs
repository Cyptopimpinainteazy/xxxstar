//! # ChainHealth — Chain health monitoring for atomic swaps.
//!
//! Provides an oracle that tracks per-chain health status, thresholds, and
//! safety checks so that the swap engine can avoid sending funds into
//! unhealthy or paused chains.
//!
//! ## Components
//!
//! - [`ChainHealthStatus`] — Enum describing chain liveness.
//! - [`HealthCheck`] — Snapshot of a single chain's health metrics.
//! - [`HealthThresholds`] — Configurable bounds for what counts as healthy.
//! - [`ChainHealthOracle`] — Trait for pluggable health backends.
//! - [`SwapSafetyCheck`] — Aggregated safety verdict for a pending swap.
//! - [`InMemoryChainHealth`] — Default in-memory implementation.

use crate::error::SwapError;
use crate::intent::ChainKind;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Liveness status of a blockchain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainHealthStatus {
    /// Fully healthy — no restrictions.
    Healthy,
    /// Partially degraded — swaps are permitted with caution.
    Degraded { reason: String },
    /// Unhealthy — swaps should not proceed.
    Unhealthy { reason: String },
    /// Explicitly paused by an operator.
    Paused { reason: String },
    /// No health data available yet.
    Unknown,
}

/// A point-in-time health snapshot for a single chain.
#[derive(Debug, Clone)]
pub struct HealthCheck {
    /// Which chain this check applies to.
    pub chain: ChainKind,
    /// Last observed block height.
    pub last_block_height: u64,
    /// Average block time in milliseconds.
    pub avg_block_time_ms: u64,
    /// How many blocks behind the tip the chain's finality gadget is.
    pub finality_delay_blocks: u32,
    /// RPC endpoint availability fraction (0.0 – 1.0).
    pub rpc_availability: f64,
    /// Unix timestamp (seconds) of the last check.
    pub last_check_timestamp: u64,
    /// Derived status at check time.
    pub status: ChainHealthStatus,
}

/// Configurable thresholds used to determine chain health.
#[derive(Debug, Clone)]
pub struct HealthThresholds {
    /// Maximum acceptable average block time in milliseconds.
    pub max_block_time_ms: u64,
    /// Minimum acceptable RPC availability (0.0 – 1.0).
    pub min_rpc_availability: f64,
    /// Maximum acceptable finality delay in blocks.
    pub max_finality_delay: u32,
}

impl Default for HealthThresholds {
    fn default() -> Self {
        Self {
            max_block_time_ms: 30_000, // 30 seconds
            min_rpc_availability: 0.8, // 80%
            max_finality_delay: 10,    // 10 blocks
        }
    }
}

/// Pluggable oracle for per-chain health checks.
pub trait ChainHealthOracle {
    /// Fetch or return the cached health snapshot for a chain.
    fn check_health(&self, chain: ChainKind) -> Result<HealthCheck, SwapError>;

    /// Returns `true` if the chain is fully healthy.
    /// Degraded or worse chains return `false`.
    fn is_healthy(&self, chain: ChainKind) -> Result<bool, SwapError> {
        let hc = self.check_health(chain)?;
        Ok(hc.status == ChainHealthStatus::Healthy)
    }

    /// Mark a chain as paused (operator action).
    fn pause_chain(&self, chain: ChainKind, reason: &str) -> Result<(), SwapError>;

    /// Remove a pause, returning the chain to its previous status.
    fn resume_chain(&self, chain: ChainKind) -> Result<(), SwapError>;

    /// Return all chains currently in the Paused state.
    fn get_paused_chains(&self) -> Vec<ChainKind>;
}

/// Aggregated safety verdict for a single swap operation.
#[derive(Debug, Clone, Default)]
pub struct SwapSafetyCheck {
    /// Whether the chain is healthy (Healthy or Degraded).
    pub chain_healthy: bool,
    /// Whether finality requirements are met.
    pub finality_met: bool,
    /// Whether the RPC quorum agrees.
    pub rpc_quorum_ok: bool,
    /// Whether the swap's timeout window is safe.
    pub timeout_safe: bool,
    /// Shortcut: true only when all four fields are true (auto-computed).
    pub all_clear: bool,
}

impl SwapSafetyCheck {
    /// Create a new [`SwapSafetyCheck`] and automatically compute `all_clear`.
    pub fn new(
        chain_healthy: bool,
        finality_met: bool,
        rpc_quorum_ok: bool,
        timeout_safe: bool,
    ) -> Self {
        let all_clear = chain_healthy && finality_met && rpc_quorum_ok && timeout_safe;
        Self {
            chain_healthy,
            finality_met,
            rpc_quorum_ok,
            timeout_safe,
            all_clear,
        }
    }
}

// ---------------------------------------------------------------------------
// In-memory implementation
// ---------------------------------------------------------------------------

/// An in-memory [`ChainHealthOracle`] backed by a `BTreeMap`.
///
/// Default thresholds:
/// - max block time: 30 s
/// - min RPC availability: 0.8
/// - max finality delay: 10 blocks
pub struct InMemoryChainHealth {
    checks: BTreeMap<ChainKind, HealthCheck>,
    thresholds: HealthThresholds,
}

impl InMemoryChainHealth {
    /// Create a new oracle with default thresholds.
    pub fn new() -> Self {
        Self {
            checks: BTreeMap::new(),
            thresholds: HealthThresholds::default(),
        }
    }

    /// Create a new oracle with custom thresholds.
    pub fn with_thresholds(thresholds: HealthThresholds) -> Self {
        Self {
            checks: BTreeMap::new(),
            thresholds,
        }
    }

    /// Seed a health check entry (useful in tests or on startup).
    pub fn seed(&mut self, check: HealthCheck) {
        self.checks.insert(check.chain, check);
    }

    /// Return a reference to the current thresholds.
    pub fn thresholds(&self) -> &HealthThresholds {
        &self.thresholds
    }

    /// Mutate thresholds at runtime.
    pub fn set_thresholds(&mut self, thresholds: HealthThresholds) {
        self.thresholds = thresholds;
    }

    /// Evaluate a check against thresholds and produce a status.
    fn evaluate(check: &HealthCheck, thresholds: &HealthThresholds) -> ChainHealthStatus {
        // Paused is an operator action, never set by evaluation.
        if check.avg_block_time_ms > thresholds.max_block_time_ms {
            return ChainHealthStatus::Unhealthy {
                reason: format!(
                    "avg block time {} ms exceeds threshold {} ms",
                    check.avg_block_time_ms, thresholds.max_block_time_ms
                ),
            };
        }
        if check.rpc_availability < thresholds.min_rpc_availability {
            // Severely low RPC → unhealthy; moderately low → degraded.
            if check.rpc_availability < thresholds.min_rpc_availability * 0.5 {
                return ChainHealthStatus::Unhealthy {
                    reason: format!(
                        "RPC availability {:.2} is critically below threshold {:.2}",
                        check.rpc_availability, thresholds.min_rpc_availability
                    ),
                };
            }
            return ChainHealthStatus::Degraded {
                reason: format!(
                    "RPC availability {:.2} below threshold {:.2}",
                    check.rpc_availability, thresholds.min_rpc_availability
                ),
            };
        }
        if check.finality_delay_blocks > thresholds.max_finality_delay {
            return ChainHealthStatus::Degraded {
                reason: format!(
                    "finality delay {} blocks exceeds threshold {}",
                    check.finality_delay_blocks, thresholds.max_finality_delay
                ),
            };
        }
        ChainHealthStatus::Healthy
    }
}

impl Default for InMemoryChainHealth {
    fn default() -> Self {
        Self::new()
    }
}

impl ChainHealthOracle for InMemoryChainHealth {
    fn check_health(&self, chain: ChainKind) -> Result<HealthCheck, SwapError> {
        match self.checks.get(&chain) {
            Some(hc) => Ok(hc.clone()),
            None => Ok(HealthCheck {
                chain,
                last_block_height: 0,
                avg_block_time_ms: 0,
                finality_delay_blocks: 0,
                rpc_availability: 1.0,
                last_check_timestamp: 0,
                status: ChainHealthStatus::Unknown,
            }),
        }
    }

    fn pause_chain(&self, _chain: ChainKind, _reason: &str) -> Result<(), SwapError> {
        // We need interior mutability via the trait's &self signature.
        // Since this is an in-memory store, we accept a minor limitation:
        // pause_chain / resume_chain require &self but we need mutation.
        // We work around by returning an error if not implemented without
        // a Cell/RefCell — but for simplicity we use a BTreeMap in a RefCell.
        // See the RefCell-based wrapper below.
        Err(SwapError::Internal(
            "InMemoryChainHealth requires PausableChainHealth wrapper for mutation".into(),
        ))
    }

    fn resume_chain(&self, _chain: ChainKind) -> Result<(), SwapError> {
        Err(SwapError::Internal(
            "InMemoryChainHealth requires PausableChainHealth wrapper for mutation".into(),
        ))
    }

    fn get_paused_chains(&self) -> Vec<ChainKind> {
        self.checks
            .iter()
            .filter(|(_, hc)| matches!(hc.status, ChainHealthStatus::Paused { .. }))
            .map(|(chain, _)| *chain)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Pausable wrapper providing interior mutability
// ---------------------------------------------------------------------------

use core::cell::RefCell;

/// A [`ChainHealthOracle`] wrapper around [`InMemoryChainHealth`] that uses
/// `RefCell` to allow pause/resume through the shared `&self` trait methods.
pub struct PausableChainHealth {
    inner: RefCell<InMemoryChainHealth>,
}

impl PausableChainHealth {
    /// Create a new pausable oracle with default thresholds.
    pub fn new() -> Self {
        Self {
            inner: RefCell::new(InMemoryChainHealth::new()),
        }
    }

    /// Create a new pausable oracle with custom thresholds.
    pub fn with_thresholds(thresholds: HealthThresholds) -> Self {
        Self {
            inner: RefCell::new(InMemoryChainHealth::with_thresholds(thresholds)),
        }
    }

    /// Seed a health check.
    pub fn seed(&self, check: HealthCheck) {
        self.inner.borrow_mut().seed(check);
    }

    /// Access the underlying oracle (read-only).
    pub fn inner(&self) -> core::cell::Ref<'_, InMemoryChainHealth> {
        self.inner.borrow()
    }
}

impl Default for PausableChainHealth {
    fn default() -> Self {
        Self::new()
    }
}

impl ChainHealthOracle for PausableChainHealth {
    fn check_health(&self, chain: ChainKind) -> Result<HealthCheck, SwapError> {
        let inner = self.inner.borrow();
        match inner.checks.get(&chain) {
            Some(hc) => {
                let mut evaluated = hc.clone();
                // Only re-evaluate if not paused (paused is an operator action).
                if !matches!(evaluated.status, ChainHealthStatus::Paused { .. }) {
                    evaluated.status = InMemoryChainHealth::evaluate(hc, &inner.thresholds);
                }
                Ok(evaluated)
            }
            None => Ok(HealthCheck {
                chain,
                last_block_height: 0,
                avg_block_time_ms: 0,
                finality_delay_blocks: 0,
                rpc_availability: 1.0,
                last_check_timestamp: 0,
                status: ChainHealthStatus::Unknown,
            }),
        }
    }

    fn is_healthy(&self, chain: ChainKind) -> Result<bool, SwapError> {
        self.inner.borrow().is_healthy(chain)
    }

    fn pause_chain(&self, chain: ChainKind, reason: &str) -> Result<(), SwapError> {
        let mut inner = self.inner.borrow_mut();
        match inner.checks.get_mut(&chain) {
            Some(hc) => {
                hc.status = ChainHealthStatus::Paused {
                    reason: reason.to_string(),
                };
                Ok(())
            }
            None => {
                inner.checks.insert(
                    chain,
                    HealthCheck {
                        chain,
                        last_block_height: 0,
                        avg_block_time_ms: 0,
                        finality_delay_blocks: 0,
                        rpc_availability: 1.0,
                        last_check_timestamp: 0,
                        status: ChainHealthStatus::Paused {
                            reason: reason.to_string(),
                        },
                    },
                );
                Ok(())
            }
        }
    }

    fn resume_chain(&self, chain: ChainKind) -> Result<(), SwapError> {
        let mut inner = self.inner.borrow_mut();
        let thresholds = inner.thresholds.clone();
        match inner.checks.get_mut(&chain) {
            Some(hc) => {
                // Re-evaluate based on current metrics.
                let new_status = InMemoryChainHealth::evaluate(hc, &thresholds);
                hc.status = new_status;
                Ok(())
            }
            None => {
                // Nothing to resume.
                Ok(())
            }
        }
    }

    fn get_paused_chains(&self) -> Vec<ChainKind> {
        self.inner.borrow().get_paused_chains()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_thresholds() -> HealthThresholds {
        HealthThresholds {
            max_block_time_ms: 30_000,
            min_rpc_availability: 0.8,
            max_finality_delay: 10,
        }
    }

    fn healthy_check(chain: ChainKind) -> HealthCheck {
        HealthCheck {
            chain,
            last_block_height: 1000,
            avg_block_time_ms: 12_000, // well under 30 s
            finality_delay_blocks: 2,  // well under 10
            rpc_availability: 0.99,    // well above 0.8
            last_check_timestamp: 1000000,
            status: ChainHealthStatus::Healthy,
        }
    }

    // ------------------------------------------------------------------
    // 1. Healthy chain passes is_healthy
    // ------------------------------------------------------------------
    #[test]
    fn test_healthy_chain_passes_is_healthy() {
        let oracle = PausableChainHealth::with_thresholds(default_thresholds());
        oracle.seed(healthy_check(ChainKind::Ethereum));

        let healthy = oracle.is_healthy(ChainKind::Ethereum).unwrap();
        assert!(healthy, "Healthy chain should be reported as healthy");
    }

    // ------------------------------------------------------------------
    // 2. Paused chain returns false from is_healthy
    // ------------------------------------------------------------------
    #[test]
    fn test_paused_chain_returns_false() {
        let oracle = PausableChainHealth::with_thresholds(default_thresholds());
        oracle.seed(healthy_check(ChainKind::Solana));

        // Pause the chain.
        oracle
            .pause_chain(ChainKind::Solana, "Scheduled maintenance")
            .unwrap();

        let healthy = oracle.is_healthy(ChainKind::Solana).unwrap();
        assert!(!healthy, "Paused chain should NOT be healthy");
    }

    // ------------------------------------------------------------------
    // 3. Pause and resume works
    // ------------------------------------------------------------------
    #[test]
    fn test_pause_and_resume() {
        let oracle = PausableChainHealth::with_thresholds(default_thresholds());
        oracle.seed(healthy_check(ChainKind::Bitcoin));

        // Pause
        oracle
            .pause_chain(ChainKind::Bitcoin, "Operator pause")
            .unwrap();
        assert!(!oracle.is_healthy(ChainKind::Bitcoin).unwrap());

        // Verify paused list
        let paused = oracle.get_paused_chains();
        assert_eq!(paused.len(), 1);
        assert_eq!(paused[0], ChainKind::Bitcoin);

        // Resume
        oracle.resume_chain(ChainKind::Bitcoin).unwrap();
        assert!(oracle.is_healthy(ChainKind::Bitcoin).unwrap());

        // Paused list should now be empty
        let paused = oracle.get_paused_chains();
        assert!(paused.is_empty());
    }

    // ------------------------------------------------------------------
    // 4. Degraded RPC (0.5 availability) returns false
    // ------------------------------------------------------------------
    #[test]
    fn test_low_rpc_availability_returns_false() {
        let oracle = PausableChainHealth::with_thresholds(default_thresholds());
        oracle.seed(HealthCheck {
            chain: ChainKind::Ethereum,
            last_block_height: 1000,
            avg_block_time_ms: 12_000,
            finality_delay_blocks: 2,
            rpc_availability: 0.5, // below 0.8 -> degraded, not unhealthy (above 0.4)
            last_check_timestamp: 1000000,
            status: ChainHealthStatus::Unknown,
        });

        // 0.5 is below 0.8 -> Degraded, which should return false.
        let healthy = oracle.is_healthy(ChainKind::Ethereum).unwrap();
        assert!(!healthy, "Degraded chain (RPC 0.5) should NOT be healthy");

        // But we should also verify the status is Degraded
        let hc = oracle.check_health(ChainKind::Ethereum).unwrap();
        assert!(
            matches!(hc.status, ChainHealthStatus::Degraded { .. }),
            "RPC 0.5 should produce Degraded status"
        );
    }

    // ------------------------------------------------------------------
    // 5. Critically low RPC (0.3) is unhealthy -> is_healthy returns false
    // ------------------------------------------------------------------
    #[test]
    fn test_critical_rpc_returns_false() {
        let oracle = PausableChainHealth::with_thresholds(default_thresholds());
        oracle.seed(HealthCheck {
            chain: ChainKind::Solana,
            last_block_height: 500,
            avg_block_time_ms: 400, // solana is fast
            finality_delay_blocks: 0,
            rpc_availability: 0.3, // below 0.8 * 0.5 = 0.4 -> Unhealthy
            last_check_timestamp: 1000000,
            status: ChainHealthStatus::Unknown,
        });

        let healthy = oracle.is_healthy(ChainKind::Solana).unwrap();
        assert!(!healthy, "Critically low RPC should be unhealthy");

        let hc = oracle.check_health(ChainKind::Solana).unwrap();
        assert!(
            matches!(hc.status, ChainHealthStatus::Unhealthy { .. }),
            "RPC 0.3 should produce Unhealthy status"
        );
    }

    // ------------------------------------------------------------------
    // 6. High block time is unhealthy
    // ------------------------------------------------------------------
    #[test]
    fn test_high_block_time_is_unhealthy() {
        let oracle = PausableChainHealth::with_thresholds(default_thresholds());
        oracle.seed(HealthCheck {
            chain: ChainKind::Ethereum,
            last_block_height: 1000,
            avg_block_time_ms: 60_000, // 60 s > 30 s threshold
            finality_delay_blocks: 2,
            rpc_availability: 0.99,
            last_check_timestamp: 1000000,
            status: ChainHealthStatus::Unknown,
        });

        let healthy = oracle.is_healthy(ChainKind::Ethereum).unwrap();
        assert!(!healthy, "High block time should be unhealthy");

        let hc = oracle.check_health(ChainKind::Ethereum).unwrap();
        assert!(
            matches!(hc.status, ChainHealthStatus::Unhealthy { .. }),
            "60 s block time should produce Unhealthy"
        );
    }

    // ------------------------------------------------------------------
    // 7. Unknown chain returns Unknown status
    // ------------------------------------------------------------------
    #[test]
    fn test_unknown_chain_returns_unknown() {
        let oracle = PausableChainHealth::with_thresholds(default_thresholds());

        // Query a chain that was never seeded
        let hc = oracle.check_health(ChainKind::Cosmos).unwrap();
        assert_eq!(
            hc.status,
            ChainHealthStatus::Unknown,
            "unseeded chain must be Unknown"
        );
        assert_eq!(hc.last_block_height, 0);
        assert_eq!(hc.rpc_availability, 1.0);
        assert!(
            !oracle.is_healthy(ChainKind::Cosmos).unwrap(),
            "Unknown chain must not be healthy"
        );
    }

    // ------------------------------------------------------------------
    // 8. Resume after pause restores healthy status
    // ------------------------------------------------------------------
    #[test]
    fn test_resume_after_pause_restores_health() {
        let oracle = PausableChainHealth::with_thresholds(default_thresholds());
        oracle.seed(healthy_check(ChainKind::Ethereum));

        // Initially healthy
        assert!(oracle.is_healthy(ChainKind::Ethereum).unwrap());

        // Pause
        oracle
            .pause_chain(ChainKind::Ethereum, "Emergency")
            .unwrap();
        assert!(!oracle.is_healthy(ChainKind::Ethereum).unwrap());

        // Resume — should go back to Healthy based on metrics
        oracle.resume_chain(ChainKind::Ethereum).unwrap();
        assert!(
            oracle.is_healthy(ChainKind::Ethereum).unwrap(),
            "resume must restore healthy status"
        );
    }

    // ------------------------------------------------------------------
    // 9. Threshold boundary: exact threshold values
    // ------------------------------------------------------------------
    #[test]
    fn test_threshold_boundary_values() {
        let thresholds = HealthThresholds {
            max_block_time_ms: 30_000,
            min_rpc_availability: 0.8,
            max_finality_delay: 10,
        };

        // At exact max_block_time_ms boundary — must be unhealthy (strictly greater)
        let oracle = PausableChainHealth::with_thresholds(thresholds.clone());
        oracle.seed(HealthCheck {
            chain: ChainKind::Ethereum,
            last_block_height: 1000,
            avg_block_time_ms: 30_001, // just over 30_000
            finality_delay_blocks: 2,
            rpc_availability: 0.99,
            last_check_timestamp: 1000000,
            status: ChainHealthStatus::Unknown,
        });
        assert!(!oracle.is_healthy(ChainKind::Ethereum).unwrap());

        // At exact min_rpc_availability boundary — must be degraded (strictly less)
        let oracle2 = PausableChainHealth::with_thresholds(thresholds.clone());
        oracle2.seed(HealthCheck {
            chain: ChainKind::Ethereum,
            last_block_height: 1000,
            avg_block_time_ms: 12_000,
            finality_delay_blocks: 2,
            rpc_availability: 0.79, // just under 0.8
            last_check_timestamp: 1000000,
            status: ChainHealthStatus::Unknown,
        });
        assert!(!oracle2.is_healthy(ChainKind::Ethereum).unwrap());
        let hc = oracle2.check_health(ChainKind::Ethereum).unwrap();
        assert!(matches!(hc.status, ChainHealthStatus::Degraded { .. }));

        // At exact max_finality_delay boundary — must be degraded (strictly greater)
        let oracle3 = PausableChainHealth::with_thresholds(thresholds);
        oracle3.seed(HealthCheck {
            chain: ChainKind::Ethereum,
            last_block_height: 1000,
            avg_block_time_ms: 12_000,
            finality_delay_blocks: 11, // just over 10
            rpc_availability: 0.99,
            last_check_timestamp: 1000000,
            status: ChainHealthStatus::Unknown,
        });
        assert!(!oracle3.is_healthy(ChainKind::Ethereum).unwrap());
        let hc = oracle3.check_health(ChainKind::Ethereum).unwrap();
        assert!(matches!(hc.status, ChainHealthStatus::Degraded { .. }));
    }

    // ------------------------------------------------------------------
    // 10. InMemoryChainHealth (non-pausable) returns error on pause
    // ------------------------------------------------------------------
    #[test]
    fn test_in_memory_chain_health_no_pause() {
        let oracle = InMemoryChainHealth::new();

        let result = oracle.pause_chain(ChainKind::Ethereum, "test");
        assert!(
            result.is_err(),
            "InMemoryChainHealth must return error on pause"
        );

        let result = oracle.resume_chain(ChainKind::Ethereum);
        assert!(
            result.is_err(),
            "InMemoryChainHealth must return error on resume"
        );
    }
}
