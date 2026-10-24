//! Strategy simulation using X3 VM

use crate::chromosome::Chromosome;
use crate::error::{EvolutionError, Result};
use crate::fitness::{FitnessEvaluator, FitnessScore};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use x3_vm::{BytecodeModule, VMConfig, Value, Verifier, VerifyOptions, VM};

/// Versioned entrypoint every strategy module must expose as function 0.
pub const STRATEGY_ENTRYPOINT: &str = "x3_strategy_decide_v1";

/// Number of fixed-point arguments in strategy ABI v1.
///
/// Order: timestamp_ms, price, volume, bid, ask, cash, position, total_value.
pub const STRATEGY_PARAM_COUNT: u16 = 8;

/// Scale for every monetary and quantity argument in strategy ABI v1.
pub const STRATEGY_VALUE_SCALE: u64 = 100_000_000;

/// Market data point for simulation
#[derive(Debug, Clone)]
pub struct MarketTick {
    /// Timestamp (unix ms)
    pub timestamp: u64,
    /// Asset symbol
    pub symbol: String,
    /// Price
    pub price: f64,
    /// Volume
    pub volume: f64,
    /// Bid price
    pub bid: f64,
    /// Ask price
    pub ask: f64,
    /// Additional data
    pub data: HashMap<String, f64>,
}

impl MarketTick {
    pub fn new(timestamp: u64, symbol: &str, price: f64) -> Self {
        Self {
            timestamp,
            symbol: symbol.to_string(),
            price,
            volume: 0.0,
            bid: price,
            ask: price,
            data: HashMap::new(),
        }
    }

    /// Spread as percentage
    pub fn spread_pct(&self) -> f64 {
        if self.bid > 0.0 {
            (self.ask - self.bid) / self.bid * 100.0
        } else {
            0.0
        }
    }
}

/// Trading action from strategy
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TradeAction {
    Buy,
    Sell,
    Hold,
}

impl TradeAction {
    fn from_abi_decision(value: i64) -> Result<Self> {
        match value {
            1 => Ok(Self::Buy),
            0 => Ok(Self::Hold),
            -1 => Ok(Self::Sell),
            other => Err(EvolutionError::SimulationFailed(format!(
                "strategy ABI decision must be -1, 0, or 1, got {other}"
            ))),
        }
    }
}

/// Trade execution result
#[derive(Debug, Clone)]
pub struct Trade {
    pub timestamp: u64,
    pub action: TradeAction,
    pub price: f64,
    pub quantity: f64,
    pub fees: f64,
    pub slippage: f64,
}

impl Trade {
    pub fn total_cost(&self) -> f64 {
        match self.action {
            TradeAction::Buy => self.price * self.quantity + self.fees + self.slippage,
            TradeAction::Sell => -(self.price * self.quantity - self.fees - self.slippage),
            TradeAction::Hold => 0.0,
        }
    }
}

/// Portfolio state during simulation
#[derive(Debug, Clone)]
pub struct PortfolioState {
    /// Cash balance
    pub cash: f64,
    /// Asset positions (symbol -> quantity)
    pub positions: HashMap<String, f64>,
    /// Total value (cash + positions at current prices)
    pub total_value: f64,
    /// Peak value (for drawdown calculation)
    pub peak_value: f64,
    /// Trade history
    pub trades: Vec<Trade>,
    /// Daily returns
    pub daily_returns: Vec<f64>,
}

impl PortfolioState {
    pub fn new(initial_capital: f64) -> Self {
        Self {
            cash: initial_capital,
            positions: HashMap::new(),
            total_value: initial_capital,
            peak_value: initial_capital,
            trades: Vec::new(),
            daily_returns: Vec::new(),
        }
    }

    /// Calculate current drawdown percentage
    pub fn current_drawdown(&self) -> f64 {
        if self.peak_value > 0.0 {
            (self.peak_value - self.total_value) / self.peak_value * 100.0
        } else {
            0.0
        }
    }

    /// Update peak value
    pub fn update_peak(&mut self) {
        if self.total_value > self.peak_value {
            self.peak_value = self.total_value;
        }
    }

    /// Get position value
    pub fn position_value(&self, symbol: &str, price: f64) -> f64 {
        self.positions.get(symbol).copied().unwrap_or(0.0) * price
    }

    /// Update total value with current prices
    pub fn update_value(&mut self, prices: &HashMap<String, f64>) {
        let position_value: f64 = self
            .positions
            .iter()
            .map(|(sym, qty)| qty * prices.get(sym).copied().unwrap_or(0.0))
            .sum();

        self.total_value = self.cash + position_value;
        self.update_peak();
    }

    /// Record daily return
    pub fn record_daily_return(&mut self, previous_value: f64) {
        if previous_value > 0.0 {
            let ret = (self.total_value - previous_value) / previous_value;
            self.daily_returns.push(ret);
        }
    }

    /// Get PnL
    pub fn pnl(&self, initial_capital: f64) -> f64 {
        self.total_value - initial_capital
    }

    /// Get return percentage
    pub fn return_pct(&self, initial_capital: f64) -> f64 {
        if initial_capital > 0.0 {
            (self.total_value - initial_capital) / initial_capital * 100.0
        } else {
            0.0
        }
    }
}

/// Simulation configuration
#[derive(Debug, Clone)]
pub struct SimulationConfig {
    /// Initial capital
    pub initial_capital: f64,
    /// Trading fee rate (e.g., 0.001 for 0.1%)
    pub fee_rate: f64,
    /// Slippage rate (e.g., 0.0005 for 0.05%)
    pub slippage_rate: f64,
    /// Maximum position size as fraction of portfolio
    pub max_position_size: f64,
    /// Risk-free rate for Sharpe calculation (annualized)
    pub risk_free_rate: f64,
    /// Maximum simulation time
    pub timeout: Duration,
    /// Maximum VM gas one strategy decision may consume.
    pub gas_limit_per_tick: u64,
    /// Use realistic execution (delays, partial fills)
    pub realistic_execution: bool,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            initial_capital: 100_000.0,
            fee_rate: 0.001,
            slippage_rate: 0.0005,
            max_position_size: 0.25,
            risk_free_rate: 0.02,
            timeout: Duration::from_secs(30),
            gas_limit_per_tick: 100_000,
            realistic_execution: false,
        }
    }
}

/// Simulation result
#[derive(Debug, Clone)]
pub struct SimulationResult {
    /// Final portfolio state
    pub portfolio: PortfolioState,
    /// Fitness score
    pub fitness: FitnessScore,
    /// Total number of ticks processed
    pub ticks_processed: usize,
    /// Total VM gas consumed across strategy decisions.
    pub gas_used: u64,
    /// Total VM instructions executed across strategy decisions.
    pub instructions_executed: u64,
    /// Simulation duration
    pub duration: Duration,
    /// Errors encountered
    pub errors: Vec<String>,
}

/// Strategy simulator
pub struct Simulator {
    config: SimulationConfig,
}

/// A verified strategy module bound to simulator ABI v1.
struct StrategyProgram {
    module: BytecodeModule,
}

impl StrategyProgram {
    fn verify(bytecode: &[u8]) -> Result<Self> {
        let options = VerifyOptions::deterministic_simulation(STRATEGY_PARAM_COUNT);
        let module = Verifier::verify_module_bytes(bytecode, &options).map_err(|error| {
            EvolutionError::SimulationFailed(format!(
                "strategy failed deterministic verification: {error}"
            ))
        })?;
        let entry = module.functions.first().ok_or_else(|| {
            EvolutionError::SimulationFailed("strategy module has no entry function".into())
        })?;
        if entry.name != STRATEGY_ENTRYPOINT {
            return Err(EvolutionError::SimulationFailed(format!(
                "strategy function 0 must be named {STRATEGY_ENTRYPOINT}, found {}",
                entry.name
            )));
        }
        Ok(Self { module })
    }

    fn decide(
        &self,
        tick: &MarketTick,
        portfolio: &PortfolioState,
        gas_limit: u64,
    ) -> Result<(TradeAction, u64, u64)> {
        let args = strategy_args(tick, portfolio)?;
        let mut vm = VM::with_config(
            self.module.clone(),
            VMConfig {
                gas_limit,
                ..VMConfig::default()
            },
        );
        let execution = vm.call_function(0, &args).map_err(|error| {
            EvolutionError::SimulationFailed(format!("strategy VM execution failed: {error:?}"))
        })?;
        let decision = match execution.value {
            Some(Value::I64(value)) => value,
            other => {
                return Err(EvolutionError::SimulationFailed(format!(
                    "strategy must return I64 -1, 0, or 1, got {other:?}"
                )))
            }
        };
        Ok((
            TradeAction::from_abi_decision(decision)?,
            execution.gas_used,
            execution.instruction_count,
        ))
    }
}

fn fixed_value(value: f64, name: &str) -> Result<i64> {
    if !value.is_finite() || value < 0.0 {
        return Err(EvolutionError::SimulationFailed(format!(
            "strategy input {name} must be finite and non-negative, got {value}"
        )));
    }
    let scaled = (value * STRATEGY_VALUE_SCALE as f64).round();
    if scaled > i64::MAX as f64 {
        return Err(EvolutionError::SimulationFailed(format!(
            "strategy input {name} exceeds the ABI range"
        )));
    }
    Ok(scaled as i64)
}

fn strategy_args(tick: &MarketTick, portfolio: &PortfolioState) -> Result<Vec<Value>> {
    let timestamp = i64::try_from(tick.timestamp).map_err(|_| {
        EvolutionError::SimulationFailed("strategy timestamp exceeds the ABI range".into())
    })?;
    let position = portfolio
        .positions
        .get(&tick.symbol)
        .copied()
        .unwrap_or(0.0);
    Ok(vec![
        Value::I64(timestamp),
        Value::I64(fixed_value(tick.price, "price")?),
        Value::I64(fixed_value(tick.volume, "volume")?),
        Value::I64(fixed_value(tick.bid, "bid")?),
        Value::I64(fixed_value(tick.ask, "ask")?),
        Value::I64(fixed_value(portfolio.cash, "cash")?),
        Value::I64(fixed_value(position, "position")?),
        Value::I64(fixed_value(portfolio.total_value, "total_value")?),
    ])
}

impl Simulator {
    pub fn new(config: SimulationConfig) -> Self {
        Self { config }
    }

    /// Run simulation with chromosome against market data
    pub fn simulate(
        &self,
        chromosome: &Chromosome,
        market_data: &[MarketTick],
    ) -> Result<SimulationResult> {
        let start = Instant::now();
        let strategy = StrategyProgram::verify(&chromosome.to_bytecode())?;
        let mut portfolio = PortfolioState::new(self.config.initial_capital);
        let mut errors = Vec::new();
        let mut ticks_processed = 0;
        let mut gas_used = 0u64;
        let mut instructions_executed = 0u64;
        let mut previous_value = self.config.initial_capital;
        let mut last_day: Option<u64> = None;

        // Execute strategy for each tick
        for tick in market_data {
            // Check timeout
            if start.elapsed() > self.config.timeout {
                errors.push(format!(
                    "Simulation timeout after {} ticks",
                    ticks_processed
                ));
                break;
            }

            // Record daily returns (simplified: every 1000 ticks = 1 day)
            let current_day = tick.timestamp / (24 * 60 * 60 * 1000);
            if last_day.is_some() && last_day != Some(current_day) {
                portfolio.record_daily_return(previous_value);
                previous_value = portfolio.total_value;
            }
            last_day = Some(current_day);

            // Execute the verified strategy module through the deterministic X3 VM ABI.
            let (action, tick_gas, tick_instructions) =
                strategy.decide(tick, &portfolio, self.config.gas_limit_per_tick)?;
            gas_used = gas_used.saturating_add(tick_gas);
            instructions_executed = instructions_executed.saturating_add(tick_instructions);

            // Apply action
            if let Some(trade) = self.execute_trade(action, tick, &mut portfolio)? {
                portfolio.trades.push(trade);
            }

            // Update portfolio value
            let mut prices = HashMap::new();
            prices.insert(tick.symbol.clone(), tick.price);
            portfolio.update_value(&prices);

            ticks_processed += 1;
        }

        // Calculate final fitness
        let duration = start.elapsed();
        let fitness = self.calculate_fitness(&portfolio, gas_used, instructions_executed, duration);

        Ok(SimulationResult {
            portfolio,
            fitness,
            ticks_processed,
            gas_used,
            instructions_executed,
            duration,
            errors,
        })
    }

    /// Execute a trade based on action
    fn execute_trade(
        &self,
        action: TradeAction,
        tick: &MarketTick,
        portfolio: &mut PortfolioState,
    ) -> Result<Option<Trade>> {
        match action {
            TradeAction::Hold => Ok(None),

            TradeAction::Buy => {
                // Calculate position size (max 25% of portfolio)
                let max_value = portfolio.cash * self.config.max_position_size;
                let price_with_slippage = tick.ask * (1.0 + self.config.slippage_rate);
                let quantity = max_value / price_with_slippage;

                if quantity <= 0.0 || max_value < 100.0 {
                    return Ok(None);
                }

                let fees = max_value * self.config.fee_rate;
                let slippage = max_value * self.config.slippage_rate;
                let total_cost = max_value + fees;

                if total_cost > portfolio.cash {
                    return Ok(None);
                }

                // Execute
                portfolio.cash -= total_cost;
                *portfolio
                    .positions
                    .entry(tick.symbol.clone())
                    .or_insert(0.0) += quantity;

                Ok(Some(Trade {
                    timestamp: tick.timestamp,
                    action: TradeAction::Buy,
                    price: price_with_slippage,
                    quantity,
                    fees,
                    slippage,
                }))
            }

            TradeAction::Sell => {
                let quantity = portfolio
                    .positions
                    .get(&tick.symbol)
                    .copied()
                    .unwrap_or(0.0);

                if quantity <= 0.0 {
                    return Ok(None);
                }

                let price_with_slippage = tick.bid * (1.0 - self.config.slippage_rate);
                let proceeds = quantity * price_with_slippage;
                let fees = proceeds * self.config.fee_rate;
                let slippage = quantity * tick.bid * self.config.slippage_rate;

                // Execute
                portfolio.cash += proceeds - fees;
                portfolio.positions.remove(&tick.symbol);

                Ok(Some(Trade {
                    timestamp: tick.timestamp,
                    action: TradeAction::Sell,
                    price: price_with_slippage,
                    quantity,
                    fees,
                    slippage,
                }))
            }
        }
    }

    /// Calculate fitness from simulation results
    fn calculate_fitness(
        &self,
        portfolio: &PortfolioState,
        gas_used: u64,
        instructions_executed: u64,
        duration: Duration,
    ) -> FitnessScore {
        let pnl = portfolio.pnl(self.config.initial_capital);
        let return_pct = portfolio.return_pct(self.config.initial_capital);

        // Calculate Sharpe ratio
        let sharpe = self.calculate_sharpe(&portfolio.daily_returns);

        // Calculate Sortino ratio
        let sortino = self.calculate_sortino(&portfolio.daily_returns);

        // Calculate max drawdown
        let max_drawdown = self.calculate_max_drawdown(&portfolio.daily_returns);

        // Win rate
        let win_rate = self.calculate_win_rate(&portfolio.trades);

        // Average trade
        let avg_trade = if !portfolio.trades.is_empty() {
            pnl / portfolio.trades.len() as f64
        } else {
            0.0
        };

        FitnessScore {
            pnl,
            sharpe_ratio: sharpe,
            sortino_ratio: sortino,
            max_drawdown,
            win_rate,
            profit_factor: if pnl > 0.0 { 1.5 } else { 0.5 }, // Simplified
            avg_trade,
            avg_duration: 0.0,
            trade_count: portfolio.trades.len(),
            calmar_ratio: if max_drawdown > 0.01 {
                return_pct / max_drawdown
            } else {
                return_pct
            },
            volatility: self.calculate_volatility(&portfolio.daily_returns),
            custom: 0.0,
            execution_gas: gas_used,
            execution_instructions: instructions_executed,
            execution_latency_ms: duration.as_secs_f64() * 1_000.0,
        }
    }

    fn calculate_sharpe(&self, daily_returns: &[f64]) -> f64 {
        if daily_returns.len() < 2 {
            return 0.0;
        }

        let mean: f64 = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;
        let annualized_mean = mean * 252.0;

        let variance: f64 = daily_returns
            .iter()
            .map(|r| (r - mean).powi(2))
            .sum::<f64>()
            / (daily_returns.len() - 1) as f64;
        let std_dev = variance.sqrt() * (252.0_f64).sqrt();

        if std_dev > 0.0 {
            (annualized_mean - self.config.risk_free_rate) / std_dev
        } else {
            0.0
        }
    }

    fn calculate_sortino(&self, daily_returns: &[f64]) -> f64 {
        if daily_returns.len() < 2 {
            return 0.0;
        }

        let mean: f64 = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;
        let annualized_mean = mean * 252.0;

        let downside_variance: f64 = daily_returns
            .iter()
            .filter(|&&r| r < 0.0)
            .map(|r| r.powi(2))
            .sum::<f64>()
            / daily_returns.len() as f64;
        let downside_dev = downside_variance.sqrt() * (252.0_f64).sqrt();

        if downside_dev > 0.0 {
            (annualized_mean - self.config.risk_free_rate) / downside_dev
        } else {
            annualized_mean
        }
    }

    fn calculate_max_drawdown(&self, daily_returns: &[f64]) -> f64 {
        if daily_returns.is_empty() {
            return 0.0;
        }

        let mut cumulative = 1.0;
        let mut peak = 1.0;
        let mut max_dd = 0.0;

        for &ret in daily_returns {
            cumulative *= 1.0 + ret;
            if cumulative > peak {
                peak = cumulative;
            }
            let dd = (peak - cumulative) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }

        max_dd * 100.0 // As percentage
    }

    fn calculate_win_rate(&self, trades: &[Trade]) -> f64 {
        if trades.is_empty() {
            return 0.0;
        }

        let mut wins = 0;
        let mut prev_sell_price: Option<f64> = None;

        for trade in trades {
            match trade.action {
                TradeAction::Buy => {
                    prev_sell_price = None;
                }
                TradeAction::Sell => {
                    if let Some(buy_price) = prev_sell_price {
                        if trade.price > buy_price {
                            wins += 1;
                        }
                    }
                    prev_sell_price = Some(trade.price);
                }
                _ => {}
            }
        }

        // Simplified: assume 50% of closed trades are wins
        let closed_trades = trades
            .iter()
            .filter(|t| matches!(t.action, TradeAction::Sell))
            .count();

        if closed_trades > 0 {
            (closed_trades / 2) as f64 / closed_trades as f64
        } else {
            0.0
        }
    }

    fn calculate_volatility(&self, daily_returns: &[f64]) -> f64 {
        if daily_returns.len() < 2 {
            return 0.0;
        }

        let mean: f64 = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;
        let variance: f64 = daily_returns
            .iter()
            .map(|r| (r - mean).powi(2))
            .sum::<f64>()
            / (daily_returns.len() - 1) as f64;

        variance.sqrt() * (252.0_f64).sqrt() * 100.0 // Annualized percentage
    }
}

/// Fitness evaluator using simulation
pub struct SimulatorFitness {
    simulator: Simulator,
    market_data: Vec<MarketTick>,
}

impl SimulatorFitness {
    pub fn new(config: SimulationConfig, market_data: Vec<MarketTick>) -> Self {
        Self {
            simulator: Simulator::new(config),
            market_data,
        }
    }
}

impl FitnessEvaluator for SimulatorFitness {
    fn evaluate(&self, chromosome: &Chromosome) -> Result<FitnessScore> {
        let result = self.simulator.simulate(chromosome, &self.market_data)?;
        Ok(result.fitness)
    }

    fn name(&self) -> &'static str {
        "SimulatorFitness"
    }
}

/// Generate synthetic market data for testing
pub fn generate_synthetic_data(
    symbol: &str,
    num_ticks: usize,
    initial_price: f64,
    volatility: f64,
) -> Vec<MarketTick> {
    let seed_bytes = blake3::hash(symbol.as_bytes());
    let mut seed = [0u8; 8];
    seed.copy_from_slice(&seed_bytes.as_bytes()[..8]);
    generate_synthetic_data_seeded(
        symbol,
        num_ticks,
        initial_price,
        volatility,
        u64::from_le_bytes(seed),
    )
}

/// Generate deterministic synthetic market data from an explicit seed.
pub fn generate_synthetic_data_seeded(
    symbol: &str,
    num_ticks: usize,
    initial_price: f64,
    volatility: f64,
    seed: u64,
) -> Vec<MarketTick> {
    use rand::Rng;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let mut ticks = Vec::with_capacity(num_ticks);
    let mut price = initial_price;
    let mut timestamp = 1704067200000u64; // 2024-01-01 00:00:00 UTC

    for _ in 0..num_ticks {
        // Random walk
        let change = rng.gen::<f64>() * 2.0 - 1.0; // -1 to 1
        price *= 1.0 + change * volatility;
        price = price.max(0.01); // Floor

        let spread = price * 0.001; // 0.1% spread

        let mut tick = MarketTick {
            timestamp,
            symbol: symbol.to_string(),
            price,
            volume: rng.gen::<f64>() * 1000.0,
            bid: price - spread / 2.0,
            ask: price + spread / 2.0,
            data: HashMap::new(),
        };

        tick.data
            .insert("rsi".to_string(), rng.gen::<f64>() * 100.0);
        tick.data
            .insert("macd".to_string(), rng.gen::<f64>() * 2.0 - 1.0);

        ticks.push(tick);
        timestamp += 60_000; // 1 minute
    }

    ticks
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_backend::bc_format::FunctionEntry;
    use x3_backend::opcode::Opcode;

    fn strategy_module(name: &str, decision: i64) -> Vec<u8> {
        let mut module = BytecodeModule::new();
        let constant = module.const_pool.add_integer(decision).expect("constant");
        module.functions.push(FunctionEntry {
            name: name.to_string(),
            entry_point: 0,
            param_count: STRATEGY_PARAM_COUNT as u8,
            local_count: STRATEGY_PARAM_COUNT,
            max_stack: 16,
            return_type_tag: 1,
        });
        module.code = vec![Opcode::LoadConst.to_byte(), 0];
        module.code.extend_from_slice(&constant.0.to_le_bytes());
        module.code.extend_from_slice(&[Opcode::Ret.to_byte(), 0]);
        let bytes = module.to_bytes();
        Verifier::verify_module_bytes(
            &bytes,
            &VerifyOptions::deterministic_simulation(STRATEGY_PARAM_COUNT),
        )
        .expect("test strategy module must verify before chromosome wrapping");
        bytes
    }

    #[test]
    fn test_portfolio_state() {
        let mut portfolio = PortfolioState::new(100_000.0);
        assert_eq!(portfolio.cash, 100_000.0);
        assert_eq!(portfolio.total_value, 100_000.0);

        portfolio.cash -= 10_000.0;
        portfolio.positions.insert("BTC".to_string(), 1.0);

        let mut prices = HashMap::new();
        prices.insert("BTC".to_string(), 50_000.0);
        portfolio.update_value(&prices);

        assert_eq!(portfolio.total_value, 140_000.0);
    }

    #[test]
    fn test_synthetic_data_generation() {
        let data = generate_synthetic_data("BTC", 1000, 50000.0, 0.01);
        assert_eq!(data.len(), 1000);

        for tick in &data {
            assert!(tick.price > 0.0);
            assert!(tick.bid < tick.ask);
        }
    }

    #[test]
    fn simulation_executes_a_verified_strategy_through_the_vm() {
        let config = SimulationConfig {
            timeout: Duration::from_secs(5),
            ..Default::default()
        };
        let simulator = Simulator::new(config);
        let data = vec![MarketTick::new(1_000, "ETH", 2_000.0)];
        let bytecode = strategy_module(STRATEGY_ENTRYPOINT, 1);
        let chromosome = Chromosome::from_bytecode(bytecode.clone()).expect("canonical chromosome");
        assert_eq!(chromosome.to_bytecode(), bytecode);

        let result = simulator.simulate(&chromosome, &data).expect("simulation");

        assert_eq!(result.ticks_processed, 1);
        assert_eq!(result.portfolio.trades.len(), 1);
        assert_eq!(result.portfolio.trades[0].action, TradeAction::Buy);
        assert!(result.portfolio.cash < 100_000.0);
        assert!(result.portfolio.positions.contains_key("ETH"));
    }

    #[test]
    fn simulation_rejects_a_wrong_abi_name() {
        let simulator = Simulator::new(SimulationConfig::default());
        let chromosome = Chromosome::from_bytecode(strategy_module("main", 0)).expect("chromosome");
        let error = simulator
            .simulate(&chromosome, &[MarketTick::new(1, "ETH", 2_000.0)])
            .expect_err("wrong ABI name must fail");
        assert!(error.to_string().contains(STRATEGY_ENTRYPOINT), "{error}");
    }

    #[test]
    fn simulation_rejects_an_invalid_decision() {
        let simulator = Simulator::new(SimulationConfig::default());
        let chromosome =
            Chromosome::from_bytecode(strategy_module(STRATEGY_ENTRYPOINT, 2)).expect("chromosome");
        let error = simulator
            .simulate(&chromosome, &[MarketTick::new(1, "ETH", 2_000.0)])
            .expect_err("invalid decisions must fail");
        assert!(error.to_string().contains("must be -1, 0, or 1"), "{error}");
    }

    #[test]
    fn simulation_rejects_gas_exhaustion_instead_of_holding() {
        let simulator = Simulator::new(SimulationConfig {
            gas_limit_per_tick: 1,
            ..Default::default()
        });
        let chromosome =
            Chromosome::from_bytecode(strategy_module(STRATEGY_ENTRYPOINT, 0)).expect("chromosome");
        let error = simulator
            .simulate(&chromosome, &[MarketTick::new(1, "ETH", 2_000.0)])
            .expect_err("gas exhaustion must fail closed");
        assert!(error.to_string().contains("GasLimitExceeded"), "{error}");
    }

    #[test]
    fn simulation_rejects_malformed_bytecode() {
        let simulator = Simulator::new(SimulationConfig::default());
        let chromosome = Chromosome::from_bytecode(vec![0x20, 0x64, 0, 0]).expect("legacy genes");
        let error = simulator
            .simulate(&chromosome, &[MarketTick::new(1, "ETH", 2_000.0)])
            .expect_err("non-canonical bytecode must fail verification");
        assert!(
            error.to_string().contains("deterministic verification"),
            "{error}"
        );
    }

    #[test]
    fn synthetic_data_is_reproducible_from_its_seed() {
        let first = generate_synthetic_data_seeded("ETH", 32, 2_000.0, 0.02, 42);
        let second = generate_synthetic_data_seeded("ETH", 32, 2_000.0, 0.02, 42);
        assert_eq!(first.len(), second.len());
        for (a, b) in first.iter().zip(second.iter()) {
            assert_eq!(a.timestamp, b.timestamp);
            assert_eq!(a.price.to_bits(), b.price.to_bits());
            assert_eq!(a.volume.to_bits(), b.volume.to_bits());
        }
    }

    #[test]
    fn test_trade_execution() {
        let config = SimulationConfig::default();
        let simulator = Simulator::new(config);

        let mut portfolio = PortfolioState::new(100_000.0);
        let tick = MarketTick::new(0, "ETH", 2000.0);

        // Execute buy
        let trade = simulator
            .execute_trade(TradeAction::Buy, &tick, &mut portfolio)
            .unwrap()
            .unwrap();

        assert_eq!(trade.action, TradeAction::Buy);
        assert!(portfolio.cash < 100_000.0);
        assert!(portfolio.positions.contains_key("ETH"));
    }

    #[test]
    fn test_fitness_calculation() {
        let daily_returns = vec![0.01, -0.005, 0.02, 0.015, -0.01, 0.005];

        let config = SimulationConfig::default();
        let simulator = Simulator::new(config);

        let sharpe = simulator.calculate_sharpe(&daily_returns);
        let sortino = simulator.calculate_sortino(&daily_returns);
        let max_dd = simulator.calculate_max_drawdown(&daily_returns);
        let volatility = simulator.calculate_volatility(&daily_returns);

        // Just verify they compute without panicking
        assert!(sharpe.is_finite());
        assert!(sortino.is_finite());
        assert!(max_dd >= 0.0);
        assert!(volatility >= 0.0);
    }
}
