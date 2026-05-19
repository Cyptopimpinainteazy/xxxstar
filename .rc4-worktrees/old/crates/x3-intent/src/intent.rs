//! ArbIntent — the core data structure for arbitrage intents.

use crate::types::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use x3_proof::types::{AgentIdentity, BlockHeight, Hash256, IntentId};
use x3_slash::types::BondId;

/// An Arbitrage Intent — the atomic unit of work in the X3 jurisdiction.
///
/// Every execution in X3 begins with an intent. Intents carry bonds,
/// declare routes, and produce proofs. They are the law of the floor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbIntent {
    /// Unique intent identifier.
    pub id: IntentId,
    /// Agent submitting the intent.
    pub agent_id: AgentIdentity,
    /// Current lifecycle state.
    pub state: IntentState,
    /// X3-lang program bytecode hash (the program to execute).
    pub program_hash: Hash256,
    /// Execution flags.
    pub flags: IntentFlags,
    /// Bond posted for this intent.
    pub bond_id: Option<BondId>,
    /// Bond amount.
    pub bond_amount: u128,
    /// Sealed route (populated after bind_route).
    pub route: Option<SealedRoute>,
    /// Execution result (populated after execute).
    pub result: Option<ExecutionResult>,
    /// Maximum fee the agent is willing to pay (fee cap).
    pub fee_cap: u128,
    /// Block at which the intent was submitted.
    pub submitted_at: BlockHeight,
    /// Block at which the intent expires.
    pub expires_at: BlockHeight,
    /// Hash of the intent (computed over immutable fields).
    pub intent_hash: Hash256,
}

impl ArbIntent {
    /// Create a new ArbIntent. Starts in Submitted state.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: IntentId,
        agent_id: AgentIdentity,
        program_hash: Hash256,
        flags: IntentFlags,
        bond_amount: u128,
        fee_cap: u128,
        submitted_at: BlockHeight,
        finality_window: u64,
    ) -> Self {
        let mut intent = Self {
            id,
            agent_id,
            state: IntentState::Submitted,
            program_hash,
            flags,
            bond_id: None,
            bond_amount,
            route: None,
            result: None,
            fee_cap,
            submitted_at,
            expires_at: submitted_at + finality_window,
            intent_hash: [0u8; 32],
        };
        intent.intent_hash = intent.compute_hash();
        intent
    }

    /// Compute the canonical hash of this intent.
    fn compute_hash(&self) -> Hash256 {
        let mut hasher = Sha256::new();
        hasher.update(self.id.0.to_le_bytes());
        hasher.update(self.agent_id.pubkey);
        hasher.update([self.agent_id.ephemeral as u8]);
        hasher.update(self.program_hash);
        hasher.update([
            self.flags.private_execution as u8,
            self.flags.flashloan as u8,
            self.flags.zk_proof as u8,
            self.flags.slashable as u8,
            self.flags.partial_fill as u8,
        ]);
        hasher.update(self.bond_amount.to_le_bytes());
        hasher.update(self.fee_cap.to_le_bytes());
        hasher.update(self.submitted_at.to_le_bytes());
        hasher.update(self.expires_at.to_le_bytes());
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Check if the intent has expired.
    pub fn is_expired(&self, current_block: BlockHeight) -> bool {
        current_block > self.expires_at
    }

    /// Check if the intent is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            IntentState::Finalized
                | IntentState::Slashed
                | IntentState::Cancelled
                | IntentState::Expired
        )
    }

    /// Get the total number of legs in the bound route.
    pub fn leg_count(&self) -> u32 {
        self.route
            .as_ref()
            .map(|r| r.legs.len() as u32)
            .unwrap_or(0)
    }

    /// Get total state touches across all legs.
    pub fn total_state_touches(&self) -> u32 {
        self.route
            .as_ref()
            .map(|r| r.legs.iter().map(|l| l.state_touches).sum())
            .unwrap_or(0)
    }

    /// Get total capital required across all legs.
    pub fn total_capital(&self) -> u128 {
        self.route.as_ref().map(|r| r.total_capital).unwrap_or(0)
    }

    /// Check if any leg is cross-chain.
    pub fn is_cross_chain(&self) -> bool {
        self.route
            .as_ref()
            .map(|r| r.legs.iter().any(|l| l.source_chain != l.dest_chain))
            .unwrap_or(false)
    }

    /// Count cross-chain hops.
    pub fn cross_chain_hops(&self) -> u32 {
        self.route
            .as_ref()
            .map(|r| {
                r.legs
                    .iter()
                    .filter(|l| l.source_chain != l.dest_chain)
                    .count() as u32
            })
            .unwrap_or(0)
    }
}
