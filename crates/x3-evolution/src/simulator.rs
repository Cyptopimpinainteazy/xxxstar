//! Strategy simulation using X3 VM

use crate::chromosome::Chromosome;
use crate::error::{EvolutionError, Result};
use crate::fitness::{FitnessEvaluator, FitnessScore};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use x3_vm::{BytecodeModule, VMConfig, Value, Verifier, VerifyOptions, VM};
use x3bc_core::codec::{Reader, Writer};

/// Versioned entrypoint every strategy module must expose as function 0.
pub const STRATEGY_ENTRYPOINT: &str = "x3_strategy_decide_v1";

/// Number of fixed-point arguments in strategy ABI v1.
///
/// Order: timestamp_ms, price, volume, bid, ask, cash, position, total_value.
pub const STRATEGY_PARAM_COUNT: u16 = 8;

/// Scale for every monetary and quantity argument in strategy ABI v1.
pub const STRATEGY_VALUE_SCALE: u64 = 100_000_000;

/// Canonical strategy-simulation receipt format.
pub const SIMULATION_RECEIPT_VERSION: u16 = 1;

/// Domain separator for simulation receipt digests.
pub const SIMULATION_RECEIPT_DOMAIN: &[u8] = b"x3-strategy-simulation-receipt-v1";

type Fixed = i64;

const FIXED_SCALE: Fixed = STRATEGY_VALUE_SCALE as Fixed;

fn checked_ratio(value: Fixed, ratio: Fixed) -> Result<Fixed> {
    let scaled = (value as i128).checked_mul(ratio as i128).ok_or_else(|| {
        EvolutionError::SimulationFailed("fixed-point multiplication overflow".into())
    })? / FIXED_SCALE as i128;
    Fixed::try_from(scaled)
        .map_err(|_| EvolutionError::SimulationFailed("fixed-point result out of range".into()))
}

fn checked_div(value: Fixed, divisor: Fixed) -> Result<Fixed> {
    if divisor <= 0 {
        return Err(EvolutionError::SimulationFailed(
            "fixed-point divisor must be positive".into(),
        ));
    }
    let scaled = (value as i128)
        .checked_mul(FIXED_SCALE as i128)
        .ok_or_else(|| EvolutionError::SimulationFailed("fixed-point division overflow".into()))?
        / divisor as i128;
    Fixed::try_from(scaled)
        .map_err(|_| EvolutionError::SimulationFailed("fixed-point result out of range".into()))
}

fn checked_mul(left: Fixed, right: Fixed) -> Result<Fixed> {
    checked_ratio(left, right)
}

fn checked_add(left: Fixed, right: Fixed) -> Result<Fixed> {
    left.checked_add(right)
        .ok_or_else(|| EvolutionError::SimulationFailed("fixed-point addition overflow".into()))
}

fn checked_sub(left: Fixed, right: Fixed) -> Result<Fixed> {
    left.checked_sub(right)
        .ok_or_else(|| EvolutionError::SimulationFailed("fixed-point subtraction overflow".into()))
}

/// Convert a finite decimal value into the simulator's canonical 1e-8 fixed-point unit.
pub fn decimal_to_fixed(value: f64, name: &str) -> Result<Fixed> {
    if !value.is_finite() {
        return Err(EvolutionError::SimulationFailed(format!(
            "{name} must be finite, got {value}"
        )));
    }
    let scaled = (value * FIXED_SCALE as f64).round();
    if scaled < Fixed::MIN as f64 || scaled > Fixed::MAX as f64 {
        return Err(EvolutionError::SimulationFailed(format!(
            "{name} exceeds the fixed-point range"
        )));
    }
    Ok(scaled as Fixed)
}

/// Convert canonical fixed-point units back to decimal for reporting only.
pub fn fixed_to_decimal(value: Fixed) -> f64 {
    value as f64 / FIXED_SCALE as f64
}

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
    pub price: Fixed,
    pub quantity: Fixed,
    pub fees: Fixed,
    pub slippage: Fixed,
    pub realized_pnl: Fixed,
}

impl Trade {
    pub fn total_cost(&self) -> Result<Fixed> {
        match self.action {
            TradeAction::Buy => checked_add(checked_mul(self.price, self.quantity)?, self.fees),
            TradeAction::Sell => {
                let proceeds = checked_sub(checked_mul(self.price, self.quantity)?, self.fees)?;
                Ok(-proceeds)
            }
            TradeAction::Hold => Ok(0),
        }
    }
}

/// Portfolio state during simulation
#[derive(Debug, Clone)]
pub struct PortfolioState {
    /// Cash balance in canonical 1e-8 fixed-point units.
    pub cash: Fixed,
    /// Asset positions in canonical 1e-8 fixed-point units.
    pub positions: HashMap<String, Fixed>,
    /// Cost basis for each open position in canonical 1e-8 fixed-point units.
    pub cost_basis: HashMap<String, Fixed>,
    /// Latest mark price for each held asset in canonical 1e-8 fixed-point units.
    pub last_prices: HashMap<String, Fixed>,
    /// Total value in canonical 1e-8 fixed-point units.
    pub total_value: Fixed,
    /// Peak value in canonical 1e-8 fixed-point units.
    pub peak_value: Fixed,
    /// Trade history
    pub trades: Vec<Trade>,
    /// Realized PnL from closed positions in canonical 1e-8 fixed-point units.
    pub realized_pnl: Fixed,
    /// Daily returns as canonical 1e-8 fixed-point fractions.
    pub daily_returns: Vec<Fixed>,
    /// Mark-to-market equity after each tick in canonical 1e-8 fixed-point units.
    pub equity_curve: Vec<Fixed>,
}

impl PortfolioState {
    pub fn new(initial_capital: Fixed) -> Self {
        Self {
            cash: initial_capital,
            positions: HashMap::new(),
            cost_basis: HashMap::new(),
            last_prices: HashMap::new(),
            total_value: initial_capital,
            peak_value: initial_capital,
            trades: Vec::new(),
            realized_pnl: 0,
            daily_returns: Vec::new(),
            equity_curve: vec![initial_capital],
        }
    }

    /// Calculate current drawdown percentage
    pub fn current_drawdown(&self) -> f64 {
        if self.peak_value > 0 {
            (self.peak_value.saturating_sub(self.total_value) as f64 / self.peak_value as f64)
                * 100.0
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
    pub fn position_value(&self, symbol: &str) -> Result<Fixed> {
        let quantity = self.positions.get(symbol).copied().unwrap_or(0);
        let price = self.last_prices.get(symbol).copied().ok_or_else(|| {
            EvolutionError::SimulationFailed(format!("position {symbol} has no mark price"))
        })?;
        checked_mul(quantity, price)
    }

    /// Update the mark for one asset.
    pub fn mark_price(&mut self, symbol: &str, price: Fixed) -> Result<()> {
        if price <= 0 {
            return Err(EvolutionError::SimulationFailed(format!(
                "mark price for {symbol} must be positive"
            )));
        }
        self.last_prices.insert(symbol.to_string(), price);
        Ok(())
    }

    /// Mark the complete portfolio to the latest prices.
    pub fn update_value(&mut self) -> Result<()> {
        let mut position_value = 0;
        for symbol in self.positions.keys() {
            position_value = checked_add(position_value, self.position_value(symbol)?)?;
        }
        self.total_value = checked_add(self.cash, position_value)?;
        self.update_peak();
        Ok(())
    }

    /// Record daily return
    pub fn record_daily_return(&mut self, previous_value: Fixed) -> Result<()> {
        if previous_value > 0 {
            let ret = checked_div(
                checked_sub(self.total_value, previous_value)?,
                previous_value,
            )?;
            self.daily_returns.push(ret);
        }
        Ok(())
    }

    /// Get PnL
    pub fn pnl(&self, initial_capital: Fixed) -> Result<Fixed> {
        checked_sub(self.total_value, initial_capital)
    }

    /// Get return percentage
    pub fn return_pct(&self, initial_capital: Fixed) -> Result<f64> {
        if initial_capital > 0 {
            Ok(self.pnl(initial_capital)? as f64 / initial_capital as f64 * 100.0)
        } else {
            Err(EvolutionError::SimulationFailed(
                "initial capital must be positive".into(),
            ))
        }
    }

    /// Canonical hash of accounting state, independent of hash-map iteration order.
    pub fn state_hash(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"x3-portfolio-state-v1");
        hasher.update(&self.cash.to_le_bytes());
        hasher.update(&self.total_value.to_le_bytes());
        hasher.update(&self.peak_value.to_le_bytes());
        hasher.update(&self.realized_pnl.to_le_bytes());

        let mut positions: Vec<_> = self.positions.iter().collect();
        positions.sort_by(|(left, _), (right, _)| left.cmp(right));
        hasher.update(&(positions.len() as u64).to_le_bytes());
        for (symbol, quantity) in positions {
            hasher.update(&(symbol.len() as u64).to_le_bytes());
            hasher.update(symbol.as_bytes());
            hasher.update(&quantity.to_le_bytes());
        }

        let mut marks: Vec<_> = self.last_prices.iter().collect();
        marks.sort_by(|(left, _), (right, _)| left.cmp(right));
        hasher.update(&(marks.len() as u64).to_le_bytes());
        for (symbol, price) in marks {
            hasher.update(&(symbol.len() as u64).to_le_bytes());
            hasher.update(symbol.as_bytes());
            hasher.update(&price.to_le_bytes());
        }

        let mut costs: Vec<_> = self.cost_basis.iter().collect();
        costs.sort_by(|(left, _), (right, _)| left.cmp(right));
        hasher.update(&(costs.len() as u64).to_le_bytes());
        for (symbol, cost) in costs {
            hasher.update(&(symbol.len() as u64).to_le_bytes());
            hasher.update(symbol.as_bytes());
            hasher.update(&cost.to_le_bytes());
        }
        hasher.update(&(self.trades.len() as u64).to_le_bytes());
        for trade in &self.trades {
            hasher.update(&trade.timestamp.to_le_bytes());
            hasher.update(&[match trade.action {
                TradeAction::Buy => 1,
                TradeAction::Sell => 2,
                TradeAction::Hold => 0,
            }]);
            hasher.update(&trade.price.to_le_bytes());
            hasher.update(&trade.quantity.to_le_bytes());
            hasher.update(&trade.fees.to_le_bytes());
            hasher.update(&trade.slippage.to_le_bytes());
            hasher.update(&trade.realized_pnl.to_le_bytes());
        }
        hasher.update(&(self.daily_returns.len() as u64).to_le_bytes());
        for value in &self.daily_returns {
            hasher.update(&value.to_le_bytes());
        }
        hasher.update(&(self.equity_curve.len() as u64).to_le_bytes());
        for value in &self.equity_curve {
            hasher.update(&value.to_le_bytes());
        }
        hasher.finalize().into()
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
    /// Canonical hash of the final portfolio accounting state.
    pub state_hash: [u8; 32],
    /// Persistable evidence binding inputs, work, and final state.
    pub receipt: SimulationReceipt,
    /// Simulation duration
    pub duration: Duration,
    /// Errors encountered
    pub errors: Vec<String>,
}

impl SimulationResult {
    /// Verify that the receipt agrees with the result fields it accompanies.
    pub fn receipt_matches_result(&self) -> bool {
        self.receipt.is_well_formed()
            && self.receipt.state_hash == self.state_hash
            && self.receipt.ticks_processed == self.ticks_processed as u64
            && self.receipt.gas_used == self.gas_used
            && self.receipt.instructions_executed == self.instructions_executed
            && self.receipt.final_cash == self.portfolio.cash
            && self.receipt.final_total_value == self.portfolio.total_value
            && self.receipt.final_realized_pnl == self.portfolio.realized_pnl
            && self.receipt.trade_count == self.portfolio.trades.len() as u64
            && self.receipt.errors == self.errors
    }
}

/// Canonical evidence binding simulation inputs, VM work, and final fixed-point state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationReceipt {
    pub version: u16,
    pub strategy_hash: [u8; 32],
    pub market_hash: [u8; 32],
    pub config_hash: [u8; 32],
    pub state_hash: [u8; 32],
    pub ticks_processed: u64,
    pub gas_used: u64,
    pub instructions_executed: u64,
    pub final_cash: Fixed,
    pub final_total_value: Fixed,
    pub final_realized_pnl: Fixed,
    pub trade_count: u64,
    pub errors: Vec<String>,
}

impl SimulationReceipt {
    /// Canonical bytes for persistence and signing.
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = Writer::new();
        writer.u16(self.version);
        writer.array32(&self.strategy_hash);
        writer.array32(&self.market_hash);
        writer.array32(&self.config_hash);
        writer.array32(&self.state_hash);
        writer.u64(self.ticks_processed);
        writer.u64(self.gas_used);
        writer.u64(self.instructions_executed);
        writer.i128(self.final_cash as i128);
        writer.i128(self.final_total_value as i128);
        writer.i128(self.final_realized_pnl as i128);
        writer.u64(self.trade_count);
        writer.seq(&self.errors, |writer, error| writer.str(error));
        writer.finish()
    }

    /// Decode and validate canonical receipt bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut reader = Reader::new(bytes);
        let receipt = Self {
            version: reader.u16()?,
            strategy_hash: reader.array32()?,
            market_hash: reader.array32()?,
            config_hash: reader.array32()?,
            state_hash: reader.array32()?,
            ticks_processed: reader.u64()?,
            gas_used: reader.u64()?,
            instructions_executed: reader.u64()?,
            final_cash: Fixed::try_from(reader.i128()?).map_err(|_| {
                EvolutionError::SerializationError("receipt final cash out of range".into())
            })?,
            final_total_value: Fixed::try_from(reader.i128()?).map_err(|_| {
                EvolutionError::SerializationError("receipt final value out of range".into())
            })?,
            final_realized_pnl: Fixed::try_from(reader.i128()?).map_err(|_| {
                EvolutionError::SerializationError("receipt realized PnL out of range".into())
            })?,
            trade_count: reader.u64()?,
            errors: reader.seq(|reader| reader.string())?,
        };
        reader.finish()?;
        if receipt.version != SIMULATION_RECEIPT_VERSION {
            return Err(EvolutionError::SerializationError(format!(
                "unsupported simulation receipt version {}",
                receipt.version
            )));
        }
        Ok(receipt)
    }

    /// Digest the canonical receipt bytes under the simulation domain.
    pub fn digest(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(SIMULATION_RECEIPT_DOMAIN);
        hasher.update(&self.encode());
        hasher.finalize().into()
    }

    /// Cheap structural checks before a receiver chooses to replay.
    pub fn is_well_formed(&self) -> bool {
        self.version == SIMULATION_RECEIPT_VERSION
            && self.strategy_hash != [0u8; 32]
            && self.market_hash != [0u8; 32]
            && self.config_hash != [0u8; 32]
            && self.state_hash != [0u8; 32]
            && self.trade_count as usize <= self.ticks_processed as usize
    }
}

fn simulation_config_hash(config: &SimulationConfig, fixed: &FixedSimulationConfig) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"x3-strategy-simulation-config-v1");
    hasher.update(STRATEGY_ENTRYPOINT.as_bytes());
    hasher.update(&STRATEGY_PARAM_COUNT.to_le_bytes());
    hasher.update(&STRATEGY_VALUE_SCALE.to_le_bytes());
    hasher.update(&fixed.initial_capital.to_le_bytes());
    hasher.update(&fixed.fee_rate.to_le_bytes());
    hasher.update(&fixed.slippage_rate.to_le_bytes());
    hasher.update(&fixed.max_position_size.to_le_bytes());
    hasher.update(&config.risk_free_rate.to_bits().to_le_bytes());
    hasher.update(&config.timeout.as_nanos().to_le_bytes());
    hasher.update(&config.gas_limit_per_tick.to_le_bytes());
    hasher.update(&[u8::from(config.realistic_execution)]);
    hasher.finalize().into()
}

fn simulation_market_hash(market: &[PreparedTick]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"x3-strategy-simulation-market-v1");
    hasher.update(&(market.len() as u64).to_le_bytes());
    for tick in market {
        hasher.update(&tick.timestamp.to_le_bytes());
        hasher.update(&(tick.symbol.len() as u64).to_le_bytes());
        hasher.update(tick.symbol.as_bytes());
        hasher.update(&tick.price.to_le_bytes());
        hasher.update(&tick.volume.to_le_bytes());
        hasher.update(&tick.bid.to_le_bytes());
        hasher.update(&tick.ask.to_le_bytes());
    }
    hasher.finalize().into()
}

fn hex_digest(bytes: &[u8]) -> String {
    use std::fmt::Write;

    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

/// Strategy simulator
pub struct Simulator {
    config: SimulationConfig,
}

struct FixedSimulationConfig {
    initial_capital: Fixed,
    fee_rate: Fixed,
    slippage_rate: Fixed,
    max_position_size: Fixed,
}

struct PreparedTick {
    timestamp: u64,
    symbol: String,
    price: Fixed,
    volume: Fixed,
    bid: Fixed,
    ask: Fixed,
}

fn prepare_tick(tick: &MarketTick) -> Result<PreparedTick> {
    let price = decimal_to_fixed(tick.price, "price")?;
    let bid = decimal_to_fixed(tick.bid, "bid")?;
    let ask = decimal_to_fixed(tick.ask, "ask")?;
    if price <= 0 || bid <= 0 || ask <= 0 || bid > ask {
        return Err(EvolutionError::SimulationFailed(format!(
            "invalid market tick for {}: price={price}, bid={bid}, ask={ask}",
            tick.symbol
        )));
    }
    Ok(PreparedTick {
        timestamp: tick.timestamp,
        symbol: tick.symbol.clone(),
        price,
        volume: decimal_to_fixed(tick.volume, "volume")?,
        bid,
        ask,
    })
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
        tick: &PreparedTick,
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

fn strategy_args(tick: &PreparedTick, portfolio: &PortfolioState) -> Result<Vec<Value>> {
    let timestamp = i64::try_from(tick.timestamp).map_err(|_| {
        EvolutionError::SimulationFailed("strategy timestamp exceeds the ABI range".into())
    })?;
    let position = portfolio.positions.get(&tick.symbol).copied().unwrap_or(0);
    Ok(vec![
        Value::I64(timestamp),
        Value::I64(tick.price),
        Value::I64(tick.volume),
        Value::I64(tick.bid),
        Value::I64(tick.ask),
        Value::I64(portfolio.cash),
        Value::I64(position),
        Value::I64(portfolio.total_value),
    ])
}

impl Simulator {
    pub fn new(config: SimulationConfig) -> Self {
        Self { config }
    }

    fn fixed_config(&self) -> Result<FixedSimulationConfig> {
        if self.config.initial_capital <= 0.0 {
            return Err(EvolutionError::SimulationFailed(
                "initial capital must be positive".into(),
            ));
        }
        for (name, rate) in [
            ("fee_rate", self.config.fee_rate),
            ("slippage_rate", self.config.slippage_rate),
            ("max_position_size", self.config.max_position_size),
        ] {
            if !rate.is_finite() || !(0.0..=1.0).contains(&rate) {
                return Err(EvolutionError::SimulationFailed(format!(
                    "{name} must be finite and between 0 and 1"
                )));
            }
        }
        Ok(FixedSimulationConfig {
            initial_capital: decimal_to_fixed(self.config.initial_capital, "initial_capital")?,
            fee_rate: decimal_to_fixed(self.config.fee_rate, "fee_rate")?,
            slippage_rate: decimal_to_fixed(self.config.slippage_rate, "slippage_rate")?,
            max_position_size: decimal_to_fixed(
                self.config.max_position_size,
                "max_position_size",
            )?,
        })
    }

    /// Run simulation with chromosome against market data
    pub fn simulate(
        &self,
        chromosome: &Chromosome,
        market_data: &[MarketTick],
    ) -> Result<SimulationResult> {
        let start = Instant::now();
        let config = self.fixed_config()?;
        let strategy_bytecode = chromosome.to_bytecode();
        let strategy_hash = blake3::hash(&strategy_bytecode).into();
        let strategy = StrategyProgram::verify(&strategy_bytecode)?;
        let prepared: Vec<PreparedTick> = market_data
            .iter()
            .map(prepare_tick)
            .collect::<Result<_>>()?;
        let market_hash = simulation_market_hash(&prepared);
        let config_hash = simulation_config_hash(&self.config, &config);
        let mut portfolio = PortfolioState::new(config.initial_capital);
        let mut errors = Vec::new();
        let mut ticks_processed = 0;
        let mut gas_used = 0u64;
        let mut instructions_executed = 0u64;
        let mut previous_value = config.initial_capital;
        let mut last_day: Option<u64> = None;

        // Execute strategy for each tick
        for tick in &prepared {
            // Check timeout
            if start.elapsed() > self.config.timeout {
                errors.push(format!(
                    "Simulation timeout after {} ticks",
                    ticks_processed
                ));
                break;
            }

            portfolio.mark_price(&tick.symbol, tick.price)?;
            portfolio.update_value()?;

            let current_day = tick.timestamp / (24 * 60 * 60 * 1000);
            if last_day.is_some() && last_day != Some(current_day) {
                portfolio.record_daily_return(previous_value)?;
                previous_value = portfolio.total_value;
            }
            last_day = Some(current_day);

            // Execute the verified strategy module through the deterministic X3 VM ABI.
            let (action, tick_gas, tick_instructions) =
                strategy.decide(tick, &portfolio, self.config.gas_limit_per_tick)?;
            gas_used = gas_used.saturating_add(tick_gas);
            instructions_executed = instructions_executed.saturating_add(tick_instructions);

            // Apply action
            if let Some(trade) = self.execute_trade(action, tick, &config, &mut portfolio)? {
                portfolio.trades.push(trade);
            }

            portfolio.update_value()?;
            portfolio.equity_curve.push(portfolio.total_value);

            ticks_processed += 1;
        }

        // Calculate final fitness
        let duration = start.elapsed();
        let state_hash = portfolio.state_hash();
        let mut fitness = self.calculate_fitness(
            &portfolio,
            config.initial_capital,
            gas_used,
            instructions_executed,
            duration,
        )?;
        fitness.simulation_state_hash = state_hash;
        let receipt = SimulationReceipt {
            version: SIMULATION_RECEIPT_VERSION,
            strategy_hash,
            market_hash,
            config_hash,
            state_hash,
            ticks_processed: ticks_processed as u64,
            gas_used,
            instructions_executed,
            final_cash: portfolio.cash,
            final_total_value: portfolio.total_value,
            final_realized_pnl: portfolio.realized_pnl,
            trade_count: portfolio.trades.len() as u64,
            errors: errors.clone(),
        };
        fitness.simulation_receipt_hash = receipt.digest();

        Ok(SimulationResult {
            portfolio,
            fitness,
            ticks_processed,
            gas_used,
            instructions_executed,
            state_hash,
            receipt,
            duration,
            errors,
        })
    }

    /// Re-run a simulation and require exact receipt agreement.
    pub fn replay(
        &self,
        chromosome: &Chromosome,
        market_data: &[MarketTick],
        expected: &SimulationReceipt,
    ) -> Result<SimulationResult> {
        let result = self.simulate(chromosome, market_data)?;
        if &result.receipt != expected {
            return Err(EvolutionError::SimulationFailed(format!(
                "simulation receipt mismatch: expected {}, replay produced {}",
                hex_digest(&expected.digest()),
                hex_digest(&result.receipt.digest())
            )));
        }
        Ok(result)
    }

    /// Execute a trade based on action
    fn execute_trade(
        &self,
        action: TradeAction,
        tick: &PreparedTick,
        config: &FixedSimulationConfig,
        portfolio: &mut PortfolioState,
    ) -> Result<Option<Trade>> {
        match action {
            TradeAction::Hold => Ok(None),

            TradeAction::Buy => {
                let max_value = checked_ratio(portfolio.cash, config.max_position_size)?;
                let price_multiplier = checked_add(FIXED_SCALE, config.slippage_rate)?;
                let price_with_slippage = checked_ratio(tick.ask, price_multiplier)?;
                let quantity = checked_div(max_value, price_with_slippage)?;

                if quantity <= 0 || max_value < 100 * FIXED_SCALE {
                    return Ok(None);
                }

                let notional = checked_mul(quantity, price_with_slippage)?;
                let fees = checked_ratio(notional, config.fee_rate)?;
                let total_cost = checked_add(notional, fees)?;
                let unslipped_notional = checked_mul(quantity, tick.ask)?;
                let slippage = checked_sub(max_value, unslipped_notional)?;

                if total_cost > portfolio.cash {
                    return Ok(None);
                }

                portfolio.cash = checked_sub(portfolio.cash, total_cost)?;
                let previous_quantity = portfolio.positions.get(&tick.symbol).copied().unwrap_or(0);
                portfolio.positions.insert(
                    tick.symbol.clone(),
                    checked_add(previous_quantity, quantity)?,
                );
                let previous_basis = portfolio.cost_basis.get(&tick.symbol).copied().unwrap_or(0);
                portfolio.cost_basis.insert(
                    tick.symbol.clone(),
                    checked_add(previous_basis, total_cost)?,
                );

                Ok(Some(Trade {
                    timestamp: tick.timestamp,
                    action: TradeAction::Buy,
                    price: price_with_slippage,
                    quantity,
                    fees,
                    slippage,
                    realized_pnl: 0,
                }))
            }

            TradeAction::Sell => {
                let quantity = portfolio.positions.get(&tick.symbol).copied().unwrap_or(0);

                if quantity <= 0 {
                    return Ok(None);
                }

                let price_multiplier = checked_sub(FIXED_SCALE, config.slippage_rate)?;
                let price_with_slippage = checked_ratio(tick.bid, price_multiplier)?;
                let gross_proceeds = checked_mul(quantity, price_with_slippage)?;
                let fees = checked_ratio(gross_proceeds, config.fee_rate)?;
                let net_proceeds = checked_sub(gross_proceeds, fees)?;
                let basis = portfolio
                    .cost_basis
                    .get(&tick.symbol)
                    .copied()
                    .ok_or_else(|| {
                        EvolutionError::SimulationFailed(format!(
                            "position {} has no cost basis",
                            tick.symbol
                        ))
                    })?;
                let realized_pnl = checked_sub(net_proceeds, basis)?;
                let unslipped_proceeds = checked_mul(quantity, tick.bid)?;
                let slippage = checked_sub(unslipped_proceeds, gross_proceeds)?;

                portfolio.cash = checked_add(portfolio.cash, net_proceeds)?;
                portfolio.realized_pnl = checked_add(portfolio.realized_pnl, realized_pnl)?;
                portfolio.positions.remove(&tick.symbol);
                portfolio.cost_basis.remove(&tick.symbol);

                Ok(Some(Trade {
                    timestamp: tick.timestamp,
                    action: TradeAction::Sell,
                    price: price_with_slippage,
                    quantity,
                    fees,
                    slippage,
                    realized_pnl,
                }))
            }
        }
    }

    /// Calculate fitness from simulation results
    fn calculate_fitness(
        &self,
        portfolio: &PortfolioState,
        initial_capital: Fixed,
        gas_used: u64,
        instructions_executed: u64,
        duration: Duration,
    ) -> Result<FitnessScore> {
        let pnl_fixed = portfolio.pnl(initial_capital)?;
        let pnl = fixed_to_decimal(pnl_fixed);
        let return_pct = portfolio.return_pct(initial_capital)?;

        // Calculate Sharpe ratio
        let sharpe = self.calculate_sharpe(&portfolio.daily_returns);

        // Calculate Sortino ratio
        let sortino = self.calculate_sortino(&portfolio.daily_returns);

        // Calculate max drawdown
        let max_drawdown = self.calculate_max_drawdown(&portfolio.equity_curve);

        // Win rate
        let win_rate = self.calculate_win_rate(&portfolio.trades);
        let profit_factor = self.calculate_profit_factor(&portfolio.trades);

        // Average trade
        let closed = portfolio
            .trades
            .iter()
            .filter(|trade| trade.action == TradeAction::Sell)
            .count();
        let avg_trade = if closed > 0 {
            fixed_to_decimal(portfolio.realized_pnl) / closed as f64
        } else {
            0.0
        };

        Ok(FitnessScore {
            pnl,
            sharpe_ratio: sharpe,
            sortino_ratio: sortino,
            max_drawdown,
            win_rate,
            profit_factor,
            avg_trade,
            avg_duration: 0.0,
            trade_count: closed,
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
            simulation_state_hash: [0u8; 32],
            simulation_receipt_hash: [0u8; 32],
        })
    }

    fn calculate_sharpe(&self, daily_returns: &[Fixed]) -> f64 {
        if daily_returns.len() < 2 {
            return 0.0;
        }

        let returns: Vec<f64> = daily_returns
            .iter()
            .map(|value| fixed_to_decimal(*value))
            .collect();
        let mean: f64 = returns.iter().sum::<f64>() / returns.len() as f64;
        let annualized_mean = mean * 252.0;

        let variance: f64 =
            returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (returns.len() - 1) as f64;
        let std_dev = variance.sqrt() * (252.0_f64).sqrt();

        if std_dev > 0.0 {
            (annualized_mean - self.config.risk_free_rate) / std_dev
        } else {
            0.0
        }
    }

    fn calculate_sortino(&self, daily_returns: &[Fixed]) -> f64 {
        if daily_returns.len() < 2 {
            return 0.0;
        }

        let returns: Vec<f64> = daily_returns
            .iter()
            .map(|value| fixed_to_decimal(*value))
            .collect();
        let mean: f64 = returns.iter().sum::<f64>() / returns.len() as f64;
        let annualized_mean = mean * 252.0;

        let downside_variance: f64 = returns
            .iter()
            .filter(|&&r| r < 0.0)
            .map(|r| r.powi(2))
            .sum::<f64>()
            / returns.len() as f64;
        let downside_dev = downside_variance.sqrt() * (252.0_f64).sqrt();

        if downside_dev > 0.0 {
            (annualized_mean - self.config.risk_free_rate) / downside_dev
        } else {
            annualized_mean
        }
    }

    fn calculate_max_drawdown(&self, equity_curve: &[Fixed]) -> f64 {
        let Some(&first) = equity_curve.first() else {
            return 0.0;
        };
        let mut peak = first;
        let mut max_drawdown = 0.0f64;
        for &equity in equity_curve {
            if equity > peak {
                peak = equity;
            }
            if peak > 0 {
                let drawdown = (peak.saturating_sub(equity) as f64 / peak as f64) * 100.0;
                max_drawdown = max_drawdown.max(drawdown);
            }
        }
        max_drawdown
    }

    fn calculate_win_rate(&self, trades: &[Trade]) -> f64 {
        let closed: Vec<&Trade> = trades
            .iter()
            .filter(|trade| trade.action == TradeAction::Sell)
            .collect();
        if closed.is_empty() {
            return 0.0;
        }
        closed.iter().filter(|trade| trade.realized_pnl > 0).count() as f64 / closed.len() as f64
    }

    fn calculate_profit_factor(&self, trades: &[Trade]) -> f64 {
        let gross_profit: i128 = trades
            .iter()
            .filter(|trade| trade.realized_pnl > 0)
            .map(|trade| trade.realized_pnl as i128)
            .sum();
        let gross_loss: i128 = trades
            .iter()
            .filter(|trade| trade.realized_pnl < 0)
            .map(|trade| -(trade.realized_pnl as i128))
            .sum();
        if gross_loss > 0 {
            gross_profit as f64 / gross_loss as f64
        } else if gross_profit > 0 {
            10.0
        } else {
            0.0
        }
    }

    fn calculate_volatility(&self, daily_returns: &[Fixed]) -> f64 {
        if daily_returns.len() < 2 {
            return 0.0;
        }

        let returns: Vec<f64> = daily_returns
            .iter()
            .map(|value| fixed_to_decimal(*value))
            .collect();
        let mean: f64 = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance: f64 =
            returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (returns.len() - 1) as f64;

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
        if !result.errors.is_empty() {
            return Err(EvolutionError::SimulationFailed(format!(
                "simulation did not complete: {}",
                result.errors.join("; ")
            )));
        }
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

    fn fixed(value: f64) -> Fixed {
        decimal_to_fixed(value, "test value").expect("fixed value")
    }

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
        let mut portfolio = PortfolioState::new(fixed(100_000.0));
        assert_eq!(portfolio.cash, fixed(100_000.0));
        assert_eq!(portfolio.total_value, fixed(100_000.0));

        portfolio.cash -= fixed(10_000.0);
        portfolio.positions.insert("BTC".to_string(), fixed(1.0));
        portfolio
            .mark_price("BTC", fixed(50_000.0))
            .expect("mark price");

        portfolio.update_value().expect("portfolio value");

        assert_eq!(portfolio.total_value, fixed(140_000.0));
    }

    #[test]
    fn mark_to_market_values_every_held_asset() {
        let mut portfolio = PortfolioState::new(fixed(90_000.0));
        portfolio.positions.insert("ETH".into(), fixed(1.0));
        portfolio.positions.insert("BTC".into(), fixed(0.1));
        portfolio.mark_price("ETH", fixed(2_000.0)).unwrap();
        portfolio.mark_price("BTC", fixed(50_000.0)).unwrap();
        portfolio.update_value().unwrap();

        assert_eq!(portfolio.total_value, fixed(97_000.0));
    }

    #[test]
    fn missing_mark_price_fails_instead_of_valuing_an_asset_at_zero() {
        let mut portfolio = PortfolioState::new(fixed(10_000.0));
        portfolio.positions.insert("ETH".into(), fixed(1.0));
        let error = portfolio
            .update_value()
            .expect_err("unmarked positions must fail closed");
        assert!(error.to_string().contains("no mark price"), "{error}");
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
        assert!(result.portfolio.cash < fixed(100_000.0));
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
    fn repeated_simulations_have_identical_canonical_state_hash() {
        let simulator = Simulator::new(SimulationConfig::default());
        let chromosome =
            Chromosome::from_bytecode(strategy_module(STRATEGY_ENTRYPOINT, 1)).unwrap();
        let market = vec![
            MarketTick::new(1_000, "ETH", 2_000.0),
            MarketTick::new(2_000, "ETH", 2_100.0),
        ];

        let first = simulator.simulate(&chromosome, &market).unwrap();
        let second = simulator.simulate(&chromosome, &market).unwrap();

        assert_eq!(first.state_hash, second.state_hash);
        assert_eq!(first.portfolio.equity_curve, second.portfolio.equity_curve);
        assert_eq!(first.portfolio.cash, second.portfolio.cash);
        assert_eq!(first.portfolio.positions, second.portfolio.positions);
    }

    #[test]
    fn simulation_receipt_round_trips_and_replays_exactly() {
        let simulator = Simulator::new(SimulationConfig::default());
        let chromosome =
            Chromosome::from_bytecode(strategy_module(STRATEGY_ENTRYPOINT, 1)).unwrap();
        let market = vec![
            MarketTick::new(1_000, "ETH", 2_000.0),
            MarketTick::new(2_000, "ETH", 2_100.0),
        ];
        let result = simulator.simulate(&chromosome, &market).unwrap();
        assert!(result.receipt_matches_result());

        let encoded = result.receipt.encode();
        let decoded = SimulationReceipt::decode(&encoded).expect("canonical receipt");
        assert_eq!(decoded, result.receipt);
        assert_eq!(decoded.digest(), result.receipt.digest());

        simulator
            .replay(&chromosome, &market, &decoded)
            .expect("exact replay");
    }

    #[test]
    fn replay_refuses_a_tampered_receipt() {
        let simulator = Simulator::new(SimulationConfig::default());
        let chromosome =
            Chromosome::from_bytecode(strategy_module(STRATEGY_ENTRYPOINT, 1)).unwrap();
        let market = vec![MarketTick::new(1_000, "ETH", 2_000.0)];
        let result = simulator.simulate(&chromosome, &market).unwrap();
        let mut tampered = result.receipt;
        tampered.final_cash += 1;

        let error = simulator
            .replay(&chromosome, &market, &tampered)
            .expect_err("tampered receipt must be refused");
        assert!(error.to_string().contains("receipt mismatch"), "{error}");
    }

    #[test]
    fn truncated_receipt_bytes_are_refused() {
        let simulator = Simulator::new(SimulationConfig::default());
        let chromosome =
            Chromosome::from_bytecode(strategy_module(STRATEGY_ENTRYPOINT, 0)).unwrap();
        let result = simulator
            .simulate(&chromosome, &[MarketTick::new(1, "ETH", 2_000.0)])
            .unwrap();
        let bytes = result.receipt.encode();
        assert!(SimulationReceipt::decode(&bytes[..bytes.len() - 1]).is_err());
    }

    #[test]
    fn test_trade_execution() {
        let config = SimulationConfig::default();
        let simulator = Simulator::new(config);
        let fixed_config = simulator.fixed_config().expect("fixed config");

        let mut portfolio = PortfolioState::new(fixed(100_000.0));
        let tick = MarketTick::new(0, "ETH", 2000.0);
        let tick = prepare_tick(&tick).expect("prepared tick");

        // Execute buy
        let trade = simulator
            .execute_trade(TradeAction::Buy, &tick, &fixed_config, &mut portfolio)
            .unwrap()
            .unwrap();

        assert_eq!(trade.action, TradeAction::Buy);
        assert!(portfolio.cash < fixed(100_000.0));
        assert!(portfolio.positions.contains_key("ETH"));
    }

    #[test]
    fn round_trip_profit_is_realized_from_fixed_point_cost_basis() {
        let simulator = Simulator::new(SimulationConfig {
            fee_rate: 0.0,
            slippage_rate: 0.0,
            ..Default::default()
        });
        let config = simulator.fixed_config().unwrap();
        let mut portfolio = PortfolioState::new(fixed(100_000.0));
        let buy = prepare_tick(&MarketTick::new(1, "ETH", 100.0)).unwrap();
        let sell = prepare_tick(&MarketTick::new(2, "ETH", 120.0)).unwrap();

        let buy_trade = simulator
            .execute_trade(TradeAction::Buy, &buy, &config, &mut portfolio)
            .unwrap()
            .expect("buy");
        portfolio.trades.push(buy_trade);
        portfolio.mark_price("ETH", sell.price).unwrap();
        portfolio.update_value().unwrap();
        let sell_trade = simulator
            .execute_trade(TradeAction::Sell, &sell, &config, &mut portfolio)
            .unwrap()
            .expect("sell");
        portfolio.trades.push(sell_trade.clone());
        portfolio.update_value().unwrap();

        assert_eq!(portfolio.positions.len(), 0);
        assert_eq!(portfolio.cash, fixed(105_000.0));
        assert_eq!(portfolio.realized_pnl, fixed(5_000.0));
        assert_eq!(sell_trade.realized_pnl, fixed(5_000.0));
        assert_eq!(simulator.calculate_win_rate(&portfolio.trades), 1.0);
    }

    #[test]
    fn test_fitness_calculation() {
        let daily_returns = vec![
            fixed(0.01),
            fixed(-0.005),
            fixed(0.02),
            fixed(0.015),
            fixed(-0.01),
            fixed(0.005),
        ];
        let equity_curve = vec![
            fixed(100.0),
            fixed(101.0),
            fixed(100.5),
            fixed(102.5),
            fixed(101.5),
            fixed(102.0),
        ];

        let config = SimulationConfig::default();
        let simulator = Simulator::new(config);

        let sharpe = simulator.calculate_sharpe(&daily_returns);
        let sortino = simulator.calculate_sortino(&daily_returns);
        let max_dd = simulator.calculate_max_drawdown(&equity_curve);
        let volatility = simulator.calculate_volatility(&daily_returns);

        // Just verify they compute without panicking
        assert!(sharpe.is_finite());
        assert!(sortino.is_finite());
        assert!(max_dd >= 0.0);
        assert!(volatility >= 0.0);
    }
}
