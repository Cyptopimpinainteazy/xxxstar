#![deny(unsafe_code)]
//! # Evolution Core Pallet
//!
//! The brain of the Adaptive Intelligence Chain (AIC). This pallet enables
//! X3 X3 Chain to evolve its runtime based on network conditions,
//! usage patterns, MEV pressure, and AI-generated optimizations.
//!
//! ## Overview
//!
//! The Evolution Core implements:
//! - **Metrics Collection**: Gathers network telemetry per block
//! - **AI Analysis**: Processes metrics to identify optimization opportunities
//! - **Mutation Queue**: Stages proposed runtime modifications
//! - **Governance Integration**: Approval workflow for significant changes
//! - **Auto-Evolution**: Automatic parameter tuning within safe bounds
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
//! │   Metrics   │───▶│  Analyzer   │───▶│  Mutation   │
//! │  Collector  │    │  (AI/ML)    │    │   Queue     │
//! └─────────────┘    └─────────────┘    └─────────────┘
//!                                              │
//!                    ┌─────────────┐           │
//!                    │  Governance │◀──────────┘
//!                    │  Approval   │
//!                    └──────┬──────┘
//!                           │
//!                           ▼
//!                    ┌─────────────┐
//!                    │   Runtime   │
//!                    │   Mutate    │
//!                    └─────────────┘
//! ```
//!
//! ## Security
//!
//! - All mutations are sandboxed and simulated before application
//! - Critical parameters require governance approval
//! - Auto-evolution bounded by safety limits
//! - Automatic rollback on failure detection

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

pub mod runtime_api;
pub use runtime_api::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::{Get, StorageVersion},
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::{DispatchError, Percent, Saturating};
    use sp_std::vec::Vec;

    /// Current storage version
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    // ============================================================================
    // Types
    // ============================================================================

    /// Metric identifier
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq)]
    pub enum MetricId {
        /// Gas used in block
        BlockGasUsed,
        /// EVM transaction count
        EvmCallCount,
        /// SVM transaction count
        SvmCallCount,
        /// Cross-VM operations ratio
        CrossVmRatio,
        /// Mempool depth
        MempoolDepth,
        /// MEV pressure indicator (0-100)
        MevPressure,
        /// Validator throughput
        ValidatorThroughput,
        /// X3 hotpath compilation triggers
        X3HotpathHits,
        /// DEX swap volume
        SwapVolume,
        /// Flashloan utilization
        FlashloanVolume,
        /// Custom metric
        Custom(u32),
    }

    /// Mutation type
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq)]
    pub enum MutationType {
        /// Adjust gas parameters
        GasParameter { opcode: u8, new_cost: u64 },
        /// Modify base fee
        BaseFee { new_base: u128 },
        /// VM load balancing threshold
        VmLoadBalance {
            evm_weight: Percent,
            svm_weight: Percent,
        },
        /// JIT compilation threshold
        JitThreshold { new_threshold: u32 },
        /// Block time adjustment (within bounds)
        BlockTime { new_millis: u64 },
        /// Custom parameter
        CustomParam { param_id: u32, value: u128 },
    }

    /// Mutation proposal
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq)]
    #[scale_info(skip_type_params(T))]
    pub struct MutationProposal<T: Config> {
        /// Unique proposal ID
        pub id: u64,
        /// Mutation to apply
        pub mutation: MutationType,
        /// Proposer (AI agent or validator)
        pub proposer: T::AccountId,
        /// Reason/justification
        pub reason: BoundedVec<u8, T::MaxReasonLength>,
        /// Expected improvement (0-100)
        pub expected_improvement: u8,
        /// Simulation passed
        pub simulation_passed: bool,
        /// Approval count
        pub approvals: u32,
        /// Block proposed
        pub proposed_at: BlockNumberFor<T>,
        /// Status
        pub status: ProposalStatus,
    }

    /// Proposal status
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq, Default)]
    pub enum ProposalStatus {
        #[default]
        Pending,
        Approved,
        Applied,
        Rejected,
        Rolled,
    }

    /// Block metrics snapshot
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Default, PartialEq, Eq)]
    pub struct BlockMetrics {
        pub gas_used: u128,
        pub evm_calls: u32,
        pub svm_calls: u32,
        pub cross_vm_calls: u32,
        pub mempool_depth: u32,
        pub mev_pressure: u8,
        pub x3_hotpath_hits: u32,
        pub swap_volume: u128,
        pub flashloan_volume: u128,
    }

    /// Runtime parameters that can be evolved
    /// Note: evm_weight_pct and svm_weight_pct are stored as u8 (0-100) for serde compatibility
    #[derive(
        Clone,
        Encode,
        Decode,
        TypeInfo,
        MaxEncodedLen,
        Debug,
        PartialEq,
        Eq,
        serde::Serialize,
        serde::Deserialize,
    )]
    pub struct EvolvableParams {
        /// Base gas price multiplier (100 = 1x)
        pub gas_multiplier: u32,
        /// EVM execution weight percentage (0-100)
        pub evm_weight_pct: u8,
        /// SVM execution weight percentage (0-100)
        pub svm_weight_pct: u8,
        /// JIT compilation threshold
        pub jit_threshold: u32,
        /// Max parallel executions
        pub max_parallel: u32,
        /// MEV smoothing factor
        pub mev_smooth_factor: u32,
    }

    impl EvolvableParams {
        /// Get EVM weight as Percent
        pub fn evm_weight(&self) -> Percent {
            Percent::from_percent(self.evm_weight_pct)
        }

        /// Get SVM weight as Percent
        pub fn svm_weight(&self) -> Percent {
            Percent::from_percent(self.svm_weight_pct)
        }
    }

    impl Default for EvolvableParams {
        fn default() -> Self {
            Self {
                gas_multiplier: 100,
                evm_weight_pct: 50,
                svm_weight_pct: 50,
                jit_threshold: 100,
                max_parallel: 4,
                mev_smooth_factor: 10,
            }
        }
    }

    // ============================================================================
    // Config
    // ============================================================================

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Runtime event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Origin that can propose mutations (AI agents, authorized validators)
        type EvolutionAuthority: EnsureOrigin<Self::RuntimeOrigin>;

        /// Origin for emergency actions
        type EmergencyOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Minimum approval quorum for mutations (e.g., 66%)
        #[pallet::constant]
        type MinApprovalQuorum: Get<Percent>;

        /// Maximum pending proposals
        #[pallet::constant]
        type MaxPendingProposals: Get<u32>;

        /// Maximum reason length
        #[pallet::constant]
        type MaxReasonLength: Get<u32>;

        /// Blocks before proposal expires
        #[pallet::constant]
        type ProposalLifetime: Get<BlockNumberFor<Self>>;

        /// Metrics history depth
        #[pallet::constant]
        type MetricsHistoryDepth: Get<u32>;

        /// Auto-evolution bounds
        #[pallet::constant]
        type AutoEvolutionBounds: Get<(u32, u32)>; // (min%, max%)

        /// Weight information
        type WeightInfo: WeightInfo;
    }

    // ============================================================================
    // Pallet
    // ============================================================================

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    // ============================================================================
    // Storage
    // ============================================================================

    /// Current evolvable parameters
    #[pallet::storage]
    #[pallet::getter(fn current_params)]
    pub type CurrentParams<T: Config> = StorageValue<_, EvolvableParams, ValueQuery>;

    /// Block metrics history
    #[pallet::storage]
    #[pallet::getter(fn metrics_history)]
    pub type MetricsHistory<T: Config> =
        StorageMap<_, Blake2_128Concat, BlockNumberFor<T>, BlockMetrics, OptionQuery>;

    /// Pending mutation proposals
    #[pallet::storage]
    #[pallet::getter(fn proposals)]
    pub type Proposals<T: Config> =
        StorageMap<_, Blake2_128Concat, u64, MutationProposal<T>, OptionQuery>;

    /// Next proposal ID
    #[pallet::storage]
    pub type NextProposalId<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Pending proposal IDs (bounded)
    #[pallet::storage]
    pub type PendingProposals<T: Config> =
        StorageValue<_, BoundedVec<u64, T::MaxPendingProposals>, ValueQuery>;

    /// Applied mutations history (for rollback)
    #[pallet::storage]
    pub type AppliedMutations<T: Config> =
        StorageMap<_, Blake2_128Concat, BlockNumberFor<T>, MutationType, OptionQuery>;

    /// Evolution enabled flag
    #[pallet::storage]
    #[pallet::getter(fn evolution_enabled)]
    pub type EvolutionEnabled<T: Config> = StorageValue<_, bool, ValueQuery>;

    /// Auto-evolution enabled flag
    #[pallet::storage]
    #[pallet::getter(fn auto_evolution_enabled)]
    pub type AutoEvolutionEnabled<T: Config> = StorageValue<_, bool, ValueQuery>;

    /// Total mutations applied
    #[pallet::storage]
    pub type TotalMutationsApplied<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// AI agent approvers
    #[pallet::storage]
    pub type AIAgentApprovers<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

    // ============================================================================
    // Genesis
    // ============================================================================

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        pub evolution_enabled: bool,
        pub auto_evolution_enabled: bool,
        pub initial_params: EvolvableParams,
        pub ai_agents: Vec<T::AccountId>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            EvolutionEnabled::<T>::put(self.evolution_enabled);
            AutoEvolutionEnabled::<T>::put(self.auto_evolution_enabled);
            CurrentParams::<T>::put(self.initial_params.clone());
            for agent in &self.ai_agents {
                AIAgentApprovers::<T>::insert(agent, true);
            }
        }
    }

    // ============================================================================
    // Events
    // ============================================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Metrics recorded for block
        MetricsRecorded {
            block: BlockNumberFor<T>,
            metrics: BlockMetrics,
        },
        /// Mutation proposed
        MutationProposed {
            id: u64,
            proposer: T::AccountId,
            mutation: MutationType,
        },
        /// Mutation approved
        MutationApproved {
            id: u64,
            approver: T::AccountId,
            total_approvals: u32,
        },
        /// Mutation applied to runtime
        MutationApplied { id: u64, mutation: MutationType },
        /// Mutation rejected
        MutationRejected {
            id: u64,
            reason: BoundedVec<u8, T::MaxReasonLength>,
        },
        /// Mutation rolled back
        MutationRolledBack { id: u64, mutation: MutationType },
        /// Auto-evolution triggered
        AutoEvolutionTriggered {
            mutation: MutationType,
            improvement_expected: u8,
        },
        /// Evolution toggled
        EvolutionToggled { enabled: bool },
        /// AI agent registered
        AIAgentRegistered { agent: T::AccountId },
        /// Emergency stop activated
        EmergencyStop,
    }

    // ============================================================================
    // Errors
    // ============================================================================

    #[pallet::error]
    pub enum Error<T> {
        /// Evolution is disabled
        EvolutionDisabled,
        /// Proposal not found
        ProposalNotFound,
        /// Proposal expired
        ProposalExpired,
        /// Already approved by this account
        AlreadyApproved,
        /// Max pending proposals reached
        TooManyProposals,
        /// Not authorized
        NotAuthorized,
        /// Invalid mutation parameters
        InvalidMutation,
        /// Simulation failed
        SimulationFailed,
        /// Auto-evolution bounds exceeded
        BoundsExceeded,
        /// Emergency mode active
        EmergencyModeActive,
        /// Not an AI agent
        NotAIAgent,
    }

    // ============================================================================
    // Hooks
    // ============================================================================

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_finalize(block: BlockNumberFor<T>) {
            // Record metrics at end of block
            Self::record_block_metrics(block);

            // Clean up expired proposals
            Self::cleanup_expired_proposals(block);

            // Check for auto-evolution opportunities
            if Self::auto_evolution_enabled() {
                Self::check_auto_evolution(block);
            }
        }

        fn on_initialize(_block: BlockNumberFor<T>) -> Weight {
            // Pre-block checks could go here
            Weight::zero()
        }
    }

    // ============================================================================
    // Calls
    // ============================================================================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Propose a runtime mutation
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::propose_mutation())]
        pub fn propose_mutation(
            origin: OriginFor<T>,
            mutation: MutationType,
            reason: BoundedVec<u8, T::MaxReasonLength>,
            expected_improvement: u8,
        ) -> DispatchResult {
            let proposer = ensure_signed(origin)?;
            ensure!(Self::evolution_enabled(), Error::<T>::EvolutionDisabled);
            ensure!(
                AIAgentApprovers::<T>::get(&proposer),
                Error::<T>::NotAIAgent
            );

            // Validate mutation
            Self::validate_mutation(&mutation)?;

            // Simulate mutation (basic check)
            let simulation_passed = Self::simulate_mutation(&mutation);

            // Create proposal
            let id = NextProposalId::<T>::get();
            let proposal = MutationProposal {
                id,
                mutation: mutation.clone(),
                proposer: proposer.clone(),
                reason,
                expected_improvement,
                simulation_passed,
                approvals: 1, // Proposer auto-approves
                proposed_at: frame_system::Pallet::<T>::block_number(),
                status: ProposalStatus::Pending,
            };

