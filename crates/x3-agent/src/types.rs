//! Core agent types.
//!
//! Per the X3 Master Architecture spec (vΩ-1.0):
//!   `Agent = (Code, Policy, Constraints, Proof)`
//!
//! Every registered agent carries a policy declaration, a constraint set, and a
//! cryptographic proof commitment that is verified before execution.

use serde::{Deserialize, Serialize};
use x3_proof::types::{AgentIdentity, BlockHeight};

// ---------------------------------------------------------------------------
// Agent Policy — what the agent is constitutionally permitted to do
// ---------------------------------------------------------------------------

/// Declares which execution contexts an agent is authorized for.
/// Agents negotiate exclusively via proof exchange; policy specifies
/// the scope of permitted operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentPolicy {
    /// Set of permitted action categories (e.g. "swap", "lend", "bridge").
    pub permitted_actions: Vec<String>,
    /// Which VMs may this agent target (evm | svm | x3vm).
    pub permitted_vms: Vec<VmTarget>,
    /// Maximum number of concurrent open intents.
    pub max_concurrent_intents: u32,
    /// Whether this agent may submit governance proposals.
    pub governance_participation: bool,
}

impl Default for AgentPolicy {
    fn default() -> Self {
        Self {
            permitted_actions: vec![],
            permitted_vms: vec![VmTarget::X3Vm],
            max_concurrent_intents: 8,
            governance_participation: false,
        }
    }
}

/// VM targeting enum for agent policy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VmTarget {
    Evm,
    Svm,
    X3Vm,
}

// ---------------------------------------------------------------------------
// Agent Constraints — budget and action bounds
// ---------------------------------------------------------------------------

/// Hard limits on an agent's resource usage, enforced by the constitutional
/// invariant engine before each execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConstraints {
    /// Maximum spend per epoch (in smallest token units).
    /// Must not exceed `InvariantBounds::max_agent_epoch_budget`.
    pub max_epoch_budget: u128,
    /// Maximum gas per single transaction.
    pub max_gas_per_tx: u64,
    /// Maximum number of state writes per intent execution.
    pub max_state_writes: u32,
    /// The agent's proof must be re-verified after this many executions.
    pub proof_refresh_interval: u64,
}

impl Default for AgentConstraints {
    fn default() -> Self {
        Self {
            max_epoch_budget: 10_000 * 1_000_000_000_000_000_000u128,
            max_gas_per_tx: 30_000_000,
            max_state_writes: 1024,
            proof_refresh_interval: 1000,
        }
    }
}

/// Agent registration status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentStatus {
    /// Agent is registered and active.
    Active,
    /// Agent is suspended (bond below minimum).
    Suspended,
    /// Agent has voluntarily deregistered.
    Deregistered,
    /// Agent was forcibly deactivated (critical slash).
    Deactivated,
}

/// Full agent record — the permanent identity of an agent in the jurisdiction.
///
/// Implements the spec's `Agent = (Code, Policy, Constraints, Proof)` model:
/// - Code: identified by `identity` (persistent key → deterministic execution)
/// - Policy: `policy` field declares permitted actions and VMs
/// - Constraints: `constraints` field sets hard resource limits
/// - Proof: `proof_commitment` is a non-zero hash of the off-chain proof bundle
///   that must be verified before the agent may execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRecord {
    /// Primary identity (persistent key). Represents the `Code` component.
    pub identity: AgentIdentity,
    /// Registration block.
    pub registered_at: BlockHeight,
    /// Current status.
    pub status: AgentStatus,
    /// Initial bond amount.
    pub initial_bond: u128,
    /// Current effective bond (after any slashing).
    pub current_bond: u128,
    /// Linked ephemeral identities.
    pub ephemeral_keys: Vec<[u8; 32]>,
    /// Execution statistics.
    pub stats: AgentStats,
    /// Reputation data (computed from stats).
    pub reputation: x3_fees::types::AgentReputation,

    // --- Proof-Carrying Code fields (vΩ-1.0) ---
    /// Policy declaration: permitted actions, VMs, governance participation.
    /// Constitutes the `Policy` component of `Agent = (Code, Policy, Constraints, Proof)`.
    pub policy: AgentPolicy,

    /// Hard resource constraints enforced by the constitutional invariant engine
    /// before each execution. Constitutes the `Constraints` component.
    pub constraints: AgentConstraints,

    /// SHA-256 commitment to the off-chain formal proof bundle for this agent.
    /// A zero value means the agent has not yet submitted a proof and MUST NOT execute.
    /// Constitutes the `Proof` component.
    /// Refreshed every `constraints.proof_refresh_interval` executions.
    pub proof_commitment: [u8; 32],

    /// Block at which `proof_commitment` was last submitted/verified.
    pub proof_verified_at: BlockHeight,

    /// Number of executions since last proof refresh.
    pub executions_since_proof_refresh: u64,
}

/// Agent execution statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentStats {
    /// Total intents submitted.
    pub intents_submitted: u64,
    /// Total intents executed successfully.
    pub intents_succeeded: u64,
    /// Total intents failed.
    pub intents_failed: u64,
    /// Total intents cancelled.
    pub intents_cancelled: u64,
    /// Total intents expired.
    pub intents_expired: u64,
    /// Total slash events.
    pub slash_count: u64,
    /// Total amount slashed across all events.
    pub total_slashed: u128,
    /// Total volume executed.
    pub total_volume: u128,
    /// Total fees paid.
    pub total_fees_paid: u128,
    /// Total profit realized.
    pub total_profit: i128,
    /// Last activity block.
    pub last_active_at: BlockHeight,
}

/// Configuration for the agent system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Minimum bond to register as an agent.
    pub min_registration_bond: u128,
    /// Maximum ephemeral keys per agent.
    pub max_ephemeral_keys: usize,
    /// Number of slashes before automatic deactivation.
    pub critical_slash_threshold: u64,
    /// Minimum bond to remain active (below this = suspended).
    pub min_active_bond: u128,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            min_registration_bond: 10_000_000,
            max_ephemeral_keys: 10,
            critical_slash_threshold: 3,
            min_active_bond: 1_000_000,
        }
    }
}

/// Event emitted by the agent system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentEvent {
    /// Agent registered.
    Registered {
        pubkey: [u8; 32],
        bond: u128,
        block: BlockHeight,
    },
    /// Ephemeral key linked.
    EphemeralKeyLinked {
        pubkey: [u8; 32],
        ephemeral: [u8; 32],
        block: BlockHeight,
    },
    /// Execution recorded.
    ExecutionRecorded {
        pubkey: [u8; 32],
        success: bool,
        volume: u128,
        block: BlockHeight,
    },
    /// Agent slashed.
    Slashed {
        pubkey: [u8; 32],
        amount: u128,
        reason: String,
        block: BlockHeight,
    },
    /// Agent deactivated.
    Deactivated {
        pubkey: [u8; 32],
        reason: String,
        block: BlockHeight,
    },
    /// Agent deregistered.
    Deregistered {
        pubkey: [u8; 32],
        bond_returned: u128,
        block: BlockHeight,
    },
}
