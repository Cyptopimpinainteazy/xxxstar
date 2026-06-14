#![deny(unsafe_code)]
//! # X3 Unified Agent Registry Pallet
//!
//! **SECURITY-CRITICAL**: This pallet is the single source of truth for all agent
//! identity, permissions, staking, slashing, and economics on the X3 network.
//!
//! ## Consolidation
//!
//! This pallet unifies functionality previously scattered across:
//! - `pallet-agent-accounts` — agent identity, quotas, permissions
//! - `pallet-x3-account-registry` — Atlas ID mapping, account kinds
//! - `pallet-x3-agent-law` — policy enforcement, violation tracking
//! - `pallet-x3-slash` — bond lifecycle, slashing, reputation
//!
//! ## Architecture
//!
//! - **Agent Identity**: Registration, lifecycle (active/suspended/terminated),
//!   controller/operator model, Atlas ID binding
//! - **Permissions**: Granular capability flags, policy rules
//! - **Staking**: Bond posting, release, expiry processing
//! - **Slashing**: Severity-based slashing, immutable records, treasury routing
//! - **Economics**: PnL tracking, rewards distribution, burn mechanics
//! - **Policy Enforcement**: Pre-dispatch checks via SignedExtension
//!
//! ## Integration Order (SECURITY-CRITICAL)
//!
//! In `runtime::SignedExtra`:
//! ```ignore
//! x3_invariants::InvariantCheck,           // 1. Hard fail gates FIRST
//! x3_agent_registry::AgentRegistryCheck,   // 2. Unified agent policy enforcement
//! x3_swarm::CapabilityEnvelopeCheck,       // 3. Long-range attack validation
//! x3_kernel::AtomicSettlementCheck,        // 4. Cross-VM atomicity
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod types;
pub use types::*;

pub mod weights;
pub use weights::WeightInfo;

use frame_support::{
    pallet_prelude::*,
    traits::{Currency, ExistenceRequirement, ReservableCurrency},
    Blake2_128Concat,
};
use frame_system::pallet_prelude::*;
use sp_core::H256;
use sp_runtime::traits::{Hash, SaturatedConversion, Saturating};
use sp_std::prelude::*;
use x3_accounting_events::{AccountingEvent, AccountingSpine, FeeDestination, RevenueModule};

type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.

        /// Currency for deposits, bonds, and fees.
        type Currency: ReservableCurrency<Self::AccountId>;

        /// Origin that can register new agents.
        type RegisterOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Origin that can modify agent permissions and policies.
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Maximum agents per controller.
        #[pallet::constant]
        type MaxAgentsPerController: Get<u32>;

        /// Registration deposit.
        #[pallet::constant]
        type RegistrationDeposit: Get<BalanceOf<Self>>;

        /// Minimum bond amount.
        #[pallet::constant]
        type MinBondAmount: Get<BalanceOf<Self>>;

        /// Finality window in blocks (bonds must settle within this).
        #[pallet::constant]
        type FinalityWindow: Get<BlockNumberFor<Self>>;

        /// Default gas limit per block.
        #[pallet::constant]
        type DefaultGasPerBlock: Get<u128>;

        /// Default compute limit per block.
        #[pallet::constant]
        type DefaultComputePerBlock: Get<u128>;

        /// Default gas limit per epoch.
        #[pallet::constant]
        type DefaultGasPerEpoch: Get<u128>;

        /// Default compute limit per epoch.
        #[pallet::constant]
        type DefaultComputePerEpoch: Get<u128>;

        /// Blocks per epoch.
        #[pallet::constant]
        type BlocksPerEpoch: Get<BlockNumberFor<Self>>;

        /// Reputation threshold below which capability is auto-revoked.
        #[pallet::constant]
        type ReputationThreshold: Get<u64>;

        /// Maximum tasks per block.
        #[pallet::constant]
        type MaxTasksPerBlock: Get<u32>;

        /// Rate limit: max extrinsics per epoch.
        #[pallet::constant]
        type RateLimitMaxExtrinsicsPerEpoch: Get<u32>;

        /// Whether to apply reputation damage on critical slashes.
        #[pallet::constant]
        type ReputationDamageEnabled: Get<bool>;

        /// Slash recipient (typically treasury).
        type SlashRecipient: Get<Self::AccountId>;

        /// Canonical accounting spine.
        type AccountingSpine: AccountingSpine<u64, u128>;

        /// Weight information.
        type WeightInfo: WeightInfo;
    }

    // ========================================================================
    // Storage — Agent Identity
    // ========================================================================

    /// Counter for agent IDs.
    #[pallet::storage]
    #[pallet::getter(fn next_agent_id)]
    pub type NextAgentId<T> = StorageValue<_, AgentId, ValueQuery>;

    /// All registered agents.
    #[pallet::storage]
    #[pallet::getter(fn agents)]
    pub type Agents<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        AgentId,
        AgentRecord<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// Agents owned by each controller.
    #[pallet::storage]
    #[pallet::getter(fn controller_agents)]
    pub type ControllerAgents<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<AgentId, T::MaxAgentsPerController>,
        ValueQuery,
    >;

    /// Operator to agent mapping.
    #[pallet::storage]
    #[pallet::getter(fn operator_agent)]
    pub type OperatorAgent<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, AgentId, OptionQuery>;

    /// Atlas ID to agent mapping.
    #[pallet::storage]
    #[pallet::getter(fn atlas_to_agent)]
    pub type AtlasToAgent<T: Config> = StorageMap<_, Blake2_128Concat, u64, AgentId, OptionQuery>;

    // ========================================================================
    // Storage — Quotas & Permissions
    // ========================================================================

    /// Agent quotas.
    #[pallet::storage]
    #[pallet::getter(fn quotas)]
    pub type Quotas<T: Config> =
        StorageMap<_, Blake2_128Concat, AgentId, AgentQuota<BlockNumberFor<T>>, OptionQuery>;

    /// Agent permissions.
    #[pallet::storage]
    #[pallet::getter(fn permissions)]
    pub type Permissions<T: Config> =
        StorageMap<_, Blake2_128Concat, AgentId, AgentPermissions, ValueQuery>;

    /// Agent activity for current epoch.
    #[pallet::storage]
    #[pallet::getter(fn activity)]
    pub type Activity<T: Config> =
        StorageMap<_, Blake2_128Concat, AgentId, AgentActivity, ValueQuery>;

    /// Current epoch number.
    #[pallet::storage]
    #[pallet::getter(fn current_epoch)]
    pub type CurrentEpoch<T> = StorageValue<_, u64, ValueQuery>;

    /// Last block of epoch reset.
    #[pallet::storage]
    #[pallet::getter(fn last_epoch_block)]
    pub type LastEpochBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    // ========================================================================
    // Storage — Staking & Slashing
    // ========================================================================

    /// Bond ledger.
    #[pallet::storage]
    #[pallet::getter(fn bonds)]
    pub type Bonds<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, AgentBond<T::AccountId, BalanceOf<T>>, OptionQuery>;

    /// Bonds by agent.
    #[pallet::storage]
    #[pallet::getter(fn bonds_by_agent)]
    pub type BondsByAgent<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, BoundedVec<H256, ConstU32<100>>, ValueQuery>;

    /// Slash records (immutable history).
    #[pallet::storage]
    #[pallet::getter(fn slashes)]
    pub type SlashRecords<T: Config> =
        StorageMap<_, Blake2_128Concat, u64, SlashRecord<T::AccountId>, OptionQuery>;

    /// Next slash ID counter.
    #[pallet::storage]
    pub type SlashIdCounter<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Next bond ID counter.
    #[pallet::storage]
    pub type BondIdCounter<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Reputation scores.
    #[pallet::storage]
    #[pallet::getter(fn reputation)]
    pub type ReputationScores<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, i64, ValueQuery>;

    /// Slashed in current epoch.
    #[pallet::storage]
    #[pallet::getter(fn slashed_this_epoch)]
    pub type SlashedThisEpoch<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

    // ========================================================================
    // Storage — Policy Enforcement
    // ========================================================================

    /// Active policies governing each agent.
    #[pallet::storage]
    #[pallet::getter(fn agent_policies)]
    pub type ActivePolicies<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<PolicyRule<T::AccountId>, ConstU32<16>>,
        ValueQuery,
    >;

    /// Violation count per agent.
    #[pallet::storage]
    #[pallet::getter(fn violation_count)]
    pub type ViolationCount<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

    /// Extrinsic count per agent in current epoch.
    #[pallet::storage]
    #[pallet::getter(fn extrinsic_count_this_epoch)]
    pub type ExtrinsicCountThisEpoch<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

    /// Last epoch recorded for each agent.
    #[pallet::storage]
    #[pallet::getter(fn last_epoch)]
    pub type LastEpoch<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, BlockNumberFor<T>, ValueQuery>;

    /// Blacklisted agents (block number when blacklist expires).
    #[pallet::storage]
    #[pallet::getter(fn blacklist_expiry)]
    pub type Blacklist<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, BlockNumberFor<T>, OptionQuery>;

    /// Task counts per agent per block.
    #[pallet::storage]
    #[pallet::getter(fn tasks_this_block)]
    pub type TasksThisBlock<T: Config> =
        StorageMap<_, Blake2_128Concat, (BlockNumberFor<T>, T::AccountId), u32, ValueQuery>;

    // ========================================================================
    // Storage — Agent Economics
    // ========================================================================

    /// Agent economics snapshots.
    #[pallet::storage]
    #[pallet::getter(fn agent_economics)]
    pub type AgentEconomicsStore<T: Config> =
        StorageMap<_, Blake2_128Concat, AgentId, AgentEconomics<BalanceOf<T>>, ValueQuery>;

    /// Total agents counter.
    #[pallet::storage]
    #[pallet::getter(fn total_agents)]
    pub type TotalAgents<T> = StorageValue<_, u32, ValueQuery>;

    /// Active agents counter.
    #[pallet::storage]
    #[pallet::getter(fn active_agents)]
    pub type ActiveAgents<T> = StorageValue<_, u32, ValueQuery>;

    // ========================================================================
    // Storage — Agent Economics (Reward Pool)
    // ========================================================================

    /// Reward configuration per proof type.
    #[pallet::storage]
    #[pallet::getter(fn proof_reward_config)]
    pub type ProofRewardConfigStore<T: Config> =
        StorageValue<_, ProofRewardConfig<BalanceOf<T>>, ValueQuery>;

    /// Accumulated unclaimed rewards per agent.
    #[pallet::storage]
    #[pallet::getter(fn agent_reward_pool)]
    pub type AgentRewardPool<T: Config> =
        StorageMap<_, Blake2_128Concat, AgentId, BalanceOf<T>, ValueQuery>;

    /// Total balance in the reward pool (tracked for accounting).
    #[pallet::storage]
    #[pallet::getter(fn total_reward_pool)]
    pub type TotalRewardPool<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// Reward distribution history (ring buffer, last 100 entries).
    #[pallet::storage]
    #[pallet::getter(fn reward_distribution_history)]
    pub type RewardDistributionHistory<T: Config> = StorageValue<
        _,
        BoundedVec<
            RewardDistribution<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>,
            ConstU32<100>,
        >,
        ValueQuery,
    >;

    // ========================================================================
    // Events
    // ========================================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        // -- Identity Events --
        AgentRegistered {
            agent_id: AgentId,
            controller: T::AccountId,
            operator: T::AccountId,
            kind: AgentKind,
        },
        AgentStatusChanged {
            agent_id: AgentId,
            old_status: AgentStatus,
            new_status: AgentStatus,
        },
        OperatorChanged {
            agent_id: AgentId,
            old_operator: T::AccountId,
            new_operator: T::AccountId,
        },
        AgentSuspended {
            agent_id: AgentId,
            reason: BoundedVec<u8, ConstU32<256>>,
        },
        AgentTerminated {
            agent_id: AgentId,
        },
        AtlasIdBound {
            agent_id: AgentId,
            atlas_id: u64,
        },

        // -- Permission Events --
        PermissionsUpdated {
            agent_id: AgentId,
            permissions: AgentPermissions,
        },
        QuotaUpdated {
            agent_id: AgentId,
            quota: AgentQuota<BlockNumberFor<T>>,
        },
        PolicyRegistered {
            agent: T::AccountId,
            policy_count: u32,
        },
        PolicyViolation {
            agent: T::AccountId,
            violation_type: ViolationType,
        },

        // -- Staking Events --
        BondPosted {
            bond_id: H256,
            agent: T::AccountId,
            amount: BalanceOf<T>,
            expires_at: BlockNumberFor<T>,
        },
        BondReleased {
            bond_id: H256,
            agent: T::AccountId,
            amount: BalanceOf<T>,
        },
        BondExpired {
            bond_id: H256,
            agent: T::AccountId,
        },

        // -- Slashing Events --
        SlashExecuted {
            slash_id: u64,
            agent: T::AccountId,
            bond_id: H256,
            severity: u8,
            amount_slashed: BalanceOf<T>,
        },
        ReputationDamaged {
            agent: T::AccountId,
            damage: i64,
        },

        // -- Economics Events --
        RewardsDistributed {
            agent_id: AgentId,
            amount: BalanceOf<T>,
        },
        ResourceConsumed {
            agent_id: AgentId,
            gas_used: u128,
            compute_used: u128,
        },
        ReputationChanged {
            agent_id: AgentId,
            old_score: u32,
            new_score: u32,
        },
        EpochStarted {
            epoch: u64,
            block: BlockNumberFor<T>,
        },
        AgentAction {
            agent_id: AgentId,
            action_type: ActionType,
            data: BoundedVec<u8, ConstU32<512>>,
        },

        // -- Reward Pool Events --
        /// Reward config was updated by admin.
        ProofRewardConfigUpdated {
            config: ProofRewardConfig<BalanceOf<T>>,
        },
        /// Reward pool was funded by an account.
        RewardPoolFunded {
            from: T::AccountId,
            amount: BalanceOf<T>,
            new_total: BalanceOf<T>,
        },
        /// Agent claimed accumulated rewards.
        RewardsClaimed {
            agent_id: AgentId,
            recipient: T::AccountId,
            amount: BalanceOf<T>,
        },
        /// Automatic reward distributed for proof verification.
        ProofRewardDistributed {
            agent_id: AgentId,
            amount: BalanceOf<T>,
            reason: BoundedVec<u8, ConstU32<64>>,
        },
    }

    // ========================================================================
    // Errors
    // ========================================================================

    #[pallet::error]
    pub enum Error<T> {
        // -- Identity Errors --
        AgentNotFound,
        NotController,
        NotOperator,
        AgentAlreadyExists,
        TooManyAgents,
        AgentNotActive,
        AgentSuspended,
        AgentTerminated,
        InvalidStatusTransition,
        MetadataTooLong,
        InsufficientDeposit,
        AtlasIdAlreadyBound,

        // -- Permission Errors --
        PermissionDenied,
        QuotaExceeded,
        TooManyPolicies,
        InvalidPolicyRule,
        CapabilityNotPermitted,
        ReputationBelowMinimum,
        MaxTasksPerBlockExceeded,
        CollusionAttempted,
        RateLimitExceeded,
        AgentBlacklisted,

        // -- Staking Errors --
        BondNotFound,
        InsufficientFunds,
        BondTooSmall,
        InvalidBondState,
        NotAuthorized,
        ArithmeticError,
        ReputationOutOfBounds,

        // -- Reward Pool Errors --
        /// Reward pool has insufficient balance.
        RewardPoolInsufficient,
        /// Agent has no rewards to claim.
        NoRewardsToClaim,
    }

    // ========================================================================
    // Hooks
    // ========================================================================

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: BlockNumberFor<T>) -> Weight {
            let last_epoch = LastEpochBlock::<T>::get();
            let blocks_per_epoch = T::BlocksPerEpoch::get();

            if n >= last_epoch.saturating_add(blocks_per_epoch) {
                Self::start_new_epoch(n);
                T::DbWeight::get().reads_writes(3, 2)
            } else {
                Weight::zero()
            }
        }

        fn on_finalize(_block: BlockNumberFor<T>) {
            const MAX_BONDS_PER_BLOCK: usize = 20;
            let now: u32 = frame_system::Pallet::<T>::block_number().saturated_into::<u32>();

            let expired_bonds: Vec<(H256, _)> = Bonds::<T>::iter()
                .filter(|(_, bond_state)| {
                    matches!(bond_state.status, BondStatus::Active) && bond_state.expires_at <= now
                })
                .take(MAX_BONDS_PER_BLOCK)
                .collect();

            for (bond_id, bond_state) in expired_bonds {
                T::Currency::unreserve(&bond_state.agent, bond_state.amount);
                let recipient = T::SlashRecipient::get();
                if let Err(e) = T::Currency::transfer(
                    &bond_state.agent,
                    &recipient,
                    bond_state.amount,
                    ExistenceRequirement::AllowDeath,
                ) {
                    log::warn!(
                        target: "x3-agent-registry",
                        "on_finalize: treasury transfer failed for bond {:?}: {:?}",
                        bond_id, e
                    );
                }

                let slash_id = SlashIdCounter::<T>::get();
                SlashIdCounter::<T>::set(slash_id.saturating_add(1));
                let slash_record = SlashRecord {
                    slash_id,
                    agent: bond_state.agent.clone(),
                    bond_id,
                    severity: 3,
                    amount_slashed: bond_state.amount.saturated_into::<u128>(),
                    reason: b"bond_expiry"
                        .to_vec()
                        .try_into()
                        .expect("bond_expiry fits in bounded reason"),
                    slashed_at: now,
                };
                SlashRecords::<T>::insert(slash_id, slash_record);

                let mut bond_state = bond_state;
                bond_state.status = BondStatus::Expired;
                Bonds::<T>::insert(bond_id, bond_state.clone());

                Self::deposit_event(Event::BondExpired {
                    bond_id,
                    agent: bond_state.agent,
                });
            }
        }
    }

    // ========================================================================
    // Extrinsics — Identity
    // ========================================================================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a new agent.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_agent())]
        pub fn register_agent(
            origin: OriginFor<T>,
            operator: T::AccountId,
            name: BoundedVec<u8, ConstU32<64>>,
            metadata: BoundedVec<u8, ConstU32<1024>>,
            kind: AgentKind,
        ) -> DispatchResult {
            let controller = ensure_signed(origin)?;

            let mut controller_agents = ControllerAgents::<T>::get(&controller);
            ensure!(
                (controller_agents.len() as u32) < T::MaxAgentsPerController::get(),
                Error::<T>::TooManyAgents
            );
            ensure!(
                !OperatorAgent::<T>::contains_key(&operator),
                Error::<T>::NotAuthorized
            );

            T::Currency::reserve(&controller, T::RegistrationDeposit::get())?;

            let agent_id = NextAgentId::<T>::get();
            let current_block = frame_system::Pallet::<T>::block_number();

            let agent = AgentRecord {
                id: agent_id,
                controller: controller.clone(),
                operator: operator.clone(),
                name,
                metadata,
                status: AgentStatus::Active,
                reputation: 100,
                deposit: T::RegistrationDeposit::get(),
                registered_at: current_block,
                last_active: current_block,
                atlas_id: None,
                kind,
            };

            let quota = AgentQuota {
                gas_per_block: T::DefaultGasPerBlock::get(),
                compute_per_block: T::DefaultComputePerBlock::get(),
                gas_per_epoch: T::DefaultGasPerEpoch::get(),
                compute_per_epoch: T::DefaultComputePerEpoch::get(),
                epoch_start: current_block,
            };

            let permissions = AgentPermissions::default();
            let economics = AgentEconomics {
                total_rewards: Zero::zero(),
                total_slashed: Zero::zero(),
                current_bonded: Zero::zero(),
                pnl: Zero::zero(),
                successful_tasks: 0,
                failed_tasks: 0,
            };

            Agents::<T>::insert(agent_id, agent);
            Quotas::<T>::insert(agent_id, quota);
            Permissions::<T>::insert(agent_id, permissions);
            AgentEconomicsStore::<T>::insert(agent_id, economics);
            OperatorAgent::<T>::insert(&operator, agent_id);

            controller_agents
                .try_push(agent_id)
                .map_err(|_| Error::<T>::TooManyAgents)?;
            ControllerAgents::<T>::insert(&controller, controller_agents);

            NextAgentId::<T>::put(agent_id.saturating_add(1));
            TotalAgents::<T>::mutate(|n| *n = n.saturating_add(1));
            ActiveAgents::<T>::mutate(|n| *n = n.saturating_add(1));

            Self::deposit_event(Event::AgentRegistered {
                agent_id,
                controller,
                operator,
                kind,
            });

            Ok(())
        }

        /// Bind an Atlas ID to an agent.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::bind_atlas_id())]
        pub fn bind_atlas_id(
            origin: OriginFor<T>,
            agent_id: AgentId,
            atlas_id: u64,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            Agents::<T>::try_mutate(agent_id, |maybe_agent| -> DispatchResult {
                let agent = maybe_agent.as_mut().ok_or(Error::<T>::AgentNotFound)?;
                ensure!(agent.controller == who, Error::<T>::NotController);
                ensure!(agent.atlas_id.is_none(), Error::<T>::AtlasIdAlreadyBound);
                ensure!(
                    !AtlasToAgent::<T>::contains_key(&atlas_id),
                    Error::<T>::AtlasIdAlreadyBound
                );

                agent.atlas_id = Some(atlas_id);
                AtlasToAgent::<T>::insert(&atlas_id, agent_id);

                Self::deposit_event(Event::AtlasIdBound { agent_id, atlas_id });

                Ok(())
            })
        }

        /// Update agent operator.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::update_operator())]
        pub fn update_operator(
            origin: OriginFor<T>,
            agent_id: AgentId,
            new_operator: T::AccountId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            Agents::<T>::try_mutate(agent_id, |maybe_agent| -> DispatchResult {
                let agent = maybe_agent.as_mut().ok_or(Error::<T>::AgentNotFound)?;
                ensure!(agent.controller == who, Error::<T>::NotController);
                ensure!(
                    agent.status == AgentStatus::Active,
                    Error::<T>::AgentNotActive
                );
                ensure!(
                    !OperatorAgent::<T>::contains_key(&new_operator),
                    Error::<T>::NotAuthorized
                );

                let old_operator = sp_std::mem::replace(&mut agent.operator, new_operator.clone());
                OperatorAgent::<T>::remove(&old_operator);
                OperatorAgent::<T>::insert(&new_operator, agent_id);

                Self::deposit_event(Event::OperatorChanged {
                    agent_id,
                    old_operator,
                    new_operator,
                });

                Ok(())
            })
        }

        /// Update agent permissions.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::update_permissions())]
        pub fn update_permissions(
            origin: OriginFor<T>,
            agent_id: AgentId,
            permissions: AgentPermissions,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let agent = Agents::<T>::get(agent_id).ok_or(Error::<T>::AgentNotFound)?;
            ensure!(agent.controller == who, Error::<T>::NotController);

            Permissions::<T>::insert(agent_id, permissions.clone());

            Self::deposit_event(Event::PermissionsUpdated {
                agent_id,
                permissions,
            });

            Ok(())
        }

        /// Update agent quota (admin only).
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::update_quota())]
        pub fn update_quota(
            origin: OriginFor<T>,
            agent_id: AgentId,
            gas_per_block: u128,
            compute_per_block: u128,
            gas_per_epoch: u128,
            compute_per_epoch: u128,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            ensure!(
                Agents::<T>::contains_key(agent_id),
                Error::<T>::AgentNotFound
            );

            let current_block = frame_system::Pallet::<T>::block_number();
            let quota = AgentQuota {
                gas_per_block,
                compute_per_block,
                gas_per_epoch,
                compute_per_epoch,
                epoch_start: current_block,
            };

            Quotas::<T>::insert(agent_id, quota.clone());

            Self::deposit_event(Event::QuotaUpdated { agent_id, quota });
            Ok(())
        }

        /// Suspend an agent.
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::suspend_agent())]
        pub fn suspend_agent(
            origin: OriginFor<T>,
            agent_id: AgentId,
            reason: BoundedVec<u8, ConstU32<256>>,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            Agents::<T>::try_mutate(agent_id, |maybe_agent| -> DispatchResult {
                let agent = maybe_agent.as_mut().ok_or(Error::<T>::AgentNotFound)?;
                let old_status = agent.status;
                ensure!(
                    old_status == AgentStatus::Active,
                    Error::<T>::InvalidStatusTransition
                );

                agent.status = AgentStatus::Suspended;
                ActiveAgents::<T>::mutate(|n| *n = n.saturating_sub(1));

                Self::deposit_event(Event::AgentStatusChanged {
                    agent_id,
                    old_status,
                    new_status: AgentStatus::Suspended,
                });
                Self::deposit_event(Event::AgentSuspended { agent_id, reason });

                Ok(())
            })
        }

        /// Reactivate a suspended agent.
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::reactivate_agent())]
        pub fn reactivate_agent(origin: OriginFor<T>, agent_id: AgentId) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            Agents::<T>::try_mutate(agent_id, |maybe_agent| -> DispatchResult {
                let agent = maybe_agent.as_mut().ok_or(Error::<T>::AgentNotFound)?;
                let old_status = agent.status;
                ensure!(
                    old_status == AgentStatus::Suspended,
                    Error::<T>::InvalidStatusTransition
                );

                agent.status = AgentStatus::Active;
                agent.last_active = frame_system::Pallet::<T>::block_number();
                ActiveAgents::<T>::mutate(|n| *n = n.saturating_add(1));

                Self::deposit_event(Event::AgentStatusChanged {
                    agent_id,
                    old_status,
                    new_status: AgentStatus::Active,
                });

                Ok(())
            })
        }

        /// Terminate an agent (permanent).
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::terminate_agent())]
        pub fn terminate_agent(origin: OriginFor<T>, agent_id: AgentId) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let agent = Agents::<T>::get(agent_id).ok_or(Error::<T>::AgentNotFound)?;
            ensure!(agent.controller == who, Error::<T>::NotController);
            ensure!(
                agent.status != AgentStatus::Terminated,
                Error::<T>::AgentTerminated
            );

            let was_active = agent.status == AgentStatus::Active;

            T::Currency::unreserve(&agent.controller, agent.deposit);
            OperatorAgent::<T>::remove(&agent.operator);

            ControllerAgents::<T>::mutate(&agent.controller, |agents| {
                agents.retain(|&id| id != agent_id);
            });

            Agents::<T>::mutate(agent_id, |maybe_agent| {
                if let Some(a) = maybe_agent {
                    a.status = AgentStatus::Terminated;
                }
            });

            Quotas::<T>::remove(agent_id);
            Permissions::<T>::remove(agent_id);
            Activity::<T>::remove(agent_id);

            if was_active {
                ActiveAgents::<T>::mutate(|n| *n = n.saturating_sub(1));
            }

            Self::deposit_event(Event::AgentTerminated { agent_id });
            Ok(())
        }

        // ====================================================================
        // Extrinsics — Policy Enforcement
        // ====================================================================

        /// Register or update policies for an agent.
        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::register_policy())]
        pub fn register_policy(
            origin: OriginFor<T>,
            agent: T::AccountId,
            policies: Vec<PolicyRule<T::AccountId>>,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            ensure!(policies.len() <= 16, Error::<T>::TooManyPolicies);

            let policy_count = policies.len() as u32;
            let bounded_policies: BoundedVec<PolicyRule<T::AccountId>, ConstU32<16>> = policies
                .try_into()
                .map_err(|_| Error::<T>::TooManyPolicies)?;

            ActivePolicies::<T>::insert(&agent, bounded_policies);
            ViolationCount::<T>::insert(&agent, 0);

            Self::deposit_event(Event::PolicyRegistered {
                agent,
                policy_count,
            });

            Ok(())
        }

        /// Remove agent from blacklist.

        #[pallet::call_index(9)]
        #[pallet::weight(T::WeightInfo::remove_blacklist())]
        pub fn remove_blacklist(origin: OriginFor<T>, agent: T::AccountId) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            Blacklist::<T>::remove(&agent);
            Ok(())
        }

        // ====================================================================
        // Extrinsics — Staking
        // ====================================================================

        /// Post a bond to participate in execution.
        #[pallet::call_index(10)]
        #[pallet::weight(T::WeightInfo::post_bond())]
        pub fn post_bond(
            origin: OriginFor<T>,
            amount: BalanceOf<T>,
            intent_id: Option<H256>,
        ) -> DispatchResult {
            let agent = ensure_signed(origin)?;
            ensure!(amount >= T::MinBondAmount::get(), Error::<T>::BondTooSmall);

            T::Currency::reserve(&agent, amount).map_err(|_| Error::<T>::InsufficientFunds)?;

            let bond_counter = BondIdCounter::<T>::get();
            let bond_hash = T::Hashing::hash_of(&(agent.clone(), bond_counter));
            let bond_id: H256 = bond_hash.using_encoded(|b| H256::from_slice(b));
            BondIdCounter::<T>::put(bond_counter.saturating_add(1));

            let now: u32 = frame_system::Pallet::<T>::block_number().saturated_into::<u32>();
            let expires_at = now.saturating_add(T::FinalityWindow::get().saturated_into::<u32>());

            let bond_state = AgentBond {
                bond_id,
                agent: agent.clone(),
                amount,
                posted_at: now,
                expires_at,
                intent_id,
                status: BondStatus::Active,
            };

            Bonds::<T>::insert(bond_id, bond_state);
            BondsByAgent::<T>::mutate(&agent, |bonds| {
                let _ = bonds.try_push(bond_id);
            });

            // Update economics
            AgentEconomicsStore::<T>::mutate(
                Self::agent_id_for_account(&agent).unwrap_or(0),
                |econ| {
                    econ.current_bonded = econ.current_bonded.saturating_add(amount);
                },
            );

            Self::deposit_event(Event::BondPosted {
                bond_id,
                agent,
                amount,
                expires_at: expires_at.into(),
            });

            Ok(())
        }

        /// Release a bond after successful execution.
        #[pallet::call_index(11)]
        #[pallet::weight(T::WeightInfo::release_bond())]
        pub fn release_bond(origin: OriginFor<T>, bond_id: H256) -> DispatchResult {
            ensure_root(origin)?;

            let bond_state = Bonds::<T>::get(bond_id).ok_or(Error::<T>::BondNotFound)?;
            ensure!(
                matches!(bond_state.status, BondStatus::Active),
                Error::<T>::InvalidBondState
            );

            T::Currency::unreserve(&bond_state.agent, bond_state.amount);

            let mut bond_state = bond_state;
            bond_state.status = BondStatus::Released;
            Bonds::<T>::insert(bond_id, bond_state.clone());

            // Update economics
            AgentEconomicsStore::<T>::mutate(
                Self::agent_id_for_account(&bond_state.agent).unwrap_or(0),
                |econ| {
                    econ.current_bonded = econ.current_bonded.saturating_sub(bond_state.amount);
                    econ.total_rewards = econ.total_rewards.saturating_add(bond_state.amount);
                    econ.successful_tasks = econ.successful_tasks.saturating_add(1);
                },
            );

            Self::deposit_event(Event::BondReleased {
                bond_id,
                agent: bond_state.agent,
                amount: bond_state.amount,
            });

            Ok(())
        }

        /// Execute a slash on an agent's bond.
        #[pallet::call_index(12)]
        #[pallet::weight(T::WeightInfo::slash_bond())]
        pub fn slash_bond(
            origin: OriginFor<T>,
            bond_id: H256,
            severity: u8,
            reason: Vec<u8>,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let bond_state = Bonds::<T>::get(bond_id).ok_or(Error::<T>::BondNotFound)?;
            ensure!(
                matches!(bond_state.status, BondStatus::Active),
                Error::<T>::InvalidBondState
            );
            ensure!(severity <= 3, Error::<T>::ArithmeticError);

            // Calculate slash amount based on severity
            let slash_bps: u32 = match severity {
                0 => 100,   // Minor: 1%
                1 => 500,   // Moderate: 5%
                2 => 2500,  // Major: 25%
                3 => 10000, // Critical: 100%
                _ => 2500,
            };

            let slash_amount: BalanceOf<T> =
                (bond_state.amount / 10000u32.into()) * (slash_bps as u32).into();

            // Unreserve and transfer slashed amount to treasury
            T::Currency::unreserve(&bond_state.agent, bond_state.amount);
            let recipient = T::SlashRecipient::get();
            T::Currency::transfer(
                &bond_state.agent,
                &recipient,
                slash_amount,
                ExistenceRequirement::AllowDeath,
            )
            .map_err(|_| Error::<T>::ArithmeticError)?;

            // Record slash
            let slash_id = SlashIdCounter::<T>::get();
            SlashIdCounter::<T>::set(slash_id.saturating_add(1));

            let bounded_reason: BoundedVec<u8, ConstU32<256>> =
                reason.try_into().map_err(|_| Error::<T>::ArithmeticError)?;

            let slash_record = SlashRecord {
                slash_id,
                agent: bond_state.agent.clone(),
                bond_id,
                severity,
                amount_slashed: slash_amount.saturated_into::<u128>(),
                reason: bounded_reason,
                slashed_at: frame_system::Pallet::<T>::block_number().saturated_into::<u32>(),
            };
            SlashRecords::<T>::insert(slash_id, slash_record);

            // Update bond status
            let mut bond_state = bond_state;
            bond_state.status = BondStatus::FullySlashed;
            Bonds::<T>::insert(bond_id, bond_state.clone());

            // Emit accounting event
            T::AccountingSpine::emit(AccountingEvent::fee_collected(
                RevenueModule::Other([0u8; 32]),
                0u32,
                0u32,
                bond_state.amount.saturated_into::<u128>(),
                slash_amount.saturated_into::<u128>(),
                FeeDestination::ProtocolTreasury,
                bond_id.to_fixed_bytes(),
                frame_system::Pallet::<T>::block_number().saturated_into::<u64>(),
            ));

            // Apply reputation damage if critical
            if severity == 3 && T::ReputationDamageEnabled::get() {
                ReputationScores::<T>::mutate(&bond_state.agent, |rep| {
                    *rep = rep.saturating_sub(100);
                });
                Self::deposit_event(Event::ReputationDamaged {
                    agent: bond_state.agent.clone(),
                    damage: -100,
                });
            }

            // Track slashed amount for epoch
            SlashedThisEpoch::<T>::mutate(&bond_state.agent, |total| {
                *total = total.saturating_add(slash_amount);
            });

            // Update agent economics
            if let Some(agent_id) = Self::agent_id_for_account(&bond_state.agent) {
                AgentEconomicsStore::<T>::mutate(agent_id, |econ| {
                    econ.total_slashed = econ.total_slashed.saturating_add(slash_amount);
                    econ.current_bonded = econ.current_bonded.saturating_sub(bond_state.amount);
                    econ.failed_tasks = econ.failed_tasks.saturating_add(1);
                });
            }

            Self::deposit_event(Event::SlashExecuted {
                slash_id,
                agent: bond_state.agent,
                bond_id,
                severity,
                amount_slashed: slash_amount,
            });

            Ok(())
        }

        // ====================================================================
        // Extrinsics — Economics & Activity
        // ====================================================================

        /// Record resource consumption for an agent.
        #[pallet::call_index(13)]
        #[pallet::weight(T::WeightInfo::record_consumption())]
        pub fn record_consumption(
            origin: OriginFor<T>,
            agent_id: AgentId,
            gas_used: u128,
            compute_used: u128,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;

            let agent = Agents::<T>::get(agent_id).ok_or(Error::<T>::AgentNotFound)?;
            ensure!(
                agent.status == AgentStatus::Active,
                Error::<T>::AgentNotActive
            );

            let mut activity = Activity::<T>::get(agent_id);
            activity.gas_used_block = activity.gas_used_block.saturating_add(gas_used);
            activity.compute_used_block = activity.compute_used_block.saturating_add(compute_used);
            activity.gas_used_epoch = activity.gas_used_epoch.saturating_add(gas_used);
            activity.compute_used_epoch = activity.compute_used_epoch.saturating_add(compute_used);
            activity.total_actions = activity.total_actions.saturating_add(1);

            // Check quota limits BEFORE inserting (activity is still owned here)
            let quota = Quotas::<T>::get(agent_id).ok_or(Error::<T>::QuotaExceeded)?;
            ensure!(
                activity.gas_used_block <= quota.gas_per_block,
                Error::<T>::QuotaExceeded
            );
            ensure!(
                activity.compute_used_block <= quota.compute_per_block,
                Error::<T>::QuotaExceeded
            );
            ensure!(
                activity.gas_used_epoch <= quota.gas_per_epoch,
                Error::<T>::QuotaExceeded
            );
            ensure!(
                activity.compute_used_epoch <= quota.compute_per_epoch,
                Error::<T>::QuotaExceeded
            );

            // Now insert activity (after quota checks pass)
            Activity::<T>::insert(agent_id, activity);

            // Update last active
            Agents::<T>::mutate(agent_id, |maybe_agent| {
                if let Some(a) = maybe_agent {
                    a.last_active = frame_system::Pallet::<T>::block_number();
                }
            });

            Self::deposit_event(Event::ResourceConsumed {
                agent_id,
                gas_used,
                compute_used,
            });

            Ok(())
        }

        /// Update agent reputation score.
        #[pallet::call_index(14)]
        #[pallet::weight(T::WeightInfo::update_reputation())]
        pub fn update_reputation(
            origin: OriginFor<T>,
            agent_id: AgentId,
            new_score: u32,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            Agents::<T>::try_mutate(agent_id, |maybe_agent| -> DispatchResult {
                let agent = maybe_agent.as_mut().ok_or(Error::<T>::AgentNotFound)?;
                let old_score = agent.reputation;
                ensure!(new_score <= 200, Error::<T>::ReputationOutOfBounds);

                agent.reputation = new_score;

                Self::deposit_event(Event::ReputationChanged {
                    agent_id,
                    old_score,
                    new_score,
                });

                Ok(())
            })
        }

        /// Distribute rewards to an agent.
        #[pallet::call_index(15)]
        #[pallet::weight(T::WeightInfo::distribute_rewards())]
        pub fn distribute_rewards(
            origin: OriginFor<T>,
            agent_id: AgentId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            let agent = Agents::<T>::get(agent_id).ok_or(Error::<T>::AgentNotFound)?;
            ensure!(
                agent.status == AgentStatus::Active,
                Error::<T>::AgentNotActive
            );

            // Transfer rewards from treasury to agent controller
            let treasury = T::SlashRecipient::get();
            T::Currency::transfer(
                &treasury,
                &agent.controller,
                amount,
                ExistenceRequirement::AllowDeath,
            )
            .map_err(|_| Error::<T>::ArithmeticError)?;

            // Update economics
            AgentEconomicsStore::<T>::mutate(agent_id, |econ| {
                econ.total_rewards = econ.total_rewards.saturating_add(amount);
                econ.successful_tasks = econ.successful_tasks.saturating_add(1);
            });

            Self::deposit_event(Event::RewardsDistributed { agent_id, amount });

            Ok(())
        }

        /// Emit an agent action event.
        #[pallet::call_index(16)]
        #[pallet::weight(T::WeightInfo::emit_action())]
        pub fn emit_action(
            origin: OriginFor<T>,
            agent_id: AgentId,
            action_type: ActionType,
            data: BoundedVec<u8, ConstU32<512>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let agent = Agents::<T>::get(agent_id).ok_or(Error::<T>::AgentNotFound)?;
            ensure!(
                agent.controller == who || agent.operator == who,
                Error::<T>::NotAuthorized
            );

            Self::deposit_event(Event::AgentAction {
                agent_id,
                action_type,
                data,
            });

            Ok(())
        }

        // ====================================================================
        // Extrinsics — Reward Pool
        // ====================================================================

        /// Set the reward configuration for proof verification (admin only).
        #[pallet::call_index(17)]
        #[pallet::weight(T::WeightInfo::set_proof_reward())]
        pub fn set_proof_reward(
            origin: OriginFor<T>,
            config: ProofRewardConfig<BalanceOf<T>>,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            ProofRewardConfigStore::<T>::put(&config);
            Self::deposit_event(Event::ProofRewardConfigUpdated { config });
            Ok(())
        }

        /// Fund the reward pool from any account.
        #[pallet::call_index(18)]
        #[pallet::weight(T::WeightInfo::fund_reward_pool())]
        pub fn fund_reward_pool(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let treasury = T::SlashRecipient::get();
            T::Currency::transfer(&who, &treasury, amount, ExistenceRequirement::AllowDeath)
                .map_err(|_| Error::<T>::ArithmeticError)?;
            TotalRewardPool::<T>::mutate(|total| *total = total.saturating_add(amount));
            let new_total = TotalRewardPool::<T>::get();
            Self::deposit_event(Event::RewardPoolFunded {
                from: who,
                amount,
                new_total,
            });
            Ok(())
        }

        /// Claim accumulated rewards for an agent.
        #[pallet::call_index(19)]
        #[pallet::weight(T::WeightInfo::claim_rewards())]
        pub fn claim_rewards(origin: OriginFor<T>, agent_id: AgentId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let agent = Agents::<T>::get(agent_id).ok_or(Error::<T>::AgentNotFound)?;
            ensure!(agent.controller == who, Error::<T>::NotController);

            let reward = AgentRewardPool::<T>::get(agent_id);
            ensure!(reward > Zero::zero(), Error::<T>::NoRewardsToClaim);

            let treasury = T::SlashRecipient::get();
            T::Currency::transfer(&treasury, &who, reward, ExistenceRequirement::AllowDeath)
                .map_err(|_| Error::<T>::RewardPoolInsufficient)?;

            AgentRewardPool::<T>::remove(agent_id);
            TotalRewardPool::<T>::mutate(|total| *total = total.saturating_sub(reward));

            // Update economics
            AgentEconomicsStore::<T>::mutate(agent_id, |econ| {
                econ.total_rewards = econ.total_rewards.saturating_add(reward);
            });

            Self::deposit_event(Event::RewardsClaimed {
                agent_id,
                recipient: who,
                amount: reward,
            });
            Ok(())
        }
    }

    // ========================================================================
    // Internal Helpers
    // ========================================================================

    impl<T: Config> Pallet<T> {
        /// Start a new epoch, resetting per-epoch counters.
        pub fn start_new_epoch(block: BlockNumberFor<T>) {
            let epoch = CurrentEpoch::<T>::get().saturating_add(1);
            CurrentEpoch::<T>::put(epoch);
            LastEpochBlock::<T>::put(block);

            // Reset per-epoch activity for all agents
            let agent_ids: Vec<AgentId> = Agents::<T>::iter_keys().collect();
            for agent_id in agent_ids {
                Activity::<T>::mutate(agent_id, |activity| {
                    activity.gas_used_epoch = 0;
                    activity.compute_used_epoch = 0;
                });
            }

            // Reset extrinsic counts
            let accounts: Vec<T::AccountId> = ExtrinsicCountThisEpoch::<T>::iter_keys().collect();
            for account in accounts {
                ExtrinsicCountThisEpoch::<T>::remove(&account);
            }

            // Reset slashed this epoch
            let slashed_accounts: Vec<T::AccountId> = SlashedThisEpoch::<T>::iter_keys().collect();
            for account in slashed_accounts {
                SlashedThisEpoch::<T>::remove(&account);
            }

            Self::deposit_event(Event::EpochStarted { epoch, block });
        }

        /// Check if an account has a specific permission.
        pub fn has_permission(agent_id: AgentId, permission: PermissionType) -> bool {
            let perms = Permissions::<T>::get(agent_id);
            match permission {
                PermissionType::Deploy => perms.can_deploy,
                PermissionType::Stake => perms.can_stake,
                PermissionType::Vote => perms.can_vote,
                PermissionType::Trade => perms.can_trade,
                PermissionType::Transfer => perms.can_transfer,
                PermissionType::CallContracts => perms.can_call_contracts,
                PermissionType::SubmitProofs => perms.can_submit_proofs,
                PermissionType::Validate => perms.can_validate,
            }
        }

        /// Get agent ID for an account (by operator mapping).
        pub fn agent_id_for_account(account: &T::AccountId) -> Option<AgentId> {
            OperatorAgent::<T>::get(account)
        }

        /// Get full agent state for runtime API.
        pub fn get_agent_state(
            agent_id: AgentId,
        ) -> Option<AgentFullState<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>> {
            let record = Agents::<T>::get(agent_id)?;
            let quota = Quotas::<T>::get(agent_id).unwrap_or_default();
            let permissions = Permissions::<T>::get(agent_id);
            let activity = Activity::<T>::get(agent_id);
            let economics = AgentEconomicsStore::<T>::get(agent_id);

            let bonds: Vec<AgentBond<T::AccountId, BalanceOf<T>>> =
                BondsByAgent::<T>::get(&record.controller)
                    .iter()
                    .filter_map(|bond_id| Bonds::<T>::get(bond_id))
                    .collect();

            let policies = ActivePolicies::<T>::get(&record.controller);

            Some(AgentFullState {
                record,
                quota,
                permissions,
                activity,
                bonds,
                economics,
                policies: policies.to_vec(),
            })
        }

        /// Reset per-block activity counters.
        pub fn reset_block_activity() {
            let agent_ids: Vec<AgentId> = Agents::<T>::iter_keys().collect();
            for agent_id in agent_ids {
                Activity::<T>::mutate(agent_id, |activity| {
                    activity.gas_used_block = 0;
                    activity.compute_used_block = 0;
                });
            }
        }

        /// Check all policies for an agent.
        pub fn check_policies(agent: &T::AccountId) -> Result<(), DispatchError> {
            let policies = ActivePolicies::<T>::get(agent);
            for policy in policies.iter() {
                match policy {
                    PolicyRule::ReputationMinimum(min) => {
                        let rep = ReputationScores::<T>::get(agent);
                        ensure!((rep as u64) >= *min, Error::<T>::ReputationBelowMinimum);
                    }
                    PolicyRule::MaxTasksPerBlock(max_tasks) => {
                        let current_block = frame_system::Pallet::<T>::block_number();
                        let tasks = TasksThisBlock::<T>::get((current_block, agent.clone()));
                        ensure!(tasks < *max_tasks, Error::<T>::MaxTasksPerBlockExceeded);
                    }
                    PolicyRule::NoCollusionWith(blocked) => {
                        // Check if agent is interacting with any blocked account
                        // This is a simplified check — full implementation requires
                        // inspecting the call target
                        for blocked_account in blocked.iter() {
                            ensure!(agent != blocked_account, Error::<T>::CollusionAttempted);
                        }
                    }
                    PolicyRule::RateLimit(max_extrinsics) => {
                        let count = ExtrinsicCountThisEpoch::<T>::get(agent);
                        ensure!(count < *max_extrinsics, Error::<T>::RateLimitExceeded);
                    }
                    PolicyRule::CapabilityAllowed(_) => {
                        // Capability check is done at the call level
                        // by the SignedExtension
                    }
                }
            }
            Ok(())
        }

        /// Check rate limit for an agent.
        pub fn check_rate_limit(agent: &T::AccountId) -> DispatchResult {
            let count = ExtrinsicCountThisEpoch::<T>::get(agent);
            let max_extrinsics = T::RateLimitMaxExtrinsicsPerEpoch::get();
            ensure!(count < max_extrinsics, Error::<T>::RateLimitExceeded);
            ExtrinsicCountThisEpoch::<T>::insert(agent, count.saturating_add(1));
            Ok(())
        }

        /// Check if an agent is blacklisted.
        pub fn check_blacklist(agent: &T::AccountId) -> DispatchResult {
            if let Some(expires_at) = Blacklist::<T>::get(agent) {
                let current_block = frame_system::Pallet::<T>::block_number();
                ensure!(current_block > expires_at, Error::<T>::AgentBlacklisted);
                // Blacklist expired, clean it up
                Blacklist::<T>::remove(agent);
            }
            Ok(())
        }

        /// Internal slash — reduce reputation and apply penalty.
        pub fn internal_slash(
            agent: &T::AccountId,
            penalty: u64,
            reason: &SlashingReason,
        ) -> DispatchResult {
            ReputationScores::<T>::mutate(agent, |rep| {
                *rep = rep.saturating_sub(penalty as i64);
            });

            log::warn!(
                target: "x3-agent-registry",
                "Agent {:?} slashed for {:?} (penalty: {})",
                agent, reason, penalty
            );

            Ok(())
        }

        /// Blacklist an agent for a duration.
        pub fn blacklist_agent(
            agent: &T::AccountId,
            duration: BlockNumberFor<T>,
        ) -> DispatchResult {
            let current_block = frame_system::Pallet::<T>::block_number();
            let expires_at = current_block.saturating_add(duration);
            Blacklist::<T>::insert(agent, expires_at);
            Ok(())
        }

        /// Calculate penalty based on slashing reason.
        pub fn calculate_penalty(reason: &SlashingReason) -> u64 {
            match reason {
                SlashingReason::InvalidProof => 500,
                SlashingReason::TaskGriefing => 200,
                SlashingReason::CollusionDetected => 800,
                SlashingReason::PolicyViolation => 350,
                SlashingReason::RepeatOffender => 1200,
                SlashingReason::BondExpired => 100,
            }
        }

        /// Reward an agent for proof verification (called by proof-carrying agent pallet).
        pub fn reward_agent_for_proof(
            agent_id: AgentId,
            reason: BoundedVec<u8, ConstU32<64>>,
        ) -> DispatchResult {
            let config = ProofRewardConfigStore::<T>::get();
            ensure!(config.enabled, Error::<T>::PermissionDenied);

            // Check agent exists BEFORE mutating any storage
            let agent = Agents::<T>::get(agent_id).ok_or(Error::<T>::AgentNotFound)?;

            let total_reward = config.base_reward.saturating_add(config.verification_bonus);
            let pool_balance = TotalRewardPool::<T>::get();
            ensure!(
                pool_balance >= total_reward,
                Error::<T>::RewardPoolInsufficient
            );

            AgentRewardPool::<T>::mutate(agent_id, |balance| {
                *balance = balance.saturating_add(total_reward);
            });

            // Record distribution history
            let distribution = RewardDistribution {
                agent_id,
                recipient: agent.controller.clone(),
                amount: total_reward,
                block: frame_system::Pallet::<T>::block_number(),
                reason: reason.clone(),
            };
            RewardDistributionHistory::<T>::mutate(|history| {
                let _ = history.try_push(distribution);
            });

            Self::deposit_event(Event::ProofRewardDistributed {
                agent_id,
                amount: total_reward,
                reason,
            });

            Ok(())
        }
    }
}
