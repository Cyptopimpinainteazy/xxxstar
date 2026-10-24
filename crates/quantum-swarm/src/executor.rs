//! Quantum Swarm Executor
//!
//! The main orchestrator that coordinates:
//! - Quantum optimization (QAOA, VQE, QUBO)
//! - Evolution engine (genetic algorithms)
//! - Arena competition (tournaments)
//! - Strategy execution (arbitrage, portfolio, etc.)

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::arena::{Arena, ArenaConfig, ArenaStats, TournamentResult};
use crate::config::QuantumSwarmConfig;
use crate::error::{SwarmError, SwarmResult};
use crate::evolution::{EvolutionConfig, EvolutionEngine, EvolutionStats, Genome};
use crate::quantum::{LocalSimulator, QaoaOptimizer, QuantumBackend, QuboSolver, VqeOptimizer};
use crate::strategy::{MarketData, Signal, Strategy, StrategyFactory, StrategyMetrics};
use crate::types::{OptimizationProblem, QuantumJob, QuantumResult, StrategyId};

/// Creates the evaluator for a genome. The returned strategy must use the genome's ID.
pub type StrategyBuilder = Arc<dyn Fn(&Genome) -> SwarmResult<Box<dyn Strategy>> + Send + Sync>;

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
    /// Builds evaluators for initial and newly bred genomes.
    strategy_builder: Option<StrategyBuilder>,
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
        let evolution_config = crate::evolution::EvolutionConfig {
            seed: config.simulation_seed,
            ..Default::default()
        };
        let arena_config = crate::arena::ArenaConfig {
            evolution_config: evolution_config.clone(),
            ..Default::default()
        };

        Self {
            mode: ExecutorMode::Hybrid,
            state: ExecutorState::Idle,
            evolution: EvolutionEngine::new(evolution_config),
            arena: Arena::new(arena_config),
            quantum_backend: Box::new(LocalSimulator::with_seed(
                quantum_config.max_qubits,
                config.simulation_seed,
            )),
            qaoa: QaoaOptimizer::new(crate::quantum::QaoaConfig {
                seed: config.simulation_seed,
                ..Default::default()
            }),
            vqe: VqeOptimizer::new(crate::quantum::VqeConfig {
                seed: config.simulation_seed,
                ..Default::default()
            }),
            qubo: QuboSolver::new().with_seed(config.simulation_seed),
            strategies: HashMap::new(),
            strategy_builder: None,
            stats: ExecutorStats::default(),
            start_time: std::time::Instant::now(),
            config,
        }
    }

    /// Set operating mode
    pub fn set_mode(&mut self, mode: ExecutorMode) {
        self.mode = mode;
    }

    /// Install the evaluator factory used for every active and newly bred genome.
    pub fn set_strategy_builder<F>(&mut self, builder: F)
    where
        F: Fn(&Genome) -> SwarmResult<Box<dyn Strategy>> + Send + Sync + 'static,
    {
        self.strategy_builder = Some(Arc::new(builder));
    }

    /// Builder-style variant of [`Self::set_strategy_builder`].
    pub fn with_strategy_builder<F>(mut self, builder: F) -> Self
    where
        F: Fn(&Genome) -> SwarmResult<Box<dyn Strategy>> + Send + Sync + 'static,
    {
        self.set_strategy_builder(builder);
        self
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
        self.hydrate_strategies()?;

        self.state = ExecutorState::Idle;
        Ok(())
    }

    /// Run one cycle of evolution + tournament
    pub fn step(&mut self, market_data: &MarketData) -> SwarmResult<StepResult> {
        let mut result = StepResult::default();

        // 1. Evaluate strategies on market data
        self.state = ExecutorState::Executing;
        self.hydrate_strategies()?;
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
        self.hydrate_strategies()?;

        // Update stats
        self.stats.best_fitness = self.stats.best_fitness.max(self.evolution.best_fitness());
        self.stats.strategies_evaluated += result.strategies_evaluated as u64;
        self.stats.uptime_seconds = self.start_time.elapsed().as_secs();

        self.state = ExecutorState::Idle;
        Ok(result)
    }

    /// Create evaluators for active genomes and discard evaluators for retired genomes.
    fn hydrate_strategies(&mut self) -> SwarmResult<()> {
        let active_genomes: Vec<Genome> = self
            .arena
            .active_combatants()
            .into_iter()
            .map(|combatant| combatant.genome.clone())
            .collect();
        let active_ids: HashSet<StrategyId> =
            active_genomes.iter().map(|genome| genome.id).collect();

        if let Some(builder) = self.strategy_builder.clone() {
            for genome in &active_genomes {
                if self.strategies.contains_key(&genome.id) {
                    continue;
                }
                let strategy = builder(genome)?;
                if strategy.id() != genome.id {
                    return Err(SwarmError::Strategy(format!(
                        "strategy builder returned {} for genome {}",
                        strategy.id(),
                        genome.id
                    )));
                }
                self.strategies.insert(genome.id, strategy);
            }
        }

        self.strategies
            .retain(|strategy_id, _| active_ids.contains(strategy_id));
        Ok(())
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

    /// Install the evaluator factory before initialization.
    pub async fn set_strategy_builder<F>(&self, builder: F)
    where
        F: Fn(&Genome) -> SwarmResult<Box<dyn Strategy>> + Send + Sync + 'static,
    {
        let mut executor = self.inner.write().await;
        executor.set_strategy_builder(builder);
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

    fn arbitrage_config() -> ArbitrageConfig {
        ArbitrageConfig {
            min_profit_bps: 10,
            max_position: 100_000,
            venues: vec!["Uniswap".into(), "Sushiswap".into()],
            cross_chain: false,
        }
    }

    fn market_data() -> MarketData {
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
        market_data
    }

    fn seeded_executor(seed: u64) -> QuantumSwarmExecutor {
        let config = QuantumSwarmConfig {
            simulation_seed: seed,
            ..Default::default()
        };
        let mut executor = QuantumSwarmExecutor::new(config);
        let arbitrage = arbitrage_config();
        executor.set_strategy_builder(move |genome| {
            Ok(Box::new(ArbitrageStrategy::new(
                genome.clone(),
                arbitrage.clone(),
            )))
        });
        executor
            .initialize(&Genome::new_float(5, 0.0, 1.0))
            .expect("initialize");
        executor
    }

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
        let mut executor = seeded_executor(0);
        let market_data = market_data();

        let first = executor.step(&market_data).unwrap();
        let second = executor
            .step(&market_data)
            .expect("newly bred genomes must be hydrated");

        assert!(first.strategies_evaluated > 0);
        assert!(first.evolution_stats.is_some());
        assert!(first
            .tournament_result
            .as_ref()
            .is_some_and(|tournament| tournament.new_births > 0));
        assert_eq!(second.strategies_evaluated, first.strategies_evaluated);
        assert!(executor.stats.best_fitness > 0.0);
    }

    #[test]
    fn same_seed_reproduces_population_tournament_and_births() {
        let run = || {
            let mut executor = seeded_executor(42);
            let market_data = market_data();
            let first = executor.step(&market_data).expect("first step");
            let second = executor.step(&market_data).expect("second step");

            let mut population: Vec<_> = executor
                .evolution
                .population()
                .iter()
                .map(|genome| (genome.id, format!("{:?}", genome.genes)))
                .collect();
            population.sort_by_key(|(id, _)| *id);
            let mut active: Vec<_> = executor
                .arena
                .active_combatants()
                .into_iter()
                .map(|combatant| combatant.id)
                .collect();
            active.sort();

            let matches = |step: &StepResult| {
                step.tournament_result
                    .as_ref()
                    .expect("tournament")
                    .matches
                    .iter()
                    .map(|result| (result.match_id, result.winner, result.loser))
                    .collect::<Vec<_>>()
            };
            (matches(&first), matches(&second), population, active)
        };

        assert_eq!(run(), run());
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
    fn strategy_builder_must_preserve_the_genome_identity() {
        let mut executor = QuantumSwarmExecutor::new(QuantumSwarmConfig::default());
        executor.set_strategy_builder(|_| {
            let wrong_genome = Genome::new_float(5, 0.0, 1.0);
            Ok(Box::new(ArbitrageStrategy::new(
                wrong_genome,
                ArbitrageConfig {
                    min_profit_bps: 10,
                    max_position: 100_000,
                    venues: vec!["Uniswap".into(), "Sushiswap".into()],
                    cross_chain: false,
                },
            )))
        });
        let error = executor
            .initialize(&Genome::new_float(5, 0.0, 1.0))
            .expect_err("identity mismatch must fail");
        assert!(
            error.to_string().contains("strategy builder returned"),
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
