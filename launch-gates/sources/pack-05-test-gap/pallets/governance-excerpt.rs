#![deny(unsafe_code)]
//! # X3Chain Governance Pallet
//!
//! A comprehensive governance system for the X3 blockchain supporting:
//! - Configurable voting logic (quorum, thresholds, conviction)
//! - Proposal lifecycle management
//! - Delegation with conviction multipliers
//! - Referendum mechanics
//! - Runtime upgrade hooks
//!
//! ## Overview
//!
//! The governance pallet enables decentralized decision-making through proposals
//! and referendums. Token holders can vote directly or delegate their voting power
//! to trusted representatives with configurable conviction levels.
//!
//! ## Conviction Voting
//!
//! Conviction allows voters to lock tokens for longer periods to amplify voting power:
//! - None (0x): 0.1x voting power, no lock
//! - Locked1x: 1x voting power, 1 period lock
//! - Locked2x: 2x voting power, 2 period lock
//! - Locked3x: 3x voting power, 4 period lock
//! - Locked4x: 4x voting power, 8 period lock
//! - Locked5x: 5x voting power, 16 period lock
//! - Locked6x: 6x voting power, 32 period lock

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

pub mod types;
pub use types::*;

pub mod runtime_api;
pub use runtime_api::*;

pub(crate) mod migrations;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
        pallet_prelude::*,
        traits::{
            schedule::Named as ScheduleNamed, Currency, LockableCurrency, ReservableCurrency,
        },
        Blake2_128Concat,
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::{
        traits::{Saturating, Zero},
        DispatchError, Percent,
    };
    use sp_std::prelude::*;

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// Type alias for Proposal to reduce complexity in storage definition
    type ProposalOf<T> = Proposal<
        <T as frame_system::Config>::AccountId,
        BalanceOf<T>,
        BlockNumberFor<T>,
        <T as Config>::RuntimeCall,
    >;

    use frame_support::traits::StorageVersion;

    pub(crate) const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::without_storage_info]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The overarching call type.
        type RuntimeCall: Parameter
            + From<Call<Self>>
            + GetDispatchInfo
            + IsType<<Self as frame_system::Config>::RuntimeCall>
            + Dispatchable<RuntimeOrigin = Self::RuntimeOrigin, PostInfo = PostDispatchInfo>;

        /// Currency type for voting and deposits.
        type Currency: ReservableCurrency<Self::AccountId>
            + LockableCurrency<Self::AccountId, Moment = BlockNumberFor<Self>>;

        /// Origin that can submit proposals.
        type SubmitOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;

        /// Origin that can fast-track proposals (e.g., technical committee).
        type FastTrackOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Origin that can cancel proposals in emergency.
        type CancelOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Origin for runtime upgrades.
        type RuntimeUpgradeOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Scheduler for enacting approved proposals.
        type Scheduler: ScheduleNamed<
            BlockNumberFor<Self>,
            <Self as Config>::RuntimeCall,
            Self::PalletsOrigin,
        >;

        /// The Scheduler's pallet origin type.
        type PalletsOrigin: From<frame_system::RawOrigin<Self::AccountId>>;

        /// Minimum deposit required to submit a proposal.
        #[pallet::constant]
        type ProposalDeposit: Get<BalanceOf<Self>>;

        /// Duration of the voting period in blocks.
        #[pallet::constant]
        type VotingPeriod: Get<BlockNumberFor<Self>>;

        /// Duration of the enactment delay after approval.
        #[pallet::constant]
        type EnactmentPeriod: Get<BlockNumberFor<Self>>;

        /// Minimum percentage of total issuance that must vote for quorum.
        #[pallet::constant]
        type Quorum: Get<Percent>;

        /// Approval threshold as percentage of votes.
        #[pallet::constant]
        type ApprovalThreshold: Get<Percent>;

        /// Maximum number of proposals that can exist at once.
        #[pallet::constant]
        type MaxProposals: Get<u32>;

        /// Maximum number of votes per account.
        #[pallet::constant]
        type MaxVotes: Get<u32>;

        /// Maximum delegations per account.
        #[pallet::constant]
        type MaxDelegations: Get<u32>;

        /// Lock period multiplier for conviction voting (in blocks).
        #[pallet::constant]
        type ConvictionPeriod: Get<BlockNumberFor<Self>>;

        /// Weight information for extrinsics.
        type WeightInfo: WeightInfo;

        // ============================================================================
        // AI Governance Configuration
        // ============================================================================

        /// Maximum AI proposal payload size
        #[pallet::constant]
        type MaxAIProposalPayload: Get<u32>;

        /// Origin for AI proposal submission (AI agents)
        type AISubmitOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;

        /// Origin for AI proposal review (human reviewers)
        type AIReviewOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;

        /// Origin for emergency kill switch activation
        type EmergencyOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;
    }

    // ========================================================================
    // Storage Items
    // ========================================================================

    /// Counter for proposal IDs.
    #[pallet::storage]
    #[pallet::getter(fn proposal_count)]
    pub type ProposalCount<T> = StorageValue<_, u32, ValueQuery>;

    /// All proposals.
    #[pallet::storage]
    #[pallet::getter(fn proposals)]
    pub type Proposals<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, ProposalOf<T>, OptionQuery>;

    /// Votes for each proposal.
    #[pallet::storage]
    #[pallet::getter(fn proposal_votes)]
    pub type ProposalVotes<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, ProposalTally<BalanceOf<T>>, ValueQuery>;

    /// Individual votes per account per proposal.
    #[pallet::storage]
    #[pallet::getter(fn voting)]
    pub type Voting<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        u32,
        Vote<BalanceOf<T>>,
        OptionQuery,
    >;

    /// Delegation relationships.
    #[pallet::storage]
    #[pallet::getter(fn delegations)]
    pub type Delegations<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Delegation<T::AccountId, BalanceOf<T>>,
        OptionQuery,
    >;

    /// Accounts that have delegated to a specific target.
    #[pallet::storage]
    #[pallet::getter(fn delegators)]
    pub type Delegators<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<T::AccountId, T::MaxDelegations>,
        ValueQuery,
    >;

    /// Token locks for conviction voting.
    #[pallet::storage]
    #[pallet::getter(fn locks)]
    pub type Locks<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<VoteLock<BalanceOf<T>, BlockNumberFor<T>>, T::MaxVotes>,
        ValueQuery,
    >;

    /// Approved referendums pending enactment.
    #[pallet::storage]
    #[pallet::getter(fn pending_enactments)]
    pub type PendingEnactments<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<u32, T::MaxProposals>,
        ValueQuery,
    >;

    /// Governance configuration (can be updated via governance).
    #[pallet::storage]
    #[pallet::getter(fn config)]
    pub type GovernanceConfig<T: Config> =
        StorageValue<_, GovernanceParams<BalanceOf<T>, BlockNumberFor<T>>, ValueQuery>;

    // ============================================================================
    // AI Governance Storage
    // ============================================================================

    /// AI Governance configuration
    #[pallet::storage]
    #[pallet::getter(fn ai_config)]
    pub type AIConfig<T: Config> = StorageValue<_, AIGovernanceConfig, ValueQuery>;

    /// AI proposals
    #[pallet::storage]
    #[pallet::getter(fn ai_proposals)]
    pub type AIProposals<T: Config> =
        StorageMap<_, Blake2_128Concat, u64, AIProposal<T>, OptionQuery>;

    /// Next AI proposal ID
    #[pallet::storage]
    pub type NextAIProposalId<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Simulation results for AI proposals
    #[pallet::storage]
    #[pallet::getter(fn simulation_results)]
    pub type SimulationResults<T: Config> =
        StorageMap<_, Blake2_128Concat, u64, SimulationResult, OptionQuery>;

    /// Authorization tracking for AI proposals
    #[pallet::storage]
    #[pallet::getter(fn ai_authorizations)]
    pub type AIAuthorizations<T: Config> =
        StorageMap<_, Blake2_128Concat, u64, AuthorizationRequirements<T>, OptionQuery>;

    /// Sandboxed executions
    #[pallet::storage]
    #[pallet::getter(fn sandboxed_executions)]
    pub type SandboxedExecutions<T: Config> =
        StorageMap<_, Blake2_128Concat, u64, SandboxedExecution, OptionQuery>;

    /// Current kill switch level
    #[pallet::storage]
    #[pallet::getter(fn kill_switch_level)]
    pub type KillSwitchLevelStorage<T: Config> = StorageValue<_, KillSwitchLevel, ValueQuery>;

    /// Kill switch activation history
    #[pallet::storage]
    #[pallet::getter(fn kill_switch_history)]
    pub type KillSwitchHistory<T: Config> =
        StorageMap<_, Blake2_128Concat, BlockNumberFor<T>, KillSwitchActivation<T>, OptionQuery>;

    /// AI reviewers (authorized human reviewers)
    #[pallet::storage]
    pub type AIReviewers<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

    /// AI proposal approvals (proposal_id => reviewer => approved)
    #[pallet::storage]
    pub type AIProposalApprovals<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        u64,
        Blake2_128Concat,
        T::AccountId,
        bool,
        ValueQuery,
    >;

    /// AI execution authorization approvals (proposal_id => emergency signer => approved)
    #[pallet::storage]
    pub type AIExecutionApprovals<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        u64,
        Blake2_128Concat,
        T::AccountId,
        bool,
        ValueQuery,
    >;

    // ========================================================================
    // Events
    // ========================================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new proposal was submitted.
        ProposalSubmitted {
            proposal_id: u32,
            proposer: T::AccountId,
            deposit: BalanceOf<T>,
        },
        /// A vote was cast.
        Voted {
            voter: T::AccountId,
            proposal_id: u32,
            vote: VoteDirection,
            balance: BalanceOf<T>,
            conviction: Conviction,
        },
        /// A proposal was approved.
        ProposalApproved {
            proposal_id: u32,
            ayes: BalanceOf<T>,
            nays: BalanceOf<T>,
        },
        /// A proposal was rejected.
        ProposalRejected {
            proposal_id: u32,
            ayes: BalanceOf<T>,
            nays: BalanceOf<T>,
        },
        /// A proposal was enacted.
        ProposalEnacted {
            proposal_id: u32,
            result: DispatchResult,
        },
        /// A proposal was cancelled.
        ProposalCancelled { proposal_id: u32 },
        /// Voting delegation was set.
        Delegated {
            delegator: T::AccountId,
            target: T::AccountId,
            conviction: Conviction,
        },
        /// Delegation was removed.
        Undelegated { delegator: T::AccountId },
        /// A proposal was fast-tracked.
        FastTracked {
            proposal_id: u32,
            voting_period: BlockNumberFor<T>,
        },
        /// Governance parameters were updated.
        ConfigUpdated { quorum: Percent, threshold: Percent },
        /// Tokens were unlocked after conviction period.
        TokensUnlocked {
            account: T::AccountId,
            amount: BalanceOf<T>,
        },

        // ============================================================================
        // AI Governance Events
        // ============================================================================
        /// AI proposal submitted
        AIProposalSubmitted {
            proposal_id: u64,
            proposer: T::AccountId,
            proposal_type: AIProposalType,
        },
        /// AI proposal simulation completed
        AIProposalSimulated {
            proposal_id: u64,
            success: bool,
            gas_used: u64,
        },
        /// AI proposal approved by reviewer
        AIProposalApproved {
            proposal_id: u64,
            reviewer: T::AccountId,
            total_approvals: u32,
        },
        /// AI proposal authorized for execution
        AIProposalAuthorized {
            proposal_id: u64,
            execution_block: BlockNumberFor<T>,
        },
        /// AI proposal executed in sandbox
        AIProposalExecuted { proposal_id: u64, success: bool },
        /// AI proposal rolled back
        AIProposalRolledBack {
            proposal_id: u64,
            reason: BoundedVec<u8, ConstU32<256>>,
        },
        /// Kill switch activated
        KillSwitchActivated {
            level: KillSwitchLevel,
            activator: T::AccountId,
            reason: BoundedVec<u8, ConstU32<512>>,
        },
        /// Kill switch deactivated
        KillSwitchDeactivated {
            previous_level: KillSwitchLevel,
            new_level: KillSwitchLevel,
        },
        /// AI reviewer registered
        AIReviewerRegistered { reviewer: T::AccountId },
        /// AI governance config updated
        AIConfigUpdated,
    }

    // ========================================================================
    // Errors
    // ========================================================================

    #[pallet::error]
    pub enum Error<T> {
        /// Proposal not found.
        ProposalNotFound,
        /// Proposal already exists.
        ProposalAlreadyExists,
        /// Maximum proposals reached.
        TooManyProposals,
        /// Insufficient deposit for proposal.
        InsufficientDeposit,
        /// Voting period has ended.
        VotingEnded,
        /// Voting period has not ended.
        VotingNotEnded,
        /// Already voted on this proposal.
        AlreadyVoted,
        /// Account has not voted.
        NotVoted,
        /// Account has delegated votes.
        AlreadyDelegated,
        /// Cannot delegate to self.
        SelfDelegation,
        /// Circular delegation detected.
        CircularDelegation,
        /// Maximum delegations reached.
        TooManyDelegations,
        /// Maximum votes reached.
        TooManyVotes,
        /// Proposal not in voting state.
        NotInVoting,
        /// Quorum not reached.
        QuorumNotReached,
        /// Proposal already enacted.
        AlreadyEnacted,
        /// Insufficient balance for vote.
        InsufficientBalance,
        /// Tokens are locked.
        TokensLocked,
        /// Invalid conviction value.
        InvalidConviction,
