//! Intent builder — thin wrapper over `x3-intent` that the SDK exposes.
//!
//! Provides a builder pattern for constructing `ArbIntent` values from
//! higher-level SDK inputs, enforcing the SDK-side preconditions before
//! handing off to `IntentLifecycle`.

use crate::error::UcError;
use x3_fees::types::FeeVector;
use x3_intent::{
    intent::ArbIntent,
    lifecycle::IntentLifecycle,
    types::{IntentFlags, RouteLeg},
};
use x3_proof::types::{AgentIdentity, BlockHeight, Hash256, IntentId};
use x3_slash::types::BondId;

/// SDK-level intent builder.
///
/// ```
/// use x3_universal_contracts::intents::IntentBuilder;
///
/// let _builder = IntentBuilder::new(
///     [1u128],       // intent ID
///     [0u8; 32],     // agent pubkey
///     [0u8; 32],     // program hash
/// );
/// ```
pub struct IntentBuilder {
    id: IntentId,
    agent_id: AgentIdentity,
    program_hash: Hash256,
    bond_amount: u128,
    fee_cap: u128,
    slashable: bool,
    submitted_at: BlockHeight,
    finality_window: u64,
    route_legs: Vec<RouteLeg>,
}

impl IntentBuilder {
    /// Create a new builder.  Defaults: bond 0, fee_cap 1, non-slashable,
    /// block 1, window 100 (suitable for tests; override in production).
    pub fn new(id: [u128; 1], agent_pubkey: [u8; 32], program_hash: Hash256) -> Self {
        Self {
            id: IntentId(id[0]),
            agent_id: AgentIdentity {
                pubkey: agent_pubkey,
                ephemeral: false,
            },
            program_hash,
            bond_amount: 0,
            fee_cap: 1,
            slashable: false,
            submitted_at: 1,
            finality_window: 100,
            route_legs: Vec::new(),
        }
    }

    pub fn bond(mut self, amount: u128) -> Self {
        self.bond_amount = amount;
        self
    }

    pub fn fee_cap(mut self, cap: u128) -> Self {
        self.fee_cap = cap;
        self
    }

    pub fn slashable(mut self) -> Self {
        self.slashable = true;
        self
    }

    pub fn submitted_at(mut self, block: BlockHeight) -> Self {
        self.submitted_at = block;
        self
    }

    pub fn finality_window(mut self, blocks: u64) -> Self {
        self.finality_window = blocks;
        self
    }

    /// Add a route leg to the intent.
    pub fn add_leg(mut self, leg: RouteLeg) -> Self {
        self.route_legs.push(leg);
        self
    }

    /// Build the `ArbIntent`.  Returns `UcError::ZeroFeeCap` if fee_cap is zero.
    pub fn build(self) -> Result<ArbIntent, UcError> {
        if self.fee_cap == 0 {
            return Err(UcError::ZeroFeeCap);
        }
        let flags = IntentFlags {
            private_execution: false,
            flashloan: false,
            zk_proof: false,
            slashable: self.slashable,
            partial_fill: false,
        };

        let mut intent = IntentLifecycle::submit_intent(
            self.id,
            self.agent_id,
            self.program_hash,
            flags,
            self.bond_amount,
            self.fee_cap,
            self.submitted_at,
            self.finality_window,
        )
        .map_err(|e| UcError::IntentError(format!("{}", e)))?;

        // If route legs were provided, bind them now.
        if !self.route_legs.is_empty() {
            let fee = FeeVector {
                base_fee: 0,
                complexity_fee: 0,
                capital_fee: 0,
                reputation_discount: 0,
                total: 0,
            };
            let bond_id = BondId(0); // Placeholder; real bond IDs are assigned by bond pallet.
            IntentLifecycle::bind_route(
                &mut intent,
                self.route_legs,
                fee,
                bond_id,
                self.submitted_at,
            )
            .map_err(|e| UcError::IntentError(format!("{}", e)))?;
        }

        Ok(intent)
    }
}
