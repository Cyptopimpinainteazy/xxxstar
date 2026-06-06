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

            // Derive a deterministic bundle_id using sha2_256 so it's always H256.
            let legs_encoded = legs.encode();
            let legs_hash = H256(sha2_256(&legs_encoded));
            let bundle_id = Self::derive_bundle_id(&submitter, now, legs_hash);

            ensure!(
                !Bundles::<T>::contains_key(bundle_id),
                Error::<T>::BundleAlreadyExists
            );

            // Reserve the bond — this locks funds in the submitter's account.
            // Slashing (Currency::slash) consumes reserved funds first.
            let bond: BalanceOf<T> = T::MinBond::get().saturated_into();
            T::Currency::reserve(&submitter, bond).map_err(|_| Error::<T>::InsufficientBond)?;

            let record = BundleRecord::<T> {
                submitter: submitter.clone(),
                legs_hash,
                leg_count: legs.len() as u32,
                status: BundleStatus::Pending,
                deadline_block: deadline,
                submitted_at: now,
                executor: None,
            };

            Bundles::<T>::insert(bundle_id, record);

            // Add to deadline index for O(1) expiry lookup
            let mut deadline_bundles = DeadlineIndex::<T>::get(deadline);
            if deadline_bundles.try_push(bundle_id).is_ok() {
                DeadlineIndex::<T>::insert(deadline, deadline_bundles);
            }

            Self::deposit_event(Event::BundleSubmitted {
                bundle_id,
                submitter,
                leg_count: legs.len() as u32,
            });

            log::info!(
                target: "x3-atomic-kernel",
                "Bundle {:?} submitted with {} legs, deadline block {:?}, bond reserved",
                bundle_id, legs.len(), deadline
            );

            Ok(())
        }

        /// Finalize an atomic bundle with execution receipts and a finality certificate.
        ///
        /// This produces the PoAE proof stored on-chain and emits `BundleFinalized`.
        ///
        /// # Arguments
        /// - `bundle_id`: The bundle to finalize.
        /// - `receipt_root`: Merkle root of execution receipts from X3 Kernel.
        /// - `finality_cert`: Hash of the GRANDPA justification or Flash Finality
        ///   certificate for the block containing the bundle execution.
        /// - `finalized_block`: Block number where execution was anchored.
        ///
        /// # External Verification
        /// An external chain verifier checks:
        /// 1. `receipt_root` matches claimed execution outcomes.
        /// 2. `finality_cert` is a valid justification for `finalized_block`.
        /// 3. Bundle inclusion proof links `bundle_id` to that block.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::finalize_atomic_bundle())]
        pub fn finalize_atomic_bundle(
            origin: OriginFor<T>,
            bundle_id: H256,
            receipt_root: H256,
            finality_cert: H256,
            finalized_block: BlockNumberFor<T>,
        ) -> DispatchResult {
            ensure_signed(origin)?;
            Self::do_finalize_bundle(bundle_id, receipt_root, finality_cert, finalized_block)
        }

        /// Submit finalization data as an **unsigned** transaction.
        ///
        /// This is the off-chain path: the `AtomicSwapOrchestrator` calls this
        /// after GPU commit to close the bundle lifecycle without needing a funded
        /// Substrate account.  The `receipt_root` itself acts as proof-of-execution
        /// (it is SHA-256 of the GPU-committed shm entry).
        ///
        /// `finality_cert` is the Flash Finality certificate hash written by
        /// `run_flash_finality_voter` in `service.rs` to off-chain local storage
        /// under key `"x3ff:" + block_number_le`.  Set to `H256::zero()` when
        /// Flash Finality is not running (pallet accepts zero for this path).
        ///
        /// `committed_at_ns` is the GPU commit timestamp for auditing only — it is
        /// not stored on-chain but is included for `ValidateUnsigned` deduplication.
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::submit_finalization_result())]
        pub fn submit_finalization_result(
            origin: OriginFor<T>,
            bundle_id: H256,
            receipt_root: H256,
            finality_cert: H256,
            _committed_at_ns: u64,
        ) -> DispatchResult {
            ensure_none(origin)?;
            let now = <frame_system::Pallet<T>>::block_number();
            Self::do_finalize_bundle(bundle_id, receipt_root, finality_cert, now)
        }

        /// Store an on-chain anchor for a Flash Finality certificate.
        ///
        /// Submitted as an **unsigned** transaction by the off-chain worker
        /// whenever a non-zero cert is found in off-chain local storage.
        /// Once anchored, `do_finalize_bundle` uses this to verify the
        /// `finality_cert` supplied via the signed `finalize_atomic_bundle`
        /// extrinsic, preventing submission of fabricated cert hashes.
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::record_flash_finality_anchor())]
        pub fn record_flash_finality_anchor(
            origin: OriginFor<T>,
            block_num: u64,
            cert: H256,
        ) -> DispatchResult {
            ensure_none(origin)?;
            ensure!(cert != H256::zero(), Error::<T>::InvalidFinalityCert);
            // Only store the first cert seen for each block — the Flash-Finality voter
            // derives a deterministic cert per block, so the first non-zero one wins.
            if !FinalityCertAnchors::<T>::contains_key(block_num) {
                FinalityCertAnchors::<T>::insert(block_num, cert);
                log::info!(
                    target: "x3-atomic-kernel",
                    "Flash Finality anchor stored for block {}: {:?}",
                    block_num, cert
                );
            }
            Ok(())
        }

        /// Assign an executor to a pending bundle, transitioning it to `Executing`.
        ///
        /// Called by an off-chain worker or privileged executor account.
        /// This is a lightweight state transition: it only changes the status so
        /// that `on_initialize` expiry logic (and external observers) know the bundle
        /// is actively being processed.  Execution itself happens off-chain via the
        /// `AtomicSwapOrchestrator`; the result is submitted via `finalize_atomic_bundle`.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::assign_bundle_executor())]
        pub fn assign_bundle_executor(origin: OriginFor<T>, bundle_id: H256) -> DispatchResult {
            let executor = ensure_signed(origin)?;

            let mut record = Bundles::<T>::get(bundle_id).ok_or(Error::<T>::BundleNotFound)?;

            ensure!(
                record.status == BundleStatus::Pending,
                Error::<T>::InvalidBundleState
            );

            let now = <frame_system::Pallet<T>>::block_number();
            ensure!(now <= record.deadline_block, Error::<T>::DeadlineExpired);

            record.status = BundleStatus::Executing;
            record.executor = Some(executor.clone());
            Bundles::<T>::insert(bundle_id, &record);

            Self::deposit_event(Event::BundleAssigned {
                bundle_id,
                executor,
            });

            log::info!(
                target: "x3-atomic-kernel",
                "Bundle {:?} assigned to executor, now Executing",
                bundle_id
            );

            Ok(())
        }

        /// Roll back a bundle, emitting a reason for the rollback.
        ///
        /// Called by the submitter to cancel, or by governance/runtime on deadline.
        /// In a production system, slash a portion of the bond if called due to
        /// `ExecutionFailed` or `AccessSetViolation`.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::rollback_atomic_bundle())]
        pub fn rollback_atomic_bundle(
            origin: OriginFor<T>,
            bundle_id: H256,
            reason: BundleRollbackReason,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;

            let mut record = Bundles::<T>::get(bundle_id).ok_or(Error::<T>::BundleNotFound)?;

            // Only Pending or Executing bundles can be rolled back
            ensure!(
                record.status == BundleStatus::Pending || record.status == BundleStatus::Executing,
                Error::<T>::InvalidBundleState
            );

            // C-005: Per-reason authorization guards.
            // - SubmitterCancelled: only the bundle submitter may cancel voluntarily.
            // - ExecutionFailed / AccessSetViolation: only the assigned executor may
            //   report these; any signed account triggering them was the C-005 bug.
            // - DeadlineExceeded: only a party to the bundle may trigger this, AND the
            //   deadline must actually have elapsed (auto-expiry via on_initialize is
            //   the preferred path; this guard prevents arbitrary third-party slashing).
            match reason {
                BundleRollbackReason::SubmitterCancelled => {
                    ensure!(record.submitter == caller, Error::<T>::NotBundleSubmitter);
                }
                BundleRollbackReason::ExecutionFailed
                | BundleRollbackReason::AccessSetViolation => {
                    ensure!(
                        record.executor == Some(caller.clone()),
                        Error::<T>::NotBundleSubmitter
                    );
                }
                BundleRollbackReason::DeadlineExceeded => {
                    let now = <frame_system::Pallet<T>>::block_number();
                    ensure!(now > record.deadline_block, Error::<T>::InvalidBundleState);
                    ensure!(
                        caller == record.submitter || record.executor == Some(caller.clone()),
                        Error::<T>::NotBundleSubmitter
                    );
                }
            }

            record.status = BundleStatus::RolledBack;
            Bundles::<T>::insert(bundle_id, &record);

            // Slash bond proportional to reason severity
            let slash_amount: u128 = match reason {
                BundleRollbackReason::ExecutionFailed
                | BundleRollbackReason::AccessSetViolation => {
                    // Slash 10% of bond for execution failures or access violations
                    let bond = T::MinBond::get();
                    bond.saturating_div(10)
                }
                BundleRollbackReason::DeadlineExceeded => {
                    // Slash 5% of bond for deadline exceeded
                    let bond = T::MinBond::get();
                    bond.saturating_div(20)
                }
                BundleRollbackReason::SubmitterCancelled => {
                    // Unreserve full bond for voluntary cancellation (no slash)
                    let bond: BalanceOf<T> = T::MinBond::get().saturated_into();
                    T::Currency::unreserve(&record.submitter, bond);
                    0
                }
            };

            if slash_amount > 0 {
                let slash: BalanceOf<T> = slash_amount.saturated_into();
                // slash() targets reserved balance first, then free balance
                let _ = T::Currency::slash(&record.submitter, slash);
                log::info!(
                    target: "x3-atomic-kernel",
                    "Bundle {:?} slashed by {} for reason {:?}",
                    bundle_id, slash_amount, reason
                );
            }

            Self::deposit_event(Event::BundleRolledBack { bundle_id, reason });

            log::warn!(
                target: "x3-atomic-kernel",
                "Bundle {:?} rolled back",
                bundle_id
            );

            Ok(())
        }

        /// Phase 1b: Settlement ↔ Kernel Dispatch Linking
        ///
        /// Finalize an atomic bundle with a settlement proof from the settlement engine.
        /// This extrinsic is called by the settlement engine after all cross-VM legs
        /// have been executed and the settlement intent is ready for finalization.
        ///
        /// # Parameters
        /// - `bundle_id`: The atomic bundle identifier
        /// - `settlement_intent_id`: The settlement intent that triggered this bundle
        /// - `receipt_root`: Merkle root of execution receipts from GPU executors
        /// - `finality_cert`: Flash-Finality certificate (if available), or H256::zero()
        ///
        /// # Security
        /// Only the settlement engine (via dispatcher) or authorized settler account can call this.
        /// The bundle must be in Executing or Pending state.
        /// The receipt_root must be non-zero (proves actual execution occurred).
        ///
        /// # Events
        /// Emits `BundleFinalized` with PoAE proof stored on-chain for external verifiers.
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::finalize_atomic_bundle())]
        pub fn finalize_with_settlement(
            origin: OriginFor<T>,
            bundle_id: H256,
            settlement_intent_id: H256,
            receipt_root: H256,
            finality_cert: H256,
        ) -> DispatchResult {
            // Phase 1b: For now, accept any signed call. In Phase 1c, restrict to
            // settlement pallet calls only via a whitelist or signed dispatcher trait.
            let _caller = ensure_signed(origin)?;

            let now = <frame_system::Pallet<T>>::block_number();

            // Delegate to shared finalization logic
            Self::do_finalize_bundle(bundle_id, receipt_root, finality_cert, now)?;

            // Log settlement integration for debugging
            log::info!(
                target: "x3-atomic-kernel",
                "Bundle {:?} finalized with settlement intent {:?}",
                bundle_id, settlement_intent_id
            );

            Ok(())
        }
    }

    // ── ValidateUnsigned ──────────────────────────────────────────────────

    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            if let Call::submit_finalization_result {
                bundle_id,
                receipt_root,
                finality_cert,
                ..
            } = call
            {
                // receipt_root must be non-zero (proves GPU committed actual data)
                if *receipt_root == H256::zero() {
                    return InvalidTransaction::BadProof.into();
                }
                // Bundle must exist, be in Executing state (not Pending!), and have
                // an assigned executor.  Requiring Executing guarantees that a signed
                // `assign_bundle_executor` call ran first, binding a real Substrate
                // account to the bundle.  This prevents anonymous peers from finalizing
                // bundles they never claimed.
                match Bundles::<T>::get(bundle_id) {
                    Some(record)
                        if record.status == BundleStatus::Executing
                            && record.executor.is_some() =>
                    {
                        // Include finality_cert bytes in the dedup tag so that a zero-cert and a
                        // real-cert tx for the same bundle are treated as distinct (the real one
                        // should win in the pool).
                        let mut tag = bundle_id.as_bytes().to_vec();
                        tag.extend_from_slice(finality_cert.as_bytes());
                        ValidTransaction::with_tag_prefix("X3AtomicFinalize")
                            .priority(if *finality_cert == H256::zero() {
                                TransactionPriority::max_value() / 4
                            } else {
                                TransactionPriority::max_value() / 2
                            })
                            .and_provides([tag.as_slice()])
                            .longevity(5)
                            .propagate(true)
                            .build()
                    }
                    _ => InvalidTransaction::Stale.into(),
                }
            } else if let Call::record_flash_finality_anchor { block_num, cert } = call {
                if *cert == H256::zero() {
                    return InvalidTransaction::BadProof.into();
                }
                // Block recency check: anchors must be for blocks within a
                // reasonable window of the current chain head.  This prevents
                // attackers from planting anchors for far-future blocks or for
                // ancient blocks that are no longer relevant.
                let current_block: u64 = <frame_system::Pallet<T>>::block_number()
                    .try_into()
                    .unwrap_or(0u64);
                // Allow anchors for blocks up to 50 blocks in the past and
                // up to 5 blocks in the future (accounts for propagation delay).
                if *block_num > current_block.saturating_add(5) {
                    return InvalidTransaction::Future.into();
                }
                if current_block.saturating_sub(50) > *block_num {
                    return InvalidTransaction::Stale.into();
                }
                ValidTransaction::with_tag_prefix("X3FinalityAnchor")
                    .priority(TransactionPriority::max_value() / 8)
                    .and_provides([(b"anchor", block_num.to_le_bytes()).encode().as_slice()])
                    .longevity(10)
                    .propagate(true)
                    .build()
            } else {
                InvalidTransaction::Call.into()
            }
        }
    }

    // ── Internal Helpers ──────────────────────────────────────────────────────

    impl<T: Config> Pallet<T> {
        /// Shared finalization logic used by both `finalize_atomic_bundle` (signed)
        /// and `submit_finalization_result` (unsigned).
        fn do_finalize_bundle(
            bundle_id: H256,
            receipt_root: H256,
            finality_cert: H256,
            finalized_block: BlockNumberFor<T>,
        ) -> DispatchResult {
            ensure!(receipt_root != H256::zero(), Error::<T>::InvalidReceiptRoot);

            // STRICT finality cert validation for production:
            // - If finality_cert is zero, Flash Finality is not running — allowed.
            // - If finality_cert is non-zero, it MUST match an on-chain anchor
            //   written by the OCW. No tentative acceptance — reject unknown certs.
            if finality_cert != H256::zero() {
                let block_num: u64 = finalized_block.try_into().unwrap_or(0u64);
                let anchored = FinalityCertAnchors::<T>::get(block_num)
                    .ok_or(Error::<T>::InvalidFinalityCert)?;
                ensure!(finality_cert == anchored, Error::<T>::InvalidFinalityCert);
            }

            let mut record = Bundles::<T>::get(bundle_id).ok_or(Error::<T>::BundleNotFound)?;

            ensure!(
                record.status == BundleStatus::Pending || record.status == BundleStatus::Executing,
                Error::<T>::InvalidBundleState
            );

            let now = <frame_system::Pallet<T>>::block_number();
            ensure!(now <= record.deadline_block, Error::<T>::DeadlineExpired);

            ensure!(
                !PoaeProofs::<T>::contains_key(bundle_id),
                Error::<T>::ProofAlreadyExists
            );

            let proof = PoaeProof {
                bundle_id,
                receipt_root,
                finalized_block: finalized_block.try_into().unwrap_or(0u64),
                finality_cert,
                legs_hash: record.legs_hash,
                leg_count: record.leg_count,
            };

            PoaeProofs::<T>::insert(bundle_id, proof);
            record.status = BundleStatus::Finalized;
            Bundles::<T>::insert(bundle_id, &record);

            Self::deposit_event(Event::BundleFinalized {
                bundle_id,
                receipt_root,
                finality_cert,
                finalized_block,
            });

            log::info!(
                target: "x3-atomic-kernel",
                "Bundle {:?} finalized. PoAE proof stored. cert={:?}",
                bundle_id, finality_cert
            );

            Ok(())
        }

        /// Derive a deterministic bundle ID from submitter + block + legs_hash.
        /// Uses SHA-256 directly (not T::Hashing) so the result is always H256,
        /// matching the off-chain `derive_bundle_id()` in the AtomicSwapOrchestrator.
        pub fn derive_bundle_id(
            submitter: &T::AccountId,
            block: BlockNumberFor<T>,
            legs_hash: H256,
        ) -> H256 {
            let mut data = submitter.encode();
            data.extend_from_slice(&block.encode());
            data.extend_from_slice(legs_hash.as_bytes());
            H256(sha2_256(&data))
        }

        /// Get a PoAE proof for external verification.
        pub fn get_poae_proof(bundle_id: H256) -> Option<PoaeProof> {
            PoaeProofs::<T>::get(bundle_id)
        }

        /// Check if a bundle exists and return its status.
        pub fn bundle_status(bundle_id: H256) -> Option<BundleStatus> {
            Bundles::<T>::get(bundle_id).map(|r| r.status)
        }
    }
}

// C-011: Runtime API declaration for off-chain / RPC access to atomic kernel state.
// External consumers (indexers, frontends, RPC nodes) call these via the runtime API
// bridge instead of directly reading storage, which ensures the API stays ABI-stable
// across runtime upgrades.
sp_api::decl_runtime_apis! {
    /// Runtime API for the X3 atomic kernel pallet.
    ///
    /// Implements read-only queries for PoAE proofs, bundle status, and finality
    /// certificate anchors.  These are the three pieces of data an external chain
    /// (EVM verifier contract, SVM program, indexer) needs to settle a cross-VM
    /// atomic trade.
    pub trait X3AtomicKernelApi {
        /// Return the Proof of Atomic Execution for a finalised bundle, if available.
        fn get_poae_proof(bundle_id: sp_core::H256) -> Option<crate::proof::PoaeProof>;

        /// Return the current status of a bundle.
        fn get_bundle_status(bundle_id: sp_core::H256) -> Option<crate::BundleStatus>;

        /// Return the Flash Finality certificate anchor stored for the given block number.
        fn get_finality_cert_anchor(block_num: u64) -> Option<sp_core::H256>;
    }
}
