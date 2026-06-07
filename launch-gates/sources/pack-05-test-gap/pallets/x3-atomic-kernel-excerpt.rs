#![deny(unsafe_code)]
//! # X3 Atomic Kernel Pallet
//!
//! ## Overview
//!
//! This pallet is the **orchestration layer** above the existing `X3 Kernel`.
//! The X3 Kernel handles single-transaction atomic execution with EVM/SVM/X3
//! tri-VM integration. This pallet adds:
//!
//! - **Bundle lifecycle** — submit, execute, finalize, or rollback a ordered
//!   set of cross-VM trade legs in one atomic context.
//! - **PoAE proof generation** — Proof of Atomic Execution, anchored to a
//!   finalized block's justification (GRANDPA or Flash Finality certificate).
//!   Required for cross-chain settlement on external EVM/SVM chains.
//! - **Executor deposits** — bundles require a bond from the submitter;
//!   misbehavior (undeclared writes, access set violations) burns part of it.
//! - **Declared access sets** — each bundle leg must declare reads/writes;
//!   the kernel enforces these, enabling deterministic parallel execution.
//!
//! ## Bundle Lifecycle
//!
//! ```text
//! submit_atomic_bundle(legs, deadline)
//!   → BundleStatus::Pending → event BundleSubmitted
//!
//! [Off-chain executor or block proposer executes legs via X3 Kernel]
//!
//! finalize_atomic_bundle(bundle_id, receipts, finality_cert)
//!   → BundleStatus::Finalized → PoAE proof stored → event BundleFinalized
//!
//! [External chain verifier calls verify_poae(bundle_id, proof)]
//!
//! rollback_atomic_bundle(bundle_id, reason)
//!   → BundleStatus::RolledBack → bond slashed → event BundleRolledBack
//! ```
//!
//! ## PoAE Proof Format
//!
//! ```text
//! PoaeProof {
//!   bundle_id:       H256         — unique bundle identifier
//!   receipt_root:    H256         — Merkle root of execution receipts
//!   finalized_block: BlockNumber  — block number where bundle was finalized
//!   finality_cert:   H256         — GRANDPA justification hash or Flash cert hash
//! }
//! ```
//!
//! A verifier on an external chain checks:
//! 1. `receipt_root` commits to the claimed execution outcomes.
//! 2. `finality_cert` is a valid GRANDPA justification for `finalized_block`.
//! 3. The bundle inclusion proof links `bundle_id` to that block.
//!
//! ## Audit Alignment
//!
//! Per the deep-research audit:
//! > "If you implement one end-to-end pipeline to production quality, make it
//! > the swap program. It's the shortest line from 'cool docs' to 'people
//! > trust money on it.'"
//!
//! This pallet, combined with `atomic-trade-engine` and `x3-flash-finality`,
//! is that pipeline.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

// Re-export proof type for use in RPC and external verifiers
pub mod proof;

// Re-export weights for runtime integration
pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
    use super::proof::{BundleLeg, PoaeProof};
    use crate::weights::WeightInfo;
    use frame_support::{
        dispatch::DispatchResult,
        pallet_prelude::*,
        traits::{Currency, ReservableCurrency},
    };
    use frame_system::offchain::SubmitTransaction;
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_io::hashing::sha2_256;
    use sp_runtime::offchain::StorageKind;
    use sp_runtime::traits::{SaturatedConversion, Saturating};
    use sp_runtime::transaction_validity::{
        InvalidTransaction, TransactionPriority, TransactionSource, TransactionValidity,
        ValidTransaction,
    };

    // ── Config ────────────────────────────────────────────────────────────────

    /// Convenience alias for the pallet's currency balance type.
    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::config]
    pub trait Config:
        frame_system::Config + frame_system::offchain::SendTransactionTypes<Call<Self>>
    {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The currency used for executor bonds.
        /// Must implement `ReservableCurrency` so bonds are properly locked at submission
        /// and released (or slashed) at finalization/rollback.
        type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

        /// Weight functions for extrinsic calls.
        /// Use `weights::SubstrateWeight<T>` for production, `()` for tests.
        type WeightInfo: crate::weights::WeightInfo;

        /// Minimum bond required to submit a bundle.
        /// Denominated in the smallest currency unit.
        #[pallet::constant]
        type MinBond: Get<u128>;

        /// Maximum legs per bundle (limits state explosion).
        #[pallet::constant]
        type MaxLegsPerBundle: Get<u32>;

        /// Maximum time (in blocks) a Pending bundle may wait before auto-rollback.
        #[pallet::constant]
        type BundleDeadlineBlocks: Get<BlockNumberFor<Self>>;
    }

    // ── Storage ───────────────────────────────────────────────────────────────

    /// Active bundles by bundle_id.
    #[pallet::storage]
    #[pallet::getter(fn bundles)]
    pub type Bundles<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        H256, // bundle_id
        BundleRecord<T>,
        OptionQuery,
    >;

    /// PoAE proofs by bundle_id — stored on-chain for external verifiers.
    #[pallet::storage]
    #[pallet::getter(fn poae_proofs)]
    pub type PoaeProofs<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        H256, // bundle_id
        PoaeProof,
        OptionQuery,
    >;

    /// Deadline index: block number → bundle IDs pending deadline
    /// Enables O(1) lookup for bundle expiration instead of O(n) scan
    #[pallet::storage]
    #[pallet::getter(fn deadline_index)]
    pub type DeadlineIndex<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>, // deadline block
        BoundedVec<H256, T::MaxLegsPerBundle>,
        ValueQuery,
    >;

    /// On-chain anchors for Flash-Finality certificates, keyed by block number
    /// (as LE-encoded u64).  The off-chain worker writes an entry here (via an
    /// unsigned extrinsic `record_flash_finality_anchor`) whenever it observes
    /// a valid certificate in off-chain local storage.  `do_finalize_bundle`
    /// checks this map when the caller supplies a non-zero `finality_cert`.
    #[pallet::storage]
    pub type FinalityCertAnchors<T: Config> = StorageMap<_, Twox64Concat, u64, H256, OptionQuery>;

    // ── Types ─────────────────────────────────────────────────────────────────

    /// Bundle execution status.
    #[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, MaxEncodedLen, TypeInfo)]
    pub enum BundleStatus {
        /// Submitted, waiting for executor assignment.
        Pending,
        /// Currently being executed by an assigned executor.
        Executing,
        /// All legs executed successfully; PoAE proof attached.
        Finalized,
        /// Execution failed or deadline expired; bond partially slashed.
        RolledBack,
    }

    /// On-chain record for a submitted atomic bundle.
    #[derive(Debug, Clone, Encode, Decode, MaxEncodedLen, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct BundleRecord<T: Config> {
        /// Submitter / bond holder.
        pub submitter: T::AccountId,
        /// Hash of the encoded legs for integrity checking.
        pub legs_hash: H256,
        /// Number of legs.
        pub leg_count: u32,
        /// Current lifecycle status.
        pub status: BundleStatus,
        /// Block number when this bundle must be finalized or auto-rolled back.
        pub deadline_block: BlockNumberFor<T>,
        /// Block number when the bundle was submitted.
        pub submitted_at: BlockNumberFor<T>,
        /// The account that claimed this bundle via `assign_bundle_executor`.
        /// `None` while the bundle is in `Pending` status.
        /// Unsigned finalization is only accepted when this is `Some`.
        pub executor: Option<T::AccountId>,
    }

    // ── Pallet ────────────────────────────────────────────────────────────────

    /// Storage layout version. Increment whenever storage types or keys change
    /// and provide an `on_runtime_upgrade` migration. Missing this declaration
    /// causes silent data corruption on upgrade — P0 per DEEP_AUDIT_PROTOCOL §4.
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    // ── Events ────────────────────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new atomic bundle was submitted.
        BundleSubmitted {
            bundle_id: H256,
            submitter: T::AccountId,
            leg_count: u32,
        },
        /// A bundle was successfully finalized with a PoAE proof.
        BundleFinalized {
            bundle_id: H256,
            receipt_root: H256,
            finality_cert: H256,
            finalized_block: BlockNumberFor<T>,
        },
        /// A bundle was rolled back (execution failed or deadline exceeded).
        BundleRolledBack {
            bundle_id: H256,
            reason: BundleRollbackReason,
        },
        /// A bundle has been assigned to an executor.
        BundleAssigned {
            bundle_id: H256,
            executor: T::AccountId,
        },
    }

    /// Reason a bundle was rolled back.
    #[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, MaxEncodedLen, TypeInfo)]
    pub enum BundleRollbackReason {
        /// One or more legs failed execution.
        ExecutionFailed,
        /// A leg violated its declared access set (undeclared write detected).
        AccessSetViolation,
        /// Bundle deadline exceeded without finalization.
        DeadlineExceeded,
        /// Manually triggered by the submitter.
        SubmitterCancelled,
    }

    // ── Errors ────────────────────────────────────────────────────────────────

    #[pallet::error]
    pub enum Error<T> {
        /// Bundle ID already exists.
        BundleAlreadyExists,
        /// Bundle not found.
        BundleNotFound,
        /// Bundle is not in the expected state for this operation.
        InvalidBundleState,
        /// Too many legs in this bundle.
        TooManyLegs,
        /// Bundle deadline has already passed.
        DeadlineExpired,
        /// Insufficient bond from submitter.
        InsufficientBond,
        /// PoAE proof already exists for this bundle.
        ProofAlreadyExists,
        /// Caller is not the bundle submitter.
        NotBundleSubmitter,
        /// Receipt root is malformed or empty.
        InvalidReceiptRoot,
        /// Finality certificate does not match the on-chain anchor for the
        /// finalized block.  Submitted cert differs from the one written by
        /// the Flash Finality voter.
        InvalidFinalityCert,
    }

    // ── Hooks ─────────────────────────────────────────────────────────────────

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// Off-chain worker: auto-submits unsigned `submit_finalization_result` for
        /// any `Executing` bundle whose GPU-committed finalization data is waiting
        /// in off-chain local storage.
        ///
        /// The `AtomicSwapOrchestrator` (running as a node-side service) writes the
        /// finalization record via `sp_io::offchain::local_storage_set` using the key
        /// convention:  `b"x3fin:" + bundle_id_bytes (32)`.
        /// The value is 40 bytes: `receipt_root (32) || committed_at_ns (8, LE)`.
        ///
        /// The Flash Finality voter in `service.rs` writes the cert hash under
        /// key `b"x3ff:" + block_number_le (8 bytes)` = 13-byte key, 32-byte value.
        /// The OCW reads this cert and attaches it to the PoAE proof so external
        /// verifiers can validate finality without trusting a zero placeholder.
        fn offchain_worker(now: BlockNumberFor<T>) {
            log::debug!(
                target: "x3-atomic-kernel",
                "[OCW] block {:?}: scanning Executing bundles",
                now
            );

            // Look up any Flash Finality certificate for the current block.
            // Key: "x3ff:" (5 bytes) + block_number as LE u64 (8 bytes) = 13 bytes
            // Value: cert_hash (32 bytes) written by run_flash_finality_voter in service.rs
            let block_num_u64: u64 = now.try_into().unwrap_or(0u64);
            let finality_cert: H256 = {
                let mut cert_key = b"x3ff:".to_vec();
                cert_key.extend_from_slice(&block_num_u64.to_le_bytes());
                match sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &cert_key) {
                    Some(v) if v.len() == 32 => H256::from_slice(&v),
                    _ => H256::zero(), // Flash Finality not yet running — pallet accepts zero
                }
            };

            if finality_cert != H256::zero() {
                log::info!(
                    target: "x3-atomic-kernel",
                    "[OCW] block {:?}: Flash Finality cert found: {:?}",
                    now, finality_cert
                );
                // Anchor the cert on-chain so do_finalize_bundle can verify
                // non-zero certs submitted via the signed extrinsic path.
                let anchor_call = Call::record_flash_finality_anchor {
                    block_num: block_num_u64,
                    cert: finality_cert,
                };
                let _ = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(
                    anchor_call.into(),
                );
            }

            for (bundle_id, record) in Bundles::<T>::iter() {
                if record.status != BundleStatus::Executing {
                    continue;
                }

                // Build the storage key used by the orchestrator to signal finalization.
                let mut key = b"x3fin:".to_vec();
                key.extend_from_slice(bundle_id.as_bytes());

                let data = match sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &key) {
                    Some(v) if v.len() >= 40 => v,
                    _ => continue,
                };

                // Parse receipt_root and committed_at_ns from the 40-byte payload.
                let receipt_root = H256::from_slice(&data[..32]);
                let committed_at_ns =
                    u64::from_le_bytes(data[32..40].try_into().unwrap_or([0u8; 8]));

                if receipt_root == H256::zero() {
                    log::warn!(
                        target: "x3-atomic-kernel",
                        "[OCW] bundle {:?}: skipping zero receipt_root in local storage",
                        bundle_id
                    );
                    continue;
                }

                let call = Call::submit_finalization_result {
                    bundle_id,
                    receipt_root,
                    finality_cert,
                    committed_at_ns,
                };

                match SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()) {
                    Ok(()) => {
                        // Clear the entry so we don't resubmit next block.
                        sp_io::offchain::local_storage_clear(StorageKind::PERSISTENT, &key);
                        log::info!(
                            target: "x3-atomic-kernel",
                            "[OCW] submitted finalization for bundle {:?} (receipt_root={:?}, finality_cert={:?})",
                            bundle_id, receipt_root, finality_cert
                        );
                    }
                    Err(()) => {
                        log::error!(
                            target: "x3-atomic-kernel",
                            "[OCW] failed to submit unsigned tx for bundle {:?}",
                            bundle_id
                        );
                    }
                }
            }
        }

        /// On each block, expire bundles that have passed their deadline.
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            // Use DeadlineIndex for O(1) lookup of bundles expiring at this block
            // instead of iterating all pending bundles (O(n))

            // Get all bundle IDs that have deadlines at or before current block
            let expired_bundle_ids = DeadlineIndex::<T>::get(now);

            let mut processed_count: u32 = 0;

            // Process each expired bundle
            for bundle_id in expired_bundle_ids.iter() {
                if let Some(record) = Bundles::<T>::get(bundle_id) {
                    if record.status == BundleStatus::Pending
                        || record.status == BundleStatus::Executing
                    {
                        // Bundle has expired - trigger rollback
                        let mut updated_record = record.clone();
                        updated_record.status = BundleStatus::RolledBack;
                        Bundles::<T>::insert(bundle_id, updated_record);

                        // Slash for deadline exceeded (5%)
                        let bond = T::MinBond::get();
                        let slash_amount = bond.saturating_div(20);
                        if slash_amount > 0 {
                            let slash: BalanceOf<T> = slash_amount.saturated_into();
                            let _ = T::Currency::slash(&record.submitter, slash);
                        }

                        Self::deposit_event(Event::BundleRolledBack {
                            bundle_id: *bundle_id,
                            reason: BundleRollbackReason::DeadlineExceeded,
                        });

                        log::warn!(
                            target: "x3-atomic-kernel",
                            "Bundle {:?} expired at block {:?}, slashed {}",
                            bundle_id, now, slash_amount
                        );

                        processed_count += 1;
                    }
                }
            }

            // Clean up the deadline index for this block
            if !expired_bundle_ids.is_empty() {
                DeadlineIndex::<T>::remove(now);
            }

            // Return weight based on processed bundles
            Weight::from_parts((processed_count as u64) * 10_000, 0)
        }
    }

    // ── Dispatchable Calls ────────────────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Submit an atomic bundle of cross-VM trade legs.
        ///
        /// The submitter must have sufficient balance for the bond.
        /// The bundle is assigned a deterministic `bundle_id` derived from
        /// the submitter, block number, and legs hash.
        ///
        /// # Bond Lifecycle
        /// - **Reserved** here at submission (submitter cannot spend bonded funds).
        /// - **Unreserved** on `SubmitterCancelled` rollback (voluntary cancel, no slash).
        /// - **Slashed** (via `Currency::slash` on reserved funds) on execution failure
        ///   or deadline expiry.
        ///
        /// # Security
        /// - Max legs enforced by `MaxLegsPerBundle`.
        /// - Deadline enforced by `BundleDeadlineBlocks`.
        /// - Bond reserved on submission, slashed on rollback.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::submit_atomic_bundle(legs.len() as u32))]
        pub fn submit_atomic_bundle(
            origin: OriginFor<T>,
            legs: BoundedVec<BundleLeg, T::MaxLegsPerBundle>,
            deadline_blocks: BlockNumberFor<T>,
        ) -> DispatchResult {
            let submitter = ensure_signed(origin)?;

            ensure!(!legs.is_empty(), Error::<T>::TooManyLegs);
            ensure!(
                legs.len() as u32 <= T::MaxLegsPerBundle::get(),
                Error::<T>::TooManyLegs
            );

            let now = <frame_system::Pallet<T>>::block_number();
            let deadline = now.saturating_add(deadline_blocks.min(T::BundleDeadlineBlocks::get()));

