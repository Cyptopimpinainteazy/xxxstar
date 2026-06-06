#![deny(unsafe_code)]
//! # X3 Settlement Engine Pallet
//!
//! ## THE SETTLEMENT ROOT OF TRUST
//!
//! X3 is the final arbiter for all atomic settlements across:
//! - **EVM**: Ethereum and 100+ compatible chains
//! - **SVM**: Solana-compatible execution
//! - **BTC**: Native Bitcoin UTXO settlement (not wrapped)
//! - **X3VM**: Native governance and invariant enforcement
//!
//! ## Core Principle
//!
//! > "External chains are execution domains. X3 is the final arbiter."
//!
//! All trades—whether BTC, EVM, or SVM—must:
//! 1. Resolve through X3 atomic escrows
//! 2. Emit canonical settlement events
//! 3. Be verifiable on X3 even if execution happens elsewhere
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                        X3 SETTLEMENT ENGINE                             │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────┐ │
//! │  │ AtomicIntent    │  │ CrossVMEscrow   │  │ BTCAtomicGateway        │ │
//! │  │ Registry        │  │                 │  │                         │ │
//! │  │                 │  │ • Lock assets   │  │ • UTXO tracking         │ │
//! │  │ • Intent create │  │ • Release/refund│  │ • SPV proof verify      │ │
//! │  │ • State machine │  │ • Cross-VM sync │  │ • Adaptor sigs          │ │
//! │  └─────────────────┘  └─────────────────┘  └─────────────────────────┘ │
//! │                                                                         │
//! │  ┌─────────────────┐  ┌─────────────────────────────────────────────┐  │
//! │  │ FinalityOracle  │  │ InvariantEnforcer                           │  │
//! │  │                 │  │                                             │  │
//! │  │ • Chain finality│  │ • No partial execution                      │  │
//! │  │ • Reorg risk    │  │ • No BTC release without X3 confirmation    │  │
//! │  │ • Depth tracking│  │ • All intents must resolve (finalize/refund)│  │
//! │  └─────────────────┘  │ • Timeouts always favor user funds          │  │
//! │                       └─────────────────────────────────────────────┘  │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Settlement Flow
//!
//! ```text
//! MATCH → X3_INTENT_CREATED
//!       → ASSETS_LOCKED_X3
//!       → EXTERNAL_EXECUTION (BTC / EVM / SVM)
//!       → PROOF_SUBMITTED_TO_X3
//!       → FINALIZE_X3
//!
//! If anything fails:
//!       → REFUND_X3 (automatic, provable)
//! ```
//!
//! ## Invariants (NON-NEGOTIABLE)
//!
//! 1. No asset finalized unless ALL legs are provably complete
//! 2. No BTC release without X3 confirmation
//! 3. No cross-VM partial state
//! 4. All intents must resolve (finalize or refund)
//! 5. Timeouts ALWAYS favor user funds

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unused_imports)]
#![allow(
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::derivable_impls,
    clippy::manual_is_multiple_of,
    clippy::new_without_default,
    clippy::too_many_arguments
)]

pub mod atomic_lock;
pub mod bridge_integration;
pub mod bridge_tests;
pub mod btc_gateway;
pub mod collateral;
pub mod escrow;
pub mod finality;
pub mod intent;
pub mod invariants;
pub mod runtime_api;
pub mod types;
pub mod weights;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub use types::*;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use crate::atomic_lock;
    use crate::bridge_integration::CrossChainValidatorProvider;
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ReservableCurrency, StorageVersion, UnixTime},
    };
    use frame_system::pallet_prelude::*;
    use sp_core::{ed25519, ConstU32, H256};
    use sp_io::hashing::blake2_256;
    use sp_runtime::{SaturatedConversion, Saturating};
    use sp_std::vec::Vec;

    /// Current storage version
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    // ============================================================================
    // Types
    // ============================================================================

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    // ============================================================================
    // Pallet Definition
    // ============================================================================

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_x3_kernel::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Weight information for extrinsics.
        type SettlementWeightInfo: crate::weights::WeightInfo;

        /// Currency for deposits and fees.
        type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

        /// Cross-chain validator provider for proof verification
        type CrossChainValidator: bridge_integration::CrossChainValidatorProvider;

        /// Unix time provider for timeout enforcement.
        type UnixTime: UnixTime;

        /// Maximum legs per settlement intent.
        #[pallet::constant]
        type MaxSettlementLegs: Get<u32>;

        /// Maximum pending intents per account.
        #[pallet::constant]
        type MaxPendingIntents: Get<u32>;

        /// Default timeout for settlement (in seconds).
        #[pallet::constant]
        type DefaultSettlementTimeout: Get<u64>;

        /// Minimum BTC confirmation depth.
        #[pallet::constant]
        type MinBtcConfirmations: Get<u32>;

        /// Challenge period for optimistic settlements (in blocks).
        #[pallet::constant]
        type ChallengePeriod: Get<BlockNumberFor<Self>>;

        /// Settlement finality timeout (in blocks). After this many blocks, settlements must finalize or auto-refund.
        /// Default: 28,800 blocks ≈ 24 hours at 3-second block times.
        /// Ensures settlements cannot remain in pending state indefinitely.
        #[pallet::constant]
        type SettlementTimeoutBlocks: Get<BlockNumberFor<Self>>;
    }

    // ============================================================================
    // Storage
    // ============================================================================

    /// Atomic Intent Registry: Maps intent_id → SettlementIntent
    #[pallet::storage]
    #[pallet::getter(fn settlement_intents)]
    pub type SettlementIntents<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, SettlementIntent<T::AccountId>, OptionQuery>;

    /// Settlement creation block tracker: Maps intent_id → block_number
    /// Used to enforce the SettlementTimeoutBlocks deadline. After SettlementTimeoutBlocks
    /// have elapsed since creation, the settlement must finalize or auto-refund to prevent
    /// indefinite pending states and ensure cross-chain atomicity.
    #[pallet::storage]
    #[pallet::getter(fn settlement_creation_block)]
    pub type SettlementCreationBlocks<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, BlockNumberFor<T>, OptionQuery>;

    /// Intent state machine: Maps intent_id → IntentState
    #[pallet::storage]
    #[pallet::getter(fn intent_states)]
    pub type IntentStates<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, IntentState, ValueQuery>;

    /// Cross-VM Escrow: Maps (intent_id, leg_index) → EscrowState
    #[pallet::storage]
    #[pallet::getter(fn escrow_states)]
    pub type EscrowStates<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        H256, // intent_id
        Blake2_128Concat,
        u32, // leg_index
        EscrowLeg<T::AccountId>,
        OptionQuery,
    >;

    /// Claimed settlement legs: Maps (intent_id, leg_index) → claimed flag.
    ///
    /// This prevents replayed `claim_settlement` calls from incrementing
    /// `legs_claimed` without binding each claim to a concrete escrow leg.
    #[pallet::storage]
    #[pallet::getter(fn claimed_legs)]
    pub type ClaimedLegs<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, H256, Blake2_128Concat, u32, bool, ValueQuery>;

    /// BTC UTXO Registry: Maps (btc_txid, vout) → BTCUtxoState
    ///
    /// Keyed by BOTH txid AND output index to prevent UTXO double-spend where two
    /// outputs from the same transaction (same txid, different vout) would otherwise
    /// overwrite each other.
    #[pallet::storage]
    #[pallet::getter(fn btc_utxos)]
    pub type BtcUtxos<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        H256, // btc_txid
        Blake2_128Concat,
        u32, // vout
        BtcUtxoState,
        OptionQuery,
    >;

    /// BTC Block Headers (SPV): Maps block_hash → BTCBlockHeader
    #[pallet::storage]
    #[pallet::getter(fn btc_headers)]
    pub type BtcHeaders<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, BtcBlockHeader, OptionQuery>;

    /// Best known BTC block height
    #[pallet::storage]
    #[pallet::getter(fn btc_best_height)]
    pub type BtcBestHeight<T: Config> = StorageValue<_, u64, ValueQuery>;

    // ========================================================================
    // Collateral / Bonds
    // ========================================================================

    /// Bond record storage: bond_id -> BondRecord
    #[pallet::storage]
    #[pallet::getter(fn bonds)]
    pub type Bonds<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, BondRecord<T::AccountId, BalanceOf<T>>, OptionQuery>;

    /// Mapping from owner -> vector of bond ids (bounded for simplicity)
    #[pallet::storage]
    #[pallet::getter(fn bonds_by_owner)]
    pub type BondsByOwner<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, BoundedVec<H256, ConstU32<100>>, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultBondCounter() -> u64 {
        0
    }

    /// Next bond counter (for simple unique id seed)
    #[pallet::storage]
    #[pallet::getter(fn bond_counter)]
    pub type BondCounter<T: Config> = StorageValue<_, u64, ValueQuery, DefaultBondCounter>;

    // Bond record stored on-chain
    #[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, PartialEq, Eq)]
    #[scale_info(skip_type_params(AccountId, Balance))]
    pub struct BondRecord<AccountId, Balance> {
        pub id: H256,
        pub owner: AccountId,
        pub asset: BoundedVec<u8, ConstU32<64>>,
        pub amount: Balance,
        pub bond_type: u8,
        pub state: u8, // 0=Locked,1=Withdrawable,2=Slashed
        pub created_at: u64,
    }

    #[pallet::storage]
    #[pallet::getter(fn chain_finality)]
    pub type ChainFinality<T: Config> =
        StorageMap<_, Blake2_128Concat, ExternalChainId, FinalityConfig, OptionQuery>;

    /// Pending intents per account (for rate limiting)
    #[pallet::storage]
    #[pallet::getter(fn pending_intents)]
    pub type PendingIntents<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

    /// Global intent counter (for statistics)
    #[pallet::storage]
    #[pallet::getter(fn total_intents)]
    pub type TotalIntents<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Total settled volume (in base units)
    #[pallet::storage]
    #[pallet::getter(fn total_settled_volume)]
    pub type TotalSettledVolume<T: Config> = StorageValue<_, u128, ValueQuery>;

    /// Invariant violation counter (for monitoring)
    #[pallet::storage]
    #[pallet::getter(fn invariant_violations)]
    pub type InvariantViolations<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Atomic Locks for 2PC: Maps intent_id → AtomicLock
    /// Used to enforce 2-phase commit atomicity during settlement.
    /// Tracks lock phase (Prepare, Commit, Released, Slashed) and deadlines.
    #[pallet::storage]
    #[pallet::getter(fn atomic_locks)]
    pub type AtomicLocks<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        H256, // intent_id
        atomic_lock::AtomicLock<BalanceOf<T>, T::AccountId>,
        OptionQuery,
    >;

    /// Deadline index: block_number → bounded list of intent_ids whose timeout expires at that block.
    ///
    /// Populated by `create_intent` and consumed (removed) by `on_initialize` to provide automatic,
    /// O(1)-lookup timeout refunds without a full storage scan. Capped at 20 intents per block to
    /// bound on_initialize execution time; excess intents can be refunded via `refund_settlement`.
    #[pallet::storage]
    pub type IntentDeadlineIndex<T: Config> =
        StorageMap<_, Twox64Concat, BlockNumberFor<T>, BoundedVec<H256, ConstU32<20>>, ValueQuery>;

    /// Atomic lock expiry index: deadline_block → intent_ids whose AtomicLock expires at that block.
    ///
    /// Populated by `lock_escrow` when a lock is created (Prepare phase) and updated when the lock
    /// transitions to CommitInProgress (new finalize deadline). Consumed by `on_finalize` to replace
    /// the unbounded `AtomicLocks::iter()` scan — reduces on_finalize cost from O(all_locks) to
    /// O(locks_expiring_at_this_block), preventing a chain-halt DoS via lock-spam.  P0 fix.
    #[pallet::storage]
    pub type AtomicLockExpiryIndex<T: Config> = StorageMap<
        _,
        Twox64Concat,
        u32, // deadline block number (matches commit_deadline / finalize_deadline: u32 in LockPhase)
        BoundedVec<H256, ConstU32<20>>,
        ValueQuery,
    >;

    /// Settlement Transfers: Maps transfer_id → SettlementTransfer
    /// Tracks individual cross-chain settlements with their status lifecycle:
    /// Pending → Completed (settle_transfer) or Pending → Refunded (trigger_refund)
    #[pallet::storage]
    #[pallet::getter(fn settlement_transfers)]
    pub type SettlementTransfers<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        H256, // transfer_id
        SettlementTransfer<T::AccountId, BalanceOf<T>>,
        OptionQuery,
    >;

    /// Proof Cache: Tracks submitted proofs to prevent replay attacks
    /// Maps proof_hash → () to mark proofs as already submitted globally
    /// Prevents the same proof from being used multiple times across any intent
    #[pallet::storage]
    pub type ProofCache<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        H256, // proof_hash
        (),
        OptionQuery,
    >;

    // ============================================================================
    // Events
    // ============================================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Trade matched, settlement intent created on X3
        /// [intent_id, maker, taker, asset_a, asset_b]
        X3IntentCreated {
            intent_id: H256,
            maker: T::AccountId,
            taker: T::AccountId,
            asset_a: AssetSpec,
            asset_b: AssetSpec,
            secret_hash: H256,
            timeout: u64,
        },

        /// Assets locked in X3 escrow
        /// [intent_id, leg_index, chain, amount]
        X3AssetsLocked {
            intent_id: H256,
            leg_index: u32,
            chain: ExternalChainId,
            amount: u128,
            escrow_address: Vec<u8>,
        },

        /// External execution started (off X3)
        /// [intent_id, chain, tx_hash]
        ExternalExecutionStarted {
            intent_id: H256,
            chain: ExternalChainId,
            tx_hash: H256,
        },

        /// Bond deposited on-chain
        BondDeposited {
            bond_id: H256,
            owner: T::AccountId,
            amount: BalanceOf<T>,
        },

        /// Bond withdrawn/finalized
        BondWithdrawn {
            bond_id: H256,
            owner: T::AccountId,
            amount: BalanceOf<T>,
        },

        /// Bond slashed
        BondSlashed { bond_id: H256 },

        /// External proof submitted to X3
        /// [intent_id, chain, proof_type, tx_hash]
        ExternalProofSubmitted {
            intent_id: H256,
            chain: ExternalChainId,
            proof_type: ProofType,
            tx_hash: H256,
            confirmations: u32,
        },

        /// Settlement finalized on X3 (ALL legs complete)
        /// [intent_id, total_value_usd]
        X3Finalized {
            intent_id: H256,
            maker_received: u128,
            taker_received: u128,
            settlement_time_ms: u64,
        },

        /// Settlement refunded on X3 (timeout or failure)
        /// [intent_id, reason]
        X3Refunded {
            intent_id: H256,
            reason: RefundReason,
            maker_returned: u128,
            taker_returned: u128,
        },

        /// Invariant violation detected (CRITICAL)
        /// [intent_id, violation_type]
        InvariantViolation {
            intent_id: H256,
            violation_type: InvariantViolationType,
            details: Vec<u8>,
        },

        /// BTC UTXO confirmed for settlement
        /// [intent_id, btc_txid, vout, confirmations]
        BtcUtxoConfirmed {
            intent_id: H256,
            btc_txid: H256,
            vout: u32,
            confirmations: u32,
            amount_sats: u64,
        },

        /// BTC released after X3 confirmation
        /// [intent_id, btc_txid, recipient]
        BtcReleased {
            intent_id: H256,
            btc_txid: H256,
            recipient: Vec<u8>,
            amount_sats: u64,
        },

        /// Atomic lock timed out and executor slashed
        /// [intent_id, executor_id, amount_slashed]
        AtomicLockTimeoutSlashed {
            intent_id: H256,
            executor_id: [u8; 32],
            amount_slashed: BalanceOf<T>,
        },

        /// Settlement transfer executed by relayer (PHASE C STUB)
        /// FROZEN under v1-alpha API (2026-04-24, commit d99252ca42)
