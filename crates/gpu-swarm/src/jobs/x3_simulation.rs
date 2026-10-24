//! X3 Strategy Simulation Job
//!
//! Executes X3 bytecode strategies against historical or live market data,
//! scoring them based on PnL, Sharpe ratio, drawdown, and other metrics.

use crate::error::{SwarmError, SwarmResult};
use crate::jobs::{JobOutput, JobType, SwarmJob};
use crate::task::TaskPriority;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use x3_evolution::{
    EvolutionConfig, EvolutionEngine, FitnessScore, MarketTick, SimulationConfig, SimulatorFitness,
};

/// Configuration for X3 simulation job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimConfig {
    /// Number of strategies to evaluate
    pub population_size: usize,
    /// Generations for evolution
    pub generations: usize,
    /// Historical data window (seconds)
    pub data_window_secs: u64,
    /// Simulation timestep (ms)
    pub timestep_ms: u64,
    /// Initial capital for each strategy
    pub initial_capital: f64,
    /// Trading fee rate
    pub fee_rate: f64,
    /// Slippage rate
    pub slippage_rate: f64,
    /// Maximum position size
    pub max_position_pct: f64,
    /// Mutation rate for evolution
    pub mutation_rate: f64,
    /// Crossover rate for evolution
    pub crossover_rate: f64,
    /// Elite preservation ratio
    pub elite_ratio: f64,
    /// Target assets to trade
    pub assets: Vec<String>,
    /// Random seed for reproducibility
    pub seed: Option<u64>,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            population_size: 100,
            generations: 50,
            data_window_secs: 86400, // 24 hours
            timestep_ms: 1000,       // 1 second
            initial_capital: 100_000.0,
            fee_rate: 0.001,
            slippage_rate: 0.0005,
            max_position_pct: 0.25,
            mutation_rate: 0.1,
            crossover_rate: 0.7,
            elite_ratio: 0.1,
            assets: vec!["ETH/USDC".to_string(), "BTC/USDC".to_string()],
            seed: None,
        }
    }
}

/// A strategy candidate for evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyCandidate {
    /// Strategy ID (hash of bytecode)
    pub id: [u8; 32],
    /// X3 bytecode
    pub bytecode: Vec<u8>,
    /// Generation created
    pub generation: usize,
    /// Parent strategy IDs (for lineage)
    pub parents: Vec<[u8; 32]>,
    /// Fitness score
    pub fitness: Option<FitnessMetrics>,
    /// Canonical portfolio state hash from the strategy simulation.
    pub state_hash: [u8; 32],
}

/// Fitness metrics for strategy evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessMetrics {
    /// Total PnL percentage
    pub pnl_pct: f64,
    /// Sharpe ratio (risk-adjusted return)
    pub sharpe_ratio: f64,
    /// Sortino ratio (downside risk-adjusted)
    pub sortino_ratio: f64,
    /// Maximum drawdown percentage
    pub max_drawdown_pct: f64,
    /// Win rate (profitable trades / total trades)
    pub win_rate: f64,
    /// Profit factor (gross profit / gross loss)
    pub profit_factor: f64,
    /// Calmar ratio (return / max drawdown)
    pub calmar_ratio: f64,
    /// Total number of trades
    pub trade_count: usize,
    /// Average trade duration (seconds)
    pub avg_trade_duration: f64,
    /// Gas cost estimate
    pub gas_estimate: u64,
    /// Execution latency (ms)
    pub latency_ms: f64,
    /// Composite score (weighted combination)
    pub total_score: f64,
}

impl FitnessMetrics {
    /// Calculate composite score from individual metrics
    pub fn calculate_total(&mut self) {
        const W_PNL: f64 = 0.25;
        const W_SHARPE: f64 = 0.20;
        const W_DRAWDOWN: f64 = 0.15;
        const W_WIN_RATE: f64 = 0.10;
        const W_PROFIT_FACTOR: f64 = 0.15;
        const W_SORTINO: f64 = 0.10;
        const W_LATENCY: f64 = 0.05;

        let pnl_norm = self.pnl_pct.tanh();
        let sharpe_norm = (self.sharpe_ratio / 3.0).tanh();
        let drawdown_norm = 1.0 + self.max_drawdown_pct.max(-1.0);
        let pf_norm = (self.profit_factor / 2.0).min(1.0);
        let sortino_norm = (self.sortino_ratio / 4.0).tanh();
        let latency_norm = 1.0 - (self.latency_ms / 1000.0).min(1.0);

        self.total_score = W_PNL * pnl_norm
            + W_SHARPE * sharpe_norm
            + W_DRAWDOWN * drawdown_norm
            + W_WIN_RATE * self.win_rate
            + W_PROFIT_FACTOR * pf_norm
            + W_SORTINO * sortino_norm
            + W_LATENCY * latency_norm;
    }
}

/// Result from X3 simulation job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    /// Top performing strategies (sorted by fitness)
    pub top_strategies: Vec<StrategyCandidate>,
    /// Generation statistics
    pub generation_stats: Vec<GenerationStats>,
    /// Best fitness achieved
    pub best_fitness: f64,
    /// Total strategies evaluated
    pub total_evaluated: usize,
    /// Execution duration (ms)
    pub duration_ms: u64,
    /// Compute units consumed
    pub compute_units: u64,
    /// Result hash for verification
    pub result_hash: [u8; 32],
}

/// Statistics for a single generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationStats {
    pub generation: usize,
    pub best_fitness: f64,
    pub avg_fitness: f64,
    pub worst_fitness: f64,
    pub diversity: f64,
}

/// X3 Strategy Simulation Job
pub struct X3SimulationJob {
    /// Job configuration
    pub config: SimConfig,
    /// Seed strategies (bytecode)
    pub seed_strategies: Vec<Vec<u8>>,
    /// Historical market data
    pub market_data: Vec<MarketDataPoint>,
}

/// Market data point for simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketDataPoint {
    pub timestamp: u64,
    pub asset: String,
    pub price: f64,
    pub volume: f64,
    pub bid: f64,
    pub ask: f64,
}

impl X3SimulationJob {
    pub fn new(config: SimConfig) -> Self {
        Self {
            config,
            seed_strategies: Vec::new(),
            market_data: Vec::new(),
        }
    }

    /// Add seed strategies
    pub fn with_seeds(mut self, seeds: Vec<Vec<u8>>) -> Self {
        self.seed_strategies = seeds;
        self
    }

    /// Add market data
    pub fn with_market_data(mut self, data: Vec<MarketDataPoint>) -> Self {
        self.market_data = data;
        self
    }

    /// Execute simulation using x3-evolution
    fn run_simulation(&self) -> SwarmResult<SimulationResult> {
        if self.seed_strategies.is_empty() {
            return Err(SwarmError::InvalidPayload(
                "simulation requires at least one canonical strategy seed".into(),
            ));
        }
        if self.market_data.is_empty() {
            return Err(SwarmError::InvalidPayload(
                "simulation requires market data".into(),
            ));
        }
        if self.config.population_size < 2 || self.config.generations == 0 {
            return Err(SwarmError::InvalidPayload(
                "simulation population must be at least 2 and generations must be non-zero".into(),
            ));
        }
        let seed = self.config.seed.ok_or_else(|| {
            SwarmError::InvalidPayload(
                "distributed simulation requires an explicit deterministic seed".into(),
            )
        })?;

        let start = std::time::Instant::now();
        let market_data: Vec<MarketTick> = self
            .market_data
            .iter()
            .map(|point| {
                let mut data = HashMap::new();
                data.insert("volume".into(), point.volume);
                MarketTick {
                    timestamp: point.timestamp,
                    symbol: point.asset.clone(),
                    price: point.price,
                    volume: point.volume,
                    bid: point.bid,
                    ask: point.ask,
                    data,
                }
            })
            .collect();

        let evolution_config = EvolutionConfig {
            population_size: self.config.population_size,
            generations: self.config.generations,
            mutation_rate: self.config.mutation_rate,
            crossover_rate: self.config.crossover_rate,
            elite_ratio: self.config.elite_ratio,
            tournament_size: 3.min(self.config.population_size),
            seed: Some(seed),
            simulations_per_strategy: market_data.len(),
            max_bytecode_size: 1024 * 1024,
            parallel: false,
        };
        let simulator_config = SimulationConfig {
            initial_capital: self.config.initial_capital,
            fee_rate: self.config.fee_rate,
            slippage_rate: self.config.slippage_rate,
            max_position_size: self.config.max_position_pct,
            risk_free_rate: 0.02,
            timeout: Duration::from_secs(60),
            gas_limit_per_tick: 100_000,
            realistic_execution: false,
        };

        let mut engine = EvolutionEngine::new(evolution_config);
        engine
            .seed_population(self.seed_strategies.clone())
            .map_err(|error| {
                SwarmError::InvalidPayload(format!("invalid strategy seed: {error}"))
            })?;
        let evaluator = SimulatorFitness::new(simulator_config, market_data);
        let evolution = engine.evolve(&evaluator).map_err(|error| {
            SwarmError::ExecutionError(format!("evolution simulation failed: {error}"))
        })?;

        let top_strategies = engine
            .ranked(5)
            .into_iter()
            .filter_map(|individual| {
                let fitness = individual.fitness.as_ref()?;
                Some(StrategyCandidate {
                    id: individual.chromosome.hash(),
                    bytecode: individual.chromosome.to_bytecode(),
                    generation: individual.birth_generation,
                    parents: individual.parent_ids,
                    fitness: Some(FitnessMetrics::from_score(
                        fitness,
                        self.config.initial_capital,
                    )),
                    state_hash: fitness.simulation_state_hash,
                })
            })
            .collect();
        let generation_stats = evolution
            .generation_stats
            .iter()
            .map(|stats| GenerationStats {
                generation: stats.generation,
                best_fitness: stats.best_fitness,
                avg_fitness: stats.average_fitness,
                worst_fitness: stats.worst_fitness,
                diversity: stats.fitness_variance.sqrt(),
            })
            .collect();
        let total_evaluated = self
            .config
            .population_size
            .saturating_mul(evolution.generations_run);

        let mut result = SimulationResult {
            top_strategies,
            generation_stats,
            best_fitness: evolution.best_fitness,
            total_evaluated,
            duration_ms: start.elapsed().as_millis() as u64,
            compute_units: evolution.total_execution_gas,
            result_hash: [0u8; 32],
        };
        result.result_hash = simulation_result_hash(&result);
        Ok(result)
    }
}

impl FitnessMetrics {
    fn from_score(score: &FitnessScore, initial_capital: f64) -> Self {
        let mut metrics = Self {
            pnl_pct: if initial_capital > 0.0 {
                score.pnl / initial_capital * 100.0
            } else {
                0.0
            },
            sharpe_ratio: score.sharpe_ratio,
            sortino_ratio: score.sortino_ratio,
            max_drawdown_pct: -score.max_drawdown.abs(),
            win_rate: score.win_rate,
            profit_factor: score.profit_factor,
            calmar_ratio: score.calmar_ratio,
            trade_count: score.trade_count,
            avg_trade_duration: score.avg_duration,
            gas_estimate: score.execution_gas,
            latency_ms: score.execution_latency_ms,
            total_score: 0.0,
        };
        metrics.calculate_total();
        metrics
    }
}

fn simulation_result_hash(result: &SimulationResult) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"x3-swarm-simulation-result-v1");
    hasher.update(&(result.top_strategies.len() as u64).to_le_bytes());
    for strategy in &result.top_strategies {
        hasher.update(&strategy.id);
        hasher.update(&(strategy.bytecode.len() as u64).to_le_bytes());
        hasher.update(&strategy.bytecode);
        hasher.update(&(strategy.generation as u64).to_le_bytes());
        for parent in &strategy.parents {
            hasher.update(parent);
        }
        hasher.update(&strategy.state_hash);
        if let Some(fitness) = &strategy.fitness {
            for value in [
                fitness.pnl_pct,
                fitness.sharpe_ratio,
                fitness.sortino_ratio,
                fitness.max_drawdown_pct,
                fitness.win_rate,
                fitness.profit_factor,
                fitness.calmar_ratio,
                fitness.avg_trade_duration,
                fitness.total_score,
            ] {
                hasher.update(&value.to_bits().to_le_bytes());
            }
            hasher.update(&(fitness.trade_count as u64).to_le_bytes());
            hasher.update(&fitness.gas_estimate.to_le_bytes());
        }
    }
    hasher.update(&(result.generation_stats.len() as u64).to_le_bytes());
    for stats in &result.generation_stats {
        hasher.update(&(stats.generation as u64).to_le_bytes());
        for value in [
            stats.best_fitness,
            stats.avg_fitness,
            stats.worst_fitness,
            stats.diversity,
        ] {
            hasher.update(&value.to_bits().to_le_bytes());
        }
    }
    hasher.update(&result.best_fitness.to_bits().to_le_bytes());
    hasher.update(&(result.total_evaluated as u64).to_le_bytes());
    hasher.finalize().into()
}

impl SwarmJob for X3SimulationJob {
    fn job_type(&self) -> JobType {
        JobType::X3Simulation
    }

    fn compute_units(&self) -> u64 {
        // Estimate: 100 CU per strategy per generation
        (self.config.population_size * self.config.generations * 100) as u64
    }

    fn timeout(&self) -> Duration {
        // 1 second per generation, minimum 60 seconds
        Duration::from_secs((self.config.generations as u64).max(60))
    }

    fn execute(&self) -> SwarmResult<JobOutput> {
        let result = self.run_simulation()?;
        Ok(JobOutput::X3Simulation(result))
    }

    fn verify(&self, result: &JobOutput) -> SwarmResult<bool> {
        match result {
            JobOutput::X3Simulation(sim_result) => {
                Ok(simulation_result_hash(sim_result) == sim_result.result_hash)
            }
            _ => Err(SwarmError::InvalidResult("Wrong result type".into())),
        }
    }

    fn priority(&self) -> TaskPriority {
        TaskPriority::High
    }

    fn requires_gpu(&self) -> bool {
        false
    }

    fn min_vram_mb(&self) -> u32 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_backend::bc_format::{BytecodeModule, FunctionEntry};
    use x3_backend::opcode::Opcode;
    use x3_evolution::{STRATEGY_ENTRYPOINT, STRATEGY_PARAM_COUNT};

    fn strategy_module(decision: i64) -> Vec<u8> {
        let mut module = BytecodeModule::new();
        let constant = module.const_pool.add_integer(decision).expect("constant");
        module.functions.push(FunctionEntry {
            name: STRATEGY_ENTRYPOINT.to_string(),
            entry_point: 0,
            param_count: STRATEGY_PARAM_COUNT as u8,
            local_count: STRATEGY_PARAM_COUNT,
            max_stack: 16,
            return_type_tag: 1,
        });
        module.code = vec![Opcode::LoadConst.to_byte(), 0];
        module.code.extend_from_slice(&constant.0.to_le_bytes());
        module.code.extend_from_slice(&[Opcode::Ret.to_byte(), 0]);
        module.to_bytes()
    }

    fn market_data() -> Vec<MarketDataPoint> {
        (0..4)
            .map(|index| MarketDataPoint {
                timestamp: 1_000 + index * 1_000,
                asset: "ETH/USDC".into(),
                price: 2_000.0 + index as f64 * 10.0,
                volume: 100.0,
                bid: 1_999.0 + index as f64 * 10.0,
                ask: 2_001.0 + index as f64 * 10.0,
            })
            .collect()
    }

    #[test]
    fn test_sim_config_default() {
        let config = SimConfig::default();
        assert_eq!(config.population_size, 100);
        assert_eq!(config.generations, 50);
    }

    #[test]
    fn test_fitness_calculation() {
        let mut fitness = FitnessMetrics {
            pnl_pct: 10.0,
            sharpe_ratio: 2.0,
            sortino_ratio: 2.5,
            max_drawdown_pct: -5.0,
            win_rate: 0.6,
            profit_factor: 1.5,
            calmar_ratio: 2.0,
            trade_count: 100,
            avg_trade_duration: 3600.0,
            gas_estimate: 50000,
            latency_ms: 15.0,
            total_score: 0.0,
        };

        fitness.calculate_total();
        assert!(fitness.total_score > 0.0);
        assert!(fitness.total_score < 1.0);
    }

    #[test]
    fn simulation_requires_seed_strategies_and_market_data() {
        let config = SimConfig {
            population_size: 10,
            generations: 5,
            ..Default::default()
        };

        let job = X3SimulationJob::new(config);
        let error = job.execute().expect_err("missing inputs must fail closed");
        assert!(
            error.to_string().contains("canonical strategy seed"),
            "{error}"
        );
    }

    #[test]
    fn distributed_simulation_requires_a_deterministic_seed() {
        let config = SimConfig {
            population_size: 2,
            generations: 1,
            seed: None,
            ..Default::default()
        };
        let job = X3SimulationJob::new(config)
            .with_seeds(vec![strategy_module(1)])
            .with_market_data(market_data());
        let error = job
            .execute()
            .expect_err("unseeded simulation must fail closed");
        assert!(error.to_string().contains("deterministic seed"), "{error}");
    }

    #[test]
    fn simulation_runs_evolution_through_the_x3_vm() {
        let config = SimConfig {
            population_size: 4,
            generations: 1,
            seed: Some(7),
            ..Default::default()
        };
        let job = X3SimulationJob::new(config)
            .with_seeds(vec![strategy_module(1)])
            .with_market_data(market_data());

        let mut output = job.execute().expect("simulation");
        let JobOutput::X3Simulation(result) = &output else {
            panic!("wrong job output");
        };
        assert_eq!(result.generation_stats.len(), 1);
        assert!(!result.top_strategies.is_empty());
        assert_ne!(result.top_strategies[0].state_hash, [0u8; 32]);
        assert!(result.compute_units > 0);
        assert!(!job.requires_gpu());
        assert!(job.verify(&output).expect("verification"));

        let JobOutput::X3Simulation(result) = &mut output else {
            unreachable!();
        };
        result.top_strategies[0].state_hash[0] ^= 0xff;
        assert!(!job.verify(&output).expect("hash recomputation"));
    }

    #[test]
    fn simulation_rejects_an_invalid_seed_module() {
        let config = SimConfig {
            population_size: 2,
            generations: 1,
            seed: Some(7),
            ..Default::default()
        };
        let job = X3SimulationJob::new(config)
            .with_seeds(vec![vec![0x20, 0x64, 0, 0]])
            .with_market_data(market_data());

        assert!(job.execute().is_err());
    }
}
