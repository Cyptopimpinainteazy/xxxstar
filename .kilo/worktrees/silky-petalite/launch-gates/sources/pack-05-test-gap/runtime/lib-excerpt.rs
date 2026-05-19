#![deny(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]
#![allow(dead_code)]
#![allow(clippy::manual_contains)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::single_component_path_imports)]

// Required for impl_runtime_apis! macro in no_std
#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(all(not(feature = "std"), target_arch = "wasm32"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

use codec::{Decode, Encode};
use sp_std::vec::Vec;
use frame_support::PalletId;
pub use frame_support::{
    construct_runtime, parameter_types,
    traits::{ConstBool, ConstU16, ConstU32, ConstU64, ConstU8, Everything, Get},
    weights::{
        constants::{
            BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
        },
        ConstantMultiplier, IdentityFee, WeightToFee,
    },
};
use frame_support::{traits::Currency, weights::Weight};
use frame_system::limits;
use pallet_agent_accounts;
use pallet_agent_memory;
use pallet_atomic_trade_engine;
use pallet_aura;
use pallet_balances;
use pallet_collective;
use pallet_evolution_core;
use pallet_governance;
use pallet_grandpa;
use pallet_preimage;
use pallet_scheduler;
#[cfg(feature = "dev")]
use pallet_sudo;
use pallet_swarm;
use pallet_timestamp;
use pallet_x3_jury_anchor;
use pallet_transaction_payment::CurrencyAdapter;
use pallet_treasury;
use pallet_x3_atomic_kernel;
use pallet_x3_kernel;
use pallet_x3_settlement_engine;
use pallet_x3_verifier;
use scale_info::TypeInfo;
use sp_api::impl_runtime_apis;
use sp_core::{OpaqueMetadata, H160, H256, U256};
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys,
    traits::{
        AccountIdConversion, AccountIdLookup, BlakeTwo256, Block as BlockT, ConvertInto,
        IdentifyAccount, Verify,
    },
    MultiAddress, MultiSignature, Perbill,
};
use sp_session::{GetSessionNumber, GetValidatorCount};
use sp_std::prelude::*;

mod precompiles;
use precompiles::FrontierPrecompiles;

// ════════════════════════════════════════════════════════════════════════════════════
// GPU Validator Runtime API Types
// ════════════════════════════════════════════════════════════════════════════════════
#[cfg(feature = "gpu-validator")]
pub mod gpu_validator_api {
    use super::AccountId;
    use codec::{Decode, Encode};
    use scale_info::TypeInfo;
    use sp_std::vec::Vec;

    /// GPU validator status response
    #[derive(Debug, Clone, Encode, Decode, TypeInfo)]
    pub struct GpuValidatorStatus {
        /// Validator ID
        pub validator_id: u32,
        /// Health status: "healthy", "degraded", "unhealthy"
        pub health_status: Vec<u8>,
        /// Total proofs processed
        pub total_proofs_processed: u64,
        /// Successful proofs
        pub successful_proofs: u64,
        /// Failed proofs
        pub failed_proofs: u64,
        /// GPU devices online
        pub gpu_devices_online: u32,
        /// CPU fallback active
        pub cpu_fallback_active: bool,
        /// Last health check block
        pub last_health_check_block: u32,
    }

    /// Orchestrator health status
    #[derive(Debug, Clone, Encode, Decode, TypeInfo)]
    pub struct OrchestratorHealthStatus {
        /// Overall status: "operational", "degraded", "error"
        pub status: Vec<u8>,
        /// Uptime seconds
        pub uptime_seconds: u64,
        /// Active validators
        pub active_validators: u32,
        /// Quarantined validators
        pub quarantined_validators: u32,
        /// Pending task count
        pub pending_tasks: u32,
        /// Tasks completed this epoch
        pub tasks_completed: u64,
        /// Average task latency ms
        pub avg_task_latency_ms: u32,
        /// Network health: 0-100
        pub network_health_percent: u8,
    }

    /// GPU proof submission result
    #[derive(Debug, Clone, Encode, Decode, TypeInfo)]
    pub struct GpuProofResult {
        /// Proof hash
        pub proof_hash: [u8; 32],
        /// Status: "accepted", "rejected", "pending"
        pub status: Vec<u8>,
        /// Error message if rejected
        pub error_message: Vec<u8>,
        /// Validator processed by
        pub processed_by_validator: u32,
    }

    sp_api::decl_runtime_apis! {
        /// GPU Validator runtime API for querying validator status and submitting proofs
        pub trait GpuValidatorRuntimeApi {
            /// Get GPU validator status
            fn gpu_validator_status(validator_id: u32) -> Option<GpuValidatorStatus>;
            /// Query orchestrator health
            fn query_orchestrator_health() -> OrchestratorHealthStatus;
            /// Submit GPU validator proof
            fn submit_gpu_validator_proof(proof: Vec<u8>, validator_id: u32) -> GpuProofResult;
        }

        /// Cross-chain header validation and proof aggregation API (Phase 9)
        pub trait CrossChainStateRootApi {
            /// Validate EVM block header and return proof
            fn validate_evm_header(
                block_number: u64,
                block_hash: sp_core::H256,
                state_root: sp_core::H256,
            ) -> Option<crate::cross_chain_state_root_api::EvmHeaderProof>;

            /// Validate SVM (Solana) block header and return proof
            fn validate_svm_header(
                slot: u64,
                block_hash: sp_core::H256,
                state_root: sp_core::H256,
            ) -> Option<crate::cross_chain_state_root_api::SvmHeaderProof>;

            /// Query cross-chain validation status
            fn query_cross_chain_status() -> crate::cross_chain_state_root_api::CrossChainValidationStatus;

            /// Aggregate multiple proofs into a single cross-chain proof
            fn aggregate_cross_chain_proofs(
                proofs: Vec<crate::cross_chain_state_root_api::CrossChainProofBatch>,
            ) -> Option<crate::cross_chain_state_root_api::CrossChainProofBatch>;

            /// Query the last validated EVM header
            fn query_last_evm_header() -> Option<crate::cross_chain_state_root_api::EvmHeaderInfo>;

            /// Query the last validated SVM header
            fn query_last_svm_header() -> Option<crate::cross_chain_state_root_api::SvmHeaderInfo>;

            /// Verify if an EVM merkle root is cached for a block
            fn verify_evm_merkle_root(block_number: u64, merkle_root: sp_core::H256) -> bool;

            /// Verify if an SVM validator set is cached for a slot
            fn verify_svm_validator_set(slot: u64, validator_set_hash: sp_core::H256) -> bool;
        }

        /// Governance-driven settlement finality and dispute resolution API (Phase 10a)
        pub trait GovernanceSettlementApi {
            /// Submit a dispute challenge against a settlement proof
            fn submit_dispute(
                proof_hash: sp_core::H256,
                reason: Vec<u8>,
            ) -> Option<crate::governance_settlement_api::DisputeRecord>;

            /// Query the voting state of an active dispute
            fn query_dispute_status(
                proof_hash: sp_core::H256,
            ) -> Option<crate::governance_settlement_api::DisputeRecord>;

            /// Confirm that a proof has reached settlement finality
            fn confirm_settlement_finality(
                proof_hash: sp_core::H256,
            ) -> Option<crate::governance_settlement_api::ProofFinalityStatus>;
        }

        /// Settlement finality and validator attestation API (Phase 10a)
        pub trait SettlementFinalityApi {
            /// Query finality confirmation metrics
            fn query_finality_metrics() -> crate::governance_settlement_api::FinalityMetrics;

            /// Get validator dispute resolution reputation score
            fn query_validator_reputation(
                validator_id: AccountId,
            ) -> crate::governance_settlement_api::ValidatorReputation;

            /// Check if a merkle-aggregated batch has finality
            fn query_batch_finality_status(
                merkle_root: sp_core::H256,
            ) -> Option<crate::governance_settlement_api::BatchFinalityStatus>;
        }
    }
}

pub mod fraud_proofs;

#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

// WASM binary generated by substrate-wasm-builder
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

// When building for WASM (no-std), provide empty binaries
#[cfg(not(feature = "std"))]
pub const WASM_BINARY: Option<&[u8]> = None;
#[cfg(not(feature = "std"))]
pub const WASM_BINARY_BLOATY: Option<&[u8]> = None;

/// Opaque types used by the CLI commands.
pub mod opaque {
    use super::*;

    pub type BlockNumber = super::BlockNumber;
    pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
    pub type UncheckedExtrinsic = sp_runtime::OpaqueExtrinsic;
    pub type Block = generic::Block<Header, UncheckedExtrinsic>;
    pub type BlockId = generic::BlockId<Block>;
}

pub type BlockNumber = u32;
pub type Index = u32;
/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Nonce = Index;
pub type Signature = MultiSignature;
pub type Hash = H256;
pub type Moment = u64;
pub type Balance = u128;
pub type AssetId = u32;
pub type AtlasId = H256;
pub type Address = MultiAddress<AccountId, ()>;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

pub const MILLISECS_PER_BLOCK: u64 = 200; // 200ms target for higher throughput and lower latency

pub const fn blocks_from_millis(milliseconds: u64) -> BlockNumber {
    (milliseconds / MILLISECS_PER_BLOCK) as BlockNumber
}

pub struct RuntimeVersion;
impl frame_support::traits::Get<sp_version::RuntimeVersion> for RuntimeVersion {
    fn get() -> sp_version::RuntimeVersion {
        VERSION
    }
}
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

pub const NANO_ATLAS: Balance = 1;
pub const MICRO_ATLAS: Balance = 1_000 * NANO_ATLAS;
pub const MILLI_ATLAS: Balance = 1_000 * MICRO_ATLAS;
pub const X3: Balance = 1_000 * MILLI_ATLAS;
pub const NATIVE_GAS_PRICE: u64 = 1_000_000_000;

#[sp_version::runtime_version]
pub const VERSION: sp_version::RuntimeVersion = sp_version::RuntimeVersion {
    spec_name: create_runtime_str!("x3-chain"),
    impl_name: create_runtime_str!("x3-chain"),
    authoring_version: 1,
    // v5: 200ms slot duration migration. Nodes MUST check spec_version to select
    // the correct slot duration for pre/post-upgrade blocks to prevent Aura
    // slot monotonicity failures. See node/src/service.rs slot_duration_for_spec().
    spec_version: 5,
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 1,
    state_version: 1,
};

parameter_types! {
    pub const BlockHashCount: BlockNumber = 2_400;
    pub const SS58Prefix: u16 = 42;
    pub const MinimumPeriod: Moment = (MILLISECS_PER_BLOCK / 2) as Moment;
    pub const ExistentialDeposit: Balance = 100 * MICRO_ATLAS;
    pub const TransactionByteFee: Balance = 10 * MICRO_ATLAS;
    pub const MaxAssetsPerAccount: u32 = 32;
    pub const MaxAssetSymbolLength: u32 = 16;
    pub const MaxPayloadLength: u32 = 128 * 1024;
    pub const MaxEvmPayloadLength: u32 = 64 * 1024;  // 64 KB for EVM payloads
    pub const MaxSvmPayloadLength: u32 = 64 * 1024;  // 64 KB for SVM payloads
    pub const MaxX3PayloadLength: u32 = 64 * 1024;  // 64 KB for X3 payloads
    pub const MaxCombinedPayloadLength: u32 = 128 * 1024;  // 128 KB combined limit
    pub const MaxCombinedPayloadLengthV2: u32 = 192 * 1024;  // 192 KB combined (EVM+SVM+X3)
    pub const MaxAuthorities: u32 = 100;  // Maximum 100 authorities
    pub const MinAuthorities: u32 = 1;  // Minimum 1 authority required
    pub const DefaultEvmGasLimit: u64 = 12_000_000;  // tuned for 200ms slots on commodity validators
    pub const DefaultSvmComputeLimit: u64 = 200_000;  // 200k compute units for SVM
    pub const DefaultX3GasLimit: u64 = 6_000_000;  // tuned for 200ms slots on commodity validators
    pub const CrossVmPrepareTtl: BlockNumber = 50; // 50 blocks (~10s at 200ms)
    pub const MaxPreparedCrossVmOps: u32 = 1024;
    pub const MaxPreparedOpsPerBlock: u32 = 64;
    /// Maximum replay-store entries pruned per block. Bounds
    /// `on_initialize` work by the cross-VM replay-store pruner.
    pub const MaxReplayPruneItemsPerBlock: u32 = 256;
    pub const RequireCrossVmProof: bool = true;
    /// EVM bridge escrow contract address for atomic cross-VM swaps.
    pub BridgeEvmEscrow: H160 = H160([
        0x12, 0x34, 0x56, 0x78, 0x90, 0x12, 0x34, 0x56, 0x78, 0x90,
        0x12, 0x34, 0x56, 0x78, 0x90, 0x12, 0x34, 0x56, 0x78, 0x90,
    ]);
    /// SVM bridge escrow program address for atomic cross-VM swaps.
    pub BridgeSvmEscrow: [u8; 32] = [
        0x58, 0x33, 0x42, 0x72, 0x69, 0x64, 0x67, 0x65, 0x45, 0x73, 0x63, 0x72, 0x6f, 0x77,
        0x31, 0x31, 0x31, 0x31, 0x31, 0x31, 0x31, 0x31, 0x31, 0x31, 0x31, 0x31, 0x31, 0x31,
        0x31, 0x31, 0x31, 0x31,
    ];
    pub BlockWeights: limits::BlockWeights = limits::BlockWeights::with_sensible_defaults(
        // Keep max execution budget below slot time (200ms) to avoid author/import divergence.
        Weight::from_parts((WEIGHT_REF_TIME_PER_SECOND / 1000) * 150, 5 * 1024 * 1024),
        Perbill::from_percent(90),
    );
    pub BlockLength: limits::BlockLength = limits::BlockLength::max_with_normal_ratio(
        5 * 1024 * 1024, // 5MB hard cap to reduce import pressure
        Perbill::from_percent(90),
    );
}

parameter_types! {
    pub const ChainId: u64 = 650_000;
    pub const GasLimitPovSizeRatio: u64 = 40;
    pub WeightPerGas: Weight = Weight::from_parts(20_000, 0);
}

pub struct BlockGasLimit;
impl Get<U256> for BlockGasLimit {
    fn get() -> U256 {
        U256::from(15_000_000u64)
    }
}

pub struct PrecompilesValue;
impl Get<FrontierPrecompiles<Runtime>> for PrecompilesValue {
    fn get() -> FrontierPrecompiles<Runtime> {
        FrontierPrecompiles::new()
    }
}

#[cfg(feature = "std")]
pub fn native_version() -> sp_version::NativeVersion {
    sp_version::NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

parameter_types! {
    pub const MaxSetIdSessionEntries: u64 = 168; // ~1 week at 1 hour sessions
    pub const OperationalFeeMultiplier: u8 = 5;
}

parameter_types! {
    pub const CouncilMotionDuration: BlockNumber = (3 * 24 * 60 * 60 * 1000) / MILLISECS_PER_BLOCK as BlockNumber; // 3 days in ms / block time
    pub const CouncilMaxProposals: u32 = 100;
    pub const CouncilMaxMembers: u32 = 100;
    pub MaxProposalWeight: Weight = Perbill::from_percent(50) * BlockWeights::get().max_block;
}

// ── Fraud-proof pallet constants ─────────────────────────────────────────────
parameter_types! {
    /// Maximum transactions in a single scheduler witness (prevents DoS).
    pub const FraudProofMaxTxCount: u32 = 256;
    /// Blocks within which a fraud proof must be submitted after the disputed block.
    /// At 200 ms/block this is 24 hours (432_000 × 0.2s = 86_400s).
    pub const FraudProofDisputeWindowBlocks: u32 = 432_000;
    /// Reward paid to the reporter on accepted fraud proof (1 ATLAS).
    pub const FraudProofReporterReward: Balance = X3;
}

// ── Sequencer pallet constants ───────────────────────────────────────────────
parameter_types! {
    /// Maximum transactions per sequencer batch.
    pub const SeqMaxTxsPerBatch: u32 = 2048;
    /// Maximum payload size per sequenced transaction (bytes).
    pub const SeqMaxPayloadSize: u32 = 128 * 1024; // 128 KB
    /// Per-byte fee for sequencing (anti-spam).
    pub const SeqPerByteFee: u128 = 10;  // 10 nATLAS per byte
    /// Minimum base fee per transaction.
    pub const SeqBaseFee: u128 = 1_000;  // 1 µATLAS
    /// Enable X3 Atomic Kernel.
    pub const AtomicKernelEnabled: bool = true;
}

/// Runtime constants for performance and safety parameters.
pub const ATOMIC_KERNEL_VERSION: u32 = 1;
pub const ATOMIC_KERNEL_MAX_BATCH_GAS: u64 = 12_000_000; // consistent with DefaultEvmGasLimit

// ── DA pallet constants ──────────────────────────────────────────────────────
parameter_types! {
    /// Maximum blob size for DA (4 MB).
    pub const DaMaxBlobSize: u32 = 4 * 1024 * 1024;
    /// Per-byte fee for DA submissions.
    pub const DaPerByteFee: u128 = 5;  // 5 nATLAS per byte
    /// Maximum shard proofs per blob.
    pub const DaMaxShardProofs: u32 = 128;
    /// DA retention window (blocks) — ~24 hours at 200ms blocks.
    pub const DaRetentionBlocks: BlockNumber = 432_000;
}

#[cfg(feature = "dev")]
construct_runtime!(
    pub enum Runtime {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Aura: pallet_aura,
        Grandpa: pallet_grandpa,
        Session: pallet_session,
        Balances: pallet_balances,
        TransactionPayment: pallet_transaction_payment,
        Scheduler: pallet_scheduler,
        Preimage: pallet_preimage,
        EVM: pallet_evm,
        AtlasKernel: pallet_x3_kernel,
        X3Coin: pallet_x3_coin,
        AtomicTradeEngine: pallet_atomic_trade_engine,
        Council: pallet_collective::<Instance1>,
        Sudo: pallet_sudo,
        Governance: pallet_governance,
        Treasury: pallet_treasury,
        AgentAccounts: pallet_agent_accounts,
        AgentMemory: pallet_agent_memory,
        EvolutionCore: pallet_evolution_core,
        X3Verifier: pallet_x3_verifier,
        X3DomainRegistry: pallet_x3_domain_registry,
        X3JuryAnchor: pallet_x3_jury_anchor,
        X3SettlementEngine: pallet_x3_settlement_engine,
        Swarm: pallet_swarm,
        DepinMarketplace: pallet_depin_marketplace,
        PrivateExecution: pallet_private_execution,
        X3Sequencer: pallet_x3_sequencer,
        FraudProofs: crate::fraud_proofs::pallet::pallet,
        X3Da: pallet_x3_da,
        X3AtomicKernel: pallet_x3_atomic_kernel,
        X3AssetRegistry: pallet_x3_asset_registry,
        X3SupplyLedger: pallet_x3_supply_ledger,
        X3CrossVmRouter: pallet_x3_cross_vm_router,
        X3TokenFactory: pallet_x3_token_factory,
        CrossChainValidator: pallet_cross_chain_validator,
    }
);

#[cfg(not(feature = "dev"))]
construct_runtime!(
    pub enum Runtime {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Aura: pallet_aura,
        Grandpa: pallet_grandpa,
        Session: pallet_session,
        Balances: pallet_balances,
        TransactionPayment: pallet_transaction_payment,
        Scheduler: pallet_scheduler,
        Preimage: pallet_preimage,
        EVM: pallet_evm,
        AtlasKernel: pallet_x3_kernel,
        X3Coin: pallet_x3_coin,
        AtomicTradeEngine: pallet_atomic_trade_engine,
        Council: pallet_collective::<Instance1>,
        Governance: pallet_governance,
        Treasury: pallet_treasury,
        AgentAccounts: pallet_agent_accounts,
        AgentMemory: pallet_agent_memory,
        EvolutionCore: pallet_evolution_core,
        X3Verifier: pallet_x3_verifier,
        X3DomainRegistry: pallet_x3_domain_registry,
        X3JuryAnchor: pallet_x3_jury_anchor,
        X3SettlementEngine: pallet_x3_settlement_engine,
        Swarm: pallet_swarm,
        DepinMarketplace: pallet_depin_marketplace,
        PrivateExecution: pallet_private_execution,
        X3Sequencer: pallet_x3_sequencer,
        X3Da: pallet_x3_da,
        // ISSUE #3 FIX: FraudProofs moved AFTER X3Da to avoid forward reference
        // FraudProofs now reads X3Da state after block execution completes
        FraudProofs: crate::fraud_proofs::pallet::pallet,
        X3AtomicKernel: pallet_x3_atomic_kernel,
        X3AssetRegistry: pallet_x3_asset_registry,
        X3SupplyLedger: pallet_x3_supply_ledger,
        X3CrossVmRouter: pallet_x3_cross_vm_router,
        X3TokenFactory: pallet_x3_token_factory,
        CrossChainValidator: pallet_cross_chain_validator,
    }
);

pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
pub type UncheckedExtrinsic =
    generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
// Runtime storage migrations tuple. Add migration structs for pallets that need upgrades.
// Note: Only x3-kernel has migrations currently implemented
pub type Migrations = (pallet_x3_kernel::migrations::Migration<Runtime>,);

// Use the migrations tuple in the executive so migrations run on runtime upgrades
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
    Migrations,
>;

impl_opaque_keys! {
    pub struct SessionKeys {
        pub aura: Aura,
    }
}

pub type SignedExtra = (
    frame_system::CheckNonZeroSender<Runtime>,
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);

pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;

// ===== Config Impls (after construct_runtime!) =====

type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

pub struct DealWithFees;
impl frame_support::traits::OnUnbalanced<NegativeImbalance> for DealWithFees {
    fn on_unbalanced(amount: NegativeImbalance) {
        drop(amount);
    }
}

pub struct FixedGasPrice;
impl pallet_evm::FeeCalculator for FixedGasPrice {
    fn min_gas_price() -> (U256, Weight) {
        (U256::from(NATIVE_GAS_PRICE), Weight::zero())
    }
}

impl frame_system::Config for Runtime {
    type BaseCallFilter = Everything;
    type Block = Block;
    type BlockWeights = BlockWeights;
    type BlockLength = BlockLength;
    type DbWeight = RocksDbWeight;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Hash = Hash;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = AccountIdLookup<AccountId, ()>;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = RuntimeVersion;
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = frame_system::weights::SubstrateWeight<Runtime>;
    type SS58Prefix = ConstU16<42>;
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
    type Nonce = Index;
}

impl pallet_timestamp::Config for Runtime {
    type Moment = Moment;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

impl pallet_aura::Config for Runtime {
    type AuthorityId = sp_consensus_aura::sr25519::AuthorityId;
    type MaxAuthorities = MaxAuthorities;
    type DisabledValidators = ();
    type AllowMultipleBlocksPerSlot = ConstBool<true>; // Enable multiple blocks per slot for higher TPS
}

parameter_types! {
    pub const ReportLongevity: u64 = 1000;
}

impl pallet_session::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = ConvertInto;
    type ShouldEndSession = pallet_session::PeriodicSessions<ConstU32<1800>, ConstU32<0>>;
    type NextSessionRotation = pallet_session::PeriodicSessions<ConstU32<1800>, ConstU32<0>>;
    type SessionManager = ();
    type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
    type Keys = SessionKeys;
    type WeightInfo = pallet_session::weights::SubstrateWeight<Self>;
}

pub type Historical = pallet_session::historical::Pallet<Runtime>;

impl pallet_session::historical::Config for Runtime {
    type FullIdentification = AccountId;
    type FullIdentificationOf = ConvertInto;
}

impl pallet_grandpa::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type KeyOwnerProof = sp_core::Void;
    type EquivocationReportSystem = ();
    type WeightInfo = ();
    type MaxAuthorities = MaxAuthorities;
    type MaxSetIdSessionEntries = MaxSetIdSessionEntries;
}

impl pallet_balances::Config for Runtime {
    type Balance = Balance;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ConstU32<50>;
    type MaxHolds = ConstU32<0>;
    type MaxFreezes = ConstU32<0>;
    type ReserveIdentifier = [u8; 8];
    type FreezeIdentifier = ();
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
    type RuntimeHoldReason = ();
}

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = CurrencyAdapter<Balances, DealWithFees>;
    type OperationalFeeMultiplier = OperationalFeeMultiplier;
    type WeightToFee = IdentityFee<Balance>;
    type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
    type FeeMultiplierUpdate = ();
}

#[cfg(feature = "dev")]
impl pallet_sudo::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

pub type EnsureRootOrHalfCouncil = frame_support::traits::EitherOfDiverse<
    frame_system::EnsureRoot<AccountId>,
    pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>,
>;

pub type EnsureCouncilMember = pallet_collective::EnsureMember<AccountId, CouncilCollective>;

pub type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
    type RuntimeOrigin = RuntimeOrigin;
    type Proposal = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type MotionDuration = CouncilMotionDuration;
    type MaxProposals = CouncilMaxProposals;
    type MaxMembers = CouncilMaxMembers;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
    type SetMembersOrigin = frame_system::EnsureRoot<AccountId>;
    type MaxProposalWeight = MaxProposalWeight;
}

// ── Fraud-proof inline pallet config ─────────────────────────────────────────
impl crate::fraud_proofs::pallet::pallet::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type MaxTxCount = FraudProofMaxTxCount;
    type DisputeWindowBlocks = FraudProofDisputeWindowBlocks;
    type ReporterRewardAmount = FraudProofReporterReward;
    type GovernanceOrigin = EnsureRootOrHalfCouncil;
}

impl pallet_evm::Config for Runtime {
    type FeeCalculator = FixedGasPrice;
    type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
    type WeightPerGas = WeightPerGas;
    type BlockHashMapping = pallet_evm::SubstrateBlockHashMapping<Self>;
    type CallOrigin = pallet_evm::EnsureAddressRoot<AccountId>;
    type WithdrawOrigin = pallet_evm::EnsureAddressTruncated;
    type AddressMapping = pallet_evm::HashedAddressMapping<BlakeTwo256>;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type PrecompilesType = FrontierPrecompiles<Self>;
    type PrecompilesValue = PrecompilesValue;
    type ChainId = ChainId;
    type BlockGasLimit = BlockGasLimit;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type OnChargeTransaction = pallet_evm::EVMCurrencyAdapter<Balances, ()>;
    type OnCreate = ();
    type FindAuthor = ();
    type GasLimitPovSizeRatio = GasLimitPovSizeRatio;
    type Timestamp = Timestamp;
    type WeightInfo = pallet_evm::weights::SubstrateWeight<Self>;
}

/// Production cross-chain proof verifier.
///
/// Validates `LockProof` and `MerkleReceipt` payloads with structural
/// sanity checks and enforces a byzantine threshold of validator signatures
/// from the currently configured X3 kernel authorities.
pub struct SubstrateProofVerifier;

impl pallet_x3_kernel::CrossChainProofVerifier<AccountId> for SubstrateProofVerifier {
    fn verify_proof(
        _origin: &AccountId,
        _operation: &x3_cross_vm_bridge::CrossVmOperation,
        proof: &pallet_x3_kernel::CrossChainProof,
    ) -> Result<(), frame_support::sp_runtime::DispatchError> {
        use codec::Encode;
        use pallet_x3_kernel::CrossChainProof;

        fn threshold(authority_count: usize) -> usize {
            // 2/3 + 1, but always at least 1.
            let needed = (authority_count.saturating_mul(2) / 3).saturating_add(1);
            core::cmp::max(1, needed)
        }

        fn account_to_key_bytes(
            account: &AccountId,
        ) -> Result<[u8; 32], frame_support::sp_runtime::DispatchError> {
            let encoded = account.encode();
            if encoded.len() != 32 {
                return Err(frame_support::sp_runtime::DispatchError::Other(
                    "Authority key must SCALE-encode to 32 bytes",
                ));
            }
            let mut out = [0u8; 32];
            out.copy_from_slice(&encoded);
            Ok(out)
        }

        fn verify_signature_any(pubkey_bytes: [u8; 32], message: &[u8], signature: &[u8]) -> bool {
            if signature.len() != 64 {
                return false;
            }

            // sr25519
            {
                let pubkey = sp_core::sr25519::Public::from_raw(pubkey_bytes);
                let sig = sp_core::sr25519::Signature::from_raw({
                    let mut buf = [0u8; 64];
                    buf.copy_from_slice(signature);
                    buf
                });
                if sp_io::crypto::sr25519_verify(&sig, message, &pubkey) {
                    return true;
                }
            }

            // ed25519
            {
                let pubkey = sp_core::ed25519::Public::from_raw(pubkey_bytes);
                let sig = sp_core::ed25519::Signature::from_raw({
                    let mut buf = [0u8; 64];
                    buf.copy_from_slice(signature);
                    buf
                });
                sp_io::crypto::ed25519_verify(&sig, message, &pubkey)
            }
        }

        fn require_len(
            actual: usize,
            expected: usize,
            label: &'static str,
        ) -> Result<(), frame_support::sp_runtime::DispatchError> {
            if actual != expected {
                return Err(frame_support::sp_runtime::DispatchError::Other(label));
            }
            Ok(())
        }

        let authorities = pallet_x3_kernel::Authorities::<Runtime>::get();
        let authority_keys: sp_std::collections::btree_set::BTreeSet<[u8; 32]> = authorities
            .into_iter()
            .map(|a| account_to_key_bytes(&a))
            .collect::<Result<_, _>>()?;
        let needed = threshold(authority_keys.len());

        match proof {
            CrossChainProof::None => Ok(()),
            CrossChainProof::LockProof(bytes) => {
                if bytes.is_empty() {
                    return Err(frame_support::sp_runtime::DispatchError::Other(
                        "LockProof: empty bytes",
                    ));
                }
                // Format:
                // [0..32)  event_hash
                // [32]     sig_count (u8)
                // repeat sig_count times:
                //   [validator_id:32][signature:64]
                if bytes.len() < 33 {
                    return Err(frame_support::sp_runtime::DispatchError::Other(
                        "LockProof: payload too short (< 33 bytes)",
                    ));
                }

                let event_hash = &bytes[0..32];
                let sig_count = bytes[32] as usize;
                if sig_count == 0 {
                    return Err(frame_support::sp_runtime::DispatchError::Other(
                        "LockProof: signature count must be > 0",
                    ));
                }

                let expected_len = 33usize.saturating_add(sig_count.saturating_mul(96));
                require_len(
                    bytes.len(),
                    expected_len,
                    "LockProof: malformed payload length",
                )?;

                let mut valid = 0usize;
                let mut seen: sp_std::collections::btree_set::BTreeSet<[u8; 32]> =
                    sp_std::collections::btree_set::BTreeSet::new();

                for idx in 0..sig_count {
                    let offset = 33 + idx * 96;
                    let mut validator_id = [0u8; 32];
                    validator_id.copy_from_slice(&bytes[offset..offset + 32]);
                    let signature = &bytes[offset + 32..offset + 96];

                    if !authority_keys.contains(&validator_id) {
                        continue;
                    }
                    if !seen.insert(validator_id) {
                        continue;
                    }
                    if verify_signature_any(validator_id, event_hash, signature) {
                        valid = valid.saturating_add(1);
                    }
                }

                if valid < needed {
                    return Err(frame_support::sp_runtime::DispatchError::Other(
                        "LockProof: insufficient validator signatures",
                    ));
                }

                Ok(())
            }
            CrossChainProof::MerkleReceipt(bytes) => {
                if bytes.is_empty() {
                    return Err(frame_support::sp_runtime::DispatchError::Other(
                        "MerkleReceipt: empty bytes",
                    ));
                }
                // Format:
                // [0..32)  state_root
                // [32..40) finalized_block (u64 LE)
                // [40..48) execution_index (u64 LE)
                // [48..52) merkle_proof_len (u32 LE)
                // [..]     merkle_proof_bytes (len)
                // [..]     sig_count (u8)
                // repeat sig_count times:
                //   [validator_id:32][signature:64] over sha2_256(state_root||finalized_block||execution_index||merkle_proof_bytes)
                if bytes.len() < 53 {
                    return Err(frame_support::sp_runtime::DispatchError::Other(
                        "MerkleReceipt: payload too short",
                    ));
                }

                let mut state_root = [0u8; 32];
                state_root.copy_from_slice(&bytes[0..32]);
                if state_root == [0u8; 32] {
                    return Err(frame_support::sp_runtime::DispatchError::Other(
                        "MerkleReceipt: state_root must be non-zero",
                    ));
                }

                let finalized_block =
                    u64::from_le_bytes(bytes[32..40].try_into().map_err(|_| {
                        frame_support::sp_runtime::DispatchError::Other(
                            "MerkleReceipt: invalid finalized_block",
                        )
                    })?);
                if finalized_block == 0 {
                    return Err(frame_support::sp_runtime::DispatchError::Other(
                        "MerkleReceipt: finalized_block must be > 0",
                    ));
                }

                let execution_index =
                    u64::from_le_bytes(bytes[40..48].try_into().map_err(|_| {
                        frame_support::sp_runtime::DispatchError::Other(
                            "MerkleReceipt: invalid execution_index",
                        )
                    })?);

                let merkle_len = u32::from_le_bytes(bytes[48..52].try_into().map_err(|_| {
                    frame_support::sp_runtime::DispatchError::Other(
                        "MerkleReceipt: invalid merkle_proof_len",
                    )
                })?) as usize;
                if merkle_len == 0 {
                    return Err(frame_support::sp_runtime::DispatchError::Other(
                        "MerkleReceipt: merkle_proof_bytes must be non-empty",
                    ));
                }

                let proof_start = 52usize;
                let proof_end = proof_start.checked_add(merkle_len).ok_or(
                    frame_support::sp_runtime::DispatchError::Other(
                        "MerkleReceipt: proof length overflow",
                    ),
                )?;
                if bytes.len() < proof_end + 1 {
                    return Err(frame_support::sp_runtime::DispatchError::Other(
                        "MerkleReceipt: payload too short for merkle proof",
                    ));
                }

                let merkle_proof_bytes = &bytes[proof_start..proof_end];
                let sig_count = bytes[proof_end] as usize;
                if sig_count == 0 {
                    return Err(frame_support::sp_runtime::DispatchError::Other(
                        "MerkleReceipt: signature count must be > 0",
                    ));
                }

                let expected_len = (proof_end + 1).saturating_add(sig_count.saturating_mul(96));
                require_len(
                    bytes.len(),
                    expected_len,
                    "MerkleReceipt: malformed payload length",
                )?;

                let mut msg =
                    sp_std::vec::Vec::with_capacity(32 + 8 + 8 + merkle_proof_bytes.len());
                msg.extend_from_slice(&state_root);
                msg.extend_from_slice(&finalized_block.to_le_bytes());
                msg.extend_from_slice(&execution_index.to_le_bytes());
                msg.extend_from_slice(merkle_proof_bytes);
                let settlement_hash = sp_io::hashing::sha2_256(&msg);

                let mut valid = 0usize;
                let mut seen: sp_std::collections::btree_set::BTreeSet<[u8; 32]> =
                    sp_std::collections::btree_set::BTreeSet::new();

                for idx in 0..sig_count {
                    let offset = (proof_end + 1) + idx * 96;
                    let mut validator_id = [0u8; 32];
                    validator_id.copy_from_slice(&bytes[offset..offset + 32]);
                    let signature = &bytes[offset + 32..offset + 96];

                    if !authority_keys.contains(&validator_id) {
                        continue;
                    }
                    if !seen.insert(validator_id) {
                        continue;
                    }
                    if verify_signature_any(validator_id, &settlement_hash, signature) {
                        valid = valid.saturating_add(1);
                    }
                }

                if valid < needed {
                    return Err(frame_support::sp_runtime::DispatchError::Other(
                        "MerkleReceipt: insufficient validator signatures",
                    ));
                }

                Ok(())
            }
        }
    }
}

impl pallet_x3_kernel::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
