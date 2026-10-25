//! The trading extension's execution: the production host the trading opcodes run against.
//!
//! The host is a journaled ledger the VM owns, seeded from the chain's state before the call
//! ([`TradingPreState`]): the trader's balances, the flash pools, the venues' reserves, the oracle
//! readings and the cost schedule. Every effect a trading opcode has — balances, reserves, flash
//! liquidity, debts, bindings, costs, events, staged ledger intents, the trader's nonce — is a
//! change to this ledger, and the ledger is snapshotted by the same `AtomicBegin` that snapshots the
//! registers, globals and storage. Nothing leaves the VM until the execution returns with the trade
//! committed; then the caller receives a [`TradingDelta`] to apply and a receipt describing exactly
//! what ran. A trap at any point leaves the chain's state untouched, because nothing was ever
//! written to it.
//!
//! The rules (fees, slippage, oracle firewall, cost ceiling, profit floor) are evaluated here, with
//! checked arithmetic throughout; the static half of each rule is `x3bc_core::trading::verify_section`.

use crate::prelude::*;
use alloc::collections::BTreeMap;

use x3bc_core::amm;
pub use x3bc_core::receipt::{
    Account, CapabilityUse, CostEntry, CostKind, DebtRecord, HostCall, HostCallKind, LedgerIntent,
    Outcome, ProfitRecord, RouteLeg, RouteRecord, RouteSelection, TradeEvent, TradingReceipt,
    NO_OP, RECEIPT_FORMAT_VERSION,
};
pub use x3bc_core::state::{
    state_commitment, ChainAnchor, CostSchedule, Finality, FlashPoolState, OracleObservation,
    TradingPreState, VenueState,
};
use x3bc_core::trading::{Amount, CapabilityKind, TradeOp, TradingSection};
pub use x3bc_core::trap::TrapId;

/// Fixed-point scale for comparing prices (1e18).
const PRICE_SCALE: u128 = 1_000_000_000_000_000_000;

/// What a committed trade asks the chain to apply.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradingDelta {
    /// The trader's final balance of every asset the trade names.
    pub trader_balances: BTreeMap<u32, u128>,
    /// Each venue's final `(reserve_a, reserve_b)`.
    pub venue_reserves: BTreeMap<String, (u128, u128)>,
    /// Each flash pool's final liquidity, by `(provider, asset)`.
    pub flash_liquidity: BTreeMap<(String, u32), u128>,
    /// Fees paid to each flash provider.
    pub flash_fees: BTreeMap<String, u128>,
    /// What the chain's fee collector receives, by asset.
    pub fee_collector: BTreeMap<u32, u128>,
    /// Every value movement, in order — the same list the receipt carries.
    pub intents: Vec<LedgerIntent>,
    /// A committed trade consumes one trader nonce.
    pub trader_nonce_increment: u32,
}

impl TradingDelta {
    pub fn trader_balance(&self, asset: u32) -> Option<u128> {
        self.trader_balances.get(&asset).copied()
    }

    pub fn venue_reserve(&self, venue: &str) -> Option<(u128, u128)> {
        self.venue_reserves.get(venue).copied()
    }

    pub fn flash_fee_paid(&self, provider: &str) -> Option<u128> {
        self.flash_fees.get(provider).copied()
    }

    pub fn fee_collector(&self, asset: u32) -> u128 {
        self.fee_collector.get(&asset).copied().unwrap_or(0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OpenDebt {
    asset: u32,
    principal: u128,
    fee: u128,
    provider: String,
}

/// The journaled state. Cloned whole for a savepoint: it is small and bounded by the section's
/// limits (`x3bc_core::trading::MAX_*`).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Ledger {
    balances: BTreeMap<u32, u128>,
    flash: Vec<FlashPoolState>,
    venues: Vec<VenueState>,
    debts: Vec<Option<OpenDebt>>,
    closed: Vec<DebtRecord>,
    bindings: Vec<Option<(u32, u128)>>,
    costs: Vec<CostEntry>,
    oracle_reads: Vec<OracleObservation>,
    legs: Vec<RouteLeg>,
    events: Vec<TradeEvent>,
    intents: Vec<LedgerIntent>,
    fee_collector: BTreeMap<u32, u128>,
    flash_fees: BTreeMap<String, u128>,
    begun: bool,
    costs_charged: bool,
    profit_asserted: bool,
    committed: bool,
    gas_charged: u64,
}

/// One trade's execution state inside a VM.
#[derive(Clone, Debug)]
pub struct TradingSession {
    section: TradingSection,
    pre: TradingPreState,
    ledger: Ledger,
    savepoints: Vec<Ledger>,
    /// Append-only record of what the engine did, kept across rollbacks so a failure receipt says
    /// how far the trade got.
    transcript: Vec<HostCall>,
    /// The section operation currently executing (for a trap's `op_index`).
    current_op: u16,
}

type Step<T> = Result<T, TrapId>;

fn checked_add(a: u128, b: u128) -> Step<u128> {
    a.checked_add(b).ok_or(TrapId::ArithmeticOverflow)
}

fn checked_sub(a: u128, b: u128, shortfall: TrapId) -> Step<u128> {
    a.checked_sub(b).ok_or(shortfall)
}

/// `price_num / price_den` as a 1e18 fixed-point value.
fn scaled_price(num: u128, den: u128) -> Step<u128> {
    if den == 0 || num == 0 {
        return Err(TrapId::HostRefused);
    }
    amm::mul_div(num, PRICE_SCALE, den).ok_or(TrapId::ArithmeticOverflow)
}

/// `|a - b| / reference` in basis points, rounded up.
fn deviation_bps(a: u128, b: u128, reference: u128) -> Step<u128> {
    let diff = a.abs_diff(b);
    amm::mul_div_ceil(diff, amm::BPS, reference.max(1)).ok_or(TrapId::ArithmeticOverflow)
}

impl TradingSession {
    /// A session for `section` seeded with `pre`.
    pub fn new(section: TradingSection, pre: TradingPreState) -> Self {
        let ledger = Ledger {
            balances: pre.trader_balances.clone(),
            flash: pre.flash_pools.clone(),
            venues: pre.venues.clone(),
            debts: vec![None; section.debts.len()],
            closed: Vec::new(),
            bindings: vec![None; section.bindings.len()],
            costs: Vec::new(),
            oracle_reads: Vec::new(),
            legs: Vec::new(),
            events: Vec::new(),
            intents: Vec::new(),
            fee_collector: BTreeMap::new(),
            flash_fees: BTreeMap::new(),
            begun: false,
            costs_charged: false,
            profit_asserted: false,
            committed: false,
            gas_charged: 0,
        };
        Self {
            section,
            pre,
            ledger,
            savepoints: Vec::new(),
            transcript: Vec::new(),
            current_op: NO_OP,
        }
    }

    pub fn section(&self) -> &TradingSection {
        &self.section
    }

    pub fn pre_state(&self) -> &TradingPreState {
        &self.pre
    }

    /// Whether the trade's `TradeCommit` ran (the execution must still return for it to stand).
    pub fn committed(&self) -> bool {
        self.ledger.committed
    }

    /// The section operation that was executing when the trade stopped.
    pub fn current_op(&self) -> u16 {
        self.current_op
    }

    // ------------------------------------------------------------ journal

    /// Open a savepoint (an atomic window opened).
    pub fn savepoint(&mut self) {
        self.savepoints.push(self.ledger.clone());
    }

    /// Release the newest savepoint (the window committed): its changes now belong to the
    /// enclosing window, or to the execution if it was the outermost.
    pub fn release(&mut self) {
        self.savepoints.pop();
    }

    /// Restore the newest savepoint (the window rolled back).
    pub fn rollback(&mut self) {
        if let Some(saved) = self.savepoints.pop() {
            self.ledger = saved;
        }
    }

    // ------------------------------------------------------------ helpers

    fn record(&mut self, kind: HostCallKind, figures: Vec<u128>) {
        self.transcript.push(HostCall {
            kind,
            op_index: self.current_op,
            figures,
        });
    }

    fn asset_id(&self, index: u16) -> Step<u32> {
        self.section
            .assets
            .get(usize::from(index))
            .map(|asset| asset.id)
            .ok_or(TrapId::SequenceViolation)
    }

    fn capability(&self, index: u16, kind: CapabilityKind) -> Step<String> {
        match self.section.capabilities.get(usize::from(index)) {
            Some(cap) if cap.kind == kind => Ok(cap.name.clone()),
            _ => Err(TrapId::SequenceViolation),
        }
    }

    fn balance(&self, asset: u32) -> u128 {
        self.ledger.balances.get(&asset).copied().unwrap_or(0)
    }

    fn credit(&mut self, asset: u32, amount: u128) -> Step<()> {
        let next = checked_add(self.balance(asset), amount)?;
        self.ledger.balances.insert(asset, next);
        Ok(())
    }

    fn debit(&mut self, asset: u32, amount: u128) -> Step<()> {
        let next = checked_sub(self.balance(asset), amount, TrapId::InsufficientBalance)?;
        self.ledger.balances.insert(asset, next);
        Ok(())
    }

    fn admit_cost(&self, kind: CostKind) -> Step<()> {
        if kind.chain_mandated() || self.section.policy.allowed_cost_kinds & kind.bit() != 0 {
            Ok(())
        } else {
            Err(TrapId::CostKindForbidden)
        }
    }

    fn op(&self, index: u16) -> Step<TradeOp> {
        self.section
            .ops
            .get(usize::from(index))
            .cloned()
            .ok_or(TrapId::SequenceViolation)
    }

    fn require_open(&self) -> Step<()> {
        if !self.ledger.begun || self.ledger.committed {
            return Err(TrapId::SequenceViolation);
        }
        Ok(())
    }

    fn intent(&mut self, asset: u32, from: Account, to: Account, amount: u128) {
        self.ledger.intents.push(LedgerIntent {
            asset,
            from,
            to,
            amount,
        });
    }

    // ------------------------------------------------------------ opcodes

    /// `trade_begin`: chain, deadline, private channel, capabilities and versions.
    pub fn begin(&mut self) -> Step<()> {
        self.current_op = NO_OP;
        if self.ledger.begun {
            return Err(TrapId::SequenceViolation);
        }
        if self.pre.chain_id != self.section.chain_id {
            return Err(TrapId::ChainMismatch);
        }
        let deadline = self
            .pre
            .submitted_at_block
            .checked_add(self.section.policy.deadline_blocks)
            .ok_or(TrapId::ArithmeticOverflow)?;
        if self.pre.block_number < self.pre.submitted_at_block || self.pre.block_number > deadline {
            return Err(TrapId::DeadlineExceeded);
        }
        if self.section.policy.private_submission_required && !self.pre.private_channel {
            return Err(TrapId::PrivateSubmissionUnavailable);
        }
        for cap in &self.section.capabilities {
            let offered: Vec<u16> = match cap.kind {
                CapabilityKind::FlashProvider => self
                    .pre
                    .flash_pools
                    .iter()
                    .filter(|pool| pool.provider == cap.name)
                    .map(|pool| pool.version)
                    .collect(),
                CapabilityKind::Venue => self
                    .pre
                    .venues
                    .iter()
                    .filter(|venue| venue.name == cap.name)
                    .map(|venue| venue.version)
                    .collect(),
                // An oracle is read per swap; a missing reading is `OracleMissing` there.
                CapabilityKind::Oracle => continue,
            };
            if offered.is_empty() {
                return Err(TrapId::MissingCapability);
            }
            if offered.iter().any(|version| *version != cap.version) {
                return Err(TrapId::CapabilityVersion);
            }
        }
        self.ledger.begun = true;
        self.record(HostCallKind::Begin, vec![u128::from(self.pre.block_number)]);
        Ok(())
    }

    /// `trade_borrow op`.
    pub fn borrow(&mut self, index: u16) -> Step<()> {
        self.current_op = index;
        self.require_open()?;
        let TradeOp::Borrow {
            debt,
            provider,
            asset,
            principal,
        } = self.op(index)?
        else {
            return Err(TrapId::SequenceViolation);
        };
        let provider = self.capability(provider, CapabilityKind::FlashProvider)?;
        let asset = self.asset_id(asset)?;
        let slot = usize::from(debt);
        if self.ledger.debts.get(slot).map(Option::is_some) != Some(false) {
            return Err(TrapId::SequenceViolation);
        }
        let pool_index = self
            .ledger
            .flash
            .iter()
            .position(|pool| pool.provider == provider && pool.asset == asset)
            .ok_or(TrapId::MissingCapability)?;
        let fee_bps = self.ledger.flash[pool_index].fee_bps;
        if fee_bps > self.section.policy.max_flash_fee_bps {
            return Err(TrapId::FlashFeeExceeded);
        }
        self.admit_cost(CostKind::FlashFee)?;
        let liquidity = self.ledger.flash[pool_index].liquidity;
        if principal > liquidity {
            return Err(TrapId::HostRefused);
        }
        let fee = amm::mul_div_ceil(principal, u128::from(fee_bps), amm::BPS)
            .ok_or(TrapId::ArithmeticOverflow)?;
        self.ledger.flash[pool_index].liquidity = liquidity - principal;
        self.credit(asset, principal)?;
        self.ledger.debts[slot] = Some(OpenDebt {
            asset,
            principal,
            fee,
            provider: provider.clone(),
        });
        self.intent(
            asset,
            Account::FlashPool(provider.clone()),
            Account::Trader,
            principal,
        );
        self.ledger.events.push(TradeEvent {
            name: "borrowed".into(),
            values: vec![u128::from(asset), principal, fee],
        });
        self.record(HostCallKind::Borrow, vec![principal, fee, liquidity]);
        Ok(())
    }

    fn observations_for(&self, a: u32, b: u32) -> Vec<OracleObservation> {
        self.pre
            .oracle
            .iter()
            .filter(|o| (o.base == a && o.quote == b) || (o.base == b && o.quote == a))
            .cloned()
            .collect()
    }

    /// The oracle firewall for one swap: fresh, mutually consistent readings, and a venue spot
    /// price within the policy's deviation of them.
    fn oracle_firewall(
        &mut self,
        venue: &VenueState,
        from: u32,
        to: u32,
        ceiling_bps: u16,
    ) -> Step<()> {
        let readings = self.observations_for(from, to);
        if readings.is_empty() {
            return Err(TrapId::OracleMissing);
        }
        if let Some(freshness) = self.section.policy.quote_freshness_blocks {
            for reading in &readings {
                let age = self
                    .pre
                    .block_number
                    .checked_sub(reading.observed_at_block)
                    .ok_or(TrapId::OracleStale)?;
                if age > freshness {
                    return Err(TrapId::OracleStale);
                }
            }
        }
        // Every reading in one orientation — units of `from` per unit of `to` — so the readings and
        // the venue's spot price are compared in the same units.
        let mut prices = Vec::with_capacity(readings.len());
        for reading in &readings {
            let price = if reading.base == to {
                scaled_price(reading.price_num, reading.price_den)?
            } else {
                scaled_price(reading.price_den, reading.price_num)?
            };
            prices.push(price);
            self.ledger.oracle_reads.push(reading.clone());
        }
        let ceiling = u128::from(ceiling_bps);
        let lowest = prices.iter().copied().min().unwrap_or(0);
        let highest = prices.iter().copied().max().unwrap_or(0);
        if deviation_bps(highest, lowest, lowest)? > ceiling {
            return Err(TrapId::OracleDivergent);
        }
        // Venue spot price of `to` in `from` units: reserve(from) / reserve(to).
        let (reserve_from, reserve_to) = if venue.asset_a == from {
            (venue.reserve_a, venue.reserve_b)
        } else {
            (venue.reserve_b, venue.reserve_a)
        };
        let spot = scaled_price(reserve_from, reserve_to)?;
        for price in &prices {
            if deviation_bps(spot, *price, *price)? > ceiling {
                return Err(TrapId::OracleDeviation);
            }
        }
        Ok(())
    }

    /// `trade_swap op`.
    pub fn swap(&mut self, index: u16) -> Step<()> {
        self.current_op = index;
        self.require_open()?;
        let TradeOp::Swap {
            binding,
            venue,
            from,
            to,
            input,
            min_out,
        } = self.op(index)?
        else {
            return Err(TrapId::SequenceViolation);
        };
        let venue_name = self.capability(venue, CapabilityKind::Venue)?;
        let from = self.asset_id(from)?;
        let to = self.asset_id(to)?;
        let venue_index = self
            .ledger
            .venues
            .iter()
            .position(|v| v.name == venue_name)
            .ok_or(TrapId::MissingCapability)?;
        let pool = self.ledger.venues[venue_index].clone();
        let a_to_b = if pool.asset_a == from && pool.asset_b == to {
            true
        } else if pool.asset_b == from && pool.asset_a == to {
            false
        } else {
            return Err(TrapId::HostRefused);
        };
        let amount_in = match input {
            Amount::Literal(value) => value,
            Amount::Binding(slot) => match self.ledger.bindings.get(usize::from(slot)) {
                Some(Some((asset, amount))) if *asset == from => *amount,
                _ => return Err(TrapId::SequenceViolation),
            },
            Amount::DebtPrincipal(slot) => match self.ledger.debts.get(usize::from(slot)) {
                Some(Some(debt)) if debt.asset == from => debt.principal,
                _ => return Err(TrapId::SequenceViolation),
            },
        };
        if self.balance(from) < amount_in {
            return Err(TrapId::InsufficientBalance);
        }
        if let Some(ceiling) = self.section.policy.max_oracle_deviation_bps {
            self.oracle_firewall(&pool, from, to, ceiling)?;
        }
        let (reserve_in, reserve_out) = if a_to_b {
            (pool.reserve_a, pool.reserve_b)
        } else {
            (pool.reserve_b, pool.reserve_a)
        };
        let priced = amm::swap(amount_in, reserve_in, reserve_out, pool.fee_bps).map_err(
            |error| match error {
                amm::AmmError::Overflow => TrapId::ArithmeticOverflow,
                _ => TrapId::HostRefused,
            },
        )?;
        if priced.amount_out < min_out {
            return Err(TrapId::MinOutput);
        }
        let slippage = amm::shortfall_bps(priced.spot_out, priced.amount_out)
            .ok_or(TrapId::ArithmeticOverflow)?;
        if slippage > u128::from(self.section.policy.max_slippage_bps) {
            return Err(TrapId::Slippage);
        }
        if let Some(ceiling) = self.section.policy.max_price_impact_bps {
            let impact = amm::shortfall_bps(priced.spot_out, priced.no_fee_out)
                .ok_or(TrapId::ArithmeticOverflow)?;
            if impact > u128::from(ceiling) {
                return Err(TrapId::PriceImpact);
            }
        }
        if priced.fee > 0 {
            self.admit_cost(CostKind::VenueFee)?;
        }

        self.debit(from, amount_in)?;
        self.credit(to, priced.amount_out)?;
        let venue_state = &mut self.ledger.venues[venue_index];
        if a_to_b {
            venue_state.reserve_a = priced.reserve_in_after;
            venue_state.reserve_b = priced.reserve_out_after;
        } else {
            venue_state.reserve_b = priced.reserve_in_after;
            venue_state.reserve_a = priced.reserve_out_after;
        }
        let slot = usize::from(binding);
        match self.ledger.bindings.get_mut(slot) {
            Some(cell @ None) => *cell = Some((to, priced.amount_out)),
            _ => return Err(TrapId::SequenceViolation),
        }
        if priced.fee > 0 {
            self.ledger.costs.push(CostEntry {
                kind: CostKind::VenueFee,
                asset: from,
                amount: priced.fee,
                embedded: true,
            });
        }
        self.intent(
            from,
            Account::Trader,
            Account::Venue(venue_name.clone()),
            amount_in,
        );
        self.intent(
            to,
            Account::Venue(venue_name.clone()),
            Account::Trader,
            priced.amount_out,
        );
        self.ledger.legs.push(RouteLeg {
            venue: venue_name,
            from,
            to,
            min_out,
            input: amount_in,
            output: priced.amount_out,
        });
        self.ledger.events.push(TradeEvent {
            name: "swapped".into(),
            values: vec![
                u128::from(from),
                amount_in,
                u128::from(to),
                priced.amount_out,
            ],
        });
        self.record(
            HostCallKind::Swap,
            vec![
                amount_in,
                priced.amount_out,
                priced.fee,
                reserve_in,
                reserve_out,
                priced.reserve_in_after,
                priced.reserve_out_after,
                slippage,
            ],
        );
        Ok(())
    }

    /// `trade_repay op`.
    pub fn repay(&mut self, index: u16) -> Step<()> {
        self.current_op = index;
        self.require_open()?;
        let TradeOp::Repay { debt } = self.op(index)? else {
            return Err(TrapId::SequenceViolation);
        };
        let slot = usize::from(debt);
        let open = match self.ledger.debts.get(slot) {
            Some(Some(open)) => open.clone(),
            _ => return Err(TrapId::SequenceViolation),
        };
        let owed = checked_add(open.principal, open.fee)?;
        self.debit(open.asset, owed)?;
        let pool = self
            .ledger
            .flash
            .iter_mut()
            .find(|pool| pool.provider == open.provider && pool.asset == open.asset)
            .ok_or(TrapId::MissingCapability)?;
        pool.liquidity = checked_add(pool.liquidity, owed)?;
        let paid = self
            .ledger
            .flash_fees
            .get(&open.provider)
            .copied()
            .unwrap_or(0);
        self.ledger
            .flash_fees
            .insert(open.provider.clone(), checked_add(paid, open.fee)?);
        self.ledger.debts[slot] = None;
        self.ledger.closed.push(DebtRecord {
            debt: self.section.debts.get(slot).cloned().unwrap_or_default(),
            asset: open.asset,
            principal: open.principal,
            fee: open.fee,
            repaid: owed,
        });
        if open.fee > 0 {
            self.ledger.costs.push(CostEntry {
                kind: CostKind::FlashFee,
                asset: open.asset,
                amount: open.fee,
                embedded: true,
            });
        }
        self.intent(
            open.asset,
            Account::Trader,
            Account::FlashPool(open.provider.clone()),
            owed,
        );
        self.ledger.events.push(TradeEvent {
            name: "repaid".into(),
            values: vec![u128::from(open.asset), owed],
        });
        self.record(HostCallKind::Repay, vec![open.principal, open.fee, owed]);
        Ok(())
    }

    /// `trade_charge_costs op`: commit every cost the chain levies, gas included through the end of
    /// the function (`gas_through_end`), before any profit is evaluated.
    pub fn charge_costs(&mut self, index: u16, gas_through_end: u64, gas_limit: u64) -> Step<()> {
        self.current_op = index;
        self.require_open()?;
        if !matches!(self.op(index)?, TradeOp::ChargeCosts) || self.ledger.costs_charged {
            return Err(TrapId::SequenceViolation);
        }
        if gas_through_end > gas_limit {
            return Err(TrapId::GasExhausted);
        }
        let schedule = self.pre.costs.clone();
        let fee_asset = schedule.fee_asset;
        let gas_cost = schedule
            .gas_price
            .checked_mul(u128::from(gas_through_end))
            .ok_or(TrapId::ArithmeticOverflow)?;
        let reads = self.ledger.oracle_reads.len() as u128;
        let oracle_cost = schedule
            .oracle_fee
            .checked_mul(reads)
            .ok_or(TrapId::ArithmeticOverflow)?;
        let relay = if self.section.policy.private_submission_required {
            schedule.relay_fee
        } else {
            0
        };
        let charged = [
            (CostKind::Gas, gas_cost),
            (CostKind::SettlementFee, schedule.settlement_fee),
            (CostKind::ProtocolFee, schedule.protocol_fee),
            (CostKind::RelayFee, relay),
            (CostKind::OracleFee, oracle_cost),
            (CostKind::SolverFee, schedule.solver_fee),
            (CostKind::CrossDomainFee, schedule.cross_domain_fee),
        ];
        let mut total: u128 = 0;
        let mut entries = Vec::new();
        for (kind, amount) in charged {
            if amount == 0 {
                continue;
            }
            self.admit_cost(kind)?;
            total = checked_add(total, amount)?;
            entries.push(CostEntry {
                kind,
                asset: fee_asset,
                amount,
                embedded: false,
            });
        }

        // The ceiling covers every cost in its asset, embedded ones included.
        let ceiling_asset = self.asset_id(self.section.policy.max_cost_asset)?;
        let mut in_ceiling_asset: u128 = 0;
        for entry in self.ledger.costs.iter().chain(entries.iter()) {
            if entry.asset == ceiling_asset {
                in_ceiling_asset = checked_add(in_ceiling_asset, entry.amount)?;
            }
        }
        if in_ceiling_asset > self.section.policy.max_cost {
            return Err(TrapId::CostCeiling);
        }

        self.debit(fee_asset, total)?;
        for entry in &entries {
            let collected = self
                .ledger
                .fee_collector
                .get(&fee_asset)
                .copied()
                .unwrap_or(0);
            self.ledger
                .fee_collector
                .insert(fee_asset, checked_add(collected, entry.amount)?);
            self.intent(
                fee_asset,
                Account::Trader,
                Account::FeeCollector,
                entry.amount,
            );
        }
        self.ledger.costs.extend(entries);
        self.ledger.costs_charged = true;
        self.ledger.gas_charged = gas_through_end;
        self.ledger.events.push(TradeEvent {
            name: "costs_charged".into(),
            values: vec![u128::from(fee_asset), total],
        });
        self.record(
            HostCallKind::ChargeCosts,
            vec![u128::from(gas_through_end), total],
        );
        Ok(())
    }

    /// Net change of the trader's balance of `asset` since the pre-state.
    fn net(&self, asset: u32) -> Step<i128> {
        let before = self.pre.trader_balances.get(&asset).copied().unwrap_or(0);
        let after = self.balance(asset);
        let before = i128::try_from(before).map_err(|_| TrapId::ArithmeticOverflow)?;
        let after = i128::try_from(after).map_err(|_| TrapId::ArithmeticOverflow)?;
        after.checked_sub(before).ok_or(TrapId::ArithmeticOverflow)
    }

    /// `trade_assert_profit op`.
    pub fn assert_profit(&mut self, index: u16) -> Step<()> {
        self.current_op = index;
        self.require_open()?;
        let TradeOp::AssertMinProfit { asset, minimum } = self.op(index)? else {
            return Err(TrapId::SequenceViolation);
        };
        if !self.ledger.costs_charged {
            return Err(TrapId::SequenceViolation);
        }
        let asset = self.asset_id(asset)?;
        let net = self.net(asset)?;
        let floor = i128::try_from(minimum).map_err(|_| TrapId::ArithmeticOverflow)?;
        self.record(HostCallKind::AssertProfit, vec![net as u128, minimum]);
        if net < floor {
            return Err(TrapId::ProfitBelowFloor);
        }
        self.ledger.profit_asserted = true;
        Ok(())
    }

    /// `trade_assert_debts_closed op`.
    pub fn assert_debts_closed(&mut self, index: u16) -> Step<()> {
        self.current_op = index;
        self.require_open()?;
        if !matches!(self.op(index)?, TradeOp::AssertAllDebtsClosed) {
            return Err(TrapId::SequenceViolation);
        }
        self.record(HostCallKind::AssertDebtsClosed, Vec::new());
        if self.ledger.debts.iter().any(Option::is_some) {
            return Err(TrapId::DebtOutstanding);
        }
        Ok(())
    }

    /// `trade_assert_solvent op`.
    pub fn assert_solvent(&mut self, index: u16) -> Step<()> {
        self.current_op = index;
        self.require_open()?;
        if !matches!(self.op(index)?, TradeOp::AssertSolvent) {
            return Err(TrapId::SequenceViolation);
        }
        self.record(HostCallKind::AssertSolvent, Vec::new());
        let assets: Vec<u32> = self.section.assets.iter().map(|a| a.id).collect();
        for asset in assets {
            if self.net(asset)? < 0 {
                return Err(TrapId::Insolvent);
            }
        }
        Ok(())
    }

    /// `trade_commit`.
    pub fn commit(&mut self) -> Step<()> {
        self.current_op = NO_OP;
        self.require_open()?;
        if self.ledger.debts.iter().any(Option::is_some) {
            return Err(TrapId::DebtOutstanding);
        }
        if !self.ledger.costs_charged || !self.ledger.profit_asserted {
            return Err(TrapId::SequenceViolation);
        }
        self.ledger.committed = true;
        self.ledger.events.push(TradeEvent {
            name: "trade_committed".into(),
            values: Vec::new(),
        });
        self.record(HostCallKind::Commit, Vec::new());
        Ok(())
    }

    /// `trade_load_profit dst`: the profit asset's net change, as a checked `i64`.
    pub fn load_profit(&mut self) -> Step<i64> {
        self.current_op = NO_OP;
        if !self.ledger.profit_asserted {
            return Err(TrapId::SequenceViolation);
        }
        let asset = self
            .section
            .profit_asset()
            .ok_or(TrapId::SequenceViolation)?;
        let net = self.net(asset)?;
        self.record(HostCallKind::LoadProfit, vec![net as u128]);
        i64::try_from(net).map_err(|_| TrapId::InvalidConversion)
    }

    /// Gas charged for the trade (zero until costs are committed).
    pub fn gas_charged(&self) -> u64 {
        self.ledger.gas_charged
    }

    // ------------------------------------------------------------ results

    fn balances_of(&self, balances: &BTreeMap<u32, u128>) -> Vec<(u32, u128)> {
        let mut out: Vec<(u32, u128)> = self
            .section
            .assets
            .iter()
            .map(|asset| (asset.id, balances.get(&asset.id).copied().unwrap_or(0)))
            .collect();
        out.sort_by_key(|(asset, _)| *asset);
        out
    }

    fn post_state(&self) -> TradingPreState {
        let mut post = self.pre.clone();
        post.trader_balances = self.ledger.balances.clone();
        post.flash_pools = self.ledger.flash.clone();
        post.venues = self.ledger.venues.clone();
        post
    }

    /// What a committed trade applies. `None` unless the trade committed.
    pub fn delta(&self) -> Option<TradingDelta> {
        if !self.ledger.committed {
            return None;
        }
        let trader_balances = self
            .section
            .assets
            .iter()
            .map(|asset| (asset.id, self.balance(asset.id)))
            .collect();
        let venue_reserves = self
            .ledger
            .venues
            .iter()
            .map(|v| (v.name.clone(), (v.reserve_a, v.reserve_b)))
            .collect();
        let flash_liquidity = self
            .ledger
            .flash
            .iter()
            .map(|p| ((p.provider.clone(), p.asset), p.liquidity))
            .collect();
        Some(TradingDelta {
            trader_balances,
            venue_reserves,
            flash_liquidity,
            flash_fees: self.ledger.flash_fees.clone(),
            fee_collector: self.ledger.fee_collector.clone(),
            intents: self.ledger.intents.clone(),
            trader_nonce_increment: 1,
        })
    }

    /// The receipt for this execution.
    ///
    /// `committed` is the caller's verdict that the execution returned with the trade committed;
    /// anything else is a rollback and the receipt carries the pre-state as the post-state, no
    /// costs, no debts, no events and no intents — because none of them happened on chain.
    #[allow(clippy::too_many_arguments)]
    pub fn receipt(
        &self,
        outcome: Outcome,
        bytecode_hash: [u8; 32],
        bytecode_version: u32,
        anchor: ChainAnchor,
        gas_limit: u64,
        gas_used: u64,
    ) -> TradingReceipt {
        let committed = matches!(outcome, Outcome::Committed);
        let pre_balances = self.balances_of(&self.pre.trader_balances);
        let pre_commitment = state_commitment(&self.pre);
        let capabilities = self
            .section
            .capabilities
            .iter()
            .map(|cap| CapabilityUse {
                kind: cap.kind,
                name: cap.name.clone(),
                required_version: cap.version,
                offered_version: match cap.kind {
                    CapabilityKind::FlashProvider => self
                        .pre
                        .flash_pools
                        .iter()
                        .find(|p| p.provider == cap.name)
                        .map(|p| p.version),
                    CapabilityKind::Venue => self
                        .pre
                        .venues
                        .iter()
                        .find(|v| v.name == cap.name)
                        .map(|v| v.version),
                    CapabilityKind::Oracle => None,
                },
            })
            .collect();
        let (post_balances, post_commitment, costs, debts, events, intents, legs, oracle, net) =
            if committed {
                let profit_asset = self.section.profit_asset().unwrap_or(0);
                (
                    self.balances_of(&self.ledger.balances),
                    state_commitment(&self.post_state()),
                    self.ledger.costs.clone(),
                    self.ledger.closed.clone(),
                    self.ledger.events.clone(),
                    self.ledger.intents.clone(),
                    self.ledger.legs.clone(),
                    self.ledger.oracle_reads.clone(),
                    self.net(profit_asset).unwrap_or(0),
                )
            } else {
                (
                    pre_balances.clone(),
                    pre_commitment,
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    0,
                )
            };
        TradingReceipt {
            format_version: RECEIPT_FORMAT_VERSION,
            source_hash: self.section.source_hash,
            compiler_version: self.section.compiler_version.clone(),
            mode: self.section.mode,
            bytecode_hash,
            bytecode_version,
            chain_id: self.section.chain_id,
            anchor,
            trade_id: self.section.trade_id.clone(),
            policy_id: self.section.policy.policy_id.clone(),
            capabilities,
            gas_limit,
            gas_used,
            gas_charged: if committed {
                self.ledger.gas_charged
            } else {
                0
            },
            costs,
            oracle,
            transcript: self.transcript.clone(),
            route: RouteRecord {
                selection: RouteSelection::Compiled,
                legs,
            },
            pre_state: self.pre.clone(),
            pre_balances,
            post_balances,
            debts,
            profit: ProfitRecord {
                asset: self.section.profit_asset().unwrap_or(0),
                net,
            },
            outcome,
            events,
            intents,
            pre_state_commitment: pre_commitment,
            post_state_commitment: post_commitment,
        }
    }
}
