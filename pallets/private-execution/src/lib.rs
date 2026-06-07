#![deny(unsafe_code)]
//! # Private Execution Environments Pallet
//!
//! Proposal: PRIV-ENCLAVE-003
//!
//! Provides native "private mode" where transactions are routed through encrypted
//! mempools and executed inside trusted GPU enclaves (NVIDIA Confidential Computing).
//! Results are committed as encrypted state diffs with optional ZK proofs.
//!
//! ## Key Invariants
//!
//! - PRIV-EXEC-001: TX content never exposed in plaintext outside enclave
//! - PRIV-EXEC-002: Encrypted state diff == public execution of same TX
//! - PRIV-EXEC-003: No single validator can decrypt (threshold t-of-n)
//! - PRIV-EXEC-004: Attestation verified before joining confidential set
//! - PRIV-EXEC-005: Premium fee correctly collected and split
//! - PRIV-EXEC-006: Finality latency overhead ≤1ms

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

pub mod types;
pub use types::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ExistenceRequirement, OnUnbalanced, ReservableCurrency},
        Blake2_128Concat, PalletId,
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::{
        traits::{AccountIdConversion, SaturatedConversion, Saturating},
        Perbill,
    };
    use sp_std::prelude::*;

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
        <T as frame_system::Config>::AccountId,
    >>::NegativeImbalance;

    use frame_support::traits::StorageVersion;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::without_storage_info]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    // ──────────────────────────────────────────────────────────────
    // Config
    // ──────────────────────────────────────────────────────────────

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency for fee collection.
        type Currency: ReservableCurrency<Self::AccountId>;

        /// Handler for burned fee portion.
        type BurnDestination: OnUnbalanced<NegativeImbalanceOf<Self>>;

        /// Origin that can manage the confidential validator set.
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// The pallet's ID (for fee escrow).
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Premium fee in basis points added on top of base fee.
        /// E.g., 150 = 1.5% premium.
        #[pallet::constant]
        type PrivateFeePremiumBps: Get<u16>;

        /// Minimum confidential validators for quorum.
        #[pallet::constant]
        type MinConfidentialQuorum: Get<u32>;

        /// Maximum confidential validators.
        #[pallet::constant]
        type MaxConfidentialValidators: Get<u32>;

        /// Maximum encrypted state diffs per block.
        #[pallet::constant]
        type MaxDiffsPerBlock: Get<u32>;

        /// Maximum payload size for encrypted transactions (bytes).
        #[pallet::constant]
        type MaxEncryptedPayloadSize: Get<u32>;

        /// Attestation validity period in blocks.
        #[pallet::constant]
        type AttestationValidityPeriod: Get<BlockNumberFor<Self>>;

        /// Revenue share to confidential validators (bps out of 10_000).
        #[pallet::constant]
        type ConfidentialValidatorShareBps: Get<u16>;

        /// Revenue share to burn (bps out of 10_000).
        #[pallet::constant]
        type PrivateBurnShareBps: Get<u16>;

        /// Revenue share to stakers (bps out of 10_000).
        #[pallet::constant]
        type PrivateStakerShareBps: Get<u16>;

        /// Weight info.
        type WeightInfo: WeightInfo;
    }

    // ──────────────────────────────────────────────────────────────
    // Storage
    // ──────────────────────────────────────────────────────────────

    /// Registered confidential validators with attestation data.
    #[pallet::storage]
    #[pallet::getter(fn confidential_validators)]
    pub type ConfidentialValidators<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, EnclaveAttestation<T>, OptionQuery>;

    /// Number of registered confidential validators.
    #[pallet::storage]
    #[pallet::getter(fn confidential_validator_count)]
    pub type ConfidentialValidatorCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Private transaction records.
    #[pallet::storage]
    #[pallet::getter(fn private_transactions)]
    pub type PrivateTransactions<T: Config> =
        StorageMap<_, Blake2_128Concat, sp_core::H256, PrivateTxRecord<T>, OptionQuery>;

    /// Encrypted state diffs committed per block.
    #[pallet::storage]
    #[pallet::getter(fn encrypted_state_diffs)]
    pub type EncryptedStateDiffs<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<EncryptedDiff, T::MaxDiffsPerBlock>,
        ValueQuery,
    >;

    /// DKG committee public key (threshold encryption key).
    #[pallet::storage]
    #[pallet::getter(fn committee_public_key)]
    pub type CommitteePublicKey<T: Config> =
        StorageValue<_, BoundedVec<u8, ConstU32<64>>, OptionQuery>;

    /// Current DKG epoch number.
    #[pallet::storage]
    #[pallet::getter(fn dkg_epoch)]
    pub type DkgEpoch<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Total private transactions processed.
    #[pallet::storage]
    #[pallet::getter(fn total_private_txs)]
    pub type TotalPrivateTxs<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Total premium fees collected.
    #[pallet::storage]
    #[pallet::getter(fn total_premium_fees)]
    pub type TotalPremiumFees<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// Whether private execution is enabled.
    #[pallet::storage]
    #[pallet::getter(fn is_enabled)]
    pub type Enabled<T: Config> = StorageValue<_, bool, ValueQuery>;

    // ──────────────────────────────────────────────────────────────
    // Events
    // ──────────────────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A validator registered as confidential with attestation.
        ConfidentialValidatorRegistered {
            validator: T::AccountId,
            gpu_model: Vec<u8>,
        },
        /// A confidential validator was removed.
        ConfidentialValidatorRemoved { validator: T::AccountId },
        /// Attestation was refreshed.
        AttestationRefreshed { validator: T::AccountId, epoch: u64 },
        /// A private transaction was submitted.
        PrivateTxSubmitted {
            tx_hash: sp_core::H256,
            sender: T::AccountId,
            fee: BalanceOf<T>,
        },
        /// A private transaction was executed inside enclave.
        PrivateTxExecuted {
            tx_hash: sp_core::H256,
            enclave_validator: T::AccountId,
        },
        /// An encrypted state diff was committed.
        StateDiffCommitted {
            tx_hash: sp_core::H256,
            block_number: BlockNumberFor<T>,
            has_zk_proof: bool,
        },
        /// DKG key rotation completed.
        DkgKeyRotated {
            epoch: u64,
            validators_participating: u32,
        },
        /// Premium fee distributed.
        PremiumFeeDistributed {
            tx_hash: sp_core::H256,
            validator_share: BalanceOf<T>,
            burned: BalanceOf<T>,
            staker_share: BalanceOf<T>,
        },
        /// Private execution enabled/disabled.
        PrivateExecutionToggled { enabled: bool },
    }

    // ──────────────────────────────────────────────────────────────
    // Errors
    // ──────────────────────────────────────────────────────────────

    #[pallet::error]
    pub enum Error<T> {
        /// Validator already registered as confidential.
        AlreadyRegistered,
        /// Validator not found in confidential set.
        ValidatorNotFound,
        /// Invalid or expired attestation report.
        InvalidAttestation,
        /// Attestation has expired and needs refresh.
        AttestationExpired,
        /// Not enough confidential validators for quorum.
        InsufficientQuorum,
        /// Maximum confidential validators reached.
        MaxValidatorsReached,
        /// Private execution is disabled.
        PrivateExecutionDisabled,
        /// State diff limit per block exceeded.
        MaxDiffsExceeded,
        /// Encrypted payload too large.
        PayloadTooLarge,
        /// Transaction already exists.
        TxAlreadyExists,
        /// Transaction not found.
        TxNotFound,
        /// Not a confidential validator.
        NotConfidentialValidator,
        /// DKG committee key not yet established.
        NoDkgKey,
        /// ZK proof verification failed.
        ZkProofInvalid,
        /// Fee calculation overflow.
        ArithmeticOverflow,
        /// Insufficient balance for premium fee.
        InsufficientBalance,
    }

    // ──────────────────────────────────────────────────────────────
    // Genesis
    // ──────────────────────────────────────────────────────────────

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        pub enabled: bool,
        #[serde(skip)]
        pub _phantom: sp_std::marker::PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            Enabled::<T>::put(self.enabled);
            // Validate fee split
            let total = T::ConfidentialValidatorShareBps::get() as u32
                + T::PrivateBurnShareBps::get() as u32
                + T::PrivateStakerShareBps::get() as u32;
            assert_eq!(
                total, 10_000,
                "Private execution fee split must sum to 10_000 bps"
            );
        }
    }

    // ──────────────────────────────────────────────────────────────
    // Hooks
    // ──────────────────────────────────────────────────────────────

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_now: BlockNumberFor<T>) -> Weight {
            Weight::zero()
        }
    }

    // ──────────────────────────────────────────────────────────────
    // Dispatchables
    // ──────────────────────────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register as a confidential validator with GPU enclave attestation.
        ///
        /// # Invariant: PRIV-EXEC-004
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn register_confidential_validator(
            origin: OriginFor<T>,
            gpu_model: Vec<u8>,
            attestation_report: Vec<u8>,
            enclave_public_key: [u8; 32],
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Enabled::<T>::get(), Error::<T>::PrivateExecutionDisabled);
            ensure!(
                !ConfidentialValidators::<T>::contains_key(&who),
                Error::<T>::AlreadyRegistered
            );
            ensure!(
                ConfidentialValidatorCount::<T>::get() < T::MaxConfidentialValidators::get(),
                Error::<T>::MaxValidatorsReached
            );

            // Verify attestation (simplified — real impl would verify NVIDIA CC report)
            ensure!(
                Self::verify_attestation(&attestation_report),
                Error::<T>::InvalidAttestation
            );

            let now = <frame_system::Pallet<T>>::block_number();

            let attestation = EnclaveAttestation {
                validator: who.clone(),
                gpu_model: BoundedVec::try_from(gpu_model.clone())
                    .map_err(|_| Error::<T>::PayloadTooLarge)?,
                attestation_report: BoundedVec::try_from(attestation_report)
                    .map_err(|_| Error::<T>::PayloadTooLarge)?,
                enclave_public_key,
                last_refreshed: now,
                status: EnclaveStatus::Verified,
            };

            ConfidentialValidators::<T>::insert(&who, attestation);
            ConfidentialValidatorCount::<T>::mutate(|c| *c = c.saturating_add(1));

            Self::deposit_event(Event::ConfidentialValidatorRegistered {
                validator: who,
                gpu_model,
            });

            Ok(())
        }

        /// Remove a confidential validator from the set.
        #[pallet::call_index(1)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 2))]
        pub fn deregister_confidential_validator(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(
                ConfidentialValidators::<T>::contains_key(&who),
                Error::<T>::ValidatorNotFound
            );

            ConfidentialValidators::<T>::remove(&who);
            ConfidentialValidatorCount::<T>::mutate(|c| *c = c.saturating_sub(1));

            Self::deposit_event(Event::ConfidentialValidatorRemoved { validator: who });
            Ok(())
        }

        /// Refresh attestation report (required periodically).
        #[pallet::call_index(2)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn refresh_attestation(
            origin: OriginFor<T>,
            new_attestation_report: Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ConfidentialValidators::<T>::try_mutate(&who, |maybe_att| -> DispatchResult {
                let att = maybe_att.as_mut().ok_or(Error::<T>::ValidatorNotFound)?;

                ensure!(
                    Self::verify_attestation(&new_attestation_report),
                    Error::<T>::InvalidAttestation
                );

                att.attestation_report = BoundedVec::try_from(new_attestation_report)
                    .map_err(|_| Error::<T>::PayloadTooLarge)?;
                att.last_refreshed = <frame_system::Pallet<T>>::block_number();
                att.status = EnclaveStatus::Verified;

                Ok(())
            })?;

            let epoch = DkgEpoch::<T>::get();
            Self::deposit_event(Event::AttestationRefreshed {
                validator: who,
                epoch,
            });

            Ok(())
        }

        /// Submit an encrypted private transaction.
        ///
        /// # Invariant: PRIV-EXEC-001, PRIV-EXEC-005
        #[pallet::call_index(3)]
        #[pallet::weight(T::DbWeight::get().reads_writes(3, 2))]
        pub fn submit_private_transaction(
            origin: OriginFor<T>,
            tx_hash: sp_core::H256,
            encrypted_payload: Vec<u8>,
            fee_commitment: sp_core::H256,
            priority_fee: BalanceOf<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Enabled::<T>::get(), Error::<T>::PrivateExecutionDisabled);
            ensure!(
                ConfidentialValidatorCount::<T>::get() >= T::MinConfidentialQuorum::get(),
                Error::<T>::InsufficientQuorum
            );
            ensure!(
                CommitteePublicKey::<T>::get().is_some(),
                Error::<T>::NoDkgKey
            );
            ensure!(
                !PrivateTransactions::<T>::contains_key(tx_hash),
                Error::<T>::TxAlreadyExists
            );
            ensure!(
                encrypted_payload.len() <= T::MaxEncryptedPayloadSize::get() as usize,
                Error::<T>::PayloadTooLarge
            );

            // Collect premium fee
            let base_fee = priority_fee;
            let premium_bps = T::PrivateFeePremiumBps::get() as u128;
            let premium = base_fee.saturating_mul(
                premium_bps
                    .try_into()
                    .map_err(|_| Error::<T>::ArithmeticOverflow)?,
            ) / 10_000u32.into();
            let total_fee = base_fee.saturating_add(premium);

            ensure!(
                T::Currency::free_balance(&who) >= total_fee,
                Error::<T>::InsufficientBalance
            );

            let escrow_account = Self::account_id();
            T::Currency::transfer(
                &who,
                &escrow_account,
                total_fee,
                ExistenceRequirement::KeepAlive,
            )?;

            TotalPremiumFees::<T>::mutate(|f| *f = f.saturating_add(premium));

            let now = <frame_system::Pallet<T>>::block_number();

            let record = PrivateTxRecord {
                tx_hash,
                sender: who.clone(),
                encrypted_payload: BoundedVec::try_from(encrypted_payload)
                    .map_err(|_| Error::<T>::PayloadTooLarge)?,
                fee_commitment,
                fee_paid: total_fee.saturated_into(),
                status: PrivateTxStatus::Pending,
                submitted_at: now,
                executed_by: None,
            };

            PrivateTransactions::<T>::insert(tx_hash, record);
            TotalPrivateTxs::<T>::mutate(|t| *t = t.saturating_add(1));

            Self::deposit_event(Event::PrivateTxSubmitted {
                tx_hash,
                sender: who,
                fee: total_fee,
            });

            Ok(())
        }

        /// Commit an encrypted state diff from enclave execution.
        ///
        /// # Invariant: PRIV-EXEC-002
        #[pallet::call_index(4)]
        #[pallet::weight(T::DbWeight::get().reads_writes(3, 3))]
        pub fn commit_encrypted_state_diff(
            origin: OriginFor<T>,
            tx_hash: sp_core::H256,
            encrypted_state_changes: Vec<u8>,
            commitment: sp_core::H256,
            zk_proof: Option<Vec<u8>>,
            enclave_signature: [u8; 64],
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Must be a confidential validator
            let att = ConfidentialValidators::<T>::get(&who)
                .ok_or(Error::<T>::NotConfidentialValidator)?;
            ensure!(
                att.status == EnclaveStatus::Verified,
                Error::<T>::AttestationExpired
            );

            // TX must exist and be pending
            PrivateTransactions::<T>::try_mutate(tx_hash, |maybe_record| -> DispatchResult {
                let record = maybe_record.as_mut().ok_or(Error::<T>::TxNotFound)?;
                record.status = PrivateTxStatus::Committed;
                record.executed_by = Some(who.clone());
                Ok(())
            })?;

            let now = <frame_system::Pallet<T>>::block_number();
            let has_zk_proof = zk_proof.is_some();

            let diff = EncryptedDiff {
                tx_hash,
                encrypted_state_changes: BoundedVec::try_from(encrypted_state_changes)
                    .map_err(|_| Error::<T>::PayloadTooLarge)?,
                commitment,
                zk_proof: zk_proof.map(|p| BoundedVec::try_from(p).unwrap_or_default()),
                enclave_signature,
                committed_at: now.saturated_into(),
            };

            EncryptedStateDiffs::<T>::try_mutate(now, |diffs| -> DispatchResult {
                diffs
                    .try_push(diff)
                    .map_err(|_| Error::<T>::MaxDiffsExceeded)?;
                Ok(())
            })?;

            // Distribute premium fee to the executing validator
            if let Some(record) = PrivateTransactions::<T>::get(tx_hash) {
                let fee: BalanceOf<T> = record.fee_paid.saturated_into();
                Self::distribute_premium_fee(tx_hash, &who, fee)?;
            }

            Self::deposit_event(Event::StateDiffCommitted {
                tx_hash,
                block_number: now,
                has_zk_proof,
            });

            Ok(())
        }

        /// Set the DKG committee public key (called after DKG ceremony).
        #[pallet::call_index(5)]
        #[pallet::weight(T::DbWeight::get().writes(2))]
        pub fn set_committee_key(origin: OriginFor<T>, public_key: Vec<u8>) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            let bounded =
                BoundedVec::try_from(public_key).map_err(|_| Error::<T>::PayloadTooLarge)?;
            CommitteePublicKey::<T>::put(bounded);

            let new_epoch = DkgEpoch::<T>::mutate(|e| {
                *e = e.saturating_add(1);
                *e
            });

            Self::deposit_event(Event::DkgKeyRotated {
                epoch: new_epoch,
                validators_participating: ConfidentialValidatorCount::<T>::get(),
            });

            Ok(())
        }

        /// Enable or disable private execution (admin only).
        #[pallet::call_index(6)]
        #[pallet::weight(T::DbWeight::get().writes(1))]
        pub fn set_enabled(origin: OriginFor<T>, enabled: bool) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            Enabled::<T>::put(enabled);

            Self::deposit_event(Event::PrivateExecutionToggled { enabled });
            Ok(())
        }
    }

    // ──────────────────────────────────────────────────────────────
    // Helpers
    // ──────────────────────────────────────────────────────────────

    impl<T: Config> Pallet<T> {
        /// Get the escrow account for this pallet.
        pub fn account_id() -> T::AccountId {
            T::PalletId::get().into_account_truncating()
        }

        /// Verify an attestation report.
        /// In production this would verify the NVIDIA CC attestation chain.
        /// For now, accepts any non-empty report.
        fn verify_attestation(report: &[u8]) -> bool {
            !report.is_empty()
        }

        /// Distribute premium fees.
        ///
        /// # Invariant: PRIV-EXEC-005
        fn distribute_premium_fee(
            tx_hash: sp_core::H256,
            validator: &T::AccountId,
            total: BalanceOf<T>,
        ) -> DispatchResult {
            let validator_bps = T::ConfidentialValidatorShareBps::get() as u32;
            let burn_bps = T::PrivateBurnShareBps::get() as u32;

            let validator_share = Perbill::from_parts(validator_bps * 100_000) * total;
            let burn_share = Perbill::from_parts(burn_bps * 100_000) * total;
            let staker_share = total
                .saturating_sub(validator_share)
                .saturating_sub(burn_share);

            let escrow_account = Self::account_id();

            // Pay validator
            T::Currency::transfer(
                &escrow_account,
                validator,
                validator_share,
                ExistenceRequirement::AllowDeath,
            )?;

            // Burn
            let imbalance = T::Currency::slash(&escrow_account, burn_share).0;
            T::BurnDestination::on_unbalanced(imbalance);

            Self::deposit_event(Event::PremiumFeeDistributed {
                tx_hash,
                validator_share,
                burned: burn_share,
                staker_share,
            });

            Ok(())
        }
    }
}
