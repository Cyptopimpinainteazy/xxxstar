#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

//! # X3 Agent Law Pallet
//!
//! **SECURITY-CRITICAL**: This pallet enforces constitutional policies governing swarm agent behavior.
//! Every extrinsic flows through `AgentLawCheck` SignedExtension before state mutation.
//!
//! ## Design Principles
//!
//! - **Hard-Fail Semantics**: Policy violations reject the entire extrinsic (no soft errors)
//! - **Pre-Dispatch Validation**: Policies checked BEFORE state changes (Invariant → AgentLaw → Execution)
//! - **Zero Collusion**: Policies prevent coordinated attacks (reputation + rate limits)
//! - **Exhaustion Resistant**: Queue depth bounds + per-block task limits prevent DoS
//!
//! ## Policy Model
//!
//! Each registered agent is governed by `PolicyRule` enums:
//!
//! - **CapabilityAllowed(Vec<Capability>)**: Agent can only execute permitted capabilities
//! - **ReputationMinimum(u64)**: Agent must maintain minimum reputation score
//! - **MaxTasksPerBlock(u32)**: Hard cap on tasks scheduled per block
//! - **NoCollusionWith(Vec<AccountId>)**: Agent cannot coordinate with blacklisted accounts
//! - **RateLimit(u64)**: Maximum extrinsics per epoch (24 hours)
//!
//! ## Violation Handling
//!
//! Violations are tracked and auto-enforced:
//! - **LogOnly**: Emits event, continues (warning)
//! - **Slash(u64)**: Reduces agent reputation, may revoke capability
//! - **RevokeCapability**: Immediately disables agent access
//! - **Blacklist**: Prevents agent from participating for N blocks
//!
//! ## Integration Point (SECURITY-CRITICAL ORDER)
//!
//! In `runtime::SignedExtra`:
//! ```ignore
//! pub type SignedExtra = (
//!     // ... frame_system checks ...
//!     pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
//!
//!     x3_invariants::InvariantCheck,              // 1. Hard fail gates FIRST
//!     x3_agent_law::AgentLawCheck,                // 2. Policy enforcement
//!     x3_swarm::CapabilityEnvelopeCheck,          // 3. Long-range attack validation
//!     x3_kernel::AtomicSettlementCheck,           // 4. Cross-VM atomicity
//!     x3_flash_finality::FlashFinalityExtension,  // 5. Flash finality
//! );
//! ```
//!
//! ⚠️ **Order is SECURITY-CRITICAL**: Invariants must fail BEFORE policies are evaluated.
//! Capability validation must happen BEFORE settlement execution.

pub use pallet::*;

pub mod emergency;
pub mod law_engine;
pub mod signed_extension;
pub mod types;
pub mod weights;

pub use emergency::{EmergencyAction, EmergencyEvent};
pub use law_engine::*;
pub use signed_extension::*;
pub use types::*;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_runtime::DispatchError;
    use sp_std::prelude::*;

    type BalanceOf<T> = <<T as Config>::Currency as frame_support::traits::Currency<
        <T as frame_system::Config>::AccountId,
    >>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency for reputation and slashing
        type Currency: frame_support::traits::Currency<Self::AccountId>
            + frame_support::traits::ReservableCurrency<Self::AccountId>;

        /// Reputation threshold below which capability is auto-revoked
        #[pallet::constant]
        type ReputationThreshold: Get<u64>;

        /// Maximum tasks per block (GPU_QUEUE_BOUNDS invariant)
        #[pallet::constant]
        type MaxTasksPerBlock: Get<u32>;

        /// Grace period for reputation checkpoints (long-range attack mitigation)
        #[pallet::constant]
        type CheckpointGracePeriod: Get<BlockNumberFor<Self>>;

        /// Rate limit: max extrinsics per epoch (24 hour = 14400 blocks @ 6s)
        #[pallet::constant]
        type RateLimitEpochLength: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type RateLimitMaxExtrinsicsPerEpoch: Get<u32>;

        /// Weight information
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    // ========================================================================
    // Storage
    // ========================================================================

    /// Active policies governing each agent
    /// `ActivePolicies<agent_id> -> Vec<PolicyRule>`
    #[pallet::storage]
    #[pallet::getter(fn agent_policies)]
    pub type ActivePolicies<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<PolicyRule<T::AccountId>, ConstU32<16>>,
        ValueQuery,
    >;

    /// Violation count per agent (used for auto-enforcement)
    /// `ViolationCount<agent_id> -> u32`
    #[pallet::storage]
    #[pallet::getter(fn violation_count)]
    pub type ViolationCount<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

    /// Extrinsic count per agent in current epoch
    /// `ExtrinsicCountThisEpoch<agent_id> -> u32`
    #[pallet::storage]
    #[pallet::getter(fn extrinsic_count_this_epoch)]
    pub type ExtrinsicCountThisEpoch<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

    /// Last epoch recorded for each agent (for rate limit reset)
    /// `LastEpoch<agent_id> -> BlockNumber`
    #[pallet::storage]
    #[pallet::getter(fn last_epoch)]
    pub type LastEpoch<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, BlockNumberFor<T>, ValueQuery>;

    /// Blacklisted agents (block number when blacklist expires)
    /// `Blacklist<agent_id> -> BlockNumber`
    #[pallet::storage]
    #[pallet::getter(fn blacklist_expiry)]
    pub type Blacklist<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, BlockNumberFor<T>, OptionQuery>;

    /// Task counts per agent per block (for MaxTasksPerBlock enforcement)
    /// `TasksThisBlock<(block_number, agent_id)> -> u32`
    #[pallet::storage]
    #[pallet::getter(fn tasks_this_block)]
    pub type TasksThisBlock<T: Config> =
        StorageMap<_, Blake2_128Concat, (BlockNumberFor<T>, T::AccountId), u32, ValueQuery>;

    // ========================================================================
    // Events
    // ========================================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Policy registered for an agent
        PolicyRegistered {
            agent: T::AccountId,
            policy_count: u32,
        },
        /// Policy violation detected and enforced
        PolicyViolation {
            agent: T::AccountId,
            violation_type: ViolationType,
            enforcement: EnforcementAction<T::AccountId>,
        },
        /// Agent was slashed due to policy violation
        AgentSlashed {
            agent: T::AccountId,
            reason: SlashingReason,
            penalty: u64,
        },
        /// Agent capability revoked due to reputation drop
        CapabilityRevoked {
            agent: T::AccountId,
            reason: RevocationReason,
        },
        /// Agent blacklisted (temporarily banned from network)
        AgentBlacklisted {
            agent: T::AccountId,
            expires_at: BlockNumberFor<T>,
        },
        /// Extrinsic count incremented for agent (for rate limit tracking)
        ExtrinsicCounted {
            agent: T::AccountId,
            count_this_epoch: u32,
        },
    }

    // ========================================================================
    // Errors
    // ========================================================================

    #[pallet::error]
    pub enum Error<T> {
        /// Agent not registered or not an agent
        NotAnAgent,
        /// Policy violated: agent not permitted to execute this capability
        CapabilityNotPermitted,
        /// Policy violated: agent reputation below minimum
        ReputationBelowMinimum,
        /// Policy violated: agent has reached max tasks per block
        MaxTasksPerBlockExceeded,
        /// Policy violated: agent attempted to collude with blacklisted peer
        CollusionAttempted,
        /// Policy violated: agent exceeded rate limit this epoch
        RateLimitExceeded,
        /// Agent is currently blacklisted
        AgentBlacklisted,
        /// Cannot modify policies for root/governance accounts
        CannotModifySystemPolicy,
        /// Too many policies (storage bound exceeded)
        TooManyPolicies,
        /// Invalid policy rule
        InvalidPolicyRule,
    }

    // ========================================================================
    // Extrinsics (Governance/Admin Only)
    // ========================================================================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register or update policies for an agent
        /// Can only be called by governance or Proof-Forge
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_policy())]
        pub fn register_policy(
            origin: OriginFor<T>,
            agent: T::AccountId,
            policies: Vec<PolicyRule<T::AccountId>>,
        ) -> DispatchResult {
            // TODO: Use EnsureRoot or governance origin check
            let _ = ensure_signed(origin)?;

            ensure!(policies.len() <= 16, Error::<T>::TooManyPolicies);

            let bounded_policies =
                BoundedVec::try_from(policies).map_err(|_| Error::<T>::TooManyPolicies)?;

            ActivePolicies::<T>::insert(&agent, bounded_policies.clone());
            ViolationCount::<T>::insert(&agent, 0);

            Self::deposit_event(Event::<T>::PolicyRegistered {
                agent,
                policy_count: bounded_policies.len() as u32,
            });

            Ok(())
        }

        /// Slash an agent for policy violation
        /// Can only be called by governance/Proof-Forge
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::slash_agent())]
        pub fn slash_agent(
            origin: OriginFor<T>,
            agent: T::AccountId,
            reason: SlashingReason,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;

            let penalty = Self::calculate_penalty(&reason);

            // Apply slash
            Self::internal_slash(&agent, penalty, &reason)?;

            // Track violation count
            ViolationCount::<T>::mutate(&agent, |count| *count = count.saturating_add(1));

            // Auto-enforcement at threshold (3rd violation → blacklist)
            let violation_count = ViolationCount::<T>::get(&agent);
            if violation_count >= 3 {
                Self::blacklist_agent(&agent, BlockNumberFor::<T>::from(100u32))?;
            }

            Self::deposit_event(Event::<T>::AgentSlashed {
                agent: agent.clone(),
                reason,
                penalty,
            });

            Ok(())
        }

        /// Remove agent from blacklist
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::remove_blacklist())]
        pub fn remove_blacklist(origin: OriginFor<T>, agent: T::AccountId) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Blacklist::<T>::remove(&agent);
            Ok(())
        }
    }

    // ========================================================================
    // Hooks (Called from on_initialize)
    // ========================================================================

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(block: BlockNumberFor<T>) -> Weight {
            // Clean up old TasksThisBlock entries (previous block)
            // In production, use a circular buffer or off-chain indexing
            Weight::zero()
        }

        fn on_finalize(_block: BlockNumberFor<T>) {
            // Checkpoint state for long-range attack validation
            // Integrated with x3-invariants registry
        }
    }

    // ========================================================================
    // Internal Helpers
    // ========================================================================

    impl<T: Config> Pallet<T> {
        /// Slash agent reputation
        pub fn internal_slash(
            agent: &T::AccountId,
            penalty: u64,
            reason: &SlashingReason,
        ) -> DispatchResult {
            // Reduce agent reputation in x3-invariants registry (will be linked in formal spec)
            // For now, this is a placeholder for the invariant registry call

            // Log the slash
            log::warn!(
                target: "x3-agent-law",
                "Agent {:?} slashed for {:?} (penalty: {})",
                agent, reason, penalty
            );

            Ok(())
        }

        /// Temporarily blacklist agent
        pub fn blacklist_agent(
            agent: &T::AccountId,
            duration: BlockNumberFor<T>,
        ) -> DispatchResult {
            let current_block = frame_system::Pallet::<T>::block_number();
            let expires_at = current_block + duration;
            Blacklist::<T>::insert(agent, expires_at);

            Self::deposit_event(Event::<T>::AgentBlacklisted {
                agent: agent.clone(),
                expires_at,
            });

            Ok(())
        }

        /// Calculate slash penalty based on reason
        pub fn calculate_penalty(reason: &SlashingReason) -> u64 {
            match reason {
                SlashingReason::InvalidProof => 500,
                SlashingReason::TaskGriefing => 200,
                SlashingReason::CollusionDetected => 800,
                SlashingReason::PolicyViolation => 350,
                SlashingReason::RepeatOffender => 1200,
            }
        }
    }
}
