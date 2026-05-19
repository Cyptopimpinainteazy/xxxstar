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
        /// Proposal is not approved.
        NotApproved,
        /// Cannot cancel active proposal.
        CannotCancel,
        /// Arithmetic overflow.
        ArithmeticOverflow,

        // ============================================================================
        // AI Governance Errors
        // ============================================================================
        /// AI proposal not found
        AIProposalNotFound,
        /// AI proposal not in expected status
        AIProposalInvalidStatus,
        /// Not authorized to submit AI proposals
        NotAIAuthorized,
        /// Not an AI reviewer
        NotAIReviewer,
        /// Simulation failed
        SimulationFailed,
        /// Authorization requirements not met
        AuthorizationFailed,
        /// Sandbox execution failed
        SandboxExecutionFailed,
        /// Kill switch level invalid
        InvalidKillSwitchLevel,
        /// Emergency mode active
        EmergencyModeActive,
        /// AI proposal payload too large
        AIProposalPayloadTooLarge,
        /// Emergency signer has already approved this AI execution
        AIExecutionAlreadyApproved,

        // ====================================================================
        // Constitutional proof gate errors (vΩ-1.0)
        // ====================================================================
        /// Proposal touches a constitutional invariant but carries no proof commitment.
        /// Per Article IV: voting is necessary but not sufficient. Proof is required.
        ProofRequiredForInvariantProposal,
        /// Proposal was authored against a superseded constitution hash.
        /// Must be re-submitted against the current constitution.
        ConstitutionHashMismatch,
    }

    // ========================================================================
    // Genesis Config
    // ========================================================================

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        pub _phantom: PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            // Initialize default governance config
            GovernanceConfig::<T>::put(GovernanceParams {
                quorum: T::Quorum::get(),
                approval_threshold: T::ApprovalThreshold::get(),
                voting_period: T::VotingPeriod::get(),
                enactment_period: T::EnactmentPeriod::get(),
                proposal_deposit: T::ProposalDeposit::get(),
            });

            // Initialize AI governance config
            AIConfig::<T>::put(AIGovernanceConfig::default());

            // Initialize kill switch to normal
            KillSwitchLevelStorage::<T>::put(KillSwitchLevel::Normal);
        }
    }

    // ========================================================================
    // Hooks
    // ========================================================================

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: BlockNumberFor<T>) -> Weight {
            let mut weight = Weight::zero();

            // Process pending enactments
            let enactments = PendingEnactments::<T>::take(n);
            for proposal_id in enactments.iter() {
                weight = weight.saturating_add(Self::enact_proposal(*proposal_id));
            }

            // Process expired proposals
            weight = weight.saturating_add(Self::process_expired_proposals(n));

            weight
        }
    }

    // ========================================================================
    // Extrinsics
    // ========================================================================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Submit a new governance proposal.
        ///
        /// The proposer must deposit `ProposalDeposit` which is returned if the
        /// proposal is approved or slashed if rejected.
        ///
        /// Per Constitution vΩ-1.0 Article IV: proposals that touch constitutional
        /// invariants (`touches_invariants = true`) MUST include a non-zero
        /// `proof_commitment`. Voting alone is not sufficient for execution.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::submit_proposal())]
        pub fn submit_proposal(
            origin: OriginFor<T>,
            call: Box<<T as Config>::RuntimeCall>,
            title: BoundedVec<u8, ConstU32<256>>,
            description: BoundedVec<u8, ConstU32<4096>>,
            touches_invariants: bool,
            proof_commitment: Option<[u8; 32]>,
            constitution_hash: Option<[u8; 32]>,
        ) -> DispatchResult {
            let proposer = T::SubmitOrigin::ensure_origin(origin)?;

            // Constitutional proof gate (Article IV): invariant-touching proposals
            // must carry a non-zero proof commitment at submission time.
            if touches_invariants {
                let has_proof = proof_commitment.map(|c| c != [0u8; 32]).unwrap_or(false);
                ensure!(has_proof, Error::<T>::ProofRequiredForInvariantProposal);
            }

            let config = GovernanceConfig::<T>::get();
            let deposit = config.proposal_deposit;

            // Reserve deposit
            T::Currency::reserve(&proposer, deposit)?;

            let proposal_id = ProposalCount::<T>::get();
            ensure!(
                proposal_id < T::MaxProposals::get(),
                Error::<T>::TooManyProposals
            );

            let current_block = frame_system::Pallet::<T>::block_number();
            let voting_end = current_block.saturating_add(config.voting_period);

            let proposal = Proposal {
                id: proposal_id,
                proposer: proposer.clone(),
                call: *call,
                title,
                description,
                deposit,
                status: ProposalStatus::Voting,
                submitted_at: current_block,
                voting_end,
                enacted_at: None,
                proof_commitment,
                constitution_hash,
                touches_invariants,
            };

            Proposals::<T>::insert(proposal_id, proposal);
            ProposalVotes::<T>::insert(proposal_id, ProposalTally::default());
            ProposalCount::<T>::put(proposal_id.saturating_add(1));

            Self::deposit_event(Event::ProposalSubmitted {
                proposal_id,
                proposer,
                deposit,
            });

            Ok(())
        }

        /// Cast a vote on an active proposal.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::vote())]
        pub fn vote(
            origin: OriginFor<T>,
            proposal_id: u32,
            direction: VoteDirection,
            balance: BalanceOf<T>,
            conviction: Conviction,
        ) -> DispatchResult {
            let voter = ensure_signed(origin)?;

            // Check voter is not delegating
            ensure!(
                !Delegations::<T>::contains_key(&voter),
                Error::<T>::AlreadyDelegated
            );

            let proposal = Proposals::<T>::get(proposal_id).ok_or(Error::<T>::ProposalNotFound)?;

            ensure!(
                proposal.status == ProposalStatus::Voting,
                Error::<T>::NotInVoting
            );

            let current_block = frame_system::Pallet::<T>::block_number();
            ensure!(
                current_block <= proposal.voting_end,
                Error::<T>::VotingEnded
            );

            // Check balance
            let free_balance = T::Currency::free_balance(&voter);
            ensure!(balance <= free_balance, Error::<T>::InsufficientBalance);

            // Calculate voting power with conviction
            let (mul_num, mul_denom) = conviction.multiplier();
            let voting_power = balance.saturating_mul(mul_num.into()) / mul_denom.into();

            // Add delegated voting power
            let delegated_power = Self::get_delegated_power(&voter, balance, conviction)?;
            let total_power = voting_power.saturating_add(delegated_power);

            // Remove previous vote if exists
            if let Some(prev_vote) = Voting::<T>::get(&voter, proposal_id) {
                Self::remove_vote_from_tally(proposal_id, &prev_vote)?;
            }

            // Record vote
            let vote = Vote {
                direction,
                balance,
                conviction,
                voting_power: total_power,
            };
            Voting::<T>::insert(&voter, proposal_id, vote.clone());

            // Update tally
            ProposalVotes::<T>::try_mutate(proposal_id, |tally| -> DispatchResult {
                match direction {
                    VoteDirection::Aye => {
                        tally.ayes = tally.ayes.saturating_add(total_power);
                        tally.aye_voters = tally.aye_voters.saturating_add(1);
                    }
                    VoteDirection::Nay => {
                        tally.nays = tally.nays.saturating_add(total_power);
                        tally.nay_voters = tally.nay_voters.saturating_add(1);
                    }
                    VoteDirection::Abstain => {
                        tally.abstains = tally.abstains.saturating_add(total_power);
                    }
                }
                tally.turnout = tally.turnout.saturating_add(balance);
                Ok(())
            })?;

            // Create conviction lock if needed
            if conviction != Conviction::None {
                Self::create_conviction_lock(&voter, balance, conviction)?;
            }

            Self::deposit_event(Event::Voted {
                voter,
                proposal_id,
                vote: direction,
                balance,
                conviction,
            });

            Ok(())
        }

        /// Delegate voting power to another account.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::delegate())]
        pub fn delegate(
            origin: OriginFor<T>,
            target: T::AccountId,
            conviction: Conviction,
        ) -> DispatchResult {
            let delegator = ensure_signed(origin)?;

            ensure!(delegator != target, Error::<T>::SelfDelegation);

            // Check for circular delegation
            ensure!(
                !Self::would_create_cycle(&delegator, &target),
                Error::<T>::CircularDelegation
            );

            // Remove existing delegation
            if Delegations::<T>::contains_key(&delegator) {
                Self::remove_delegation(&delegator)?;
            }

            let balance = T::Currency::free_balance(&delegator);

            let delegation = Delegation {
                target: target.clone(),
                conviction,
                balance,
            };

            Delegations::<T>::insert(&delegator, delegation);

            // Add to target's delegators list
            Delegators::<T>::try_mutate(&target, |delegators| -> DispatchResult {
                delegators
                    .try_push(delegator.clone())
                    .map_err(|_| Error::<T>::TooManyDelegations)?;
                Ok(())
            })?;

            Self::deposit_event(Event::Delegated {
                delegator,
                target,
                conviction,
            });

            Ok(())
        }

        /// Remove delegation.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::undelegate())]
        pub fn undelegate(origin: OriginFor<T>) -> DispatchResult {
            let delegator = ensure_signed(origin)?;

            Self::remove_delegation(&delegator)?;

            Self::deposit_event(Event::Undelegated { delegator });

            Ok(())
        }

        /// Fast-track a proposal (requires FastTrackOrigin).
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::fast_track())]
        pub fn fast_track(
            origin: OriginFor<T>,
            proposal_id: u32,
            voting_period: BlockNumberFor<T>,
        ) -> DispatchResult {
            T::FastTrackOrigin::ensure_origin(origin)?;

            Proposals::<T>::try_mutate(proposal_id, |maybe_proposal| -> DispatchResult {
                let proposal = maybe_proposal
                    .as_mut()
                    .ok_or(Error::<T>::ProposalNotFound)?;
                ensure!(
                    proposal.status == ProposalStatus::Voting,
                    Error::<T>::NotInVoting
                );

                let current_block = frame_system::Pallet::<T>::block_number();
                proposal.voting_end = current_block.saturating_add(voting_period);

                Ok(())
            })?;

            Self::deposit_event(Event::FastTracked {
                proposal_id,
                voting_period,
            });

            Ok(())
        }

        /// Cancel a proposal (requires CancelOrigin).
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::cancel_proposal())]
        pub fn cancel_proposal(origin: OriginFor<T>, proposal_id: u32) -> DispatchResult {
            T::CancelOrigin::ensure_origin(origin)?;

            let proposal = Proposals::<T>::get(proposal_id).ok_or(Error::<T>::ProposalNotFound)?;

            // Slash deposit (ignore imbalance - slashed funds handled by runtime)
            let _ = T::Currency::slash_reserved(&proposal.proposer, proposal.deposit);

            // Clean up
            Proposals::<T>::remove(proposal_id);
            ProposalVotes::<T>::remove(proposal_id);

            Self::deposit_event(Event::ProposalCancelled { proposal_id });

            Ok(())
        }

        /// Finalize voting and determine outcome.
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::finalize_proposal())]
        pub fn finalize_proposal(origin: OriginFor<T>, proposal_id: u32) -> DispatchResult {
            ensure_signed(origin)?;

            let mut proposal =
                Proposals::<T>::get(proposal_id).ok_or(Error::<T>::ProposalNotFound)?;

            ensure!(
                proposal.status == ProposalStatus::Voting,
                Error::<T>::NotInVoting
            );

            let current_block = frame_system::Pallet::<T>::block_number();
            ensure!(
                current_block > proposal.voting_end,
                Error::<T>::VotingNotEnded
            );

            let tally = ProposalVotes::<T>::get(proposal_id);
            let config = GovernanceConfig::<T>::get();

            // Check quorum
            let total_issuance = T::Currency::total_issuance();
            let quorum_threshold = config.quorum.mul_floor(total_issuance);
            let quorum_met = tally.turnout >= quorum_threshold;

            // Check approval threshold
            let total_votes = tally.ayes.saturating_add(tally.nays);
            let approved = if total_votes > Zero::zero() && quorum_met {
                let approval_ratio = Percent::from_rational(tally.ayes, total_votes);
                approval_ratio >= config.approval_threshold
            } else {
                false
            };

            if approved {
                proposal.status = ProposalStatus::Approved;

                // Schedule enactment
                let enact_at = current_block.saturating_add(config.enactment_period);
                PendingEnactments::<T>::try_mutate(enact_at, |proposals| -> DispatchResult {
                    proposals
                        .try_push(proposal_id)
                        .map_err(|_| Error::<T>::TooManyProposals)?;
                    Ok(())
                })?;

                // Return deposit
                T::Currency::unreserve(&proposal.proposer, proposal.deposit);

                Self::deposit_event(Event::ProposalApproved {
                    proposal_id,
                    ayes: tally.ayes,
                    nays: tally.nays,
                });
            } else {
                proposal.status = ProposalStatus::Rejected;

                // Slash deposit (or return partial based on participation)
                if quorum_met {
                    // Full slash if quorum met but rejected (ignore imbalance)
                    let _ = T::Currency::slash_reserved(&proposal.proposer, proposal.deposit);
                } else {
                    // Return deposit if quorum not met
                    T::Currency::unreserve(&proposal.proposer, proposal.deposit);
                }

                Self::deposit_event(Event::ProposalRejected {
                    proposal_id,
                    ayes: tally.ayes,
                    nays: tally.nays,
                });
            }

            Proposals::<T>::insert(proposal_id, proposal);

            Ok(())
        }

        /// Unlock tokens after conviction period expires.
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::unlock())]
        pub fn unlock(origin: OriginFor<T>, account: T::AccountId) -> DispatchResult {
            ensure_signed(origin)?;

            let current_block = frame_system::Pallet::<T>::block_number();
            let mut unlocked_amount = BalanceOf::<T>::zero();

            Locks::<T>::try_mutate(&account, |locks| -> DispatchResult {
                locks.retain(|lock| {
                    if current_block >= lock.unlock_at {
                        unlocked_amount = unlocked_amount.saturating_add(lock.amount);
                        false
                    } else {
                        true
                    }
                });
                Ok(())
            })?;

            if unlocked_amount > Zero::zero() {
                Self::deposit_event(Event::TokensUnlocked {
                    account,
                    amount: unlocked_amount,
                });
            }

            Ok(())
        }

        /// Update governance configuration (via governance).
        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::update_config())]
        pub fn update_config(
            origin: OriginFor<T>,
            new_quorum: Option<Percent>,
            new_threshold: Option<Percent>,
            new_voting_period: Option<BlockNumberFor<T>>,
            new_enactment_period: Option<BlockNumberFor<T>>,
        ) -> DispatchResult {
            T::RuntimeUpgradeOrigin::ensure_origin(origin)?;

            GovernanceConfig::<T>::mutate(|config| {
                if let Some(q) = new_quorum {
                    config.quorum = q;
                }
                if let Some(t) = new_threshold {
                    config.approval_threshold = t;
                }
                if let Some(v) = new_voting_period {
                    config.voting_period = v;
                }
                if let Some(e) = new_enactment_period {
                    config.enactment_period = e;
                }
            });

            let config = GovernanceConfig::<T>::get();
            Self::deposit_event(Event::ConfigUpdated {
                quorum: config.quorum,
                threshold: config.approval_threshold,
            });

            Ok(())
        }

        // ============================================================================
        // AI Governance Extrinsics
        // ============================================================================

        /// Submit an AI proposal (inert object, no direct execution)
        #[pallet::call_index(9)]
        #[pallet::weight(T::WeightInfo::submit_proposal())]
        pub fn submit_ai_proposal(
            origin: OriginFor<T>,
            proposal_type: AIProposalType,
            payload: BoundedVec<u8, T::MaxAIProposalPayload>,
            impact_assessment: ImpactAssessment,
            simulation_requirements: SimulationRequirements,
        ) -> DispatchResult {
            let proposer = T::AISubmitOrigin::ensure_origin(origin)?;
            ensure!(
                payload.len() <= T::MaxAIProposalPayload::get() as usize,
                Error::<T>::AIProposalPayloadTooLarge
            );

            let proposal_id = NextAIProposalId::<T>::get();
            let current_block = frame_system::Pallet::<T>::block_number();

            let proposal = AIProposal {
                id: proposal_id,
                proposer: proposer.clone(),
                proposal_type: proposal_type.clone(),
                payload,
                impact_assessment,
                simulation_requirements,
                proposed_at: current_block,
                status: AIProposalStatus::Proposed,
            };

            AIProposals::<T>::insert(proposal_id, proposal);
            NextAIProposalId::<T>::put(proposal_id + 1);

            Self::deposit_event(Event::AIProposalSubmitted {
                proposal_id,
                proposer,
                proposal_type,
            });

            Ok(())
        }

        /// Approve an AI proposal (reviewer action)
        #[pallet::call_index(10)]
        #[pallet::weight(T::WeightInfo::vote())]
        pub fn approve_ai_proposal(origin: OriginFor<T>, proposal_id: u64) -> DispatchResult {
            let reviewer = T::AIReviewOrigin::ensure_origin(origin)?;
            ensure!(AIReviewers::<T>::get(&reviewer), Error::<T>::NotAIReviewer);

            AIProposals::<T>::try_mutate(proposal_id, |maybe_proposal| {
                let proposal = maybe_proposal
                    .as_mut()
                    .ok_or(Error::<T>::AIProposalNotFound)?;
                ensure!(
                    proposal.status == AIProposalStatus::UnderReview,
                    Error::<T>::AIProposalInvalidStatus
                );

                // Record approval
                AIProposalApprovals::<T>::insert(proposal_id, &reviewer, true);

                // Count total approvals
                let approvals = AIProposalApprovals::<T>::iter_prefix(proposal_id).count() as u32;
                let config = AIConfig::<T>::get();

                Self::deposit_event(Event::AIProposalApproved {
                    proposal_id,
                    reviewer,
                    total_approvals: approvals,
                });

                // Check if we have enough approvals for simulation
                if approvals >= config.min_reviewer_approvals {
                    proposal.status = AIProposalStatus::Approved;
                    // Trigger simulation
                    Self::simulate_ai_proposal(proposal_id)?;
                }

                Ok(())
            })
        }

        /// Authorize AI proposal for execution (multisig + time-lock)
        #[pallet::call_index(11)]
        #[pallet::weight(T::WeightInfo::vote())]
        pub fn authorize_ai_proposal(origin: OriginFor<T>, proposal_id: u64) -> DispatchResult {
            let signer = T::EmergencyOrigin::ensure_origin(origin)?; // High authorization required

            AIProposals::<T>::try_mutate(proposal_id, |maybe_proposal| {
                let proposal = maybe_proposal
                    .as_mut()
                    .ok_or(Error::<T>::AIProposalNotFound)?;
                ensure!(
                    proposal.status == AIProposalStatus::SimulationPassed
                        || proposal.status == AIProposalStatus::Approved,
                    Error::<T>::AIProposalInvalidStatus
                );

                let current_block = frame_system::Pallet::<T>::block_number();
                let config = AIConfig::<T>::get();
                let execution_block = current_block.saturating_add(config.default_time_lock.into());

                // Set up authorization requirements
                let auth_reqs =
                    AIAuthorizations::<T>::get(proposal_id).unwrap_or(AuthorizationRequirements {
                        multisig_threshold: 3, // Require 3 signatures
                        time_lock_blocks: config.default_time_lock.into(),
                        reviewer_approvals: config.min_reviewer_approvals,
                    });

                ensure!(
                    !AIExecutionApprovals::<T>::get(proposal_id, &signer),
                    Error::<T>::AIExecutionAlreadyApproved
                );
                AIExecutionApprovals::<T>::insert(proposal_id, &signer, true);
                AIAuthorizations::<T>::insert(proposal_id, auth_reqs);

                // Emit authorization event on first transition into executable state.
                if proposal.status == AIProposalStatus::SimulationPassed {
                    proposal.status = AIProposalStatus::Approved;
                    Self::deposit_event(Event::AIProposalAuthorized {
                        proposal_id,
                        execution_block,
                    });
                }

                Ok(())
            })
        }

        /// Execute AI proposal in sandbox
        #[pallet::call_index(12)]
        #[pallet::weight(T::WeightInfo::submit_proposal())]
        pub fn execute_ai_proposal(origin: OriginFor<T>, proposal_id: u64) -> DispatchResult {
            T::EmergencyOrigin::ensure_origin(origin)?; // High authorization required

            let proposal =
                AIProposals::<T>::get(proposal_id).ok_or(Error::<T>::AIProposalNotFound)?;
            ensure!(
                proposal.status == AIProposalStatus::Approved,
                Error::<T>::AIProposalInvalidStatus
            );

            // Check time lock
            let current_block = frame_system::Pallet::<T>::block_number();
            let auth_reqs =
                AIAuthorizations::<T>::get(proposal_id).ok_or(Error::<T>::AuthorizationFailed)?;
            let execution_approvals = AIExecutionApprovals::<T>::iter_prefix(proposal_id)
                .filter(|(_, approved)| *approved)
                .count() as u32;
            ensure!(
                execution_approvals >= auth_reqs.multisig_threshold,
                Error::<T>::AuthorizationFailed
            );
            ensure!(
                current_block
                    >= proposal
                        .proposed_at
                        .saturating_add(auth_reqs.time_lock_blocks),
                Error::<T>::AuthorizationFailed
            );

            // Check kill switch level
            ensure!(
                KillSwitchLevelStorage::<T>::get() < KillSwitchLevel::UpgradeFreeze,
                Error::<T>::EmergencyModeActive
            );

            // Create sandboxed execution
            let sandbox = SandboxedExecution {
                gas_ceiling: proposal.simulation_requirements.gas_limit,
                block_limit: proposal.simulation_requirements.simulation_blocks,
                rollback_checkpoint: Self::create_rollback_checkpoint(),
                status: ExecutionStatus::Executing,
            };

            SandboxedExecutions::<T>::insert(proposal_id, sandbox);

            // Execute in sandbox (simulated for now)
            let success = Self::execute_in_sandbox(&proposal);

            AIProposals::<T>::try_mutate(proposal_id, |p| {
                if let Some(prop) = p {
                    prop.status = if success {
                        AIProposalStatus::Executed
                    } else {
                        AIProposalStatus::Rejected
                    };
                }
                Ok::<(), Error<T>>(())
            })?;

            AIAuthorizations::<T>::remove(proposal_id);
            let execution_signers: Vec<T::AccountId> =
                AIExecutionApprovals::<T>::iter_prefix(proposal_id)
                    .map(|(signer, _)| signer)
                    .collect();
            for signer in execution_signers {
                AIExecutionApprovals::<T>::remove(proposal_id, signer);
            }

            Self::deposit_event(Event::AIProposalExecuted {
                proposal_id,
                success,
            });

            Ok(())
        }

        /// Activate kill switch (graduated emergency controls)
        #[pallet::call_index(13)]
        #[pallet::weight(T::WeightInfo::vote())]
        pub fn activate_kill_switch(
            origin: OriginFor<T>,
            level: KillSwitchLevel,
            reason: BoundedVec<u8, ConstU32<512>>,
        ) -> DispatchResult {
            let activator = T::EmergencyOrigin::ensure_origin(origin)?;
            let current_level = KillSwitchLevelStorage::<T>::get();

            // Can only escalate, not de-escalate
            ensure!(level >= current_level, Error::<T>::InvalidKillSwitchLevel);

            KillSwitchLevelStorage::<T>::put(level);

            let activation = KillSwitchActivation {
                level,
                activator: activator.clone(),
                reason: reason.clone(),
                activated_at: frame_system::Pallet::<T>::block_number(),
                auto_deactivate_at: None,
            };

            KillSwitchHistory::<T>::insert(frame_system::Pallet::<T>::block_number(), activation);

            Self::deposit_event(Event::KillSwitchActivated {
                level,
                activator,
                reason,
            });

            Ok(())
        }

        /// Deactivate kill switch
        #[pallet::call_index(14)]
        #[pallet::weight(T::WeightInfo::vote())]
        pub fn deactivate_kill_switch(origin: OriginFor<T>) -> DispatchResult {
            T::EmergencyOrigin::ensure_origin(origin)?;
            let previous_level = KillSwitchLevelStorage::<T>::get();

            KillSwitchLevelStorage::<T>::put(KillSwitchLevel::Normal);

            Self::deposit_event(Event::KillSwitchDeactivated {
                previous_level,
                new_level: KillSwitchLevel::Normal,
            });

            Ok(())
        }

        /// Register AI reviewer
        #[pallet::call_index(15)]
        #[pallet::weight(T::WeightInfo::vote())]
        pub fn register_ai_reviewer(
            origin: OriginFor<T>,
            reviewer: T::AccountId,
        ) -> DispatchResult {
            T::EmergencyOrigin::ensure_origin(origin)?;

            AIReviewers::<T>::insert(&reviewer, true);

            Self::deposit_event(Event::AIReviewerRegistered { reviewer });

            Ok(())
        }

        /// Update AI governance configuration
        #[pallet::call_index(16)]
        #[pallet::weight(T::WeightInfo::update_config())]
        pub fn update_ai_config(
            origin: OriginFor<T>,
            new_config: AIGovernanceConfig,
        ) -> DispatchResult {
            T::RuntimeUpgradeOrigin::ensure_origin(origin)?;

            AIConfig::<T>::put(new_config);

            Self::deposit_event(Event::AIConfigUpdated);

            Ok(())
        }
    }

    // ========================================================================
    // Helper Functions
    // ========================================================================

    impl<T: Config> Pallet<T> {
        /// Enact an approved proposal.
        fn enact_proposal(proposal_id: u32) -> Weight {
            let mut weight = T::DbWeight::get().reads(1);

            if let Some(mut proposal) = Proposals::<T>::get(proposal_id) {
                if proposal.status == ProposalStatus::Approved {
                    // ----------------------------------------------------------
                    // Constitutional proof gate (Article IV, vΩ-1.0)
                    //
                    // Proposals that touch constitutional invariants MUST carry a
                    // non-zero proof commitment. Voting alone is not sufficient.
                    // If the gate fails, the proposal is cancelled (not enacted)
                    // and the deposit is slashed per Article VI.
                    // ----------------------------------------------------------
                    if proposal.touches_invariants {
                        let has_proof = proposal
                            .proof_commitment
                            .map(|c| c != [0u8; 32])
                            .unwrap_or(false);

                        if !has_proof {
                            // Slash deposit for attempting bypass without proof
                            let _ =
                                T::Currency::slash_reserved(&proposal.proposer, proposal.deposit);
                            proposal.status = ProposalStatus::Cancelled;
                            Proposals::<T>::insert(proposal_id, &proposal);
                            // Emit cancellation with proof-gate reason
                            Self::deposit_event(Event::ProposalCancelled { proposal_id });
                            weight = weight.saturating_add(T::DbWeight::get().writes(1));
                            return weight;
                        }
                    }

                    let result = proposal
                        .call
                        .clone()
                        .dispatch(frame_system::RawOrigin::Root.into());

                    let dispatch_result = result.map(|_| ()).map_err(|e| e.error);

                    proposal.status = ProposalStatus::Enacted;
                    proposal.enacted_at = Some(frame_system::Pallet::<T>::block_number());
                    Proposals::<T>::insert(proposal_id, proposal);

                    Self::deposit_event(Event::ProposalEnacted {
                        proposal_id,
                        result: dispatch_result,
                    });

                    weight = weight.saturating_add(T::DbWeight::get().writes(1));
                }
            }

            weight
        }

        /// Process expired proposals that weren't finalized.
        fn process_expired_proposals(_current_block: BlockNumberFor<T>) -> Weight {
            // This would iterate through proposals and auto-finalize expired ones
            // For efficiency, we rely on users calling finalize_proposal

            Weight::zero()
        }

        /// Calculate delegated voting power for an account.
        fn get_delegated_power(
            account: &T::AccountId,
            _balance: BalanceOf<T>,
            _conviction: Conviction,
        ) -> Result<BalanceOf<T>, DispatchError> {
            let delegators = Delegators::<T>::get(account);
            let mut total_power = BalanceOf::<T>::zero();

            for delegator in delegators.iter() {
                if let Some(delegation) = Delegations::<T>::get(delegator) {
                    let (mul_num, mul_denom) = delegation.conviction.multiplier();
                    let power =
                        delegation.balance.saturating_mul(mul_num.into()) / mul_denom.into();
                    total_power = total_power.saturating_add(power);
                }
            }

            Ok(total_power)
        }

        /// Remove a vote from the tally.
        fn remove_vote_from_tally(proposal_id: u32, vote: &Vote<BalanceOf<T>>) -> DispatchResult {
            ProposalVotes::<T>::try_mutate(proposal_id, |tally| -> DispatchResult {
                match vote.direction {
                    VoteDirection::Aye => {
                        tally.ayes = tally.ayes.saturating_sub(vote.voting_power);
                        tally.aye_voters = tally.aye_voters.saturating_sub(1);
                    }
                    VoteDirection::Nay => {
                        tally.nays = tally.nays.saturating_sub(vote.voting_power);
                        tally.nay_voters = tally.nay_voters.saturating_sub(1);
                    }
                    VoteDirection::Abstain => {
                        tally.abstains = tally.abstains.saturating_sub(vote.voting_power);
                    }
                }
                tally.turnout = tally.turnout.saturating_sub(vote.balance);
                Ok(())
            })
        }

        /// Check if delegation would create a cycle.
        fn would_create_cycle(delegator: &T::AccountId, target: &T::AccountId) -> bool {
            let mut current = target.clone();
            let max_depth = 10u32;

            for _ in 0..max_depth {
                if let Some(delegation) = Delegations::<T>::get(&current) {
                    if &delegation.target == delegator {
                        return true;
                    }
                    current = delegation.target;
                } else {
                    return false;
                }
            }

            false
        }

        /// Remove delegation from an account.
        fn remove_delegation(delegator: &T::AccountId) -> DispatchResult {
            if let Some(delegation) = Delegations::<T>::take(delegator) {
                Delegators::<T>::mutate(&delegation.target, |delegators| {
                    delegators.retain(|d| d != delegator);
                });
            }
            Ok(())
        }

        /// Create a conviction lock for tokens.
        fn create_conviction_lock(
            account: &T::AccountId,
            amount: BalanceOf<T>,
            conviction: Conviction,
        ) -> DispatchResult {
            let current_block = frame_system::Pallet::<T>::block_number();
            let lock_periods = conviction.lock_periods();
            let unlock_at = current_block
                .saturating_add(T::ConvictionPeriod::get().saturating_mul(lock_periods.into()));

            let lock = VoteLock {
                amount,
                unlock_at,
                conviction,
            };

            Locks::<T>::try_mutate(account, |locks| -> DispatchResult {
                locks.try_push(lock).map_err(|_| Error::<T>::TooManyVotes)?;
                Ok(())
            })
        }

        // ====================================================================
        // Runtime API helpers
        // ====================================================================

        /// Get a snapshot of governance state for offchain consumers.
        #[allow(clippy::type_complexity)]
        pub fn get_governance_snapshot(
        ) -> GovernanceSnapshot<T::AccountId, BalanceOf<T>, BlockNumberFor<T>> {
            let config = GovernanceConfig::<T>::get();
            let proposal_count = ProposalCount::<T>::get();

            let mut active_proposals: BoundedVec<
                ProposalSummary<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>,
                ConstU32<256>,
            > = Default::default();
            let mut pending_enactments: BoundedVec<u32, ConstU32<1024>> = Default::default();

            for id in 0..proposal_count {
                if let Some(proposal) = Proposals::<T>::get(id) {
                    if proposal.status == ProposalStatus::Voting {
                        let tally = ProposalVotes::<T>::get(id);
                        let _ = active_proposals.try_push(ProposalSummary {
                            id,
                            proposer: proposal.proposer,
                            status: proposal.status,
                            voting_end: proposal.voting_end,
                            ayes: tally.ayes,
                            nays: tally.nays,
                            turnout: tally.turnout,
                        });
                    } else if proposal.status == ProposalStatus::Approved {
                        let _ = pending_enactments.try_push(id);
                    }
                }
            }

            GovernanceSnapshot {
                proposal_count,
                active_proposals,
                pending_enactments,
                config,
            }
        }

        // ====================================================================
        // AI Governance Helper Functions
        // ====================================================================

        /// Simulate AI proposal
        fn simulate_ai_proposal(proposal_id: u64) -> DispatchResult {
            let proposal =
                AIProposals::<T>::get(proposal_id).ok_or(Error::<T>::AIProposalNotFound)?;

            // Basic simulation (in production, this would run actual deterministic tests)
            let simulation_result = SimulationResult {
                success: true, // Assume success for now
                gas_used: proposal
                    .simulation_requirements
                    .gas_limit
                    .saturating_sub(100_000),
                execution_time: proposal.simulation_requirements.simulation_blocks,
                state_changes: Default::default(),
                warnings: Default::default(),
            };

            SimulationResults::<T>::insert(proposal_id, simulation_result.clone());

            AIProposals::<T>::try_mutate(proposal_id, |p| {
                if let Some(prop) = p {
                    prop.status = if simulation_result.success {
                        AIProposalStatus::SimulationPassed
                    } else {
                        AIProposalStatus::SimulationFailed
                    };
                }
                Ok::<(), Error<T>>(())
            })?;

            Self::deposit_event(Event::AIProposalSimulated {
                proposal_id,
                success: simulation_result.success,
                gas_used: simulation_result.gas_used,
            });

            Ok(())
        }

        /// Execute AI proposal in sandbox
        fn execute_in_sandbox(proposal: &AIProposal<T>) -> bool {
            // In production, this would execute the proposal in a sandboxed environment
            // with gas limits, rollback checkpoints, and state isolation

            // For now, simulate execution based on risk assessment
            proposal.impact_assessment.risk_level < 50
        }

        /// Create rollback checkpoint
        fn create_rollback_checkpoint() -> BoundedVec<u8, ConstU32<8192>> {
            // In production, this would create a state snapshot for rollback
            // For now, return empty checkpoint
            Default::default()
        }

        /// Check if AI evolution is allowed
        pub fn is_ai_evolution_allowed() -> bool {
            KillSwitchLevelStorage::<T>::get() < KillSwitchLevel::UpgradeFreeze
        }

        /// Get current kill switch level
        pub fn get_kill_switch_level() -> KillSwitchLevel {
            KillSwitchLevelStorage::<T>::get()
        }

        /// Get AI proposal status
        pub fn get_ai_proposal_status(proposal_id: u64) -> Option<AIProposalStatus> {
            AIProposals::<T>::get(proposal_id).map(|p| p.status)
        }
    }
}
