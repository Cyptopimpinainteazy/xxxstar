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
// Mock runtime for pallet tests
// ---------------------------------------------------------------------------

#[cfg(test)]
pub mod mock {
    use super::pallet::*;
    use frame_support::{
        construct_runtime, derive_impl, parameter_types,
        traits::{ConstU32, ConstU64, Everything},
    };
    use sp_core::H256;
    use sp_runtime::{
        traits::{BlakeTwo256, IdentityLookup},
        BuildStorage,
    };

    pub type AccountId = u64;
    pub type BlockNumber = u64;
    pub type Balance = u128;

    #[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
    impl frame_system::Config for Test {
        type BaseCallFilter = Everything;
        type BlockWeights = ();
        type BlockLength = ();
        type DbWeight = ();
        type RuntimeOrigin = RuntimeOrigin;
        type RuntimeCall = RuntimeCall;
        type Nonce = u64;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type AccountId = AccountId;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Block = Block;
        type RuntimeEvent = RuntimeEvent;
        type BlockHashCount = ConstU64<250>;
        type Version = ();
        type PalletInfo = PalletInfo;
        type AccountData = pallet_balances::AccountData<Balance>;
        type OnNewAccount = ();
        type OnKilledAccount = ();
        type SystemWeightInfo = ();
        type SS58Prefix = ();
        type OnSetCode = ();
        type MaxConsumers = frame_support::traits::ConstU32<16>;
    }

    parameter_types! {
        pub const ExistentialDeposit: Balance = 1;
        pub const MaxLocks: u32 = 50;
    }

    impl pallet_balances::Config for Test {
        type MaxLocks = MaxLocks;
        type MaxReserves = ();
        type ReserveIdentifier = [u8; 8];
        type Balance = Balance;
        type RuntimeEvent = RuntimeEvent;
        type DustRemoval = ();
        type ExistentialDeposit = ExistentialDeposit;
        type AccountStore = frame_system::Pallet<Test>;
        type WeightInfo = ();
        type FreezeIdentifier = ();
        type MaxFreezes = ();
        type RuntimeHoldReason = ();
        type MaxHolds = ();
    }

    parameter_types! {
        pub const FraudProofMaxTxCount: u32 = 256;
        pub const FraudProofDisputeWindowBlocks: u32 = 100;
        pub const FraudProofReporterReward: Balance = 100;
    }

    impl Config for Test {
        type RuntimeEvent = RuntimeEvent;
        type Currency = Balances;
        type MaxTxCount = FraudProofMaxTxCount;
        type DisputeWindowBlocks = FraudProofDisputeWindowBlocks;
        type ReporterRewardAmount = FraudProofReporterReward;
        type GovernanceOrigin = frame_system::EnsureRoot<AccountId>;
    }

    pub type Block = frame_system::mocking::MockBlockU32<Test>;

    construct_runtime!(
        pub enum Runtime {
            System: frame_system,
            Balances: pallet_balances,
            FraudProofs: crate::fraud_proofs::pallet::pallet,
        }
    );

    /// Build a test externalities with initial balances.
    pub fn new_test_ext() -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap();

        pallet_balances::GenesisConfig::<Test> {
            balances: vec![
                (1u64, 1_000_000),
                (2u64, 1_000_000),
                (99u64, 1_000_000),
            ],
        }
        .assimilate_storage(&mut t)
        .unwrap();

        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| {
            frame_system::Pallet::<Test>::set_block_number(50u32.into());
        });
        ext
    }
}

// ---------------------------------------------------------------------------
// Pallet extrinsic tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::mock::*;
    use super::pallet::*;
    use crate::fraud_proofs::{
        freeze::{FreezeReason, FreezeState},
        scheduler_v1::scheduler_commitment_from_bytes,
        types::{DisputedBlockMeta, FraudProofV1, HeaderRef, PROOF_TYPE_SCHED_MISMATCH_V1},
        verifier::compute_proof_id,
    };
    use codec::Encode;
    use sp_core::H256;

    fn zero_hash() -> H256 {
        H256::from([0u8; 32])
    }

    /// Build a minimal valid fraud proof with real commitment values.
    /// `observed_hash` is the (forged) commitment in the block.
    /// `expected_hash` is what the reporter claims the correct commitment is.
    fn make_valid_proof(
        reporter: u64,
        observed_hash: H256,
        expected_hash: H256,
    ) -> (FraudProofV1<u64>, DisputedBlockMeta<u64>) {
        // Minimal 1-tx no-deps witness bytes
        let witness_bytes: Vec<u8> = vec![
            0x01, // version: u8 = 1
            0x01, 0x00, 0x00, 0x00, // rules_version: u32 = 1
            0x04, // tx_count: Compact<u32> = 1
            0x04, // tx_ids Vec length: Compact(1)
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

    // ── FRAUD-PROOF-PALLET-001: compute_proof_id is stable ──────────────────

    #[test]
    fn proof_id_is_stable() {
        let (proof, disputed) =
            make_valid_proof(1u64, H256::from([1u8; 32]), H256::from([2u8; 32]));
        let id1 = compute_proof_id(&proof, disputed.block_hash);
        let id2 = compute_proof_id(&proof, disputed.block_hash);
        assert_eq!(id1, id2);
    }

    // ── FRAUD-PROOF-PALLET-002: FreezeState default is not frozen ──────────

    #[test]
    fn default_freeze_state_not_frozen() {
        let state = FreezeState::default();
        assert!(!state.is_consensus_frozen());
    }

    // ── FRAUD-PROOF-PALLET-003: engaging freeze sets flags ─────────────────

    #[test]
    fn engage_freeze_sets_flags() {
        let mut state = FreezeState::default();
        state.engage(FreezeReason::DivergenceDetected, 10);
        assert!(state.is_consensus_frozen());
        assert_eq!(state.frozen_at_block, Some(10));
    }

    // ── FRAUD-PROOF-PALLET-004: submit_fraud_proof accepts valid proof ─────

    #[test]
    fn submit_valid_fraud_proof_succeeds() {
        new_test_ext().execute_with(|| {
            // Forge a wrong commitment that the block claims
            let forged = H256([0xFF; 32]);
            let (proof, disputed) = make_valid_proof(1u64, forged, disputed.scheduler_commitment);

            // The disputed block's meta must have the forged commitment
            let disputed_with_forged = DisputedBlockMeta {
                scheduler_commitment: forged,
                ..disputed
            };

            assert_ok!(FraudProofs::submit_fraud_proof(
                RuntimeOrigin::signed(1u64),
                proof,
                disputed_with_forged,
            ));

            // Verify freeze was engaged
            assert!(FraudProofs::is_frozen());
        });
    }

    // ── FRAUD-PROOF-PALLET-005: duplicate proof rejected ───────────────────

    #[test]
    fn duplicate_proof_rejected() {
        new_test_ext().execute_with(|| {
            let forged = H256([0xFF; 32]);
            let (proof, disputed) = make_valid_proof(1u64, forged, disputed.scheduler_commitment);
            let disputed_with_forged = DisputedBlockMeta {
                scheduler_commitment: forged,
                ..disputed
            };

            // First submission succeeds
            assert_ok!(FraudProofs::submit_fraud_proof(
                RuntimeOrigin::signed(1u64),
                proof.clone(),
                disputed_with_forged.clone(),
            ));

            // Second submission with same proof fails
            assert_noop!(
                FraudProofs::submit_fraud_proof(
                    RuntimeOrigin::signed(2u64),
                    proof,
                    disputed_with_forged,
                ),
                Error::<Test>::DuplicateProof
            );
        });
    }

    // ── FRAUD-PROOF-PALLET-006: non-fraudulent proof rejected ──────────────

    #[test]
    fn non_fraudulent_proof_rejected() {
        new_test_ext().execute_with(|| {
            let (proof, disputed) = make_valid_proof(
                1u64,
                disputed.scheduler_commitment, // observed == real (no fraud)
                disputed.scheduler_commitment, // expected == real
            );

            assert_noop!(
                FraudProofs::submit_fraud_proof(
                    RuntimeOrigin::signed(1u64),
                    proof,
                    disputed,
                ),
                Error::<Test>::ProofNotFraudulent
            );

            // Freeze should NOT be engaged
            assert!(!FraudProofs::is_frozen());
        });
    }

    // ── FRAUD-PROOF-PALLET-007: dispute window expired ─────────────────────

    #[test]
    fn dispute_window_expired_rejected() {
        new_test_ext().execute_with(|| {
            // Set current block far in the future
            frame_system::Pallet::<Test>::set_block_number(1000u32.into());

            let forged = H256([0xFF; 32]);
            let (proof, disputed) = make_valid_proof(1u64, forged, disputed.scheduler_commitment);
            let disputed_with_forged = DisputedBlockMeta {
                scheduler_commitment: forged,
                ..disputed
            };

            // Disputed block is at 5, current is 1000, window is 100
            // 1000 > 5 + 100 = 105 → expired
            assert_noop!(
                FraudProofs::submit_fraud_proof(
                    RuntimeOrigin::signed(1u64),
                    proof,
                    disputed_with_forged,
                ),
                Error::<Test>::DisputeWindowExpired
            );
        });
    }

    // ── FRAUD-PROOF-PALLET-008: governance_unfreeze works ──────────────────

    #[test]
    fn governance_unfreeze_works() {
        new_test_ext().execute_with(|| {
            // First, submit a valid proof to freeze
            let forged = H256([0xFF; 32]);
            let (proof, disputed) = make_valid_proof(1u64, forged, disputed.scheduler_commitment);
            let disputed_with_forged = DisputedBlockMeta {
                scheduler_commitment: forged,
                ..disputed
            };

            assert_ok!(FraudProofs::submit_fraud_proof(
                RuntimeOrigin::signed(1u64),
                proof,
                disputed_with_forged,
            ));
            assert!(FraudProofs::is_frozen());

            // Governance unfreeze
            assert_ok!(FraudProofs::governance_unfreeze(
                RuntimeOrigin::root(),
            ));
            assert!(!FraudProofs::is_frozen());
        });
    }

    // ── FRAUD-PROOF-PALLET-009: non-root cannot unfreeze ───────────────────

    #[test]
    fn non_root_cannot_unfreeze() {
        new_test_ext().execute_with(|| {
            assert_noop!(
                FraudProofs::governance_unfreeze(RuntimeOrigin::signed(1u64)),
                sp_runtime::DispatchError::BadOrigin,
            );
        });
    }

    // ── FRAUD-PROOF-PALLET-010: reporter gets reward ───────────────────────

    #[test]
    fn reporter_gets_reward() {
        new_test_ext().execute_with(|| {
            let reporter_balance_before = Balances::free_balance(1u64);

            let forged = H256([0xFF; 32]);
            let (proof, disputed) = make_valid_proof(1u64, forged, disputed.scheduler_commitment);
            let disputed_with_forged = DisputedBlockMeta {
                scheduler_commitment: forged,
                ..disputed
            };

            assert_ok!(FraudProofs::submit_fraud_proof(
                RuntimeOrigin::signed(1u64),
                proof,
                disputed_with_forged,
            ));

            let reporter_balance_after = Balances::free_balance(1u64);
            assert_eq!(
                reporter_balance_after,
                reporter_balance_before + FraudProofReporterReward::get(),
                "reporter should receive reward"
            );
        });
    }

    // ── FRAUD-PROOF-PALLET-011: invalid proof type rejected ────────────────

    #[test]
    fn invalid_proof_type_rejected() {
        new_test_ext().execute_with(|| {
            let forged = H256([0xFF; 32]);
            let (mut proof, disputed) =
                make_valid_proof(1u64, forged, disputed.scheduler_commitment);
            proof.proof_type = 0x02; // unknown type

            let disputed_with_forged = DisputedBlockMeta {
                scheduler_commitment: forged,
                ..disputed
            };

            assert_noop!(
                FraudProofs::submit_fraud_proof(
                    RuntimeOrigin::signed(1u64),
                    proof,
                    disputed_with_forged,
                ),
                Error::<Test>::UnsupportedProofType
            );
        });
    }

    // ── FRAUD-PROOF-PALLET-012: commitment mismatch rejected ───────────────

    #[test]
    fn commitment_mismatch_rejected() {
        new_test_ext().execute_with(|| {
            let forged = H256([0xFF; 32]);
            let (proof, disputed) = make_valid_proof(1u64, forged, disputed.scheduler_commitment);

            // Disputed meta has a DIFFERENT commitment than what proof.observed_hash says
            let disputed_wrong_commitment = DisputedBlockMeta {
                scheduler_commitment: H256([0xEE; 32]), // different from forged
                ..disputed
            };

            assert_noop!(
                FraudProofs::submit_fraud_proof(
                    RuntimeOrigin::signed(1u64),
                    proof,
                    disputed_wrong_commitment,
                ),
                Error::<Test>::CommitmentMismatch
            );
        });
    }

    // ── FRAUD-PROOF-PALLET-013: events emitted correctly ───────────────────

    #[test]
    fn events_emitted_correctly() {
        new_test_ext().execute_with(|| {
            let forged = H256([0xFF; 32]);
            let (proof, disputed) = make_valid_proof(1u64, forged, disputed.scheduler_commitment);
            let disputed_with_forged = DisputedBlockMeta {
                scheduler_commitment: forged,
                ..disputed
            };

            assert_ok!(FraudProofs::submit_fraud_proof(
                RuntimeOrigin::signed(1u64),
                proof.clone(),
                disputed_with_forged.clone(),
            ));

            // Check events
            let events = frame_system::Pallet::<Test>::events();
            let proof_id = compute_proof_id(&proof, disputed_with_forged.block_hash);

            let submitted = events.iter().any(|e| {
                matches!(
                    e.event,
                    RuntimeEvent::FraudProofs(Event::FraudProofSubmitted { proof_id: pid, .. })
                    if pid == proof_id
                )
            });
            assert!(submitted, "FraudProofSubmitted event must be emitted");

            let accepted = events.iter().any(|e| {
                matches!(
                    e.event,
                    RuntimeEvent::FraudProofs(Event::FraudProofAccepted { proof_id: pid, .. })
                    if pid == proof_id
                )
            });
            assert!(accepted, "FraudProofAccepted event must be emitted");

            let frozen = events.iter().any(|e| {
                matches!(
                    e.event,
                    RuntimeEvent::FraudProofs(Event::ConsensusFrozen { .. })
                )
            });
            assert!(frozen, "ConsensusFrozen event must be emitted");
        });
    }

    // ── FRAUD-PROOF-PALLET-014: DisputedMeta stored after acceptance ───────

    #[test]
    fn disputed_meta_stored_after_acceptance() {
        new_test_ext().execute_with(|| {
            let forged = H256([0xFF; 32]);
            let (proof, disputed) = make_valid_proof(1u64, forged, disputed.scheduler_commitment);
            let disputed_with_forged = DisputedBlockMeta {
                scheduler_commitment: forged,
                ..disputed
            };

            assert_ok!(FraudProofs::submit_fraud_proof(
                RuntimeOrigin::signed(1u64),
                proof,
                disputed_with_forged.clone(),
            ));

            let stored = FraudProofs::disputed_meta(disputed_with_forged.block_hash);
            assert!(stored.is_some(), "DisputedMeta must be stored");
            assert_eq!(stored.unwrap().proposer, 99u64);
        });
    }

    // ── FRAUD-PROOF-PALLET-015: ProofsSeen prevents replay ─────────────────

    #[test]
    fn proofs_seen_prevents_replay() {
        new_test_ext().execute_with(|| {
            let forged = H256([0xFF; 32]);
            let (proof, disputed) = make_valid_proof(1u64, forged, disputed.scheduler_commitment);
            let disputed_with_forged = DisputedBlockMeta {
                scheduler_commitment: forged,
                ..disputed
            };

            let proof_id = compute_proof_id(&proof, disputed_with_forged.block_hash);

            // Submit once
            assert_ok!(FraudProofs::submit_fraud_proof(
                RuntimeOrigin::signed(1u64),
                proof.clone(),
                disputed_with_forged.clone(),
            ));

            // ProofsSeen should contain the proof_id
            assert!(FraudProofs::proofs_seen(proof_id).is_some());

            // Re-submit with different reporter should still fail
            assert_noop!(
                FraudProofs::submit_fraud_proof(
                    RuntimeOrigin::signed(2u64),
                    proof,
                    disputed_with_forged,
                ),
                Error::<Test>::DuplicateProof
            );
        });
    }
}
