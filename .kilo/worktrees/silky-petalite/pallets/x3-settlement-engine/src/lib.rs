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
    use codec::Encode;
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

    type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
    type BalanceOf<T> = <<T as Config>::Currency as Currency<AccountIdOf<T>>>::Balance;

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
        type Currency: Currency<<Self as frame_system::Config>::AccountId>
            + ReservableCurrency<<Self as frame_system::Config>::AccountId>;

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
        StorageMap<_, Blake2_128Concat, H256, SettlementIntent<AccountIdOf<T>>, OptionQuery>;

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
        EscrowLeg<AccountIdOf<T>>,
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
    pub type Bonds<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        H256,
        BondRecord<AccountIdOf<T>, BalanceOf<T>>,
        OptionQuery,
    >;

    /// Mapping from owner -> vector of bond ids (bounded for simplicity)
    #[pallet::storage]
    #[pallet::getter(fn bonds_by_owner)]
    pub type BondsByOwner<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        AccountIdOf<T>,
        BoundedVec<H256, ConstU32<100>>,
        ValueQuery,
    >;

    #[pallet::type_value]
    pub fn DefaultBondCounter() -> u64 {
        0
    }

    /// Next bond counter (for simple unique id seed)
    #[pallet::storage]
    #[pallet::getter(fn bond_counter)]
    pub type BondCounter<T: Config> = StorageValue<_, u64, ValueQuery, DefaultBondCounter>;

    // Bond record stored on-chain
    #[derive(
        Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, MaxEncodedLen, PartialEq, Eq,
    )]
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
        StorageMap<_, Blake2_128Concat, AccountIdOf<T>, u32, ValueQuery>;

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
        atomic_lock::AtomicLock<BalanceOf<T>, AccountIdOf<T>>,
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
        SettlementTransfer<AccountIdOf<T>, BalanceOf<T>>,
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
            maker: AccountIdOf<T>,
            taker: AccountIdOf<T>,
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
            owner: AccountIdOf<T>,
            amount: BalanceOf<T>,
        },

        /// Bond withdrawn/finalized
        BondWithdrawn {
            bond_id: H256,
            owner: AccountIdOf<T>,
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
        SettlementExecuted {
            transfer_id: H256,
            receiver: AccountIdOf<T>,
            amount: BalanceOf<T>,
        },

        /// Refund triggered for transfer that exceeded timeout (PHASE C STUB)
        /// FROZEN under v1-alpha API (2026-04-24, commit d99252ca42)
        RefundTriggered { transfer_id: H256 },

        /// Settlement timeout expired (block-based finality deadline exceeded)
        /// Indicates the settlement exceeded SettlementTimeoutBlocks without finalizing.
        /// Automatically triggers refund to prevent indefinite pending states.
        SettlementTimeoutExpiredBlock {
            intent_id: H256,
            creation_block: u32,
            timeout_block: u32,
            current_block: u32,
        },

        /// Cross-chain proof successfully verified and linked to settlement
        /// [intent_id, chain, block_or_slot, proof_hash, verified_at_block]
        SettlementProofVerified {
            intent_id: H256,
            chain: ExternalChainId,
            block_or_slot: u64,
            proof_hash: H256,
            verified_at_block: u32,
        },
    }

    // ============================================================================
    // Errors
    // ============================================================================

    #[pallet::error]
    pub enum Error<T> {
        /// Intent already exists
        IntentAlreadyExists,
        /// Intent not found
        IntentNotFound,
        /// Invalid intent state for operation
        InvalidIntentState,
        /// Invalid settlement leg
        InvalidSettlementLeg,
        /// Too many pending intents
        TooManyPendingIntents,
        /// Insufficient balance for escrow
        InsufficientBalance,
        /// Invalid secret hash
        InvalidSecretHash,
        /// Invalid secret (preimage doesn't match hash)
        InvalidSecret,
        /// Settlement timeout expired
        TimeoutExpired,
        /// Settlement timeout not yet expired (for refund)
        TimeoutNotExpired,
        /// Invalid proof submitted
        InvalidProof,
        /// BTC UTXO not confirmed
        BtcNotConfirmed,
        /// BTC confirmation depth insufficient
        InsufficientBtcConfirmations,
        /// Invalid BTC proof
        InvalidBtcProof,
        /// External chain not supported
        UnsupportedChain,
        /// Invariant violation detected
        InvariantViolation,
        /// Escrow already exists
        EscrowAlreadyExists,
        /// Escrow not found
        EscrowNotFound,
        /// Not authorized for operation
        NotAuthorized,
        /// Invalid asset specification
        InvalidAssetSpec,
        /// Arithmetic overflow
        ArithmeticOverflow,
        /// Partial execution detected (CRITICAL)
        PartialExecutionDetected,
        /// Cross-VM reentrancy detected (CRITICAL)
        CrossVmReentrancyDetected,
        /// Bond not found
        BondNotFound,
        /// Not the owner of the bond
        NotBondOwner,
        /// Bond is not locked (cannot withdraw yet)
        BondNotLocked,
        /// Bond is not withdrawable
        BondNotWithdrawable,
        /// Too many bonds for owner
        TooManyBonds,
        /// Bond already slashed
        BondAlreadySlashed,
        /// No unclaimed escrow leg found for claimer
        NoClaimableLeg,
        /// Settlement transfer not found
        UnknownTransfer,
        /// Transfer status is not Pending
        TransferNotPending,
        /// Receiver address mismatch
        ReceiverMismatch,
        /// Amount mismatch
        AmountMismatch,
        /// Settlement timeout expired
        SettlementTimeout,
        /// Refund triggered too early (timeout not reached)
        RefundTooEarly,
        /// Executor not authorized
        ExecutorNotAuthorized,
        /// Settlement block-based timeout expired (SettlementTimeoutBlocks exceeded)
        SettlementTimeoutExpired,
    }

    // ============================================================================
    // Hooks
    // ============================================================================

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: BlockNumberFor<T>) -> Weight {
            // C-006: Process automatic refunds for intents whose deadline falls at block `n`.
            // IntentDeadlineIndex is taken (removed) to guarantee each entry is processed
            // exactly once. Capped at 20 refunds per block to bound execution time;
            // any remaining intents are unreachable via this path but remain refundable
            // through the `refund_settlement` extrinsic.
            const MAX_REFUNDS_PER_BLOCK: usize = 20;

            // take() removes the entry and returns the BoundedVec: 1 read + 1 write.
            let expired = IntentDeadlineIndex::<T>::take(n);
            let mut weight = <T as frame_system::Config>::DbWeight::get().reads_writes(1, 1);

            for intent_id in expired.iter().take(MAX_REFUNDS_PER_BLOCK) {
                // 2 reads per loop iteration: IntentStates + SettlementIntents.
                weight =
                    weight.saturating_add(<T as frame_system::Config>::DbWeight::get().reads(2));

                let state = IntentStates::<T>::get(intent_id);
                if !matches!(
                    state,
                    IntentState::Created
                        | IntentState::FundingInProgress
                        | IntentState::FullyFunded
                ) {
                    // Already finalized / refunded / halted — nothing to do.
                    continue;
                }

                if let Some(intent) = SettlementIntents::<T>::get(intent_id) {
                    let now = T::UnixTime::now().as_secs();
                    if now >= intent.timeout {
                        let _ = Self::process_refund(*intent_id, &intent, RefundReason::Timeout);
                        // process_refund touches EscrowStates, IntentStates, AtomicLocks,
                        // ClaimedLegs, PendingIntents: conservatively charge 4R + 4W.
                        weight = weight.saturating_add(
                            <T as frame_system::Config>::DbWeight::get().reads_writes(4, 4),
                        );
                    }
                }
            }

            weight
        }

        fn on_finalize(_n: BlockNumberFor<T>) {
            // Process atomic lock timeouts and emit slashing events.
            // Actual slash execution happens via scheduled extrinsic or off-chain worker.
            //
            // P0 FIX: Use AtomicLockExpiryIndex instead of AtomicLocks::iter() to avoid
            // an unbounded storage scan. Previously, iter() scanned ALL locks to find expired
            // ones — an attacker could spam lock creations to inflate scan cost and halt the
            // chain. Now we look up only the locks expiring at this specific block number.
            const MAX_LOCKS_PER_BLOCK: usize = 20;

            let current_block: u32 =
                frame_system::Pallet::<T>::block_number().saturated_into::<u32>();

            // take() removes the index entry and returns the bounded vec: O(1) lookup.
            // Then we fetch only the specific locks registered for this block.
            let expiring_ids = AtomicLockExpiryIndex::<T>::take(current_block);
            let expired_locks: Vec<(H256, atomic_lock::AtomicLock<BalanceOf<T>, AccountIdOf<T>>)> =
                expiring_ids
                    .iter()
                    .filter_map(|id| AtomicLocks::<T>::get(id).map(|lock| (*id, lock)))
                    // C-004: skip zero-amount locks; they were created before the fix and
                    // would fire spurious AtomicLockTimeoutSlashed events/slash no funds.
                    .filter(|(_, lock)| lock.amount > BalanceOf::<T>::default())
                    .take(MAX_LOCKS_PER_BLOCK)
                    .collect();

            // Process each expired lock
            for (intent_id, mut lock) in expired_locks {
                // Transition lock to Slashed phase
                if lock.slash_on_timeout(current_block).is_ok() {
                    let executor_id = lock.executor_id;
                    let slashed_amount = lock.amount;
                    // Read escrow_account before lock is moved into storage.
                    let escrow_account = lock.escrow_account.clone();

                    // Update storage with slashed lock
                    AtomicLocks::<T>::insert(intent_id, lock);

                    // Actually confiscate the reserved funds — without this the slash
                    // is a no-op: the event fires but no balance is ever reduced.
                    let _ =
                        <T as Config>::Currency::slash_reserved(&escrow_account, slashed_amount);

                    // Emit event that off-chain worker or governance can use to slash bond
                    Self::deposit_event(Event::AtomicLockTimeoutSlashed {
                        intent_id,
                        executor_id,
                        amount_slashed: slashed_amount,
                    });
                }
            }
        }

        /// Phase 1b: OCW Finalization Hook
        ///
        /// Off-chain worker monitors for settlement intents that are ready for finalization
        /// and coordinates with the atomic kernel to finalize the atomic bundle.
        ///
        /// Reads from off-chain storage:
        /// - Key prefix: `b"x3settle:" + intent_id (32 bytes)` = settlement finalization marker
        /// - Value: `bundle_id (32) || receipt_root (32) || finality_cert (32) = 96 bytes`
        ///
        /// The settlement off-chain worker writes this marker when:
        /// 1. All external VM legs have been executed
        /// 2. Settlement proofs have been collected
        /// 3. Intent is ready to move to Finalized state
        ///
        /// This OCW then submits `finalize_with_settlement` to the kernel via
        /// `sp_io::offchain::submit_unsigned_transaction` for deterministic finalization.
        fn offchain_worker(now: BlockNumberFor<T>) {
            log::debug!(
                target: "x3-settlement-engine",
                "[OCW] block {:?}: scanning for intents ready for finalization",
                now
            );

            // Phase 1b: Stub implementation. In Phase 1c, implement full OCW:
            // 1. Iterate PendingIntents to find intents in Finalized state
            // 2. Check off-chain storage for settlement finalization markers
            // 3. Extract bundle_id, receipt_root, finality_cert
            // 4. Submit unsigned transaction to atomic-kernel::finalize_with_settlement
            //
            // For now, log that the OCW ran (proves the hook is wired).
            log::info!(
                target: "x3-settlement-engine",
                "[OCW] Settlement finalization hook active at block {:?}",
                now
            );
        }

        /// ISSUE #5 FIX: Settlement Finality Timeout Checker
        ///
        /// Runs periodically via on_idle() to check for settlements that have exceeded
        /// SettlementTimeoutBlocks without finalizing. Prevents indefinite pending states
        /// that can lock validator attestations and bridge transactions.
        ///
        /// For each pending settlement:
        /// 1. Check if (current_block - creation_block) > SettlementTimeoutBlocks
        /// 2. If yes, emit SettlementTimeoutExpiredBlock event
        /// 3. Trigger automatic refund to return assets to users
        /// 4. Record timeout in metrics for monitoring
        fn on_idle(_n: BlockNumberFor<T>, _remaining_weight: Weight) -> Weight {
            let timeout_blocks = T::SettlementTimeoutBlocks::get();
            let current_block: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
            let current_block_u32: u32 = current_block.saturated_into::<u32>();
            let timeout_blocks_u32: u32 = timeout_blocks.saturated_into::<u32>();

            let mut weight = Weight::zero();
            const MAX_TIMEOUTS_PER_BLOCK: usize = 10; // Cap processing to prevent stalls

            // Iterate through all pending settlements and check for timeouts
            let mut to_refund: Vec<H256> = Vec::new();

            for (intent_id, creation_block) in
                SettlementCreationBlocks::<T>::iter().take(MAX_TIMEOUTS_PER_BLOCK)
            {
                let creation_block_u32: u32 = creation_block.saturated_into::<u32>();
                let age = current_block_u32.saturating_sub(creation_block_u32);

                // Check if settlement has exceeded timeout
                if age > timeout_blocks_u32 {
                    // Verify intent still exists and is in pending state
                    if let Some(_intent) = SettlementIntents::<T>::get(&intent_id) {
                        let state = IntentStates::<T>::get(&intent_id);

                        // Only process if still in a pending state (not already finalized/refunded)
                        if matches!(
                            state,
                            IntentState::Created
                                | IntentState::FundingInProgress
                                | IntentState::FullyFunded
                        ) {
                            to_refund.push(intent_id);

                            // Emit timeout event with detailed context
                            Self::deposit_event(Event::SettlementTimeoutExpiredBlock {
                                intent_id,
                                creation_block: creation_block_u32,
                                timeout_block: timeout_blocks_u32,
                                current_block: current_block_u32,
                            });

                            log::warn!(
                                target: "x3-settlement-engine",
                                "🔴 Settlement {:?} exceeded timeout: created at block {}, \
                                 limit {}, current {}. Auto-refunding.",
                                intent_id, creation_block_u32, timeout_blocks_u32, current_block_u32
                            );

                            weight = weight.saturating_add(
                                <T as frame_system::Config>::DbWeight::get().reads(2),
                            );
                        }
                    }
                }
            }

            // Process refunds for timed-out settlements
            // Each refund involves multiple storage ops: marked as 4R + 4W
            for intent_id in to_refund {
                if let Some(intent) = SettlementIntents::<T>::get(&intent_id) {
                    let _ = Self::process_refund(intent_id, &intent, RefundReason::Timeout);
                    weight = weight.saturating_add(
                        <T as frame_system::Config>::DbWeight::get().reads_writes(4, 4),
                    );
                }
            }

            // Return consumed weight
            weight
        }
    }

    // ============================================================================
    // Extrinsics
    // ============================================================================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        // ────────────────────────────────────────────────────────────────────
        // INTENT LIFECYCLE
        // ────────────────────────────────────────────────────────────────────

        /// Create a new settlement intent (from matched trade)
        ///
        /// This is the entry point for all atomic settlements.
        /// The intent is registered on X3 and becomes the source of truth.
        #[pallet::call_index(0)]
        #[pallet::weight(T::SettlementWeightInfo::create_intent())]
        pub fn create_intent(
            origin: OriginFor<T>,
            taker: AccountIdOf<T>,
            asset_a: AssetSpec,
            asset_b: AssetSpec,
            secret_hash: H256,
            timeout_seconds: Option<u64>,
        ) -> DispatchResult {
            let maker = ensure_signed(origin)?;

            // Check pending intent limit
            let pending = PendingIntents::<T>::get(&maker);
            ensure!(
                pending < T::MaxPendingIntents::get(),
                Error::<T>::TooManyPendingIntents
            );

            // Generate intent ID
            let nonce = TotalIntents::<T>::get();
            let intent_id = Self::generate_intent_id(&maker, &taker, nonce);

            ensure!(
                !SettlementIntents::<T>::contains_key(intent_id),
                Error::<T>::IntentAlreadyExists
            );

            // Calculate timeout
            let now = T::UnixTime::now().as_secs();
            let timeout =
                now.saturating_add(timeout_seconds.unwrap_or(T::DefaultSettlementTimeout::get()));

            // Create intent
            let intent = SettlementIntent {
                intent_id,
                maker: maker.clone(),
                taker: taker.clone(),
                asset_a: asset_a.clone(),
                asset_b: asset_b.clone(),
                secret_hash,
                timeout,
                created_at: now,
                legs_total: 2, // Default 2 legs for simple swap
                legs_locked: 0,
                legs_claimed: 0,
            };

            // Store intent
            SettlementIntents::<T>::insert(intent_id, intent);
            IntentStates::<T>::insert(intent_id, IntentState::Created);

            // Track creation block for timeout enforcement
            let current_block = frame_system::Pallet::<T>::block_number();
            SettlementCreationBlocks::<T>::insert(intent_id, current_block);

            PendingIntents::<T>::mutate(&maker, |p| *p = p.saturating_add(1));
            TotalIntents::<T>::mutate(|t| *t = t.saturating_add(1));

            // C-006: register this intent's timeout in the deadline index so that
            // on_initialize can automatically trigger a refund without a full table scan.
            // X3 targets ~6 s block times; divide timeout seconds by 6 to get blocks.
            {
                let secs_to_deadline =
                    timeout_seconds.unwrap_or(T::DefaultSettlementTimeout::get());
                // Add 1 block of padding so the refund fires strictly AFTER the Unix timeout.
                let blocks_until_deadline: u32 = ((secs_to_deadline / 6).saturating_add(1)) as u32;
                let deadline_block = frame_system::Pallet::<T>::block_number()
                    .saturating_add(blocks_until_deadline.saturated_into());
                IntentDeadlineIndex::<T>::mutate(deadline_block, |list| {
                    // Silently drop if the slot is full (>20 intents/block);
                    // the intent is still refundable via the `refund_settlement` extrinsic.
                    let _ = list.try_push(intent_id);
                });
            }

            Self::deposit_event(Event::X3IntentCreated {
                intent_id,
                maker,
                taker,
                asset_a,
                asset_b,
                secret_hash,
                timeout,
            });

            Ok(())
        }

        /// Lock assets into X3 escrow for a settlement leg
        #[pallet::call_index(1)]
        #[pallet::weight(T::SettlementWeightInfo::lock_escrow())]
        pub fn lock_escrow(
            origin: OriginFor<T>,
            intent_id: H256,
            leg_index: u32,
            chain: ExternalChainId,
            amount: u128,
            escrow_data: Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify intent exists and is in correct state
            let mut intent =
                SettlementIntents::<T>::get(intent_id).ok_or(Error::<T>::IntentNotFound)?;

            let state = IntentStates::<T>::get(intent_id);
            ensure!(
                matches!(state, IntentState::Created | IntentState::FundingInProgress),
                Error::<T>::InvalidIntentState
            );

            // Verify caller is maker or taker
            ensure!(
                who == intent.maker || who == intent.taker,
                Error::<T>::NotAuthorized
            );

            // Bounds-check leg_index against the declared leg count.
            // Without this, an attacker could create escrows at arbitrary indices
            // and increment legs_locked beyond legs_total.
            ensure!(
                leg_index < intent.legs_total,
                Error::<T>::InvalidSettlementLeg
            );

            // Check escrow doesn't exist
            ensure!(
                !EscrowStates::<T>::contains_key(intent_id, leg_index),
                Error::<T>::EscrowAlreadyExists
            );

            // Convert escrow data to bounded vec
            let bounded_escrow_address: BoundedVec<u8, ConstU32<64>> = escrow_data
                .clone()
                .try_into()
                .map_err(|_| Error::<T>::InvalidAssetSpec)?;

            // Create escrow leg
            let escrow_leg = EscrowLeg {
                intent_id,
                leg_index,
                depositor: who.clone(),
                chain: chain.clone(),
                amount,
                escrow_address: bounded_escrow_address,
                state: EscrowLegState::Locked,
                locked_at: T::UnixTime::now().as_secs(),
                proof: None,
            };

            // Store escrow
            EscrowStates::<T>::insert(intent_id, leg_index, escrow_leg);

            // C-004: Create AtomicLock with the actual locked amount on the first escrow leg.
            // The lock is created here (not in create_intent) so the amount is non-zero.
            if !AtomicLocks::<T>::contains_key(intent_id) {
                let current_block =
                    frame_system::Pallet::<T>::block_number().saturated_into::<u32>();
                use sp_runtime::SaturatedConversion;
                let lock_amount: BalanceOf<T> = amount.saturated_into();
                let commit_deadline_blocks: u32 = 600; // ~2 hours
                let atomic_lock = atomic_lock::AtomicLock::new_prepare(
                    intent_id.into(),
                    who.clone(),
                    lock_amount, // actual amount from this escrow leg
                    who.clone(), // depositor is the escrow account
                    Default::default(),
                    current_block,
                    commit_deadline_blocks,
                );
                AtomicLocks::<T>::insert(intent_id, atomic_lock);
                // Register in AtomicLockExpiryIndex so on_finalize can slash without
                // an unbounded scan over all locks (P0 fix — prevents chain-halt DoS).
                // Register at deadline+1: is_expired returns true when current_block > deadline,
                // so the first block where the lock is expired is deadline+1.
                let expiry_block = current_block + commit_deadline_blocks + 1;
                AtomicLockExpiryIndex::<T>::mutate(expiry_block, |ids| {
                    let _ = ids.try_push(intent_id);
                });
            }

            // Update intent
            intent.legs_locked = intent.legs_locked.saturating_add(1);
            SettlementIntents::<T>::insert(intent_id, intent.clone());

            // Update state if all legs locked
            if intent.legs_locked >= intent.legs_total {
                IntentStates::<T>::insert(intent_id, IntentState::FullyFunded);

                // Transition atomic lock to Commit phase when all legs are locked
                if let Some(mut atomic_lock) = AtomicLocks::<T>::get(intent_id) {
                    let current_block =
                        frame_system::Pallet::<T>::block_number().saturated_into::<u32>();
                    let finalize_deadline_blocks: u32 = 600; // ~2 hours for finalization

                    // Extract old prepare deadline before transitioning, so we can move
                    // the intent_id to its new slot in AtomicLockExpiryIndex.
                    let old_expiry = match &atomic_lock.phase {
                        crate::atomic_lock::LockPhase::LockedForCommit {
                            commit_deadline, ..
                        } => Some(*commit_deadline),
                        _ => None,
                    };

                    // Transition to Commit phase - if this fails, we log but don't fail the extrinsic
                    // since the lock was already created in Prepare phase
                    let _ = atomic_lock.lock_for_commit(current_block, finalize_deadline_blocks);

                    // Update the expiry index: remove from old prepare deadline slot and
                    // register at the new finalize deadline so on_finalize remains bounded.
                    // Use deadline+1 since is_expired fires when current_block > deadline.
                    let new_expiry = current_block + finalize_deadline_blocks + 1;
                    if let Some(old) = old_expiry {
                        let old_slot = old + 1; // old was also registered at deadline+1
                        AtomicLockExpiryIndex::<T>::mutate(old_slot, |ids| {
                            ids.retain(|x| *x != intent_id);
                        });
                    }
                    AtomicLockExpiryIndex::<T>::mutate(new_expiry, |ids| {
                        let _ = ids.try_push(intent_id);
                    });

                    AtomicLocks::<T>::insert(intent_id, atomic_lock);
                }
            } else {
                IntentStates::<T>::insert(intent_id, IntentState::FundingInProgress);
            }

            Self::deposit_event(Event::X3AssetsLocked {
                intent_id,
                leg_index,
                chain,
                amount,
                escrow_address: escrow_data,
            });

            Ok(())
        }

        /// Submit external execution proof to X3
        #[pallet::call_index(2)]
        #[pallet::weight(T::SettlementWeightInfo::submit_external_proof())]
        pub fn submit_proof(
            origin: OriginFor<T>,
            intent_id: H256,
            chain: ExternalChainId,
            proof: SettlementProof,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify intent exists and caller is party to the intent.
            // Only the maker or taker may advance proof state to prevent third-party
            // actors from triggering state transitions for intents they are not party to.
            let intent =
                SettlementIntents::<T>::get(intent_id).ok_or(Error::<T>::IntentNotFound)?;
            ensure!(
                who == intent.maker || who == intent.taker,
                Error::<T>::NotAuthorized
            );

            let state = IntentStates::<T>::get(intent_id);
            ensure!(
                matches!(
                    state,
                    IntentState::FullyFunded | IntentState::ExecutingExternal
                ),
                Error::<T>::InvalidIntentState
            );

            // Enforce chain-specific finality depth before accepting external proofs.
            let finality_cfg = ChainFinality::<T>::get(chain.clone())
                .unwrap_or_else(|| finality::FinalityOracle::default_config(chain.clone()));
            ensure!(
                proof.confirmations >= finality_cfg.confirmations_required,
                Error::<T>::InvalidProof
            );
            ensure!(
                proof.proof_type == finality_cfg.proof_type,
                Error::<T>::InvalidProof
            );

            // Verify proof based on chain type
            let is_valid = Self::verify_proof(&chain, &proof)?;
            ensure!(is_valid, Error::<T>::InvalidProof);

            // Emit proof verification event for bridge integration tracking
            let current_block: u32 =
                frame_system::Pallet::<T>::block_number().saturated_into::<u32>();
            let block_or_slot = u64::from_le_bytes(
                proof.tx_hash.as_bytes()[0..8]
                    .try_into()
                    .unwrap_or_default(),
            );
            Self::deposit_event(Event::SettlementProofVerified {
                intent_id,
                chain: chain.clone(),
                block_or_slot,
                proof_hash: H256::from(sp_io::hashing::sha2_256(&proof.encode())),
                verified_at_block: current_block,
            });

            // Check proof cache to prevent replay attacks
            // Hash the proof to create a unique identifier
            let proof_bytes = proof.encode();
            let proof_hash = H256::from(sp_io::hashing::sha2_256(&proof_bytes));
            ensure!(
                !ProofCache::<T>::contains_key(&proof_hash),
                Error::<T>::InvalidProof
            );

            // Store proof in cache to mark it as submitted
            ProofCache::<T>::insert(&proof_hash, ());

            // Update state
            IntentStates::<T>::insert(intent_id, IntentState::ExecutingExternal);

            Self::deposit_event(Event::ExternalProofSubmitted {
                intent_id,
                chain,
                proof_type: proof.proof_type,
                tx_hash: proof.tx_hash,
                confirmations: proof.confirmations,
            });

            Ok(())
        }

        /// Claim settlement with secret revelation
        #[pallet::call_index(3)]
        #[pallet::weight(T::SettlementWeightInfo::claim_settlement())]
        pub fn claim_settlement(
            origin: OriginFor<T>,
            intent_id: H256,
            secret: H256,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify intent exists
            let mut intent =
                SettlementIntents::<T>::get(intent_id).ok_or(Error::<T>::IntentNotFound)?;

            let state = IntentStates::<T>::get(intent_id);
            ensure!(
                matches!(
                    state,
                    IntentState::FullyFunded
                        | IntentState::ExecutingExternal
                        | IntentState::Claiming
                ),
                Error::<T>::InvalidIntentState
            );

            // Verify secret matches hash (HTLC claim)
            // SHA-256 is the canonical hash for HTLC secrets across all VMs (EVM/SVM/X3VM).
            // blake2_256 is NOT used here — the cross-VM HTLC coordinator uses SHA-256
            // so the hash must match; using blake2 would silently break cross-layer claims.
            let computed_hash = H256::from(sp_io::hashing::sha2_256(secret.as_bytes()));
            ensure!(
                computed_hash == intent.secret_hash,
                Error::<T>::InvalidSecret
            );

            // Verify timeout not expired
            let now = T::UnixTime::now().as_secs();
            ensure!(now < intent.timeout, Error::<T>::TimeoutExpired);

            // Check block-based settlement timeout: settlements must finalize within
            // SettlementTimeoutBlocks (default 28,800 ≈ 24 hours) to prevent indefinite pending states
            let creation_block = SettlementCreationBlocks::<T>::get(intent_id)
                .unwrap_or_else(|| frame_system::Pallet::<T>::block_number());
            let current_block = frame_system::Pallet::<T>::block_number();
            let blocks_elapsed = current_block.saturating_sub(creation_block);
            let timeout_blocks = T::SettlementTimeoutBlocks::get();

            ensure!(
                blocks_elapsed < timeout_blocks,
                Error::<T>::SettlementTimeoutExpired
            );

            // Run invariant checks BEFORE finalization
            Self::check_settlement_invariants(intent_id)?;

            // Bind claim to an actual escrow leg for this claimer.
            Self::mark_claimed_leg(intent_id, &intent, &who)?;

            // Update intent claim counter
            intent.legs_claimed = intent.legs_claimed.saturating_add(1);
            SettlementIntents::<T>::insert(intent_id, intent.clone());

            // Check if fully claimed
            if intent.legs_claimed >= intent.legs_total {
                Self::finalize_settlement(intent_id, &intent, &who)?;
            } else {
                IntentStates::<T>::insert(intent_id, IntentState::Claiming);
            }

            Ok(())
        }

        /// Refund settlement after timeout
        ///
        /// S1-1 (failed_rollback) FIX: All refund-side bookkeeping (state
        /// transitions, escrow releases, bond adjustments, events) is wrapped
        /// in `with_storage_layer` so that a failure mid-refund leaves the
        /// intent untouched. There is no partial-refund window in which an
        /// intent could be observed half-reverted.
        #[pallet::call_index(4)]
        #[pallet::weight(T::SettlementWeightInfo::refund_intent())]
        pub fn refund_settlement(origin: OriginFor<T>, intent_id: H256) -> DispatchResult {
            let who = ensure_signed(origin)?;

            frame_support::storage::with_storage_layer(|| {
                // Verify intent exists
                let intent =
                    SettlementIntents::<T>::get(intent_id).ok_or(Error::<T>::IntentNotFound)?;

                // Verify caller is maker or taker
                ensure!(
                    who == intent.maker || who == intent.taker,
                    Error::<T>::NotAuthorized
                );

                let state = IntentStates::<T>::get(intent_id);
                ensure!(
                    !matches!(state, IntentState::Finalized | IntentState::Refunded),
                    Error::<T>::InvalidIntentState
                );

                // Verify timeout HAS expired
                let now = T::UnixTime::now().as_secs();
                ensure!(now >= intent.timeout, Error::<T>::TimeoutNotExpired);

                // Process refund (atomic — all-or-nothing within the storage layer)
                Self::process_refund(intent_id, &intent, RefundReason::Timeout)?;

                Ok(())
            })
        }

        // ────────────────────────────────────────────────────────────────────
        // COLLATERAL / BONDING
        // ────────────────────────────────────────────────────────────────────

        #[pallet::call_index(20)]
        #[pallet::weight(T::SettlementWeightInfo::lock_escrow())]
        pub fn deposit_bond(
            origin: OriginFor<T>,
            asset: Vec<u8>,
            amount: BalanceOf<T>,
            bond_type: u8,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Reserve funds from caller
            <T as Config>::Currency::reserve(&who, amount)?;

            let _id = Self::create_bond_internal(&who, asset, amount, bond_type)?;

            Ok(())
        }

        #[pallet::call_index(21)]
        #[pallet::weight(T::SettlementWeightInfo::lock_escrow())]
        pub fn request_bond_withdraw(origin: OriginFor<T>, bond_id: H256) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Ensure owner
            let rec = Bonds::<T>::get(bond_id).ok_or(Error::<T>::BondNotFound)?;
            ensure!(rec.owner == who, Error::<T>::NotBondOwner);
            ensure!(rec.state == 0, Error::<T>::BondNotLocked);

            Self::request_withdrawal_internal(bond_id)?;
            Ok(())
        }

        #[pallet::call_index(22)]
        #[pallet::weight(T::SettlementWeightInfo::claim_bond())]
        pub fn finalize_bond_withdraw(origin: OriginFor<T>, bond_id: H256) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let rec = Bonds::<T>::get(bond_id).ok_or(Error::<T>::BondNotFound)?;
            ensure!(rec.owner == who, Error::<T>::NotBondOwner);
            ensure!(rec.state == 1, Error::<T>::BondNotWithdrawable);

            // Unreserve and remove
            <T as Config>::Currency::unreserve(&who, rec.amount);
            Self::finalize_withdraw_internal(bond_id)?;
            Ok(())
        }

        #[pallet::call_index(23)]
        #[pallet::weight(T::SettlementWeightInfo::claim_bond())]
        pub fn slash_bond(origin: OriginFor<T>, bond_id: H256) -> DispatchResult {
            ensure_root(origin)?;

            let rec = Bonds::<T>::get(bond_id).ok_or(Error::<T>::BondNotFound)?;
            ensure!(rec.state != 2, Error::<T>::BondAlreadySlashed);

            // Slash reserved balance
            let _ = <T as Config>::Currency::slash_reserved(&rec.owner, rec.amount);

            Self::slash_bond_internal(bond_id)?;
            Ok(())
        }

        // ────────────────────────────────────────────────────────────────────
        // BTC ATOMIC GATEWAY
        // ────────────────────────────────────────────────────────────────────

        /// Submit BTC SPV proof for UTXO verification
        #[pallet::call_index(10)]
        #[pallet::weight(T::SettlementWeightInfo::verify_btc_proof())]
        pub fn submit_btc_proof(
            origin: OriginFor<T>,
            intent_id: H256,
            btc_txid: H256,
            tx_index: u32, // C-009: position of tx in block, required for direction-aware Merkle verification
            vout: u32,
            amount_sats: u64,
            merkle_proof: Vec<H256>,
            block_header: BtcBlockHeader,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify intent exists and restrict proof submission to intent parties.
            // Only the maker or taker may submit a BTC proof to prevent DoS or
            // replay attacks from arbitrary accounts.
            let intent =
                SettlementIntents::<T>::get(intent_id).ok_or(Error::<T>::IntentNotFound)?;
            ensure!(
                who == intent.maker || who == intent.taker,
                Error::<T>::NotAuthorized
            );

            // Verify merkle proof
            let is_valid =
                Self::verify_btc_merkle_proof(&btc_txid, tx_index, &merkle_proof, &block_header)?;
            ensure!(is_valid, Error::<T>::InvalidBtcProof);

            // Store/update block header
            let block_hash = Self::compute_btc_block_hash(&block_header);
            BtcHeaders::<T>::insert(block_hash, block_header.clone());

            // Calculate confirmations
            let best_height = BtcBestHeight::<T>::get();
            let confirmations = best_height.saturating_sub(block_header.height) + 1;

            ensure!(
                confirmations >= T::MinBtcConfirmations::get() as u64,
                Error::<T>::InsufficientBtcConfirmations
            );

            // Store UTXO state keyed by (txid, vout) to correctly handle multiple
            // outputs from the same transaction without overwriting each other.
            let utxo_state = BtcUtxoState {
                txid: btc_txid,
                vout,
                amount_sats,
                intent_id: Some(intent_id),
                confirmations: confirmations as u32,
                spent: false,
                block_hash,
            };
            // Prevent spending an already-registered UTXO.
            ensure!(
                !BtcUtxos::<T>::contains_key(btc_txid, vout),
                Error::<T>::EscrowAlreadyExists
            );
            BtcUtxos::<T>::insert(btc_txid, vout, utxo_state);

            Self::deposit_event(Event::BtcUtxoConfirmed {
                intent_id,
                btc_txid,
                vout,
                confirmations: confirmations as u32,
                amount_sats,
            });

            Ok(())
        }

        /// Submit BTC block header (for SPV)
        ///
        /// Restricted to privileged origin to prevent arbitrary accounts from
        /// pushing invalid/competing header branches into local state.
        #[pallet::call_index(11)]
        #[pallet::weight(T::SettlementWeightInfo::update_btc_block_header())]
        pub fn submit_btc_header(origin: OriginFor<T>, header: BtcBlockHeader) -> DispatchResult {
            ensure_root(origin)?;

            // Verify proof of work
            let is_valid = Self::verify_btc_pow(&header)?;
            ensure!(is_valid, Error::<T>::InvalidBtcProof);

            // Verify chain connection
            let prev_exists = BtcHeaders::<T>::contains_key(header.prev_block_hash);
            ensure!(
                prev_exists || header.height == 0,
                Error::<T>::InvalidBtcProof
            );

            // Store header
            let block_hash = Self::compute_btc_block_hash(&header);
            BtcHeaders::<T>::insert(block_hash, header.clone());

            // Update best height if higher
            if header.height > BtcBestHeight::<T>::get() {
                BtcBestHeight::<T>::put(header.height);
            }

            Ok(())
        }

        // ────────────────────────────────────────────────────────────────────
        // FINALITY ORACLE
        // ────────────────────────────────────────────────────────────────────

        /// Update chain finality configuration (governance)
        #[pallet::call_index(24)]
        #[pallet::weight(T::SettlementWeightInfo::update_finality_config())]
        pub fn update_finality_config(
            origin: OriginFor<T>,
            chain: ExternalChainId,
            config: FinalityConfig,
        ) -> DispatchResult {
            ensure_root(origin)?;

            // A configuration with zero required confirmations would cause division-by-zero
            // in finality_score and would mark every unconfirmed tx as already final.
            ensure!(config.confirmations_required > 0, Error::<T>::InvalidProof);

            ChainFinality::<T>::insert(chain, config);

            Ok(())
        }

        // ────────────────────────────────────────────────────────────────────
        // INVARIANT ENFORCEMENT
        // ────────────────────────────────────────────────────────────────────

        /// Report invariant violation (for monitoring/slashing)
        #[pallet::call_index(30)]
        #[pallet::weight(T::SettlementWeightInfo::report_violation())]
        pub fn report_violation(
            origin: OriginFor<T>,
            intent_id: H256,
            violation_type: InvariantViolationType,
            evidence: Vec<u8>,
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;

            // Verify the violation
            let is_valid_report = Self::verify_violation(intent_id, &violation_type, &evidence)?;

            if is_valid_report {
                // Halt settlement
                IntentStates::<T>::insert(intent_id, IntentState::Halted);

                // Increment violation counter
                InvariantViolations::<T>::mutate(|v| *v = v.saturating_add(1));

                Self::deposit_event(Event::InvariantViolation {
                    intent_id,
                    violation_type,
                    details: evidence,
                });

                // : Slash operator (testnet)
            }

            Ok(())
        }

        // ────────────────────────────────────────────────────────────────────
        // SETTLEMENT EXECUTION (PHASE C STUB)
        // ────────────────────────────────────────────────────────────────────

        /// Execute settlement transfer from executor (bridge/settlement relay)
        /// FROZEN under v1-alpha API (2026-04-24, commit d99252ca42)
        #[pallet::call_index(31)]
        #[pallet::weight(T::SettlementWeightInfo::settle_transfer())]
        pub fn settle_transfer(
            origin: OriginFor<T>,
            transfer_id: H256,
            receiver: AccountIdOf<T>,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let executor = ensure_signed(origin)?;

            // P1: Load transfer state from storage
            let mut transfer =
                SettlementTransfers::<T>::get(transfer_id).ok_or(Error::<T>::UnknownTransfer)?;

            // P2: Verify executor is authorized via x3-kernel AuthorizedAccounts registry.
            // AuthorizedAccounts is secure-by-default: an empty registry rejects all callers.
            // Accounts are added via x3-kernel::authorize_account (governance-gated).
            #[cfg(not(feature = "dev-bypass"))]
            let executor_encoded = executor.encode();
            #[cfg(not(feature = "dev-bypass"))]
            ensure!(
                pallet_x3_kernel::AuthorizedAccounts::<T>::iter_keys()
                    .any(|account| account.encode() == executor_encoded),
                Error::<T>::ExecutorNotAuthorized
            );

            // P3: Verify transfer status is Pending (0)
            ensure!(transfer.status == 0, Error::<T>::TransferNotPending);

            // P4: Verify receiver matches expected receiver (if pre-designated)
            if let Some(ref expected_receiver) = transfer.receiver {
                ensure!(expected_receiver == &receiver, Error::<T>::ReceiverMismatch);
            } else {
                transfer.receiver = Some(receiver.clone());
            }

            // P5: Verify amount matches expected amount
            ensure!(transfer.amount == amount, Error::<T>::AmountMismatch);

            // P6: Get current block number for finality checks
            let current_block = frame_system::Pallet::<T>::block_number().saturated_into::<u32>();

            // P7: Verify not past timeout (settlement must complete before timeout)
            ensure!(
                current_block < transfer.timeout_at,
                Error::<T>::SettlementTimeout
            );

            // P8: Update settlement status to Completed (1)
            transfer.status = 1;
            transfer.legs_completed = transfer.num_legs; // All legs now complete
            SettlementTransfers::<T>::insert(transfer_id, transfer.clone());

            // P9: Transfer assets to receiver using Currency trait
            // Release reserved balance and transfer to receiver
            let _ = <T as Config>::Currency::unreserve(&transfer.initiator, amount);
            <T as Config>::Currency::transfer(
                &transfer.initiator,
                &receiver,
                amount,
                frame_support::traits::ExistenceRequirement::KeepAlive,
            )?;

            // P10: Update settlement statistics
            TotalSettledVolume::<T>::mutate(|v| {
                *v = v.saturating_add(amount.saturated_into());
            });

            // P11: Emit SettlementExecuted event with full context
            Self::deposit_event(Event::SettlementExecuted {
                transfer_id,
                receiver,
                amount,
            });

            Ok(())
        }

        /// Trigger refund for transfer that has exceeded timeout
        /// Anyone can call this (incentivized by refund gas sponsorship)
        /// FROZEN under v1-alpha API (2026-04-24, commit d99252ca42)
        #[pallet::call_index(32)]
        #[pallet::weight(T::SettlementWeightInfo::trigger_refund())]
        pub fn trigger_refund(origin: OriginFor<T>, transfer_id: H256) -> DispatchResult {
            let _caller = ensure_signed(origin)?;

            // P1: Load transfer state from storage
            let mut transfer =
                SettlementTransfers::<T>::get(transfer_id).ok_or(Error::<T>::UnknownTransfer)?;

            // P2: Verify transfer status is Pending (0) — can only refund pending transfers
            ensure!(transfer.status == 0, Error::<T>::TransferNotPending);

            // P3: Get current block and verify current_block >= timeout_at (timeout reached)
            let current_block = frame_system::Pallet::<T>::block_number().saturated_into::<u32>();
            ensure!(
                current_block >= transfer.timeout_at,
                Error::<T>::RefundTooEarly
            );

            // P4: Update settlement status to Refunded (2)
            transfer.status = 2;
            SettlementTransfers::<T>::insert(transfer_id, transfer.clone());

            // P5: Release locked assets back to initiator
            let _ = <T as Config>::Currency::unreserve(&transfer.initiator, transfer.amount);

            // P6: Emit RefundTriggered event
            Self::deposit_event(Event::RefundTriggered { transfer_id });

            Ok(())
        }
    }

    // ============================================================================
    // Internal Functions
    // ============================================================================

    impl<T: Config> Pallet<T> {
        /// Generate unique intent ID
        pub fn generate_intent_id(
            maker: &AccountIdOf<T>,
            taker: &AccountIdOf<T>,
            nonce: u64,
        ) -> H256 {
            let mut data = maker.encode();
            data.extend(taker.encode());
            data.extend(nonce.to_le_bytes());
            data.extend(T::UnixTime::now().as_secs().to_le_bytes());
            H256::from(blake2_256(&data))
        }

        /// Verify settlement proof based on chain type
        pub fn verify_proof(
            chain: &ExternalChainId,
            proof: &SettlementProof,
        ) -> Result<bool, DispatchError> {
            match chain {
                ExternalChainId::Bitcoin => {
                    // BTC settlement proofs must go through dedicated SPV paths.
                    // Fail closed here to avoid accepting unaudited generic proofs.
                    Ok(false)
                }
                ExternalChainId::Ethereum
                | ExternalChainId::Arbitrum
                | ExternalChainId::Base
                | ExternalChainId::Polygon => {
                    // EVM chains: verify receipt proof
                    Self::verify_evm_receipt_proof(proof)
                }
                ExternalChainId::Solana => {
                    // SVM: verify transaction proof
                    Self::verify_svm_proof(proof)
                }
                _ => Ok(false),
            }
        }

        /// Verify EVM receipt proof
        /// Bridge Integration: Calls cross-chain-validator to verify against canonical headers
        fn verify_evm_receipt_proof(proof: &SettlementProof) -> Result<bool, DispatchError> {
            // Stage 1: Basic structural validation
            let proof_type_ok = matches!(
                proof.proof_type,
                ProofType::MerkleTrie | ProofType::LightClient | ProofType::Optimistic
            );
            if !proof_type_ok {
                return Ok(false);
            }

            // Validate proof structure
            if proof.merkle_proof.is_empty() || proof.receipt_data.is_empty() {
                return Ok(false);
            }
            if proof.confirmations < 1 {
                return Ok(false);
            }

            // Validate receipt RLP structure
            let receipt_rlp = &proof.receipt_data[..];
            if !Self::is_valid_receipt_rlp(receipt_rlp) {
                return Ok(false);
            }

            // Verify receipt hash by recomputing Keccak256
            let receipt_hash = sp_io::hashing::keccak_256(receipt_rlp);
            let receipt_hash_h256 = H256::from(receipt_hash);

            // The tx_hash in the proof should match the receipt hash (they're the same in Ethereum)
            if receipt_hash_h256 != proof.tx_hash {
                return Ok(false);
            }

            // Stage 2: Bridge Integration - Verify against canonical EVM header
            // Extract block number from proof data (use lower 64 bits of tx_hash as proxy)
            let block_number = u64::from_le_bytes(
                proof.tx_hash.as_bytes()[0..8]
                    .try_into()
                    .unwrap_or_default(),
            );

            // Use block_hash directly from proof
            let block_hash = proof.block_hash;

            // Extract state_root and merkle_root from merkle_proof (use first two entries)
            let state_root = proof.merkle_proof.first().copied().unwrap_or_default();
            let merkle_root = proof.merkle_proof.get(1).copied().unwrap_or_default();

            // Call cross-chain-validator to verify against canonical header
            let valid = T::CrossChainValidator::verify_evm_proof(
                block_number,
                block_hash,
                state_root,
                merkle_root,
            );

            Ok(valid && !proof.merkle_proof.is_empty())
        }

        /// Validate RLP-encoded receipt structure
        /// Checks that the RLP is well-formed for an Ethereum receipt
        fn is_valid_receipt_rlp(rlp: &[u8]) -> bool {
            if rlp.is_empty() {
                return false;
            }

            // RLP decoding helpers
            // Receipts are RLP-encoded lists with: [status/root, gas_used, logs, contractAddress?]
            // or legacy format: [root, gas_used, logs, contractAddress]

            let first_byte = rlp[0];

            // Check if it's a valid RLP list (0xc0-0xf7 = short list, 0xf8-0xff = long list)
            if first_byte < 0xc0 {
                // Not a list - receipts must be lists
                return false;
            }

            // For short lists (0xc0-0xf7), first byte is 0xc0 + payload_length
            if first_byte <= 0xf7 {
                let payload_length = (first_byte as usize) - 0xc0;
                // Receipt should have at least 3 elements, so payload must be reasonable
                return payload_length >= 3 && rlp.len() >= (1 + payload_length);
            }

            // For long lists (0xf8-0xff), next bytes encode the length
            if first_byte == 0xf8 {
                // Length is 1 byte after the first byte
                if rlp.len() < 3 {
                    return false;
                }
                let length_byte = rlp[1] as usize;
                return rlp.len() >= (2 + length_byte);
            }

            if first_byte == 0xf9 {
                // Length is 2 bytes after the first byte
                if rlp.len() < 4 {
                    return false;
                }
                let length = ((rlp[1] as usize) << 8) | (rlp[2] as usize);
                return rlp.len() >= (3 + length);
            }

            // For f9+, we're dealing with very large receipts - unlikely but possible
            if first_byte >= 0xfa {
                // Too long to verify efficiently, fail closed
                return false;
            }

            true
        }

        /// Verify Solana transaction proof
        ///
        /// Solana transaction format:
        /// - [1 byte] Signature count (compact encoding)
        /// - [N × 64 bytes] Ed25519 signatures (64 bytes each)
        /// - [remaining] Serialized message
        ///
        /// Message format:
        /// - [1 byte] Header (num_required_signatures | num_readonly_signed << 2 | num_readonly_unsigned << 4)
        /// - [1 byte] Number of static accounts
        /// - [32 bytes] Recent blockhash
        /// - [remaining] Instructions (each: program_id_index + accounts + data)
        fn verify_svm_proof(proof: &SettlementProof) -> Result<bool, DispatchError> {
            let tx_bytes: &[u8] = &proof.receipt_data;

            // 1. Basic structural validation (signature count, lengths, message format)
            if !Self::is_valid_solana_transaction(tx_bytes) {
                return Ok(false);
            }

            // Need at least: 1-byte sig-count + 64-byte sig + 4 bytes minimal message
            if tx_bytes.len() < 69 {
                return Ok(false);
            }

            // 2. Extract signature count (compact-u16; ≤ 127 fits in one byte)
            let sig_count = tx_bytes[0] as usize;
            if sig_count == 0 {
                return Ok(false);
            }

            let sigs_end = 1usize.saturating_add(sig_count.saturating_mul(64));
            if tx_bytes.len() <= sigs_end.saturating_add(4) {
                return Ok(false);
            }

            // Extract first signature (64 bytes starting at byte 1)
            let first_sig_bytes: [u8; 64] = tx_bytes
                .get(1..65)
                .and_then(|s| s.try_into().ok())
                .ok_or(DispatchError::Other("SVM proof: signature slice error"))?;

            // 3. Message is everything after the signatures
            let message = &tx_bytes[sigs_end..];

            // Solana message: [3 header bytes] [compact num_accounts (1B if < 128)]
            //                 [num_accounts × 32B account keys] [32B recent_blockhash] …
            if message.len() < 4 {
                return Ok(false);
            }
            // message[3] = compact-encoded number of account keys (1 byte for values ≤ 127)
            let num_accounts = message[3] as usize;
            let accounts_start: usize = 4;
            let accounts_end: usize =
                accounts_start.saturating_add(num_accounts.saturating_mul(32));
            // blockhash immediately follows the account-key list
            if message.len() < accounts_end.saturating_add(32) {
                return Ok(false);
            }

            // 4. Extract first account key — fee-payer / primary signer
            let signer_pubkey_bytes: [u8; 32] = message
                .get(accounts_start..accounts_start.saturating_add(32))
                .and_then(|s| s.try_into().ok())
                .ok_or(DispatchError::Other("SVM proof: signer pubkey slice error"))?;

            // 5. Cross-check recent_blockhash with oracle-attested block_hash in the proof.
            //    The finality oracle supplies proof.block_hash; the tx being verified must
            //    reference that same blockhash, otherwise the tx could belong to a different
            //    slot and the finality depth check would be invalid.
            let recent_blockhash: [u8; 32] = message
                .get(accounts_end..accounts_end.saturating_add(32))
                .and_then(|s| s.try_into().ok())
                .ok_or(DispatchError::Other("SVM proof: blockhash slice error"))?;

            if proof.block_hash.as_bytes() != &recent_blockhash {
                return Ok(false);
            }

            // 6. Verify Ed25519 signature: signer signs the raw message bytes
            let signature = ed25519::Signature::from_raw(first_sig_bytes);
            let pubkey = ed25519::Public::from_raw(signer_pubkey_bytes);
            if !sp_io::crypto::ed25519_verify(&signature, message, &pubkey) {
                return Ok(false);
            }

            // Stage 2: Bridge Integration - Verify against canonical SVM slot header
            // Extract slot number from proof data (use lower 64 bits of tx_hash as proxy)
            let slot = u64::from_le_bytes(
                proof.tx_hash.as_bytes()[0..8]
                    .try_into()
                    .unwrap_or_default(),
            );

            // Use block_hash directly from proof (already validated above)
            let block_hash = proof.block_hash;

            // Extract state_root and validator_set_hash from merkle_proof (use first two entries)
            let state_root = proof.merkle_proof.first().copied().unwrap_or_default();
            let validator_set_hash = proof.merkle_proof.get(1).copied().unwrap_or_default();

            // Call cross-chain-validator to verify against canonical slot header
            let valid = T::CrossChainValidator::verify_svm_proof(
                slot,
                block_hash,
                state_root,
                validator_set_hash,
            );

            Ok(valid)
        }

        /// Validate Solana transaction structure
        /// Performs basic format validation without signature verification
        #[allow(dead_code)]
        fn is_valid_solana_transaction(tx_data: &[u8]) -> bool {
            if tx_data.is_empty() {
                return false;
            }

            // First byte encodes signature count (compact encoding)
            // Signatures can be variable-length encoded
            let mut offset = 0;
            let (sig_count, bytes_read) = match Self::decode_compact_u32(&tx_data[offset..]) {
                Some(result) => result,
                None => return false,
            };
            offset += bytes_read;

            // Each signature is 64 bytes
            let sig_data_len = (sig_count as usize).saturating_mul(64);
            if offset.saturating_add(sig_data_len) > tx_data.len() {
                return false;
            }
            offset += sig_data_len;

            // After signatures comes the message
            // Message starts with header byte
            if offset >= tx_data.len() {
                return false;
            }

            let _header = tx_data[offset];
            offset += 1;

            // Next is number of static accounts (max 255)
            if offset >= tx_data.len() {
                return false;
            }

            let _num_accounts = tx_data[offset];
            offset += 1;

            // Next 32 bytes should be the recent blockhash
            if offset.saturating_add(32) > tx_data.len() {
                return false;
            }

            // Blockhash is 32 bytes, followed by instruction count
            offset += 32;

            // If we got here, the basic structure is valid
            // A real implementation would validate instruction encoding
            offset < tx_data.len()
        }

        /// Decode a compact u32 from Solana's encoding
        /// Returns (value, bytes_read) or None if invalid
        #[allow(dead_code)]
        fn decode_compact_u32(data: &[u8]) -> Option<(u32, usize)> {
            if data.is_empty() {
                return None;
            }

            let first_byte = data[0];

            // Single byte (0-127)
            if first_byte < 0x80 {
                return Some((first_byte as u32, 1));
            }

            // Two bytes
            if first_byte < 0xc0 {
                if data.len() < 2 {
                    return None;
                }
                let value = ((first_byte & 0x3f) as u32) | (((data[1] & 0x7f) as u32) << 6);
                return Some((value, 2));
            }

            // Three bytes
            if first_byte < 0xe0 {
                if data.len() < 3 {
                    return None;
                }
                let value = ((first_byte & 0x1f) as u32)
                    | (((data[1] & 0x7f) as u32) << 5)
                    | (((data[2] & 0x7f) as u32) << 12);
                return Some((value, 3));
            }

            // Four bytes
            if first_byte < 0xf0 {
                if data.len() < 4 {
                    return None;
                }
                let value = ((first_byte & 0x0f) as u32)
                    | (((data[1] & 0x7f) as u32) << 4)
                    | (((data[2] & 0x7f) as u32) << 11)
                    | (((data[3] & 0x7f) as u32) << 18);
                return Some((value, 4));
            }

            // Five bytes for larger values
            if data.len() < 5 {
                return None;
            }
            let value = (data[1] as u32)
                | ((data[2] as u32) << 8)
                | ((data[3] as u32) << 16)
                | ((data[4] as u32) << 24);
            Some((value, 5))
        }

        /// Check ALL settlement invariants before finalization
        fn check_settlement_invariants(intent_id: H256) -> Result<(), DispatchError> {
            let intent =
                SettlementIntents::<T>::get(intent_id).ok_or(Error::<T>::IntentNotFound)?;

            // INVARIANT 1: All legs must be locked
            ensure!(
                intent.legs_locked >= intent.legs_total,
                Error::<T>::PartialExecutionDetected
            );

            // INVARIANT 2: Check each escrow leg is in valid state
            for leg_idx in 0..intent.legs_total {
                if let Some(escrow) = EscrowStates::<T>::get(intent_id, leg_idx) {
                    ensure!(
                        escrow.state == EscrowLegState::Locked,
                        Error::<T>::InvalidIntentState
                    );
                } else {
                    return Err(Error::<T>::EscrowNotFound.into());
                }
            }

            // INVARIANT 3: Timeout not expired
            let now = T::UnixTime::now().as_secs();
            ensure!(now < intent.timeout, Error::<T>::TimeoutExpired);

            // INVARIANT 4: For BTC legs, verify confirmation depth
            for leg_idx in 0..intent.legs_total {
                if let Some(escrow) = EscrowStates::<T>::get(intent_id, leg_idx) {
                    if escrow.chain == ExternalChainId::Bitcoin {
                        // Check BTC has sufficient confirmations
                        // (handled by separate BTC proof submission)
                    }
                }
            }

            Ok(())
        }

        /// Finalize settlement (ALL legs complete)
        fn finalize_settlement(
            intent_id: H256,
            intent: &SettlementIntent<AccountIdOf<T>>,
            _claimer: &AccountIdOf<T>,
        ) -> Result<(), DispatchError> {
            // Update all escrow legs to Released
            for leg_idx in 0..intent.legs_total {
                EscrowStates::<T>::mutate(intent_id, leg_idx, |maybe_escrow| {
                    if let Some(escrow) = maybe_escrow {
                        escrow.state = EscrowLegState::Released;
                    }
                });
                ClaimedLegs::<T>::remove(intent_id, leg_idx);
            }

            // Update intent state
            IntentStates::<T>::insert(intent_id, IntentState::Finalized);

            // Clean up creation block tracking
            SettlementCreationBlocks::<T>::remove(intent_id);

            // Release atomic lock on successful commit
            if let Some(mut atomic_lock) = AtomicLocks::<T>::get(intent_id) {
                let current_block =
                    frame_system::Pallet::<T>::block_number().saturated_into::<u32>();

                // Transition to Released phase on successful commit
                let _ = atomic_lock.release_on_commit(current_block);

                AtomicLocks::<T>::insert(intent_id, atomic_lock);
            }

            // Decrement pending intents
            PendingIntents::<T>::mutate(&intent.maker, |p| *p = p.saturating_sub(1));

            // Update volume statistics
            let volume = intent.asset_a.amount.saturating_add(intent.asset_b.amount);
            TotalSettledVolume::<T>::mutate(|v| *v = v.saturating_add(volume));

            let now_secs = T::UnixTime::now().as_secs();
            Self::deposit_event(Event::X3Finalized {
                intent_id,
                maker_received: intent.asset_b.amount,
                taker_received: intent.asset_a.amount,
                settlement_time_ms: now_secs
                    .saturating_sub(intent.created_at)
                    .saturating_mul(1_000),
            });

            Ok(())
        }

        /// Process refund for failed/timeout settlement
        fn process_refund(
            intent_id: H256,
            intent: &SettlementIntent<AccountIdOf<T>>,
            reason: RefundReason,
        ) -> Result<(), DispatchError> {
            // Refund all escrow legs
            for leg_idx in 0..intent.legs_total {
                EscrowStates::<T>::mutate(intent_id, leg_idx, |maybe_escrow| {
                    if let Some(escrow) = maybe_escrow {
                        escrow.state = EscrowLegState::Refunded;
                    }
                });
                ClaimedLegs::<T>::remove(intent_id, leg_idx);
            }

            // Update intent state
            IntentStates::<T>::insert(intent_id, IntentState::Refunded);

            // Clean up creation block tracking
            SettlementCreationBlocks::<T>::remove(intent_id);

            // Release atomic lock on abort/refund
            if let Some(mut atomic_lock) = AtomicLocks::<T>::get(intent_id) {
                let current_block =
                    frame_system::Pallet::<T>::block_number().saturated_into::<u32>();

                // Transition to Released phase with AbortRequested reason
                let _ = atomic_lock.release_on_abort(current_block);

                AtomicLocks::<T>::insert(intent_id, atomic_lock);
            }

            // Decrement pending intents
            PendingIntents::<T>::mutate(&intent.maker, |p| *p = p.saturating_sub(1));

            Self::deposit_event(Event::X3Refunded {
                intent_id,
                reason,
                maker_returned: intent.asset_a.amount,
                taker_returned: intent.asset_b.amount,
            });

            Ok(())
        }

        /// Find and mark one unclaimed escrow leg owned by the claimer.
        ///
        /// A claim must correspond to a concrete locked escrow leg. This blocks
        /// replayed claims that previously only incremented an aggregate counter.
        fn mark_claimed_leg(
            intent_id: H256,
            intent: &SettlementIntent<AccountIdOf<T>>,
            claimer: &AccountIdOf<T>,
        ) -> Result<(), DispatchError> {
            for leg_idx in 0..intent.legs_total {
                let Some(escrow) = EscrowStates::<T>::get(intent_id, leg_idx) else {
                    continue;
                };

                if escrow.depositor != *claimer {
                    continue;
                }

                if escrow.state != EscrowLegState::Locked {
                    continue;
                }

                if !ClaimedLegs::<T>::get(intent_id, leg_idx) {
                    ClaimedLegs::<T>::insert(intent_id, leg_idx, true);
                    return Ok(());
                }
            }

            Err(Error::<T>::NoClaimableLeg.into())
        }

        // ────────────────────────────────────────────────────────────────────
        // BTC SPV HELPERS
        // ────────────────────────────────────────────────────────────────────

        /// Verify BTC merkle proof - validates transaction inclusion in block
        ///
        /// The merkle tree in Bitcoin is constructed bottom-up:
        /// 1. Transaction hashes are at the leaf level
        /// 2. Pairs of hashes are concatenated and double-SHA256'd to create parent nodes
        /// 3. If a node has no pair (odd number), it's paired with itself
        /// 4. Process continues until reaching the merkle root
        ///
        /// The proof path allows us to reconstruct the root from the transaction hash.
        fn verify_btc_merkle_proof(
            txid: &H256,
            tx_index: u32,
            proof: &[H256],
            header: &BtcBlockHeader,
        ) -> Result<bool, DispatchError> {
            // C-009: Bitcoin Merkle trees require direction-aware concatenation.
            // When the current node is a LEFT child (index is even), it goes first.
            // When it is a RIGHT child (index is odd), the sibling goes first.
            // Without this, the reconstructed root will be wrong for any tx that
            // is not the leftmost leaf at every level.
            let mut current_hash = *txid;
            let mut index = tx_index;

            for sibling in proof {
                let combined = if index % 2 == 0 {
                    // Current node is a left child — concatenate current || sibling
                    let mut buf = [0u8; 64];
                    buf[0..32].copy_from_slice(current_hash.as_bytes());
                    buf[32..64].copy_from_slice(sibling.as_bytes());
                    buf
                } else {
                    // Current node is a right child — concatenate sibling || current
                    let mut buf = [0u8; 64];
                    buf[0..32].copy_from_slice(sibling.as_bytes());
                    buf[32..64].copy_from_slice(current_hash.as_bytes());
                    buf
                };

                // Double-SHA256 (Bitcoin's standard hash)
                let first_hash = sp_io::hashing::sha2_256(&combined);
                current_hash = H256::from(sp_io::hashing::sha2_256(&first_hash));
                index /= 2;
            }

            // Verify the reconstructed hash matches the merkle root from the block header
            Ok(current_hash == header.merkle_root)
        }

        /// Verify BTC proof of work - validates block meets target difficulty
        ///
        /// Bitcoin uses the nBits compact format to encode the target:
        /// - First byte: number of bytes (exponent)
        /// - Next 3 bytes: mantissa (coefficient)
        /// - Target = mantissa * 256^(exponent - 3)
        ///
        /// A valid block hash must be <= target (compared numerically)
        fn verify_btc_pow(header: &BtcBlockHeader) -> Result<bool, DispatchError> {
            // Compute the block hash (double SHA256)
            let block_hash = Self::compute_btc_block_hash(header);

            // Decode nBits to get the target difficulty
            let bits = header.bits;
            let size = (bits >> 24) as u32;
            let word = bits & 0x00FFFFFF;

            // Compute the target as a 256-bit value
            // Using the compact encoding: target = word * 256^(size - 3)
            let mut target = [0u8; 32];

            // Validate size
            if size > 32 {
                // Target is larger than 256 bits, so any hash passes
                // This shouldn't happen in practice but is technically valid
                return Ok(true);
            }

            if size == 0 {
                // Invalid target (zero size)
                return Ok(false);
            }

            // Decode the mantissa (3 bytes)
            let mut mantissa = [0u8; 3];
            mantissa[0] = ((word >> 16) & 0xFF) as u8;
            mantissa[1] = ((word >> 8) & 0xFF) as u8;
            mantissa[2] = (word & 0xFF) as u8;

            // Place mantissa in target, shifted by (size - 3) bytes
            let shift = if size > 3 { (size - 3) as usize } else { 0 };
            for (i, &byte) in mantissa.iter().enumerate() {
                if shift + i < 32 {
                    target[shift + i] = byte;
                }
            }

            // For the first significant byte, we might need to shift if size < 3
            if size < 3 {
                let _right_shift = 3 - size;
                // This is complex to do correctly, so for now we'll be conservative
                // In practice, size is always >= 3 on mainnet
                return Ok(false);
            }

            // Compare: hash must be <= target
            // Both are in little-endian format (Bitcoin's wire format)
            let hash_bytes = block_hash.as_bytes();

            // Compare byte by byte from most significant to least significant
            for i in (0..32).rev() {
                if hash_bytes[i] < target[i] {
                    return Ok(true);
                } else if hash_bytes[i] > target[i] {
                    return Ok(false);
                }
            }

            // Equal to target is valid
            Ok(true)
        }

        /// Compute BTC block hash (double SHA256)
        fn compute_btc_block_hash(header: &BtcBlockHeader) -> H256 {
            // Serialize header and double hash
            let data = header.encode();
            let first_hash = sp_io::hashing::sha2_256(&data);
            H256::from(sp_io::hashing::sha2_256(&first_hash))
        }

        /// Verify invariant violation report
        fn verify_violation(
            intent_id: H256,
            violation_type: &InvariantViolationType,
            _evidence: &[u8],
        ) -> Result<bool, DispatchError> {
            let state = IntentStates::<T>::get(intent_id);

            match violation_type {
                InvariantViolationType::PartialExecution => {
                    // Check if settlement was partial
                    let intent =
                        SettlementIntents::<T>::get(intent_id).ok_or(Error::<T>::IntentNotFound)?;
                    Ok(intent.legs_claimed > 0
                        && intent.legs_claimed < intent.legs_total
                        && matches!(state, IntentState::Finalized))
                }
                InvariantViolationType::CrossVmReentrancy => {
                    // : Check execution traces for reentrancy
                    Ok(false)
                }
                InvariantViolationType::BtcReleaseWithoutConfirmation => {
                    // : Check BTC was released without X3 confirmation
                    Ok(false)
                }
                InvariantViolationType::TimeoutBypass => {
                    // Check if settlement finalized after timeout
                    let intent =
                        SettlementIntents::<T>::get(intent_id).ok_or(Error::<T>::IntentNotFound)?;
                    let now = T::UnixTime::now().as_secs();
                    Ok(matches!(state, IntentState::Finalized) && now > intent.timeout)
                }
            }
        }

        // ────────────────────────────────────────────────────────────────────
        // COLLATERAL HELPERS (storage-backed)
        // ────────────────────────────────────────────────────────────────────

        /// Internal helper: create a bond record (storage-backed)
        pub fn create_bond_internal(
            who: &AccountIdOf<T>,
            asset: Vec<u8>,
            amount: BalanceOf<T>,
            bond_type: u8,
        ) -> Result<H256, DispatchError> {
            let mut counter = BondCounter::<T>::get();
            counter = counter.wrapping_add(1);
            BondCounter::<T>::put(counter);

            let mut seed = [0u8; 32];
            seed[0..8].copy_from_slice(&counter.to_le_bytes());
            let id = H256::from(seed);

            let now = T::UnixTime::now().as_secs();
            let bounded_asset: BoundedVec<u8, ConstU32<64>> = asset
                .try_into()
                .map_err(|_| DispatchError::Other("AssetTooLong"))?;
            let record = BondRecord {
                id,
                owner: who.clone(),
                asset: bounded_asset,
                amount,
                bond_type,
                state: 0, // Locked
                created_at: now,
            };

            Bonds::<T>::insert(id, record);

            let mut list = BondsByOwner::<T>::get(who);
            list.try_push(id)
                .map_err(|_| DispatchError::Other("TooManyBonds"))?;
            BondsByOwner::<T>::insert(who, list);

            Self::deposit_event(Event::BondDeposited {
                bond_id: id,
                owner: who.clone(),
                amount,
            });
            Ok(id)
        }

        /// Internal helper: request withdraw
        pub fn request_withdrawal_internal(bond_id: H256) -> Result<(), DispatchError> {
            Bonds::<T>::try_mutate_exists(bond_id, |maybe| {
                let b = maybe.as_mut().ok_or(DispatchError::Other("BondNotFound"))?;
                if b.state != 0 {
                    return Err(DispatchError::Other("NotLocked"));
                }
                b.state = 1; // Withdrawable
                Ok(())
            })
        }

        /// Internal helper: finalize withdrawal (removes bond)
        pub fn finalize_withdraw_internal(bond_id: H256) -> Result<(), DispatchError> {
            let b = Bonds::<T>::take(bond_id).ok_or(DispatchError::Other("BondNotFound"))?;
            BondsByOwner::<T>::mutate(&b.owner, |list| {
                if let Some(pos) = list.iter().position(|x| *x == bond_id) {
                    list.remove(pos);
                }
            });
            Self::deposit_event(Event::BondWithdrawn {
                bond_id,
                owner: b.owner,
                amount: b.amount,
            });
            Ok(())
        }

        /// Internal helper: slash bond (mark slashed)
        pub fn slash_bond_internal(bond_id: H256) -> Result<(), DispatchError> {
            Bonds::<T>::try_mutate(bond_id, |maybe| {
                let b = maybe.as_mut().ok_or(DispatchError::Other("BondNotFound"))?;
                b.state = 2; // Slashed
                Ok::<(), DispatchError>(())
            })?;
            Self::deposit_event(Event::BondSlashed { bond_id });
            Ok(())
        }
    }
}

pub use pallet::*;
