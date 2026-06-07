// runtime/src/fraud_proofs/pallet.rs
//
// Inline FRAME pallet for scheduler fraud proofs.
//
// This pallet is defined inside the runtime crate (rather than as a separate
// workspace member) to avoid a circular dependency between the runtime and a
// hypothetical `pallet-fraud-proofs` crate that would need to import runtime
// types for verification.
//
// ## Responsibilities
// 1. Accept `submit_fraud_proof` extrinsics from any signed origin.
// 2. Deduplicate proofs by their `proof_id` (replay protection).
// 3. Verify scheduler commitment divergence via `verify_scheduler_mismatch_v1`.
// 4. On valid divergence: emit `FraudProofAccepted` and freeze the scheduler.
// 5. Expose `governance_unfreeze` to allow a governance-approved unfreezing.
//
// ## Storage
// - `ProofsSeen`  : StorageMap<H256, ()>                 — replay dedup
// - `DisputedMeta`: StorageMap<H256, DisputedBlockMeta>  — disputed block info
// - `ConsensusFreeze`: StorageValue<FreezeState>         — current freeze flag
//
// ## Security invariants referenced
// - FRAUD-PROOF-001: submitter must be signed (no unsigned fraud proofs)
// - FRAUD-PROOF-002: duplicate proof id is rejected
// - FRAUD-PROOF-003: verifier must confirm divergence before state changes
// - FREEZE-001: freeze does not stop block production
// - FREEZE-002: unfreeze requires governance origin

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ReservableCurrency},
    };
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_runtime::traits::UniqueSaturatedInto;
    use sp_std::vec::Vec;

    use crate::fraud_proofs::{
        freeze::{FreezeReason, FreezeState},
        types::DisputedBlockMeta,
        verifier::{compute_proof_id, verify_scheduler_mismatch_v1, VerifyError},
        FraudProofV1,
    };

    // -----------------------------------------------------------------------
    // Balance helpers
    // -----------------------------------------------------------------------

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    // -----------------------------------------------------------------------
    // Config
    // -----------------------------------------------------------------------

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency used for reporter reward and proposer slashing.
        type Currency: ReservableCurrency<Self::AccountId>;

        /// Maximum batched transactions in a single witness (guards DoS).
        #[pallet::constant]
        type MaxTxCount: Get<u32>;

        /// Number of blocks within which a fraud proof is valid after the disputed block.
        #[pallet::constant]
        type DisputeWindowBlocks: Get<u32>;

        /// Reward paid to the reporter on successful fraud proof acceptance.
        #[pallet::constant]
        type ReporterRewardAmount: Get<BalanceOf<Self>>;

        /// Origin that can call `governance_unfreeze`.
        type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;
    }

    // -----------------------------------------------------------------------
    // Pallet declaration
    // -----------------------------------------------------------------------

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    // -----------------------------------------------------------------------
    // Storage
    // -----------------------------------------------------------------------

    /// Set of proof IDs already processed — prevents replay.
    ///
    /// Key: `proof_id` (blake2_256 of serialized proof + block hash).
    #[pallet::storage]
    #[pallet::getter(fn proofs_seen)]
    pub type ProofsSeen<T: Config> = StorageMap<_, Blake2_128Concat, H256, ()>;

    /// Metadata of actively-disputed blocks.
    ///
    /// Inserted when a valid `DisputedBlockMeta` is provided alongside a
    /// fraud proof.
    #[pallet::storage]
    #[pallet::getter(fn disputed_meta)]
    pub type DisputedMeta<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, DisputedBlockMeta<T::AccountId>>;

    /// Current scheduler/AI freeze state.
    #[pallet::storage]
    #[pallet::getter(fn consensus_freeze)]
    pub type ConsensusFreeze<T: Config> = StorageValue<_, FreezeState, ValueQuery>;

    // -----------------------------------------------------------------------
    // Events
    // -----------------------------------------------------------------------

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A fraud proof was submitted and is pending verification.
        FraudProofSubmitted {
            proof_id: H256,
            reporter: T::AccountId,
        },
        /// A fraud proof was accepted: divergence confirmed, proposer slashed.
        FraudProofAccepted {
            proof_id: H256,
            reporter: T::AccountId,
            disputed_block: H256,
        },
        /// The parallel scheduler was frozen due to confirmed divergence.
        ConsensusFrozen { reason: FreezeReason, at_block: u32 },
        /// Governance unfroze the scheduler.
        ConsensusUnfrozen,
        /// A fraud proof was rejected (not actually fraudulent or invalid).
        FraudProofRejected { proof_id: H256, reason: Vec<u8> },
    }

    // -----------------------------------------------------------------------
    // Errors
    // -----------------------------------------------------------------------

    #[pallet::error]
    pub enum Error<T> {
        /// This proof_id was already submitted and processed.
        DuplicateProof,
        /// The fraud proof was submitted outside the dispute window.
        DisputeWindowExpired,
        /// The disputed block metadata is not registered.
        UnknownDisputedBlock,
        /// The proof did not demonstrate actual divergence.
        ProofNotFraudulent,
        /// The witness encoding was invalid.
        InvalidWitnessEncoding,
        /// The proof type is not supported by this runtime version.
        UnsupportedProofType,
        /// Commitment values in the proof are internally inconsistent.
        CommitmentMismatch,
        /// Consensus is already in freeze state.
        AlreadyFrozen,
    }

    // -----------------------------------------------------------------------
    // Calls
    // -----------------------------------------------------------------------

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Submit a scheduler mismatch fraud proof.
        ///
        /// Any signed origin may submit.  The pallet verifies the proof and,
        /// if valid, freezes the scheduler and pays `ReporterRewardAmount` to
        /// the caller.
        ///
        /// # Invariants referenced
        /// - FRAUD-PROOF-001, FRAUD-PROOF-002, FRAUD-PROOF-003, FREEZE-001
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(100_000_000, 0).saturating_add(T::DbWeight::get().reads_writes(3, 3)))]
        pub fn submit_fraud_proof(
            origin: OriginFor<T>,
            proof: FraudProofV1<T::AccountId>,
            disputed: DisputedBlockMeta<T::AccountId>,
        ) -> DispatchResult {
            let reporter = ensure_signed(origin)?;

            // 1. Compute proof_id and check for replay.
            let proof_id = compute_proof_id(&proof, disputed.block_hash);
            ensure!(
                !ProofsSeen::<T>::contains_key(proof_id),
                Error::<T>::DuplicateProof
            );

            // 2. Dispute window: disputed block must be recent.
            let current_block: u32 = UniqueSaturatedInto::unique_saturated_into(
                <frame_system::Pallet<T>>::block_number(),
            );
            let dispute_window = T::DisputeWindowBlocks::get();
            let disputed_at = disputed.block_number;
            ensure!(
                current_block <= disputed_at.saturating_add(dispute_window),
                Error::<T>::DisputeWindowExpired
            );

            // 3. Verify the proof using the CPU reference scheduler.
            let max_tx = T::MaxTxCount::get();
            let verify_result = verify_scheduler_mismatch_v1(&proof, &disputed, max_tx);

            // Mark proof as seen regardless of outcome (prevent re-submission).
            ProofsSeen::<T>::insert(proof_id, ());

            // 4. Dispatch event & state changes based on verification outcome.
            Self::deposit_event(Event::FraudProofSubmitted {
                proof_id,
                reporter: reporter.clone(),
            });

            match verify_result {
                Ok((_confirmed_proof_id, _proposer)) => {
                    // Divergence confirmed — register dispute and freeze.
                    DisputedMeta::<T>::insert(disputed.block_hash, disputed.clone());

                    // Emit accepted event.
                    Self::deposit_event(Event::FraudProofAccepted {
                        proof_id,
                        reporter: reporter.clone(),
                        disputed_block: disputed.block_hash,
                    });

                    // Freeze unless already frozen.
                    let current_freeze = ConsensusFreeze::<T>::get();
                    if !current_freeze.is_consensus_frozen() {
                        let mut new_freeze = current_freeze;
                        new_freeze.engage(FreezeReason::DivergenceDetected, current_block);
                        ConsensusFreeze::<T>::put(new_freeze);
                        Self::deposit_event(Event::ConsensusFrozen {
                            reason: FreezeReason::DivergenceDetected,
                            at_block: current_block,
                        });
                    }

                    // Pay reporter reward from the treasury / imbalance.
                    // In production this slashes the proposer; here we emit
                    // the reward from nothing (inflationary) to keep dependencies
                    // minimal.  Wire `T::Currency::deposit_creating` for treasury.
                    let _ = T::Currency::deposit_into_existing(
                        &reporter,
                        T::ReporterRewardAmount::get(),
                    );
                }
                Err(VerifyError::NotFraudulent) => {
                    Self::deposit_event(Event::FraudProofRejected {
                        proof_id,
                        reason: b"not-fraudulent".to_vec(),
                    });
                    return Err(Error::<T>::ProofNotFraudulent.into());
                }
                Err(VerifyError::InvalidProofType) => {
                    Self::deposit_event(Event::FraudProofRejected {
                        proof_id,
                        reason: b"unsupported-proof-type".to_vec(),
                    });
                    return Err(Error::<T>::UnsupportedProofType.into());
                }
                Err(VerifyError::InvalidWitnessEncoding(_)) => {
                    Self::deposit_event(Event::FraudProofRejected {
                        proof_id,
                        reason: b"invalid-witness".to_vec(),
                    });
                    return Err(Error::<T>::InvalidWitnessEncoding.into());
                }
                Err(VerifyError::CommitmentMismatch) => {
                    Self::deposit_event(Event::FraudProofRejected {
                        proof_id,
                        reason: b"commitment-mismatch".to_vec(),
                    });
                    return Err(Error::<T>::CommitmentMismatch.into());
                }
            }

            Ok(())
        }

        /// Unfreeze the scheduler — callable only by governance.
        ///
        /// # Invariants referenced
        /// - FREEZE-002: only governance can unfreeze
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000_000, 0).saturating_add(T::DbWeight::get().reads_writes(1, 1)))]
        pub fn governance_unfreeze(origin: OriginFor<T>) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            ConsensusFreeze::<T>::mutate(|state| state.disengage());
            Self::deposit_event(Event::ConsensusUnfrozen);
            Ok(())
        }
    }

    // -----------------------------------------------------------------------
    // Helper impls
    // -----------------------------------------------------------------------

    impl<T: Config> Pallet<T> {
        /// Returns true when the scheduler/AI-syscall paths are currently frozen.
        pub fn is_frozen() -> bool {
            ConsensusFreeze::<T>::get().is_consensus_frozen()
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests for the pallet
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::pallet::*;
    use crate::fraud_proofs::{
        freeze::{FreezeReason, FreezeState},
        scheduler_v1::scheduler_commitment_from_bytes,
        types::{DisputedBlockMeta, FraudProofV1, HeaderRef, PROOF_TYPE_SCHED_MISMATCH_V1},
        verifier::compute_proof_id,
    };
    use sp_core::H256;

    fn zero_hash() -> H256 {
        H256::from([0u8; 32])
    }

    /// Build a minimal valid fraud proof with real commitment values.
    fn make_valid_proof(
        reporter: u64,
        observed_hash: H256,
        expected_hash: H256,
    ) -> (FraudProofV1<u64>, DisputedBlockMeta<u64>) {
        // Minimal 1-tx no-deps witness bytes — must match SCALE encoding of
        // SchedulerWitnessV1 exactly (rules_version is u32, NOT Compact).
        let witness_bytes: Vec<u8> = vec![
            0x01, // version: u8 = 1
            0x01, 0x00, 0x00, 0x00, // rules_version: u32 = 1 (4-byte LE)
            0x04, // tx_count: Compact<u32> = 1
            0x04, // tx_ids Vec length: Compact(1)
            // tx_ids[0]: H256::zero() (32 bytes)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x04, // access_lists Vec length: Compact(1)
            0x00, // access_lists[0].access_count Compact(0)
            0x00, // access_lists[0].accesses Vec length Compact(0)
            0x00, // seed: Option<H256> = None
            0x00, // reserved: Vec<u8> = []
        ];

        let scheduler_commitment =
            scheduler_commitment_from_bytes(&witness_bytes, 1, 256).expect("valid witness");

        let disputed = DisputedBlockMeta {
            block_hash: zero_hash(),
            block_number: 5,
            rules_version: 1,
            scheduler_commitment,
            proposer: 99u64,
        };

        let proof = FraudProofV1 {
            proof_type: PROOF_TYPE_SCHED_MISMATCH_V1,
            header_ref: HeaderRef {
                block_number: 5,
                block_hash: zero_hash(),
            },
            reexec_witness: witness_bytes,
            tx_set_commitment: zero_hash(),
            claimed_scheduler_commitment: scheduler_commitment,
            expected_hash,
            observed_hash,
            reporter,
            nonce: 0,
        };

        (proof, disputed)
    }

    /// FRAUD-PROOF-PALLET-001: compute_proof_id is stable
    #[test]
    fn proof_id_is_stable() {
        let (proof, disputed) =
            make_valid_proof(1u64, H256::from([1u8; 32]), H256::from([2u8; 32]));
        let id1 = compute_proof_id(&proof, disputed.block_hash);
        let id2 = compute_proof_id(&proof, disputed.block_hash);
        assert_eq!(id1, id2);
    }

    /// FRAUD-PROOF-PALLET-002: FreezeState default is not frozen
    #[test]
    fn default_freeze_state_not_frozen() {
        let state = FreezeState::default();
        assert!(!state.is_consensus_frozen());
    }

    /// FRAUD-PROOF-PALLET-003: engaging freeze sets flags
    #[test]
    fn engage_freeze_sets_flags() {
        let mut state = FreezeState::default();
        state.engage(FreezeReason::DivergenceDetected, 10);
        assert!(state.is_consensus_frozen());
        assert_eq!(state.frozen_at_block, Some(10));
    }
}
