//! Fitness scoring for strategy evaluation

use crate::chromosome::Chromosome;
use crate::error::{EvolutionError, Result};
use crate::simulator::{MarketTick, SimulationConfig, SimulationResult, Simulator};
use serde::{Deserialize, Serialize};

/// Fitness score with multiple components
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FitnessScore {
    /// Total profit/loss
    pub pnl: f64,
    /// Sharpe ratio
    pub sharpe_ratio: f64,
    /// Maximum drawdown (negative)
    pub max_drawdown: f64,
    /// Win rate (0.0 - 1.0)
    pub win_rate: f64,
    /// Number of trades
    pub trade_count: usize,
    /// Average trade duration
    pub avg_duration: f64,
    /// Profit factor (gross profit / gross loss)
    pub profit_factor: f64,
    /// Sortino ratio
    pub sortino_ratio: f64,
    /// Calmar ratio
    pub calmar_ratio: f64,
    /// Average trade PnL
    pub avg_trade: f64,
    /// Volatility (annualized)
    pub volatility: f64,
    /// Custom score component
    pub custom: f64,
    /// VM gas consumed by the evaluated strategy.
    pub execution_gas: u64,
    /// VM instructions executed by the evaluated strategy.
    pub execution_instructions: u64,
    /// Wall-clock time spent evaluating the strategy, in milliseconds.
    pub execution_latency_ms: f64,
    /// Canonical accounting-state hash produced by a simulation evaluator.
    pub simulation_state_hash: [u8; 32],
}

impl FitnessScore {
    /// Calculate total fitness score (weighted combination)
    pub fn total_score(&self) -> f64 {
        // Weights for different components
        const W_PNL: f64 = 0.25;
        const W_SHARPE: f64 = 0.20;
        const W_DRAWDOWN: f64 = 0.15;
        const W_WIN_RATE: f64 = 0.10;
        const W_PROFIT_FACTOR: f64 = 0.15;
        const W_SORTINO: f64 = 0.10;
        const W_CALMAR: f64 = 0.05;

        // Normalize components to similar scales
        let pnl_norm = self.pnl.tanh(); // Squash to [-1, 1]
        let sharpe_norm = (self.sharpe_ratio / 3.0).tanh(); // 3.0 is considered excellent
        let drawdown_norm = 1.0 + self.max_drawdown.max(-1.0); // Convert to [0, 1]
        let win_rate_norm = self.win_rate;
        let pf_norm = (self.profit_factor / 2.0).min(1.0); // Cap at 2.0
        let sortino_norm = (self.sortino_ratio / 4.0).tanh();
        let calmar_norm = (self.calmar_ratio / 3.0).tanh();

        W_PNL * pnl_norm
            + W_SHARPE * sharpe_norm
            + W_DRAWDOWN * drawdown_norm
            + W_WIN_RATE * win_rate_norm
            + W_PROFIT_FACTOR * pf_norm
            + W_SORTINO * sortino_norm
            + W_CALMAR * calmar_norm
            + self.custom * 0.0 // Custom weight can be added
    }

    /// Create a score representing complete failure
    pub fn failure() -> Self {
        Self {
            pnl: f64::NEG_INFINITY,
            sharpe_ratio: f64::NEG_INFINITY,
            max_drawdown: -1.0,
            win_rate: 0.0,
            trade_count: 0,
            avg_duration: 0.0,
            profit_factor: 0.0,
            sortino_ratio: f64::NEG_INFINITY,
            calmar_ratio: f64::NEG_INFINITY,
            avg_trade: 0.0,
            volatility: 0.0,
            custom: 0.0,
            execution_gas: 0,
            execution_instructions: 0,
            execution_latency_ms: 0.0,
            simulation_state_hash: [0u8; 32],
        }
    }

    /// Check if this is a valid score
    pub fn is_valid(&self) -> bool {
        !self.pnl.is_nan() && !self.pnl.is_infinite()
    }
}

/// Trait for fitness evaluation
pub trait FitnessEvaluator: Send + Sync {
    /// Evaluate fitness of a chromosome
    fn evaluate(&self, chromosome: &Chromosome) -> Result<FitnessScore>;

    /// Get evaluator name
    fn name(&self) -> &'static str;
}

/// PnL-based fitness evaluator using historical simulation
pub struct PnLFitness {
    /// Historical price data for simulation
    price_data: Vec<PricePoint>,
    /// Initial capital
    initial_capital: f64,
    /// Risk-free rate for Sharpe calculation
    risk_free_rate: f64,
    /// Number of simulation runs
    num_simulations: usize,
}

/// A single price point
#[derive(Debug, Clone)]
pub struct PricePoint {
    pub timestamp: u64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl PnLFitness {
    pub fn new(price_data: Vec<PricePoint>, initial_capital: f64) -> Self {
        Self {
            price_data,
            initial_capital,
            risk_free_rate: 0.02, // 2% annual
            num_simulations: 1,
        }
    }

    pub fn with_risk_free_rate(mut self, rate: f64) -> Self {
        self.risk_free_rate = rate;
        self
    }

    pub fn with_simulations(mut self, n: usize) -> Self {
        self.num_simulations = n;
        self
    }

    /// Run the canonical fixed-point strategy simulator over the supplied closes.
    fn simulate(&self, chromosome: &Chromosome) -> Result<SimulationResult> {
        let market_data: Vec<MarketTick> = self
            .price_data
            .iter()
            .map(|point| MarketTick {
                timestamp: point.timestamp,
                symbol: "ASSET".into(),
                price: point.close,
                volume: point.volume,
                bid: point.close,
                ask: point.close,
                data: Default::default(),
            })
            .collect();
        Simulator::new(SimulationConfig {
            initial_capital: self.initial_capital,
            risk_free_rate: self.risk_free_rate,
            ..Default::default()
        })
        .simulate(chromosome, &market_data)
    }
}

impl FitnessEvaluator for PnLFitness {
    fn evaluate(&self, chromosome: &Chromosome) -> Result<FitnessScore> {
        if self.num_simulations == 0 {
            return Err(EvolutionError::InvalidParameters(
                "PnLFitness simulations must be non-zero".into(),
            ));
        }
        let mut score = None;
        for _ in 0..self.num_simulations {
            let result = self.simulate(chromosome)?;
            if let Some(previous) = score.as_ref() {
                let previous: &FitnessScore = previous;
                if previous.simulation_state_hash != result.fitness.simulation_state_hash {
                    return Err(EvolutionError::SimulationFailed(
                        "deterministic PnLFitness repetitions diverged".into(),
                    ));
                }
            }
            score = Some(result.fitness);
        }
        score.ok_or_else(|| {
            EvolutionError::InvalidParameters("PnLFitness simulations must be non-zero".into())
        })
    }

    fn name(&self) -> &'static str {
        "PnLFitness"
    }
}

/// Multi-objective fitness evaluator using Pareto ranking
pub struct ParetoFitness {
    evaluators: Vec<Box<dyn FitnessEvaluator>>,
}

impl ParetoFitness {
    pub fn new() -> Self {
        Self {
            evaluators: Vec::new(),
        }
    }

    pub fn add<E: FitnessEvaluator + 'static>(mut self, evaluator: E) -> Self {
        self.evaluators.push(Box::new(evaluator));
        self
    }
}

impl Default for ParetoFitness {
    fn default() -> Self {
        Self::new()
    }
}

impl FitnessEvaluator for ParetoFitness {
    fn evaluate(&self, chromosome: &Chromosome) -> Result<FitnessScore> {
        let mut combined = FitnessScore::default();

        for evaluator in &self.evaluators {
            let score = evaluator.evaluate(chromosome)?;
            // Simple aggregation - could be enhanced with Pareto ranking
            combined.pnl += score.pnl;
            combined.sharpe_ratio += score.sharpe_ratio;
            combined.max_drawdown = combined.max_drawdown.min(score.max_drawdown);
            combined.win_rate += score.win_rate;
            combined.profit_factor += score.profit_factor;
        }

        let n = self.evaluators.len() as f64;
        if n > 0.0 {
            combined.pnl /= n;
            combined.sharpe_ratio /= n;
            combined.win_rate /= n;
            combined.profit_factor /= n;
        }

        Ok(combined)
    }

    fn name(&self) -> &'static str {
        "ParetoFitness"
    }
}

/// Mock fitness evaluator for testing
pub struct MockFitness {
    base_score: f64,
    variance: f64,
}

impl MockFitness {
    pub fn new(base_score: f64, variance: f64) -> Self {
        Self {
            base_score,
            variance,
        }
    }
}

impl FitnessEvaluator for MockFitness {
    fn evaluate(&self, chromosome: &Chromosome) -> Result<FitnessScore> {
        // Generate deterministic score based on chromosome hash
        let hash = chromosome.hash();
        let hash_value = u64::from_le_bytes([
            hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7],
        ]);

        let normalized = (hash_value as f64) / (u64::MAX as f64);
        let score = self.base_score + (normalized - 0.5) * 2.0 * self.variance;

        Ok(FitnessScore {
            pnl: score,
            sharpe_ratio: score * 2.0,
            max_drawdown: -0.1 * (1.0 - normalized),
            win_rate: 0.5 + normalized * 0.3,
            trade_count: 100,
            avg_duration: 60.0,
            profit_factor: 1.0 + score,
            sortino_ratio: score * 2.5,
            calmar_ratio: score * 3.0,
            avg_trade: score * 0.01,
            volatility: 0.15,
            custom: 0.0,
            execution_gas: 0,
            execution_instructions: 0,
            execution_latency_ms: 0.0,
            simulation_state_hash: [0u8; 32],
        })
    }

    fn name(&self) -> &'static str {
        "MockFitness"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_backend::bc_format::{BytecodeModule, FunctionEntry};
    use x3_backend::opcode::Opcode;

    fn strategy_module(decision: i64) -> Vec<u8> {
        let mut module = BytecodeModule::new();
        let constant = module.const_pool.add_integer(decision).unwrap();
        module.functions.push(FunctionEntry {
            name: crate::simulator::STRATEGY_ENTRYPOINT.into(),
            entry_point: 0,
            param_count: crate::simulator::STRATEGY_PARAM_COUNT as u8,
            local_count: crate::simulator::STRATEGY_PARAM_COUNT,
            max_stack: 16,
            return_type_tag: 1,
        });
        module.code = vec![Opcode::LoadConst.to_byte(), 0];
        module.code.extend_from_slice(&constant.0.to_le_bytes());
        module.code.extend_from_slice(&[Opcode::Ret.to_byte(), 0]);
        module.to_bytes()
    }

    #[test]
    fn test_fitness_score_total() {
        let score = FitnessScore {
            pnl: 0.5,
            sharpe_ratio: 2.0,
            max_drawdown: -0.1,
            win_rate: 0.6,
            trade_count: 100,
            avg_duration: 60.0,
            profit_factor: 1.5,
            sortino_ratio: 2.5,
            calmar_ratio: 3.0,
            avg_trade: 0.05,
            volatility: 0.15,
            custom: 0.0,
            execution_gas: 0,
            execution_instructions: 0,
            execution_latency_ms: 0.0,
            simulation_state_hash: [0u8; 32],
        };

        let total = score.total_score();
        assert!(total > 0.0);
        assert!(total < 1.0);
    }

    #[test]
    fn test_mock_fitness() {
        let bytecode = vec![0x20, 0x64, 0x00, 0x00];
        let chromosome = Chromosome::from_bytecode(bytecode).unwrap();

        let evaluator = MockFitness::new(0.5, 0.2);
        let score = evaluator.evaluate(&chromosome).unwrap();

        assert!(score.is_valid());
        assert!(score.pnl > 0.0);
    }

    #[test]
    fn test_fitness_score_failure() {
        let score = FitnessScore::failure();
        assert!(!score.is_valid());
    }

    #[test]
    fn pnl_fitness_uses_the_canonical_simulator_and_is_reproducible() {
        let chromosome = Chromosome::from_bytecode(strategy_module(1)).unwrap();
        let prices = vec![
            PricePoint {
                timestamp: 1,
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.0,
                volume: 10.0,
            },
            PricePoint {
                timestamp: 2,
                open: 110.0,
                high: 111.0,
                low: 109.0,
                close: 110.0,
                volume: 10.0,
            },
        ];
        let evaluator = PnLFitness::new(prices, 100_000.0).with_simulations(2);
        let score = evaluator.evaluate(&chromosome).expect("simulation");

        assert!(score.is_valid());
        assert_ne!(score.simulation_state_hash, [0u8; 32]);
    }
}
