//! Types for the X3 Unified Agent Registry pallet.
//!
//! Consolidates agent identity, permissions, staking, slashing, and economics
//! from the previously scattered pallets (agent-accounts, x3-account-registry,
//! x3-agent-law, x3-slash).

use frame_support::pallet_prelude::*;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_std::prelude::*;

/// Unique identifier for an agent.
pub type AgentId = u32;

/// Agent lifecycle status.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    Debug,
    Default,
)]
pub enum AgentStatus {
    /// Agent is active and can operate.
    #[default]
    Active,
    /// Agent is temporarily suspended.
    Suspended,
    /// Agent is permanently terminated.
    Terminated,
}

/// Agent record — unified identity.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Debug)]
pub struct AgentRecord<AccountId, Balance, BlockNumber> {
    /// Unique agent ID.
    pub id: AgentId,
    /// Controller account (manages the agent).
    pub controller: AccountId,
    /// Operator account (executes on behalf of agent).
    pub operator: AccountId,
    /// Agent name.
    pub name: BoundedVec<u8, ConstU32<64>>,
    /// Agent metadata (JSON).
    pub metadata: BoundedVec<u8, ConstU32<1024>>,
    /// Current status.
    pub status: AgentStatus,
    /// Reputation score (0-200, 100 = neutral).
    pub reputation: u32,
    /// Deposit amount.
    pub deposit: Balance,
    /// Block when registered.
    pub registered_at: BlockNumber,
    /// Last active block.
    pub last_active: BlockNumber,
    /// Atlas ID for cross-VM identity.
    pub atlas_id: Option<u64>,
    /// Account kind classification.
    pub kind: AgentKind,
}

/// Classification of an agent account.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    Debug,
    Default,
)]
pub enum AgentKind {
    /// Standard AI agent.
    #[default]
    AutonomousAgent,
    /// Human-controlled EOA.
    Eoa,
    /// EVM contract agent.
    EvmContract,
    /// SVM program agent.
    SvmProgram,
    /// X3 application zone.
    X3AppZone,
    /// Validator node.
    Validator,
    /// System account.
    System,
}

/// Agent quota limits.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    Debug,
    Default,
)]
pub struct AgentQuota<BlockNumber: Default> {
    /// Maximum gas per block.
    pub gas_per_block: u128,
    /// Maximum compute units per block.
    pub compute_per_block: u128,
    /// Maximum gas per epoch.
    pub gas_per_epoch: u128,
    /// Maximum compute units per epoch.
    pub compute_per_epoch: u128,
    /// Block when epoch started.
    pub epoch_start: BlockNumber,
}

/// Agent permissions.
#[derive(
    Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Debug,
)]
pub struct AgentPermissions {
    /// Can deploy contracts.
    pub can_deploy: bool,
    /// Can stake tokens.
    pub can_stake: bool,
    /// Can vote in governance.
    pub can_vote: bool,
    /// Can execute trades.
    pub can_trade: bool,
    /// Can transfer tokens.
    pub can_transfer: bool,
    /// Can call arbitrary contracts.
    pub can_call_contracts: bool,
    /// Can submit proofs.
    pub can_submit_proofs: bool,
    /// Can participate in consensus.
    pub can_validate: bool,
}

impl Default for AgentPermissions {
    fn default() -> Self {
        Self {
            can_deploy: false,
            can_stake: false,
            can_vote: false,
            can_trade: true,
            can_transfer: true,
            can_call_contracts: true,
            can_submit_proofs: true,
            can_validate: false,
        }
    }
}

/// Permission type for checking.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    Debug,
)]
pub enum PermissionType {
    Deploy,
    Stake,
    Vote,
    Trade,
    Transfer,
    CallContracts,
    SubmitProofs,
    Validate,
}

/// Agent activity tracking.
#[derive(Clone, Default, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Debug)]
pub struct AgentActivity {
    /// Gas used this block.
    pub gas_used_block: u128,
    /// Compute used this block.
    pub compute_used_block: u128,
    /// Gas used this epoch.
    pub gas_used_epoch: u128,
    /// Compute used this epoch.
    pub compute_used_epoch: u128,
    /// Total actions taken.
    pub total_actions: u64,
}

/// Bond state for agent staking.
#[derive(
    Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq,
)]
pub struct AgentBond<AccountId, Balance> {
    /// Unique bond identifier.
    pub bond_id: H256,
    /// Agent who posted the bond.
    pub agent: AccountId,
    /// Amount bonded.
    pub amount: Balance,
    /// Block at which the bond was posted.
    pub posted_at: u32,
    /// Block at which the bond expires.
    pub expires_at: u32,
    /// Associated intent ID (if any).
    pub intent_id: Option<H256>,
    /// Current status of the bond.
    pub status: BondStatus,
}

/// Bond lifecycle status.
#[derive(
    Clone,
    Copy,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    Debug,
    PartialEq,
    Eq,
)]
pub enum BondStatus {
    /// Bond is active and locked.
    Active,
    /// Bond has been fully slashed.
    FullySlashed,
    /// Bond has been released back to the agent.
    Released,
    /// Bond expired without settlement.
    Expired,
}

/// Slash record (immutable history).
#[derive(
    Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq,
)]
pub struct SlashRecord<AccountId> {
    /// Unique slash identifier.
    pub slash_id: u64,
    /// Agent being slashed.
    pub agent: AccountId,
    /// Bond being slashed.
    pub bond_id: H256,
    /// Severity of the slash (0=Minor, 1=Moderate, 2=Major, 3=Critical).
    pub severity: u8,
    /// Amount slashed.
    pub amount_slashed: u128,
    /// Reason for the slash.
    pub reason: BoundedVec<u8, ConstU32<256>>,
    /// Block at which the slash was executed.
    pub slashed_at: u32,
}

/// Policy rules governing agent behavior.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub enum PolicyRule<AccountId> {
    /// Agent can only execute these capabilities.
    CapabilityAllowed(Vec<Vec<u8>>),
    /// Agent must maintain minimum reputation score.
    ReputationMinimum(u64),
    /// Hard cap on tasks scheduled per block.
    MaxTasksPerBlock(u32),
    /// Agent cannot coordinate with these accounts.
    NoCollusionWith(Vec<AccountId>),
    /// Rate limit: max extrinsics per epoch.
    RateLimit(u32),
}

// Manual MaxEncodedLen impl because Vec<Vec<u8>> and Vec<AccountId> don't derive it.
// We use reasonable upper bounds: 32 capabilities, 32 collusion addresses, 64 bytes per capability.
impl<AccountId: MaxEncodedLen> MaxEncodedLen for PolicyRule<AccountId> {
    fn max_encoded_len() -> usize {
        use parity_scale_codec::MaxEncodedLen;
        // Each variant: 1 byte discriminant + payload
        // CapabilityAllowed: Vec<Vec<u8>> — 32 caps × (1+64) bytes each
        // NoCollusionWith: Vec<AccountId> — 32 × AccountId::max_encoded_len()
        // Others: fixed-size primitives
        let cap_allowed = 1
            + <u64 as MaxEncodedLen>::max_encoded_len()
            + 32 * (1 + <u64 as MaxEncodedLen>::max_encoded_len());
        let rep_min = 1 + <u64 as MaxEncodedLen>::max_encoded_len();
        let max_tasks = 1 + <u32 as MaxEncodedLen>::max_encoded_len();
        let no_collusion =
            1 + <u64 as MaxEncodedLen>::max_encoded_len() + 32 * AccountId::max_encoded_len();
        let rate_limit = 1 + <u32 as MaxEncodedLen>::max_encoded_len();
        cap_allowed
            .max(rep_min)
            .max(max_tasks)
            .max(no_collusion)
            .max(rate_limit)
    }
}

/// Slashing reasons.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen,
)]
pub enum SlashingReason {
    InvalidProof,
    TaskGriefing,
    CollusionDetected,
    PolicyViolation,
    RepeatOffender,
    BondExpired,
}

/// Violation type for policy enforcement events.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen,
)]
pub enum ViolationType {
    CapabilityViolation,
    ReputationViolation,
    RateLimitViolation,
    CollusionViolation,
    MaxTasksViolation,
    BlacklistViolation,
    QuotaViolation,
}

/// Action type for agent action events.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen,
)]
pub enum ActionType {
    DeployContract,
    ExecuteTrade,
    SubmitProof,
    StakeTokens,
    GovernanceVote,
    TransferAssets,
    CrossChainMessage,
    Custom(BoundedVec<u8, ConstU32<64>>),
}

/// Agent economics snapshot.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Debug, Default)]
pub struct AgentEconomics<Balance> {
    /// Total rewards earned.
    pub total_rewards: Balance,
    /// Total slashed amount.
    pub total_slashed: Balance,
    /// Current bonded amount.
    pub current_bonded: Balance,
    /// PnL (rewards - slashed).
    pub pnl: Balance,
    /// Number of successful tasks.
    pub successful_tasks: u64,
    /// Number of failed tasks.
    pub failed_tasks: u64,
}

/// Combined agent state for runtime API.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug)]
pub struct AgentFullState<AccountId, Balance, BlockNumber: Default> {
    /// Agent identity record.
    pub record: AgentRecord<AccountId, Balance, BlockNumber>,
    /// Quota limits.
    pub quota: AgentQuota<BlockNumber>,
    /// Permissions.
    pub permissions: AgentPermissions,
    /// Current activity.
    pub activity: AgentActivity,
    /// Active bonds.
    pub bonds: Vec<AgentBond<AccountId, Balance>>,
    /// Economics snapshot.
    pub economics: AgentEconomics<Balance>,
    /// Active policies.
    pub policies: Vec<PolicyRule<AccountId>>,
}

/// Agent summary for listing.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug)]
pub struct AgentSummary<AccountId, BlockNumber> {
    pub id: AgentId,
    pub name: BoundedVec<u8, ConstU32<64>>,
    pub controller: AccountId,
    pub status: AgentStatus,
    pub reputation: u32,
    pub kind: AgentKind,
    pub last_active: BlockNumber,
    pub total_bonded: u128,
    pub pnl: i128,
}

/// Statistics snapshot.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug)]
pub struct AgentStats {
    pub total_agents: u32,
    pub active_agents: u32,
    pub total_gas_consumed: u128,
    pub total_compute_consumed: u128,
    pub current_epoch: u64,
    pub total_bonded: u128,
    pub total_slashed: u128,
    pub total_rewards: u128,
}

/// Reward configuration for proof verification.
#[derive(
    Clone, PartialEq, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Debug, Default,
)]
pub struct ProofRewardConfig<Balance> {
    /// Base reward for submitting a proof.
    pub base_reward: Balance,
    /// Bonus reward for verified proofs.
    pub verification_bonus: Balance,
    /// Bonus reward for proofs that resolve challenges successfully.
    pub challenge_resolution_bonus: Balance,
    /// Whether automatic rewards are enabled.
    pub enabled: bool,
}

/// Reward distribution history entry.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Debug)]
pub struct RewardDistribution<AccountId, Balance, BlockNumber> {
    /// Agent ID that received the reward.
    pub agent_id: AgentId,
    /// Account that received the reward.
    pub recipient: AccountId,
    /// Amount rewarded.
    pub amount: Balance,
    /// Block at which the reward was distributed.
    pub block: BlockNumber,
    /// Reason for the reward.
    pub reason: BoundedVec<u8, ConstU32<64>>,
}
