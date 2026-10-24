//! Quantum Swarm Executor
//!
//! The main orchestrator that coordinates:
//! - Quantum optimization (QAOA, VQE, QUBO)
//! - Evolution engine (genetic algorithms)
//! - Arena competition (tournaments)
//! - Strategy execution (arbitrage, portfolio, etc.)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::arena::{Arena, ArenaConfig, ArenaStats, TournamentResult};
use crate::config::QuantumSwarmConfig;
use crate::error::{SwarmError, SwarmResult};
use crate::evolution::{EvolutionConfig, EvolutionEngine, EvolutionStats, Genome};
use crate::quantum::{LocalSimulator, QaoaOptimizer, QuantumBackend, QuboSolver, VqeOptimizer};
use crate::strategy::{MarketData, Signal, Strategy, StrategyFactory, StrategyMetrics};
use crate::types::{OptimizationProblem, QuantumJob, QuantumResult, StrategyId};

/// Executor operating mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutorMode {
    /// Research mode - slower, more exploration
    Research,
    /// Production mode - fast, exploit known good strategies
    Production,
    /// Hybrid mode - balance exploration/exploitation
    Hybrid,
    /// YOLO mode - maximum aggression, no fear
    Yolo,
}

/// Executor state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ExecutorState {
    /// Not started
    #[default]
    Idle,
    /// Initializing components
    Initializing,
    /// Running evolution
    Evolving,
    /// Running tournament
    Tournament,
    /// Executing strategies
    Executing,
    /// Optimizing with quantum
    QuantumOptimizing,
    /// Paused
    Paused,
    /// Stopped
    Stopped,
    /// Error state
    Error,
}

/// Executor statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutorStats {
    /// Total generations evolved
    pub generations: u64,
    /// Total tournaments run
    pub tournaments: u64,
    /// Total strategies evaluated
    pub strategies_evaluated: u64,
    /// Total quantum jobs run
    pub quantum_jobs: u64,
    /// Best fitness achieved
    pub best_fitness: f64,
    /// Total PnL (simulated)
    pub total_pnl: f64,
    /// Current state
    pub state: ExecutorState,
    /// Uptime in seconds
    pub uptime_seconds: u64,
}

/// The Quantum Swarm Executor
pub struct QuantumSwarmExecutor {
    /// Configuration
    config: QuantumSwarmConfig,
    /// Operating mode
    mode: ExecutorMode,
    /// Current state
    state: ExecutorState,
    /// Evolution engine
    evolution: EvolutionEngine,
    /// Arena for competition
    arena: Arena,
    /// Quantum backend
    quantum_backend: Box<dyn QuantumBackend>,
    /// QAOA optimizer
    qaoa: QaoaOptimizer,
    /// VQE optimizer
    vqe: VqeOptimizer,
    /// QUBO solver
    qubo: QuboSolver,
    /// Active strategies
    strategies: HashMap<StrategyId, Box<dyn Strategy>>,
    /// Statistics
    stats: ExecutorStats,
    /// Start time
    start_time: std::time::Instant,
}

impl QuantumSwarmExecutor {
    /// Create new executor with default configuration
    pub fn new(config: QuantumSwarmConfig) -> Self {
        let quantum_config = config.quantum.clone();

        // Use defaults from the actual module types (not config module types)
        let evolution_config = crate::evolution::EvolutionConfig::default();
        let arena_config = crate::arena::ArenaConfig::default();

        Self {
            mode: ExecutorMode::Hybrid,
            state: ExecutorState::Idle,
            evolution: EvolutionEngine::new(evolution_config),
            arena: Arena::new(arena_config),
            quantum_backend: Box::new(LocalSimulator::new(quantum_config.max_qubits)),
            qaoa: QaoaOptimizer::new(crate::quantum::QaoaConfig::default()),
            vqe: VqeOptimizer::new(crate::quantum::VqeConfig::default()),
            qubo: QuboSolver::new(),
            strategies: HashMap::new(),
            stats: ExecutorStats::default(),
            start_time: std::time::Instant::now(),
            config,
        }
    }

    /// Set operating mode
    pub fn set_mode(&mut self, mode: ExecutorMode) {
        self.mode = mode;
    }

    /// Get current state
    pub fn state(&self) -> ExecutorState {
        self.state
    }

    /// Initialize the executor
    pub fn initialize(&mut self, template_genome: &Genome) -> SwarmResult<()> {
        self.state = ExecutorState::Initializing;

        // Initialize evolution engine
        self.evolution.initialize(template_genome);

        // Register initial population in arena
        for genome in self.evolution.population().iter() {
            self.arena.register(genome.clone())?;
        }

        self.state = ExecutorState::Idle;
        Ok(())
    }

    /// Run one cycle of evolution + tournament
    pub fn step(&mut self, market_data: &MarketData) -> SwarmResult<StepResult> {
        let mut result = StepResult::default();

        // 1. Evaluate strategies on market data
        self.state = ExecutorState::Executing;
        let fitness_scores = self.evaluate_strategies(market_data)?;
        result.strategies_evaluated = fitness_scores.len();

        // 2. Run evolution
        self.state = ExecutorState::Evolving;
        let evolution_stats = self.evolution.evolve(&fitness_scores)?;
        result.evolution_stats = Some(evolution_stats.clone());
        self.stats.generations += 1;

        // 3. Run tournament in arena
        self.state = ExecutorState::Tournament;
        let tournament_result = self.arena.run_tournament()?;
        result.tournament_result = Some(tournament_result.clone());
        self.stats.tournaments += 1;

        // Update stats
        self.stats.best_fitness = self.stats.best_fitness.max(self.evolution.best_fitness());
        self.stats.strategies_evaluated += result.strategies_evaluated as u64;
        self.stats.uptime_seconds = self.start_time.elapsed().as_secs();

        self.state = ExecutorState::Idle;
        Ok(result)
    }

    /// Evaluate all strategies on market data
    fn evaluate_strategies(
        &mut self,
        market_data: &MarketData,
    ) -> SwarmResult<HashMap<StrategyId, f64>> {
        if market_data.prices.is_empty() {
            return Err(SwarmError::Strategy(
                "market data contains no prices".into(),
            ));
        }
        for (asset, price) in &market_data.prices {
            if !price.is_finite() || *price <= 0.0 {
                return Err(SwarmError::Strategy(format!(
                    "market price for {asset} must be finite and positive, got {price}"
                )));
            }
        }

        let mut scores = HashMap::new();

        for combatant in self.arena.active_combatants() {
            let id = combatant.id;
            let strategy = self.strategies.get(&id).ok_or_else(|| {
                SwarmError::Strategy(format!(
                    "active combatant {id} has no registered strategy evaluator"
                ))
            })?;
            let signals = strategy.evaluate(market_data)?;
            let mut score = 0.0;
            for signal in signals {
                if !signal.direction.is_finite()
                    || !signal.confidence.is_finite()
                    || !(0.0..=1.0).contains(&signal.confidence)
                {
                    return Err(SwarmError::Strategy(format!(
                        "strategy {id} emitted an invalid signal: direction={}, confidence={}",
                        signal.direction, signal.confidence
                    )));
                }
                score += signal.confidence * signal.direction.abs();
            }
            if !score.is_finite() {
                return Err(SwarmError::Strategy(format!(
                    "strategy {id} produced a non-finite fitness score"
                )));
            }
            scores.insert(id, score);
        }

        self.arena.apply_fitness(&scores)?;
        Ok(scores)
    }

    /// Should we run quantum optimization this cycle
    fn should_quantum_optimize(&self) -> bool {
        match self.mode {
            ExecutorMode::Research => self.stats.generations % 5 == 0,
            ExecutorMode::Production => self.stats.generations % 20 == 0,
            ExecutorMode::Hybrid => self.stats.generations % 10 == 0,
            ExecutorMode::Yolo => true, // Always optimize in YOLO mode
        }
    }

    /// Register a strategy
    pub fn register_strategy(&mut self, strategy: Box<dyn Strategy>) -> SwarmResult<StrategyId> {
        let id = strategy.id();
        self.strategies.insert(id, strategy);
        Ok(id)
    }

    /// Get strategy by ID
    pub fn get_strategy(&self, id: StrategyId) -> Option<&dyn Strategy> {
        self.strategies.get(&id).map(|s| s.as_ref())
    }

    /// Get statistics
    pub fn stats(&self) -> ExecutorStats {
        let mut stats = self.stats.clone();
        stats.state = self.state;
        stats.uptime_seconds = self.start_time.elapsed().as_secs();
        stats
    }

    /// Get arena statistics
    pub fn arena_stats(&self) -> ArenaStats {
        self.arena.stats()
    }

    /// Get best genome
    pub fn best_genome(&self) -> Option<&Genome> {
        self.evolution.best_genome()
    }

    /// Get leaderboard
    pub fn leaderboard(&self, top_n: usize) -> Vec<StrategyId> {
        self.arena
            .leaderboard(top_n)
            .into_iter()
            .map(|c| c.id)
            .collect()
    }

    /// Pause execution
    pub fn pause(&mut self) {
        self.state = ExecutorState::Paused;
    }

    /// Resume execution
    pub fn resume(&mut self) {
        if self.state == ExecutorState::Paused {
            self.state = ExecutorState::Idle;
        }
    }

    /// Stop execution
    pub fn stop(&mut self) {
        self.state = ExecutorState::Stopped;
    }
}

/// Result from a single step
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StepResult {
    /// Strategies evaluated
    pub strategies_evaluated: usize,
    /// Evolution statistics
    pub evolution_stats: Option<EvolutionStats>,
    /// Tournament result
    pub tournament_result: Option<TournamentResult>,
    /// Quantum optimizations performed
    pub quantum_optimizations: usize,
}

/// Async executor wrapper for concurrent execution
pub struct AsyncQuantumSwarmExecutor {
    inner: Arc<RwLock<QuantumSwarmExecutor>>,
}

impl AsyncQuantumSwarmExecutor {
    /// Create new async executor
    pub fn new(config: QuantumSwarmConfig) -> Self {
        Self {
            inner: Arc::new(RwLock::new(QuantumSwarmExecutor::new(config))),
        }
    }

    /// Initialize
    pub async fn initialize(&self, template_genome: &Genome) -> SwarmResult<()> {
        let mut executor = self.inner.write().await;
        executor.initialize(template_genome)
    }

    /// Run step
    pub async fn step(&self, market_data: &MarketData) -> SwarmResult<StepResult> {
        let mut executor = self.inner.write().await;
        executor.step(market_data)
    }

    /// Get stats
    pub async fn stats(&self) -> ExecutorStats {
        let executor = self.inner.read().await;
        executor.stats()
    }

    /// Get state
    pub async fn state(&self) -> ExecutorState {
        let executor = self.inner.read().await;
        executor.state()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::{ArbitrageConfig, ArbitrageStrategy};

    #[test]
    fn test_executor_creation() {
        let config = QuantumSwarmConfig::default();
        let executor = QuantumSwarmExecutor::new(config);

        assert_eq!(executor.state(), ExecutorState::Idle);
    }

    #[test]
    fn test_executor_initialization() {
        let config = QuantumSwarmConfig::default();
        let mut executor = QuantumSwarmExecutor::new(config);

        let template = Genome::new_float(5, 0.0, 1.0);
        executor.initialize(&template).unwrap();

        assert!(executor.arena_stats().active_combatants > 0);
    }

    #[test]
    fn test_executor_step() {
        // Use default config - the actual module defaults handle population size
        let config = QuantumSwarmConfig::default();

        let mut executor = QuantumSwarmExecutor::new(config);

        let template = Genome::new_float(5, 0.0, 1.0);
        executor.initialize(&template).unwrap();
        let genomes: Vec<_> = executor.evolution.population().iter().cloned().collect();
        for genome in genomes {
            executor
                .register_strategy(Box::new(ArbitrageStrategy::new(
                    genome,
                    ArbitrageConfig {
                        min_profit_bps: 10,
                        max_position: 100_000,
                        venues: vec!["Uniswap".into(), "Sushiswap".into()],
                        cross_chain: false,
                    },
                )))
                .unwrap();
        }

        let mut market_data = MarketData::default();
        market_data.prices.insert("ETH".to_string(), 2000.0);
        market_data.prices.insert("BTC".to_string(), 50000.0);
        market_data.venue_prices.insert(
            "Uniswap".into(),
            [("ETH".to_string(), 2_000.0), ("BTC".to_string(), 50_000.0)]
                .into_iter()
                .collect(),
        );
        market_data.venue_prices.insert(
            "Sushiswap".into(),
            [("ETH".to_string(), 2_010.0), ("BTC".to_string(), 50_100.0)]
                .into_iter()
                .collect(),
        );

        let result = executor.step(&market_data).unwrap();

        assert!(result.strategies_evaluated > 0);
        assert!(result.evolution_stats.is_some());
        assert!(executor.stats.best_fitness > 0.0);
    }

    #[test]
    fn executor_refuses_to_score_combatants_without_strategy_evaluators() {
        let mut executor = QuantumSwarmExecutor::new(QuantumSwarmConfig::default());
        let template = Genome::new_float(5, 0.0, 1.0);
        executor.initialize(&template).unwrap();
        let mut market_data = MarketData::default();
        market_data.prices.insert("ETH".into(), 2_000.0);

        let error = executor
            .step(&market_data)
            .expect_err("missing evaluators must fail closed");
        assert!(
            error
                .to_string()
                .contains("no registered strategy evaluator"),
            "{error}"
        );
    }

    #[test]
    fn test_executor_modes() {
        let config = QuantumSwarmConfig::default();
        let mut executor = QuantumSwarmExecutor::new(config);

        executor.set_mode(ExecutorMode::Yolo);
        assert!(executor.should_quantum_optimize());

        executor.set_mode(ExecutorMode::Production);
        // Won't optimize on gen 0
    }
}
