//! # Solver & Relayer Registry
//!
//! On-chain registries for solvers and relayers that participate in X3 atomic swaps.
//! Each registry tracks stake, performance, and supported assets/chains so the
//! swap orchestrator can select the best participant for a given route.
//!
//! ## Models
//!
//! - [`SolverModel`] — A solver that fills swap intents
//! - [`RelayerModel`] — A relayer that watches chains and submits proofs
//! - [`SolverRegistry`] — Collection of solvers with selection helpers
//! - [`RelayerRegistry`] — Collection of relayers with selection helpers

use crate::intent::ChainKind;
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// SolverModel
// ─────────────────────────────────────────────────────────────────────────────

/// A registered solver that fills cross-chain swap intents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverModel {
    /// Unique solver identifier.
    pub solver_id: String,
    /// Amount staked (in the native X3 unit).
    pub stake: u128,
    /// Chains this solver supports.
    pub supported_chains: Vec<ChainKind>,
    /// Asset symbols this solver supports (e.g. ["USDC", "SOL", "ETH"]).
    pub supported_assets: Vec<String>,
    /// Number of successful fills.
    pub success_count: u64,
    /// Number of failed fills.
    pub failure_count: u64,
    /// Whether the solver is currently active.
    pub active: bool,
}

impl SolverModel {
    /// Create a new solver model.
    pub fn new(
        solver_id: String,
        stake: u128,
        supported_chains: Vec<ChainKind>,
        supported_assets: Vec<String>,
    ) -> Self {
        Self {
            solver_id,
            stake,
            supported_chains,
            supported_assets,
            success_count: 0,
            failure_count: 0,
            active: true,
        }
    }

    /// Computed reputation score: success_count * 100 / (total + 1).
    /// The +1 avoids division by zero for fresh solvers.
    pub fn reputation_score(&self) -> u32 {
        let total = self.success_count + self.failure_count + 1;
        ((self.success_count * 100) / total) as u32
    }

    /// Record a successful fill.
    pub fn record_success(&mut self) {
        self.success_count += 1;
    }

    /// Record a failed fill.
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// RelayerModel
// ─────────────────────────────────────────────────────────────────────────────

/// A registered relayer that watches chains and submits proof records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayerModel {
    /// Unique relayer identifier.
    pub relayer_id: String,
    /// Amount staked (in the native X3 unit).
    pub stake: u128,
    /// Chains this relayer supports.
    pub supported_chains: Vec<ChainKind>,
    /// Number of successfully relayed swaps.
    pub success_count: u64,
    /// Number of failed relay attempts.
    pub failure_count: u64,
    /// Number of times this relayer has been slashed.
    pub slash_count: u64,
    /// Whether the relayer is currently active.
    pub active: bool,
}

impl RelayerModel {
    /// Create a new relayer model.
    pub fn new(relayer_id: String, stake: u128, supported_chains: Vec<ChainKind>) -> Self {
        Self {
            relayer_id,
            stake,
            supported_chains,
            success_count: 0,
            failure_count: 0,
            slash_count: 0,
            active: true,
        }
    }

    /// Record a successful relay.
    pub fn record_success(&mut self) {
        self.success_count += 1;
    }

    /// Record a failed relay.
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
    }

    /// Record a slash event.
    pub fn record_slash(&mut self) {
        self.slash_count += 1;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SolverRegistry
// ─────────────────────────────────────────────────────────────────────────────

/// Registry of solver models.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SolverRegistry {
    pub solvers: Vec<SolverModel>,
}

impl SolverRegistry {
    /// Create an empty solver registry.
    pub fn new() -> Self {
        Self {
            solvers: Vec::new(),
        }
    }

    /// Register a solver.
    pub fn register(&mut self, solver: SolverModel) {
        self.solvers.push(solver);
    }

    /// Deactivate a solver by ID.
    pub fn deactivate(&mut self, solver_id: &str) {
        for solver in &mut self.solvers {
            if solver.solver_id == solver_id {
                solver.active = false;
            }
        }
    }

    /// Get all active solvers.
    pub fn get_active(&self) -> Vec<&SolverModel> {
        self.solvers.iter().filter(|s| s.active).collect()
    }

    /// Get the top N solvers by reputation score for a given chain and asset.
    pub fn top_by_reputation(
        &self,
        chain: ChainKind,
        asset: &str,
        limit: usize,
    ) -> Vec<&SolverModel> {
        let mut candidates: Vec<&SolverModel> = self
            .solvers
            .iter()
            .filter(|s| {
                s.active
                    && s.supported_chains.contains(&chain)
                    && s.supported_assets.iter().any(|a| a == asset)
            })
            .collect();
        candidates.sort_by_key(|s| -(s.reputation_score() as i64));
        candidates.truncate(limit);
        candidates
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// RelayerRegistry
// ─────────────────────────────────────────────────────────────────────────────

/// Registry of relayer models.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RelayerRegistry {
    pub relayers: Vec<RelayerModel>,
}

impl RelayerRegistry {
    /// Create an empty relayer registry.
    pub fn new() -> Self {
        Self {
            relayers: Vec::new(),
        }
    }

    /// Register a relayer.
    pub fn register(&mut self, relayer: RelayerModel) {
        self.relayers.push(relayer);
    }

    /// Deactivate a relayer by ID.
    pub fn deactivate(&mut self, relayer_id: &str) {
        for relayer in &mut self.relayers {
            if relayer.relayer_id == relayer_id {
                relayer.active = false;
            }
        }
    }

    /// Get all active relayers.
    pub fn get_active(&self) -> Vec<&RelayerModel> {
        self.relayers.iter().filter(|r| r.active).collect()
    }

    /// Get the top N relayers by success rate for a given chain.
    pub fn top_by_success(&self, chain: ChainKind, limit: usize) -> Vec<&RelayerModel> {
        let mut candidates: Vec<&RelayerModel> = self
            .relayers
            .iter()
            .filter(|r| r.active && r.supported_chains.contains(&chain))
            .collect();
        candidates.sort_by_key(|r| -(r.success_count as i64));
        candidates.truncate(limit);
        candidates
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solver_reputation() {
        let mut solver = SolverModel::new(
            "solver-1".into(),
            1000,
            vec![ChainKind::Ethereum, ChainKind::Solana],
            vec!["USDC".into(), "SOL".into()],
        );
        solver.success_count = 90;
        solver.failure_count = 10;
        // (90 * 100) / (90 + 10 + 1) = 9000 / 101 ≈ 89
        assert_eq!(solver.reputation_score(), 89);
    }

    #[test]
    fn test_solver_reputation_zero() {
        let solver = SolverModel::new(
            "solver-new".into(),
            500,
            vec![ChainKind::X3],
            vec!["X3".into()],
        );
        // (0 * 100) / (0 + 0 + 1) = 0
        assert_eq!(solver.reputation_score(), 0);
    }

    #[test]
    fn test_solver_registry_top_by_reputation() {
        let mut registry = SolverRegistry::new();
        let mut s1 = SolverModel::new(
            "s1".into(),
            1000,
            vec![ChainKind::Ethereum],
            vec!["USDC".into()],
        );
        s1.success_count = 50;
        s1.failure_count = 50;

        let mut s2 = SolverModel::new(
            "s2".into(),
            2000,
            vec![ChainKind::Ethereum],
            vec!["USDC".into()],
        );
        s2.success_count = 95;
        s2.failure_count = 5;

        let mut s3 = SolverModel::new(
            "s3".into(),
            500,
            vec![ChainKind::Solana],
            vec!["SOL".into()],
        );
        s3.success_count = 100;
        registry.register(s1);
        registry.register(s2);
        registry.register(s3);

        let top = registry.top_by_reputation(ChainKind::Ethereum, "USDC", 2);
        assert_eq!(top.len(), 2);
        // s2 has higher reputation
        assert_eq!(top[0].solver_id, "s2");
        assert_eq!(top[1].solver_id, "s1");
    }

    #[test]
    fn test_solver_registry_deactivate() {
        let mut registry = SolverRegistry::new();
        registry.register(SolverModel::new(
            "s1".into(),
            1000,
            vec![ChainKind::X3],
            vec!["X3".into()],
        ));
        assert_eq!(registry.get_active().len(), 1);
        registry.deactivate("s1");
        assert_eq!(registry.get_active().len(), 0);
    }

    #[test]
    fn test_relayer_registry_register_and_get_active() {
        let mut registry = RelayerRegistry::new();
        registry.register(RelayerModel::new(
            "r1".into(),
            2000,
            vec![ChainKind::Ethereum, ChainKind::Solana],
        ));
        registry.register(RelayerModel::new(
            "r2".into(),
            1500,
            vec![ChainKind::Solana],
        ));
        assert_eq!(registry.get_active().len(), 2);
        registry.deactivate("r1");
        assert_eq!(registry.get_active().len(), 1);
    }

    #[test]
    fn test_relayer_registry_top_by_success() {
        let mut registry = RelayerRegistry::new();
        let mut r1 = RelayerModel::new("r1".into(), 1000, vec![ChainKind::Solana]);
        r1.success_count = 30;
        r1.failure_count = 5;
        let mut r2 = RelayerModel::new("r2".into(), 2000, vec![ChainKind::Solana]);
        r2.success_count = 80;
        r2.failure_count = 2;
        registry.register(r1);
        registry.register(r2);

        let top = registry.top_by_success(ChainKind::Solana, 1);
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].relayer_id, "r2");
    }

    #[test]
    fn test_relayer_model_slash() {
        let mut relayer = RelayerModel::new("r1".into(), 1000, vec![ChainKind::Ethereum]);
        assert_eq!(relayer.slash_count, 0);
        relayer.record_slash();
        assert_eq!(relayer.slash_count, 1);
        relayer.record_slash();
        assert_eq!(relayer.slash_count, 2);
    }
}
