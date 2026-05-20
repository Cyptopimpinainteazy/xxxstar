//! Pre-execution simulation for cross-chain intents.
//!
//! Before any funds are locked, every intent runs a dry-run simulation.
//! The simulation is **not** a full execution — it is a best-effort prediction
//! based on current market data, on-chain liquidity snapshots, and historical
//! bridge performance.
//!
//! ## What Simulation Checks
//!
//! - Route availability (does a path exist for the requested token pair?)
//! - Estimated output after fees and slippage
//! - Estimated total fees (bridge + DEX + gas)
//! - Slippage estimate vs declared max
//! - Liquidity depth at each hop
//! - Risk score (0 = no risk, 100 = do not execute)
//! - Possible failure cases (low liquidity, congested bridge, etc.)
//!
//! ## Integration with the Compiler
//!
//! If `requirements.require_route_simulated` is `true`, the compiler inserts
//! a `SimulateExecution` instruction as step 3. The runtime will call the
//! simulation engine and gate on `SimulationResult::route_found == true &&
//! SimulationResult::risk_score < RISK_THRESHOLD`.

use crate::intent::CrossChainIntent;
use serde::{Deserialize, Serialize};

/// The result of pre-execution simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    /// Intent ID this result belongs to.
    pub intent_id: u64,

    /// Whether a valid execution route was found.
    pub route_found: bool,

    /// Estimated output amount in destination asset base units.
    /// `None` if `route_found` is false.
    pub estimated_output: Option<u128>,

    /// Estimated fees in source asset base units.
    /// Includes bridge fees, DEX fees, and estimated gas.
    pub estimated_fees: u128,

    /// Estimated effective slippage in basis points.
    pub estimated_slippage_bps: u32,

    /// Liquidity depth score (0–100). 100 = deep liquidity, 0 = empty pool.
    pub liquidity_depth_score: u8,

    /// Risk score (0–100). Execution is blocked if this exceeds the threshold.
    pub risk_score: u8,

    /// The simulated route (list of hops).
    pub route: Vec<SimulatedHop>,

    /// Possible failure cases identified during simulation.
    pub failure_cases: Vec<SimulatedFailureCase>,

    /// Simulated execution time estimate in seconds.
    pub estimated_execution_secs: u64,

    /// True if slippage estimate exceeds the declared `max_slippage_bps`.
    pub slippage_exceeds_limit: bool,

    /// True if estimated fees exceed `max_total_fee`.
    pub fee_exceeds_cap: bool,
}

impl SimulationResult {
    /// The risk threshold above which execution is blocked.
    pub const RISK_BLOCK_THRESHOLD: u8 = 75;

    /// Returns true if the intent should be allowed to proceed to execution.
    pub fn is_safe_to_execute(&self) -> bool {
        self.route_found
            && !self.slippage_exceeds_limit
            && !self.fee_exceeds_cap
            && self.risk_score < Self::RISK_BLOCK_THRESHOLD
            && self.failure_cases.iter().all(|f| !f.is_blocking)
    }

    /// Human-readable summary for explorer display.
    pub fn summary(&self) -> String {
        if !self.route_found {
            return "No route found".to_string();
        }
        format!(
            "Output: ~{} | Fees: ~{} | Slippage: ~{}bps | Risk: {}/100 | Hops: {} | {}",
            self.estimated_output
                .map(|v| v.to_string())
                .unwrap_or_else(|| "N/A".to_string()),
            self.estimated_fees,
            self.estimated_slippage_bps,
            self.risk_score,
            self.route.len(),
            if self.is_safe_to_execute() {
                "SAFE"
            } else {
                "BLOCKED"
            }
        )
    }
}

/// One hop in the simulated execution route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedHop {
    /// Name/label of the venue (DEX, bridge, etc.)
    pub venue: String,

    /// Type of operation at this hop.
    pub operation: HopOperation,

    /// Estimated input amount at this hop.
    pub amount_in: u128,

    /// Estimated output amount at this hop.
    pub amount_out: u128,

    /// Fee for this hop in source asset base units.
    pub fee: u128,

    /// Slippage at this hop in basis points.
    pub slippage_bps: u32,

    /// Estimated time for this hop in seconds.
    pub estimated_secs: u64,
}

/// The type of operation at a simulation hop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HopOperation {
    Lock,
    Bridge,
    Swap,
    Mint,
    Release,
    Burn,
}

/// A possible failure case identified during simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedFailureCase {
    /// Short label for the failure case.
    pub label: String,
    /// Description of the failure case.
    pub description: String,
    /// True if this failure case would block execution (vs. just a warning).
    pub is_blocking: bool,
    /// Probability estimate (0.0–1.0).
    pub probability: f32,
    /// Recommended mitigation.
    pub mitigation: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Simulator
// ─────────────────────────────────────────────────────────────────────────────

/// Pre-execution simulator for cross-chain intents.
///
/// In production, this would call out to:
/// - Live DEX liquidity oracles
/// - Bridge fee APIs
/// - Gas price oracles
/// - Historical bridge performance data
///
/// This implementation provides a deterministic stub that exercises the logic
/// structure without live external calls. It is suitable for unit testing,
/// integration testing, and local development.
///
/// Replace the `fetch_*` methods with real oracle calls for production use.
pub struct IntentSimulator;

impl IntentSimulator {
    pub fn new() -> Self {
        Self
    }

    /// Simulate a cross-chain intent.
    ///
    /// Returns a [`SimulationResult`] describing the predicted execution.
    /// The simulation does NOT modify any state.
    pub fn simulate(&self, intent: &CrossChainIntent) -> SimulationResult {
        let source_chain = intent.source.asset.chain;
        let dest_chain = intent.destination.asset.chain;

        let mut failure_cases: Vec<SimulatedFailureCase> = Vec::new();

        // Check: low liquidity warning (stub: warn for same-symbol cross-chain)
        if intent.source.asset.symbol == intent.destination.asset.symbol
            && source_chain != dest_chain
        {
            // Pure bridge: generally high liquidity for major assets
        } else if intent.requires_swap() {
            failure_cases.push(SimulatedFailureCase {
                label: "low_liquidity_risk".to_string(),
                description: "Swap pool may have insufficient depth for this amount".to_string(),
                is_blocking: false,
                probability: 0.05,
                mitigation: Some("Consider reducing amount or splitting into multiple intents".to_string()),
            });
        }

        // Check: slippage against limit
        let slippage_bps = self.estimate_slippage_bps(intent);
        let slippage_exceeds_limit = intent
            .requirements
            .max_slippage_bps
            .map(|max| slippage_bps > max)
            .unwrap_or(false);

        if slippage_exceeds_limit {
            failure_cases.push(SimulatedFailureCase {
                label: "slippage_exceeds_limit".to_string(),
                description: format!(
                    "Estimated slippage {}bps exceeds declared limit {:?}bps",
                    slippage_bps,
                    intent.requirements.max_slippage_bps
                ),
                is_blocking: true,
                probability: 1.0,
                mitigation: Some("Increase slippage tolerance or reduce trade size".to_string()),
            });
        }

        // Estimate fees
        let bridge_fee = if intent.requires_bridge() {
            intent.source.amount / 1000 // 0.1% bridge fee stub
        } else {
            0
        };
        let swap_fee = if intent.requires_swap() {
            intent.source.amount / 333 // ~0.3% swap fee stub
        } else {
            0
        };
        let estimated_fees = bridge_fee + swap_fee;

        // Check: fee cap
        let fee_exceeds_cap = intent
            .requirements
            .max_total_fee
            .map(|cap| estimated_fees > cap)
            .unwrap_or(false);

        if fee_exceeds_cap {
            failure_cases.push(SimulatedFailureCase {
                label: "fee_cap_exceeded".to_string(),
                description: format!(
                    "Estimated fees {} exceed declared cap {:?}",
                    estimated_fees, intent.requirements.max_total_fee
                ),
                is_blocking: true,
                probability: 1.0,
                mitigation: Some("Increase fee cap or wait for lower gas prices".to_string()),
            });
        }

        // Estimated output (stub: simple fee subtraction)
        let estimated_output = if intent.source.amount > estimated_fees {
            Some(intent.source.amount - estimated_fees)
        } else {
            None
        };

        // Risk score (stub: combines slippage + fee risk)
        let risk_score = self.compute_risk_score(slippage_bps, &failure_cases);

        // Route hops
        let route = self.build_simulated_route(intent, estimated_fees);

        // Estimated execution time
        let estimated_execution_secs = if intent.requires_bridge() {
            // Bridge + finality wait
            source_chain.default_safe_confirmations() as u64 * 13 // ~13s per ETH block
                + if intent.requires_swap() { 30 } else { 0 }
                + 300 // destination finality
        } else {
            30 // X3-native
        };

        SimulationResult {
            intent_id: intent.id,
            route_found: true,
            estimated_output,
            estimated_fees,
            estimated_slippage_bps: slippage_bps,
            liquidity_depth_score: 80, // stub: assume reasonable liquidity
            risk_score,
            route,
            failure_cases,
            estimated_execution_secs,
            slippage_exceeds_limit,
            fee_exceeds_cap,
        }
    }

    fn estimate_slippage_bps(&self, intent: &CrossChainIntent) -> u32 {
        if !intent.requires_swap() {
            return 0;
        }
        // Stub: scale slippage with trade size
        let amount = intent.source.amount;
        if amount < 1_000_000 {
            10 // <$1 (6-dec): ~0.1%
        } else if amount < 100_000_000 {
            25 // ~0.25%
        } else if amount < 1_000_000_000 {
            50 // ~0.5%
        } else {
            150 // large trade: ~1.5%
        }
    }

    fn compute_risk_score(&self, slippage_bps: u32, failures: &[SimulatedFailureCase]) -> u8 {
        let blocking_count = failures.iter().filter(|f| f.is_blocking).count();
        let base = (slippage_bps / 10).min(50) as u8;
        let penalty = (blocking_count as u8).saturating_mul(30);
        base.saturating_add(penalty).min(100)
    }

    fn build_simulated_route(&self, intent: &CrossChainIntent, total_fee: u128) -> Vec<SimulatedHop> {
        let mut hops = Vec::new();

        if intent.requires_bridge() {
            hops.push(SimulatedHop {
                venue: format!("{}.bridge", intent.source.asset.chain.as_str()),
                operation: HopOperation::Lock,
                amount_in: intent.source.amount,
                amount_out: intent.source.amount,
                fee: total_fee / 2,
                slippage_bps: 0,
                estimated_secs: 60,
            });
        }

        if intent.requires_swap() {
            let after_bridge = intent.source.amount - (total_fee / 2);
            let swap_fee = total_fee / 2;
            hops.push(SimulatedHop {
                venue: intent
                    .route
                    .allow
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "x3.dex".to_string()),
                operation: HopOperation::Swap,
                amount_in: after_bridge,
                amount_out: after_bridge.saturating_sub(swap_fee),
                fee: swap_fee,
                slippage_bps: self.estimate_slippage_bps(intent),
                estimated_secs: 10,
            });
        }

        if intent.requires_bridge()
            && intent.destination.asset.chain != crate::types::ChainKind::X3
        {
            hops.push(SimulatedHop {
                venue: format!("{}.bridge", intent.destination.asset.chain.as_str()),
                operation: HopOperation::Release,
                amount_in: hops.last().map(|h| h.amount_out).unwrap_or(intent.source.amount),
                amount_out: intent.destination.min_amount.unwrap_or(intent.source.amount * 95 / 100),
                fee: 0,
                slippage_bps: 0,
                estimated_secs: 300,
            });
        }

        hops
    }
}

impl Default for IntentSimulator {
    fn default() -> Self {
        Self::new()
    }
}
