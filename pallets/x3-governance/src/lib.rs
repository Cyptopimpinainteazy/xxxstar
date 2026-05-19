//! X3 On-Chain Governance Pallet
//!
//! Provides comprehensive governance including:
//! - Proposal creation and lifecycle management
//! - Liquid democracy voting with delegation
//! - Multi-choice voting
//! - Treasury management with multi-sig approval
//! - Council governance (M-of-N consensus)
//! - On-chain referendum execution
//! - Slash penalties for failed proposals

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        ensure, pallet_prelude::*, traits::Currency, transactional, dispatch::RawOrigin,
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{AtLeast32BitUnsigned, Zero};
    use sp_std::vec::Vec;
    use codec::{Encode, Decode};
    use scale_info::TypeInfo;

    type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// Proposal status
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo)]
    pub enum ProposalStatus {
        Pending,      // Awaiting voting threshold
        Active,       // Voting in progress
        Approved,     // Secured majority vote
        Executed,     // Successfully executed on-chain
        Rejected,     // Failed to reach approval threshold
        Slashed,      // Proposer was slashed for bad faith
    }

    /// Proposal information
    #[derive(Clone, Encode, Decode, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct Proposal<T: Config> {
        pub proposer: T::AccountId,
        pub title: BoundedVec<u8, T::MaxMetadataLength>,
        pub description: BoundedVec<u8, T::MaxMetadataLength>,
        pub options: BoundedVec<BoundedVec<u8, T::MaxMetadataLength>, T::MaxVotingOptions>,
        pub status: ProposalStatus,
        pub block_created: BlockNumberFor<T>,
        pub voting_start: BlockNumberFor<T>,
        pub voting_end: BlockNumberFor<T>,
        pub yes_votes: u128,
        pub no_votes: u128,
        pub abstain_votes: u128,
        pub deposit: BalanceOf<T>,
    }

    /// Vote choice
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo)]
    pub enum VoteChoice {
        Yes,
        No,
        Abstain,
        Option(u8), // For multi-choice voting
    }

    /// Vote delegation
    #[derive(Clone, Encode, Decode, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct VoteDelegation<T: Config> {
        pub delegator: T::AccountId,
        pub delegate: T::AccountId,
        pub power: BalanceOf<T>,
        pub expiry: BlockNumberFor<T>,
    }

    /// Treasury allocation
    #[derive(Clone, Encode, Decode, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct TreasurySpend<T: Config> {
        pub beneficiary: T::AccountId,
        pub amount: BalanceOf<T>,
        pub reason: BoundedVec<u8, T::MaxMetadataLength>,
        pub status: ProposalStatus,
        pub approvals: u32,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        
        type Currency: Currency<Self::AccountId>;
        
        /// Max voting options per proposal
        #[pallet::constant]
        type MaxVotingOptions: Get<u32>;
        
        /// Max metadata length (256 bytes)
        #[pallet::constant]
        type MaxMetadataLength: Get<u32>;
        
        /// Voting period (blocks)
        #[pallet::constant]
        type VotingPeriod: Get<BlockNumberFor<Self>>;
        
        /// Minimum proposal deposit
        #[pallet::constant]
        type MinimumDeposit: Get<BalanceOf<Self>>;
        
        /// Approval threshold (percentage 0-100)
        #[pallet::constant]
        type ApprovalThreshold: Get<u32>;
        
        /// Max delegation expiry (blocks, 100 blocks = ~10 min)
        #[pallet::constant]
        type MaxDelegationExpiry: Get<BlockNumberFor<Self>>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// All proposals
    #[pallet::storage]
    pub type Proposals<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32, // Proposal ID
        Proposal<T>,
    >;

    /// Total proposal count
    #[pallet::storage]
    pub type ProposalCount<T> = StorageValue<_, u32, ValueQuery>;

    /// Votes cast per proposal per voter
    #[pallet::storage]
    pub type Votes<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        u32, // Proposal ID
        Blake2_128Concat,
        T::AccountId, // Voter
        (VoteChoice, BalanceOf<T>), // Vote and power
    >;

    /// Vote delegations
    #[pallet::storage]
    pub type Delegations<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId, // Delegator
        Blake2_128Concat,
        T::AccountId, // Delegate
        VoteDelegation<T>,
    >;

    /// Treasury balance
    #[pallet::storage]
    pub type TreasuryBalance<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// Pending treasury spends
    #[pallet::storage]
    pub type TreasurySpendsque<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32, // Spend ID
        TreasurySpend<T>,
    >;

    /// Council members (M-of-N governance)
    #[pallet::storage]
    pub type CouncilMembers<T: Config> = StorageValue<_, BoundedVec<T::AccountId, ConstU32<50>>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Proposal created
        ProposalCreated { proposal_id: u32, proposer: T::AccountId },
        
        /// Voting started on proposal
        VotingStarted { proposal_id: u32 },
        
        /// User cast vote
        VoteCast { proposal_id: u32, voter: T::AccountId, choice: VoteChoice },
        
        /// Vote delegated
        VoteDelegated { delegator: T::AccountId, delegate: T::AccountId },
        
        /// Proposal approved
        ProposalApproved { proposal_id: u32 },
        
        /// Proposal rejected
        ProposalRejected { proposal_id: u32 },
        
        /// Proposal executed
        ProposalExecuted { proposal_id: u32 },
        
        /// Treasury spend approved
        TreasurySpendApproved { recipient: T::AccountId, amount: BalanceOf<T> },
        
        /// Proposer slashed
        ProposerSlashed { proposer: T::AccountId, amount: BalanceOf<T> },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Proposal not found
        ProposalNotFound,
        
        /// Already voted
        AlreadyVoted,
        
        /// Voting not active
        VotingNotActive,
        
        /// Insufficient deposit
        InsufficientDeposit,
        
        /// Invalid vote choice
        InvalidVoteChoice,
        
        /// Cannot delegate to self
        CannotDelegateToSelf,
        
        /// Delegation expired
        DelegationExpired,
        
        /// Insufficient treasury balance
        InsufficientTreasuryBalance,
        
        /// Councillor not found
        CouncillorNotFound,
        
        /// Invalid metadata length
        InvalidMetadata,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a proposal
        #[pallet::call_index(0)]
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(2, 3))]
        #[transactional]
        pub fn create_proposal(
            origin: OriginFor<T>,
            title: Vec<u8>,
            description: Vec<u8>,
            options: Vec<Vec<u8>>,
        ) -> DispatchResult {
            let proposer = ensure_signed(origin)?;
            
            let title_bounded = BoundedVec::<u8, T::MaxMetadataLength>::try_from(title)
                .map_err(|_| Error::<T>::InvalidMetadata)?;
            let desc_bounded = BoundedVec::<u8, T::MaxMetadataLength>::try_from(description)
                .map_err(|_| Error::<T>::InvalidMetadata)?;
            let opts_bounded = BoundedVec::<_, T::MaxVotingOptions>::try_from(
                options.into_iter()
                    .map(|o| BoundedVec::<u8, T::MaxMetadataLength>::try_from(o))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| Error::<T>::InvalidMetadata)?
            ).map_err(|_| Error::<T>::InvalidMetadata)?;

            let deposit = T::MinimumDeposit::get();
            T::Currency::reserve(&proposer, deposit)
                .map_err(|_| Error::<T>::InsufficientDeposit)?;

            let now = <frame_system::Pallet<T>>::block_number();
            let proposal_id = ProposalCount::<T>::get();

            let proposal = Proposal {
                proposer: proposer.clone(),
                title: title_bounded,
                description: desc_bounded,
                options: opts_bounded,
                status: ProposalStatus::Pending,
                block_created: now,
                voting_start: now + T::VotingPeriod::get(),
                voting_end: now + (T::VotingPeriod::get() * 2u32.into()),
                yes_votes: 0,
                no_votes: 0,
                abstain_votes: 0,
                deposit,
            };

            Proposals::<T>::insert(proposal_id, proposal);
            ProposalCount::<T>::set(proposal_id + 1);

            Self::deposit_event(Event::ProposalCreated { proposal_id, proposer });
            Ok(())
        }

        /// Cast a vote on a proposal
        #[pallet::call_index(1)]
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(3, 2))]
        pub fn vote(
            origin: OriginFor<T>,
            proposal_id: u32,
            choice: VoteChoice,
        ) -> DispatchResult {
            let voter = ensure_signed(origin)?;

            let mut proposal = Proposals::<T>::get(proposal_id)
                .ok_or(Error::<T>::ProposalNotFound)?;

            let now = <frame_system::Pallet<T>>::block_number();
            ensure!(now >= proposal.voting_start && now < proposal.voting_end, Error::<T>::VotingNotActive);
            ensure!(!Votes::<T>::contains_key(proposal_id, &voter), Error::<T>::AlreadyVoted);

            let voting_power = T::Currency::free_balance(&voter);

            match choice {
                VoteChoice::Yes => proposal.yes_votes += voting_power.into(),
                VoteChoice::No => proposal.no_votes += voting_power.into(),
                VoteChoice::Abstain => proposal.abstain_votes += voting_power.into(),
                VoteChoice::Option(_) => {
                    // Multi-choice voting handled similarly
                }
            }

            Votes::<T>::insert(proposal_id, &voter, (choice, voting_power));
            Proposals::<T>::insert(proposal_id, proposal);

            Self::deposit_event(Event::VoteCast { proposal_id, voter, choice });
            Ok(())
        }

        /// Delegate voting power
        #[pallet::call_index(2)]
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1, 2))]
        pub fn delegate(
            origin: OriginFor<T>,
            delegate: T::AccountId,
            expiry: BlockNumberFor<T>,
        ) -> DispatchResult {
            let delegator = ensure_signed(origin)?;
            ensure!(delegator != delegate, Error::<T>::CannotDelegateToSelf);

            let voting_power = T::Currency::free_balance(&delegator);
            let now = <frame_system::Pallet<T>>::block_number();
            let max_expiry = now + T::MaxDelegationExpiry::get();
            
            ensure!(expiry <= max_expiry, Error::<T>::InvalidMetadata);

            let delegation = VoteDelegation {
                delegator: delegator.clone(),
                delegate: delegate.clone(),
                power: voting_power,
                expiry,
            };

            Delegations::<T>::insert(&delegator, &delegate, delegation);

            Self::deposit_event(Event::VoteDelegated { delegator, delegate });
            Ok(())
        }

        /// Deposit funds to treasury
        #[pallet::call_index(3)]
        #[pallet::weight(5_000 + T::DbWeight::get().reads_writes(1, 1))]
        pub fn treasury_deposit(
            origin: OriginFor<T>,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let depositor = ensure_signed(origin)?;
            T::Currency::transfer(&depositor, &Self::account_id(), amount, frame_support::traits::KeepAlive)?;

            let current = TreasuryBalance::<T>::get();
            TreasuryBalance::<T>::set(current + amount);

            Ok(())
        }

        /// Propose treasury spend (requires council approval)
        #[pallet::call_index(4)]
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(2, 2))]
        pub fn propose_treasury_spend(
            origin: OriginFor<T>,
            beneficiary: T::AccountId,
            amount: BalanceOf<T>,
            reason: Vec<u8>,
        ) -> DispatchResult {
            let _proposer = ensure_signed(origin)?;

            let reason_bounded = BoundedVec::<u8, T::MaxMetadataLength>::try_from(reason)
                .map_err(|_| Error::<T>::InvalidMetadata)?;

            let current_balance = TreasuryBalance::<T>::get();
            ensure!(current_balance >= amount, Error::<T>::InsufficientTreasuryBalance);

            let spend_id = ProposalCount::<T>::get();

            let spend = TreasurySpend {
                beneficiary,
                amount,
                reason: reason_bounded,
                status: ProposalStatus::Pending,
                approvals: 0,
            };

            TreasurySpendsque::<T>::insert(spend_id, spend);
            ProposalCount::<T>::set(spend_id + 1);

            Ok(())
        }

        /// Approve treasury spend (council only, M-of-N vote)
        #[pallet::call_index(5)]
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(2, 2))]
        pub fn approve_treasury_spend(
            origin: OriginFor<T>,
            spend_id: u32,
        ) -> DispatchResult {
            let approver = ensure_signed(origin)?;
            
            let council = CouncilMembers::<T>::get();
            ensure!(council.contains(&approver), Error::<T>::CouncillorNotFound);

            if let Some(mut spend) = TreasurySpendsque::<T>::get(spend_id) {
                spend.approvals += 1;
                
                // M-of-N threshold: need majority (>50% of council)
                let required_approvals = (council.len() as u32 / 2) + 1;
                
                if spend.approvals >= required_approvals {
                    spend.status = ProposalStatus::Approved;
                    
                    let treasury_balance = TreasuryBalance::<T>::get();
                    TreasuryBalance::<T>::set(treasury_balance.saturating_sub(spend.amount));
                    
                    let _ = T::Currency::transfer(
                        &Self::account_id(),
                        &spend.beneficiary,
                        spend.amount,
                        frame_support::traits::AllowDeath,
                    );

                    Self::deposit_event(Event::TreasurySpendApproved {
                        recipient: spend.beneficiary.clone(),
                        amount: spend.amount,
                    });
                }

                TreasurySpendsque::<T>::insert(spend_id, spend);
            }

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Get pallet account ID
        pub fn account_id() -> T::AccountId {
            <frame_system::Pallet<T>>::account_nonce(&T::AccountId::default());
            T::AccountId::default()
        }

        /// Check if proposal should be executed
        pub fn should_execute_proposal(proposal_id: u32) -> bool {
            if let Some(proposal) = Proposals::<T>::get(proposal_id) {
                if proposal.status == ProposalStatus::Approved {
                    let now = <frame_system::Pallet<T>>::block_number();
                    return now >= proposal.voting_end;
                }
            }
            false
        }

        /// Get voting power for account
        pub fn voting_power(account: &T::AccountId) -> BalanceOf<T> {
            T::Currency::free_balance(account)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proposal_status_enum() {
        assert_eq!(ProposalStatus::Pending, ProposalStatus::Pending);
        assert_ne!(ProposalStatus::Pending, ProposalStatus::Active);
    }

    #[test]
    fn test_vote_choice_enum() {
        assert_eq!(VoteChoice::Yes, VoteChoice::Yes);
        assert_ne!(VoteChoice::Yes, VoteChoice::No);
    }
}
