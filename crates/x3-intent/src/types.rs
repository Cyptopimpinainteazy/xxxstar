//! Core intent types.

use serde::{Deserialize, Serialize};
use x3_fees::types::FeeVector;
use x3_proof::types::{AgentIdentity, BlockHeight, Hash256, IntentId};

/// Intent lifecycle state — strictly ordered, no going backward.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IntentState {
    /// Intent has been submitted with bond.
    Submitted = 0,
    /// Route has been bound and sealed.
    RouteBound = 1,
    /// Execution is in progress.
    Executing = 2,
    /// Execution completed, pending finalization.
    Executed = 3,
    /// Intent has been finalized (settlement complete).
    Finalized = 4,
    /// Intent failed and was slashed.
    Slashed = 5,
    /// Intent was cancelled before route binding.
    Cancelled = 6,
    /// Intent expired (finality window exceeded).
    Expired = 7,
}

/// Intent execution flags.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct IntentFlags {
    /// Execute privately (don't broadcast intent details).
    pub private_execution: bool,
    /// Use flashloan for execution capital.
    pub flashloan: bool,
    /// Generate ZK proof (optional, increases latency).
    pub zk_proof: bool,
    /// Slashable scope enabled (required for production).
    pub slashable: bool,
    /// Allow partial fills.
    pub partial_fill: bool,
}

/// A route leg — one atomic swap/transfer in the execution path.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RouteLeg {
    /// Source chain identifier (0 = local, >0 = external chain).
    pub source_chain: u32,
    /// Destination chain identifier.
    pub dest_chain: u32,
    /// Source asset identifier.
    pub source_asset: Vec<u8>,
    /// Destination asset identifier.
    pub dest_asset: Vec<u8>,
    /// Amount in (in base units).
    pub amount_in: u128,
    /// Minimum amount out (slippage protection).
    pub min_amount_out: u128,
    /// DEX/venue address for this leg.
    pub venue: Vec<u8>,
    /// Expected state touches for this leg.
    pub state_touches: u32,
}

/// A sealed route — committed execution path.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SealedRoute {
    /// Ordered legs of execution.
    pub legs: Vec<RouteLeg>,
    /// Hash of the route (computed at seal time).
    pub route_hash: Hash256,
    /// Block at which the route was sealed.
    pub sealed_at: BlockHeight,
    /// Pre-calculated fee for this route.
    pub fee: FeeVector,
    /// Total capital required.
    pub total_capital: u128,
}

/// Execution result from the VM.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionResult {
    /// Whether execution succeeded.
    pub success: bool,
    /// Actual output amounts for each leg.
    pub actual_outputs: Vec<u128>,
    /// Total gas consumed.
    pub gas_consumed: u64,
    /// Profit/loss (signed).
    pub pnl: i128,
    /// Proof chain hash.
    pub proof_chain_hash: Hash256,
    /// State diffs produced.
    pub state_diffs_hash: Hash256,
}

/// Settlement record for a finalized intent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Settlement {
    /// The intent that was settled.
    pub intent_id: IntentId,
    /// Agent that executed.
    pub agent_id: AgentIdentity,
    /// Execution result.
    pub result: ExecutionResult,
    /// Fee paid.
    pub fee_paid: u128,
    /// Bond returned (if successful).
    pub bond_returned: u128,
    /// Block at which settlement was finalized.
    pub settled_at: BlockHeight,
}
