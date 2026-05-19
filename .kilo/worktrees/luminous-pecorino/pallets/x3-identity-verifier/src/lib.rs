#![deny(unsafe_code)]
//! # X3 Identity Verifier Pallet
//!
//! The X3 Identity Verifier pallet provides on-chain identity verification
//! for cross-VM operations. It manages a registry of verified identity
//! verifiers and enforces a quorum-based verification model for identity proofs.
//!
//! ## Overview
//!
//! This pallet enables:
//! - Registration of identity verifiers (with stake)
//! - Submission of identity verification proofs (including ZK proofs)
//! - Quorum-based verification with configurable thresholds
//! - Reward distribution for successful verifiers
//! - Slashing for malicious or incorrect verifiers
//! - Verification expiration and renewal
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │                  X3 IDENTITY VERIFIER PALLET                      │
//! │                                                                  │
//! │  ┌──────────────┐   ┌──────────────┐   ┌────────────────────┐   │
//! │  │  Verifier    │──▶│  Proof       │──▶│  Quorum            │   │
//! │  │  Registry    │   │  Submission  │   │  Verification      │   │
//! │  └──────────────┘   └──────────────┘   └────────────────────┘   │
//! │        │                    │                    │               │
//! │        ▼                    ▼                    ▼               │
//! │  ┌──────────────┐   ┌──────────────┐   ┌────────────────────┐   │
//! │  │ Registered   │   │ Pending      │   │ Verified           │   │
//! │  │ Verifiers    │   │ Proofs       │   │ Identities         │   │
//! │  └──────────────┘   └──────────────┘   └────────────────────┘   │
//! └──────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Key Concepts
//!
//! - **Verifier**: An account registered to verify identity proofs. Must stake
//!   a minimum amount to participate.
//! - **Identity Proof**: A cryptographic proof (potentially ZK-based) that
//!   demonstrates ownership of an identity attribute.
//! - **Quorum**: A minimum number of verifier confirmations required before
//!   an identity is considered verified.
//! - **Verification Expiry**: Verified identities expire after a configurable
//!   period, requiring renewal.

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

use x3_primitives::keyring::{KeyringVerifier, KeyringAttestorRegistry};

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, Get, ReservableCurrency, StorageVersion},
    };
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_runtime::traits::Saturating;
    use sp_std::vec::Vec;

    /// Current storage version
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    // ============================================================================
    // Types
    // ============================================================================

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// Verifier identifier (32 bytes)
    pub type VerifierId = H256;

    /// Proof identifier (32 bytes)
    pub type ProofId = H256;

    /// Identity identifier (32 bytes)
    pub type IdentityId = H256;

    /// Verification status
    #[derive(
        Clone,
        Encode,
        Decode,
        DecodeWithMemTracking,
        TypeInfo,
        MaxEncodedLen,
        Debug,
        PartialEq,
        Eq,
        Default,
    )]
    pub enum VerificationStatus {
        #[default]
        Pending,
        Verified,
        Rejected,
        Expired,
    }

    /// Identity proof type
    #[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Debug)]
    #[scale_info(skip_type_params(T))]
    pub enum ProofType {
        /// Standard signature-based proof
        Signature,
        /// Zero-knowledge proof
        ZKProof,
        /// Multi-factor proof combining multiple methods
        MultiFactor,
    }

    /// Registered identity verifier
    #[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Debug)]
    #[scale_info(skip_type_params(T))]
    pub struct VerifierRecord<T: Config> {
        /// Verifier account
        pub account: T::AccountId,
        /// Stake amount
        pub stake: BalanceOf<T>,
        /// Total proofs verified
        pub proofs_verified: u64,
        /// Total proofs rejected
        pub proofs_rejected: u64,
        /// Reputation score (0-100)
        pub reputation: u8,
        /// Is active
        pub active: bool,
    }

    /// Identity verification proof
    #[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Debug)]
    #[scale_info(skip_type_params(T))]
    pub struct IdentityProof<T: Config> {
        /// Unique proof identifier
        pub proof_id: ProofId,
        /// Identity being verified
        pub identity_id: IdentityId,
        /// Verifier who submitted the proof
        pub verifier: T::AccountId,
        /// Proof type
        pub proof_type: ProofType,
        /// Proof data (signature, ZK proof, challenge response, etc.)
        pub proof_data: BoundedVec<u8, T::MaxProofSize>,
        /// Associated verification hash
        pub verification_hash: H256,
        /// Block when proof was submitted
        pub submitted_at: BlockNumberFor<T>,
        /// Verification status
        pub status: VerificationStatus,
        /// Number of confirmations received
        pub confirmations: u32,
    }

    /// Identity verification result
    #[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq)]
    pub struct VerificationResult {
        /// Whether the identity was verified
        pub verified: bool,
        /// The verified identity identifier
        pub identity_id: IdentityId,
        /// Number of verifiers who confirmed
        pub confirmation_count: u32,
        /// Timestamp of verification
        pub timestamp: u64,
        /// Proof type used
        pub proof_type: ProofType,
    }

    // ============================================================================
    // Config
    // ============================================================================

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Runtime event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency for rewards and staking
        type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

        /// Origin that can register verifiers
        type VerifierRegistrar: EnsureOrigin<Self::RuntimeOrigin>;

        /// Minimum stake to become a verifier
        #[pallet::constant]
        type MinVerifierStake: Get<BalanceOf<Self>>;

        /// Maximum size of proof data
        #[pallet::constant]
        type MaxProofSize: Get<u32>;

        /// Maximum identity data size
        #[pallet::constant]
        type MaxIdentityDataSize: Get<u32>;

        /// Maximum identities per verifier
        #[pallet::constant]
        type MaxIdentitiesPerVerifier: Get<u32>;

        /// Minimum number of confirmations for verification
        #[pallet::constant]
        type MinConfirmations: Get<u32>;

        /// Maximum confirmations per proof
        #[pallet::constant]
        type MaxConfirmations: Get<u32>;

        /// Proof timeout in blocks
        #[pallet::constant]
        type ProofTimeout: Get<BlockNumberFor<Self>>;

        /// Reward amount per successful verification
        #[pallet::constant]
        type VerificationReward: Get<BalanceOf<Self>>;

        /// Slash amount for failed verification
        #[pallet::constant]
        type VerificationSlashAmount: Get<BalanceOf<Self>>;

        /// Identity expiry duration in blocks
        #[pallet::constant]
        type IdentityExpiryBlocks: Get<BlockNumberFor<Self>>;

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

    /// Registered verifiers
    #[pallet::storage]
    #[pallet::getter(fn verifiers)]
    pub type Verifiers<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, VerifierRecord<T>, OptionQuery>;

    /// Active verifiers list
    #[pallet::storage]
    #[pallet::getter(fn active_verifiers)]
    pub type ActiveVerifiers<T: Config> =
        StorageValue<_, BoundedVec<T::AccountId, T::MaxIdentitiesPerVerifier>, ValueQuery>;

    /// Pending identity proofs
    #[pallet::storage]
    #[pallet::getter(fn identity_proofs)]
    pub type IdentityProofs<T: Config> =
        StorageMap<_, Blake2_128Concat, ProofId, IdentityProof<T>, OptionQuery>;

    /// Verified identities (identity_id -> verification result)
    #[pallet::storage]
    #[pallet::getter(fn verified_identities)]
    pub type VerifiedIdentities<T: Config> =
        StorageMap<_, Blake2_128Concat, IdentityId, VerificationResult, OptionQuery>;

    /// Identity proof confirmations (proof_id -> set of verifier accounts who confirmed)
    #[pallet::storage]
    #[pallet::getter(fn proof_confirmations)]
    pub type ProofConfirmations<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ProofId,
        Blake2_128Concat,
        T::AccountId,
        bool,
        ValueQuery,
    >;

    /// Total verifiers registered
    #[pallet::storage]
    pub type TotalVerifiers<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Total proofs submitted
    #[pallet::storage]
    pub type TotalProofsSubmitted<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Total proofs verified
    #[pallet::storage]
    pub type TotalProofsVerified<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Protocol treasury for collected fees/slashes
    #[pallet::storage]
    pub type ProtocolTreasury<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    // ============================================================================
    // Genesis
    // ============================================================================

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        pub initial_verifiers: Vec<T::AccountId>,
        pub initial_stake: BalanceOf<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            for verifier in &self.initial_verifiers {
                let record = VerifierRecord {
                    account: verifier.clone(),
                    stake: self.initial_stake,
                    proofs_verified: 0,
                    proofs_rejected: 0,
                    reputation: 50,
                    active: true,
                };
                Verifiers::<T>::insert(verifier, record);
            }

            // Initialize active verifiers list
            let _ = ActiveVerifiers::<T>::try_append(|_| BoundedVec::new());
        }
    }

    // ============================================================================
    // Events
    // ============================================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New verifier registered
        VerifierRegistered {
            verifier: T::AccountId,
            stake: BalanceOf<T>,
        },
        /// Verifier deactivated
        VerifierDeactivated {
            verifier: T::AccountId,
        },
        /// Verifier reactivated
        VerifierReactivated {
            verifier: T::AccountId,
        },
        /// Identity proof submitted
        IdentityProofSubmitted {
            proof_id: ProofId,
            identity_id: IdentityId,
            verifier: T::AccountId,
            proof_type: ProofType,
        },
        /// Identity proof confirmed by another verifier
        IdentityProofConfirmed {
            proof_id: ProofId,
            identity_id: IdentityId,
            confirmator: T::AccountId,
            total_confirmations: u32,
        },
        /// Identity verification completed
        IdentityVerified {
            identity_id: IdentityId,
            proof_id: ProofId,
            result: VerificationResult,
        },
        /// Identity proof rejected
        IdentityProofRejected {
            proof_id: ProofId,
            identity_id: IdentityId,
            reason: BoundedVec<u8, ConstU32<128>>,
        },
        /// Verifier slashed for malicious behavior
        VerifierSlashed {
            verifier: T::AccountId,
            amount: BalanceOf<T>,
        },
        /// Reward distributed to verifier
        RewardDistributed {
            verifier: T::AccountId,
            amount: BalanceOf<T>,
        },
        /// Identity expired and removed from verified set
        IdentityExpired {
            identity_id: IdentityId,
        },
    }

    // ============================================================================
    // Errors
    // ============================================================================

    #[pallet::error]
    pub enum Error<T> {
        /// Verifier not found
        VerifierNotFound,
        /// Verifier already registered
        VerifierAlreadyRegistered,
        /// Verifier not active
        VerifierNotActive,
        /// Insufficient stake
        InsufficientStake,
        /// Proof not found
        ProofNotFound,
        /// Proof already verified
        ProofAlreadyVerified,
        /// Proof already rejected
        ProofAlreadyRejected,
        /// Proof has expired
        ProofExpired,
        /// Identity already verified
        IdentityAlreadyVerified,
        /// Invalid proof format
        InvalidProofFormat,
        /// Verifier already confirmed this proof
        AlreadyConfirmed,
        /// Minimum confirmations not reached
        InsufficientConfirmations,
        /// No active verifiers
        NoActiveVerifiers,
    }

    // ============================================================================
    // Hooks
    // ============================================================================

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_finalize(block: BlockNumberFor<T>) {
            // Expire old proofs that have timed out
            Self::expire_old_proofs(block);
            // Expire old verified identities
            Self::expire_old_identities(block);
        }
    }

    // ============================================================================
    // Extrinsics
    // ============================================================================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register as an identity verifier (requires stake)
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_verifier())]
        pub fn register_verifier(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(
                !Verifiers::<T>::contains_key(&who),
                Error::<T>::VerifierAlreadyRegistered
            );

            let stake = T::MinVerifierStake::get();
            T::Currency::reserve(&who, stake)?;

            let record = VerifierRecord {
                account: who.clone(),
                stake,
                proofs_verified: 0,
                proofs_rejected: 0,
                reputation: 50,
                active: true,
            };

            Verifiers::<T>::insert(&who, record);
            TotalVerifiers::<T>::mutate(|n| *n = n.saturating_add(1));

            Self::deposit_event(Event::VerifierRegistered {
                verifier: who,
                stake,
            });

            Ok(())
        }

        /// Submit an identity verification proof
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::submit_identity_proof())]
        pub fn submit_identity_proof(
            origin: OriginFor<T>,
            identity_id: IdentityId,
            proof_type: ProofType,
            proof_data: BoundedVec<u8, T::MaxProofSize>,
            verification_hash: H256,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify verifier is registered and active
            let verifier_record =
                Verifiers::<T>::get(&who).ok_or(Error::<T>::VerifierNotFound)?;
            ensure!(verifier_record.active, Error::<T>::VerifierNotActive);

            // Check if identity is already verified
            ensure!(
                !VerifiedIdentities::<T>::contains_key(&identity_id),
                Error::<T>::IdentityAlreadyVerified
            );

            // Generate proof ID
            let proof_id = Self::generate_proof_id(&who, &identity_id, &verification_hash);

            ensure!(
                !IdentityProofs::<T>::contains_key(&proof_id),
                Error::<T>::ProofNotFound
            );

            let current_block = frame_system::Pallet::<T>::block_number();

            let proof = IdentityProof {
                proof_id,
                identity_id,
                verifier: who.clone(),
                proof_type,
                proof_data,
                verification_hash,
                submitted_at: current_block,
                status: VerificationStatus::Pending,
                confirmations: 0,
            };

            IdentityProofs::<T>::insert(&proof_id, proof);
            TotalProofsSubmitted::<T>::mutate(|n| *n = n.saturating_add(1));

            Self::deposit_event(Event::IdentityProofSubmitted {
                proof_id,
                identity_id,
                verifier: who,
                proof_type,
            });

            Ok(())
        }

        /// Confirm an identity proof (attest that the proof is valid)
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::confirm_identity_proof())]
        pub fn confirm_identity_proof(
            origin: OriginFor<T>,
            proof_id: ProofId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify verifier is registered and active
            ensure!(
                Verifiers::<T>::contains_key(&who),
                Error::<T>::VerifierNotFound
            );
            let verifier_record = Verifiers::<T>::get(&who).unwrap();
            ensure!(verifier_record.active, Error::<T>::VerifierNotActive);

            // Verify proof exists and is pending
            IdentityProofs::<try_mutate>(
                proof_id,
                |maybe_proof| -> DispatchResult {
                    let proof = maybe_proof.as_mut().ok_or(Error::<T>::ProofNotFound)?;

                    ensure!(
                        proof.status == VerificationStatus::Pending,
                        Error::<T>::ProofAlreadyVerified
                    );

                    // Check if this verifier already confirmed
                    ensure!(
                        !ProofConfirmations::<T>::contains_key(&proof_id, &who),
                        Error::<T>::AlreadyConfirmed
                    );

                    // Record confirmation
                    ProofConfirmations::<T>::insert(&proof_id, &who, true);
                    proof.confirmations = proof.confirmations.saturating_add(1);

                    // Check if quorum reached
                    let min_confirmations = T::MinConfirmations::get();
                    if proof.confirmations >= min_confirmations {
                        // Mark proof as verified
                        proof.status = VerificationStatus::Verified;

                        // Store verified identity
                        let verification_result = VerificationResult {
                            verified: true,
                            identity_id: proof.identity_id,
                            confirmation_count: proof.confirmations,
                            timestamp: sp_io::offchain::timestamp().unix_millis() as u64,
                            proof_type: proof.proof_type.clone(),
                        };
                        VerifiedIdentities::<T>::insert(&proof.identity_id, &verification_result);

                        // Update verifier stats and reward
                        Self::reward_verifier(&proof.verifier, &proof.identity_id)?;

                        // Update proof submitter stats
                        Verifiers::<T>::try_mutate(&proof.verifier, |maybe_record| {
                            if let Some(record) = maybe_record {
                                record.proofs_verified = record.proofs_verified.saturating_add(1);
                            }
                            Ok::<(), Error<T>>(())
                        })?;

                        Self::deposit_event(Event::IdentityVerified {
                            identity_id: proof.identity_id,
                            proof_id,
                            result: verification_result,
                        });
                    } else {
                        Self::deposit_event(Event::IdentityProofConfirmed {
                            proof_id,
                            identity_id: proof.identity_id,
                            confirmator: who.clone(),
                            total_confirmations: proof.confirmations,
                        });
                    }

                    Ok(())
                },
            )
        }

        /// Reject an identity proof
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::reject_identity_proof())]
        pub fn reject_identity_proof(
            origin: OriginFor<T>,
            proof_id: ProofId,
            reason: BoundedVec<u8, ConstU32<128>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify verifier is registered and active
            ensure!(
                Verifiers::<T>::contains_key(&who),
                Error::<T>::VerifierNotFound
            );
            let verifier_record = Verifiers::<T>::get(&who).unwrap();
            ensure!(verifier_record.active, Error::<T>::VerifierNotActive);

            IdentityProofs::<try_mutate>(proof_id, |maybe_proof| -> DispatchResult {
                let proof = maybe_proof.as_mut().ok_or(Error::<T>::ProofNotFound)?;

                ensure!(
                    proof.status == VerificationStatus::Pending,
                    Error::<T>::ProofAlreadyVerified
                );

                proof.status = VerificationStatus::Rejected;

                // Slash the submitter for invalid proof
                Self::slash_verifier(&proof.verifier)?;

                // Update submitter stats
                Verifiers::<T>::try_mutate(&proof.verifier, |maybe_record| {
                    if let Some(record) = maybe_record {
                        record.proofs_rejected = record.proofs_rejected.saturating_add(1);
                    }
                    Ok::<(), Error<T>>(())
                })?;

                Self::deposit_event(Event::IdentityProofRejected {
                    proof_id,
                    identity_id: proof.identity_id,
                    reason,
                });

                Ok(())
            })
        }

        /// Deactivate a verifier
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::deactivate_verifier())]
        pub fn deactivate_verifier(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            Verifiers::<T>::try_mutate(&who, |maybe_record| -> DispatchResult {
                let record = maybe_record.as_mut().ok_or(Error::<T>::VerifierNotFound)?;
                ensure!(record.active, Error::<T>::VerifierNotActive);

                record.active = false;
                // Unreserve stake
                T::Currency::unreserve(&who, record.stake);

                Self::deposit_event(Event::VerifierDeactivated { verifier: who });
                Ok(())
            })
        }

        /// Reactivate a deactivated verifier
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::reactivate_verifier())]
        pub fn reactivate_verifier(
            origin: OriginFor<T>,
            verifier: T::AccountId,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            let registrar = ensure_signed(origin)?;
            let _ = registrar;

            // Ensure caller has registrar permissions
            T::VerifierRegistrar::ensure_origin(origin)?;

            Verifiers::<T>::try_mutate(&verifier, |maybe_record| -> DispatchResult {
                let record = maybe_record.as_mut().ok_or(Error::<T>::VerifierNotFound)?;
                ensure!(!record.active, Error::<T>::VerifierNotActive);

                record.active = true;
                // Re-reserve stake
                T::Currency::reserve(&verifier, record.stake)?;

                Self::deposit_event(Event::VerifierReactivated { verifier });
                Ok(())
            })
        }

        /// Renew a verified identity before it expires
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::renew_identity())]
        pub fn renew_identity(
            origin: OriginFor<T>,
            identity_id: IdentityId,
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;

            // Verify identity exists and is verified
            let mut result = VerifiedIdentities::<T>::get(&identity_id)
                .ok_or(Error::<T>::ProofNotFound)?;

            // Reset timestamp to extend expiry
            result.timestamp = sp_io::offchain::timestamp().unix_millis() as u64;
            VerifiedIdentities::<T>::insert(&identity_id, &result);

            Ok(())
        }
    }

    // ============================================================================
    // Internal Functions
    // ============================================================================

    impl<T: Config> Pallet<T> {
        /// Generate unique proof ID
        fn generate_proof_id(
            verifier: &T::AccountId,
            identity_id: &IdentityId,
            verification_hash: &H256,
        ) -> ProofId {
            let block = frame_system::Pallet::<T>::block_number();
            let mut data = verifier.encode();
            data.extend(identity_id.as_bytes());
            data.extend(verification_hash.as_bytes());
            data.extend(block.encode());
            H256::from(blake2_256(&data))
        }

        /// Reward a verifier for successful verification
        fn reward_verifier(
            verifier: &T::AccountId,
            identity_id: &IdentityId,
        ) -> DispatchResult {
            let reward = T::VerificationReward::get();

            // Transfer reward from protocol treasury
            ProtocolTreasury::<T>::try_mutate(|treasury| {
                if *treasury >= reward {
                    *treasury = treasury.saturating_sub(reward);
                    Ok(())
                } else {
                    Err(())
                }
            })
            .map_err(|_| Error::<T>::VerifierNotFound)?;

            T::Currency::transfer(
                &Self::account_id(),
                verifier,
                reward,
                frame_support::traits::ExistenceRequirement::KeepAlive,
            )?;

            // Update verifier stats
            Verifiers::<T>::try_mutate(verifier, |maybe_record| {
                if let Some(record) = maybe_record.as_mut() {
                    record.reputation = record.reputation.saturating_add(5).min(100);
                }
            });

            Self::deposit_event(Event::RewardDistributed {
                verifier: verifier.clone(),
                amount: reward,
            });

            Ok(())
        }

        /// Slash a verifier for malicious behavior
        fn slash_verifier(verifier: &T::AccountId) -> DispatchResult {
            let slash_amount = T::VerificationSlashAmount::get();

            Verifiers::<T>::try_mutate(verifier, |maybe_record| -> DispatchResult {
                let record = maybe_record.as_mut().ok_or(Error::<T>::VerifierNotFound)?;

                let actual_slash = slash_amount.min(record.stake);
                let _ = T::Currency::slash_reserved(verifier, actual_slash);
                record.stake = record.stake.saturating_sub(actual_slash);
                record.reputation = record.reputation.saturating_sub(20);

                // Deactivate if stake too low
                if record.stake < T::MinVerifierStake::get() {
                    record.active = false;
                }

                Self::deposit_event(Event::VerifierSlashed {
                    verifier: verifier.clone(),
                    amount: actual_slash,
                });

                Ok(())
            })
        }

        /// Expire proofs that have timed out
        fn expire_old_proofs(current_block: BlockNumberFor<T>) {
            let timeout = T::ProofTimeout::get();

            IdentityProofs::<T>::iter().for_each(|(proof_id, mut proof)| {
                if proof.status == VerificationStatus::Pending {
                    if current_block.saturating_sub(proof.submitted_at) >= timeout {
                        proof.status = VerificationStatus::Expired;
                        IdentityProofs::<T>::insert(&proof_id, proof);

                        // Return any reserved funds
                        let _ = T::Currency::unreserve(
                            &proof.verifier,
                            T::MinVerifierStake::get(),
                        );
                    }
                }
            });
        }

        /// Expire old verified identities
        fn expire_old_identities(current_block: BlockNumberFor<T>) {
            let expiry_blocks = T::IdentityExpiryBlocks::get();

            VerifiedIdentities::<T>::iter()
                .filter(|(_, result)| result.verified)
                .for_each(|(identity_id, result)| {
                    // Calculate the block at which verification was completed
                    // We approximate using the timestamp and current block
                    let verification_block =
                        BlockNumberFor::<T>::saturated_from::<u64>(result.timestamp);
                    if current_block.saturating_sub(verification_block) >= expiry_blocks {
                        // Remove expired identity
                        VerifiedIdentities::<T>::remove(&identity_id);

                        Self::deposit_event(Event::IdentityExpired { identity_id });
                    }
                });
        }

        /// Get the pallet's account ID (treasury)
        pub fn account_id() -> T::AccountId {
            T::PalletId::get().into_account_truncating()
        }

        /// Check if an identity is verified
        pub fn is_identity_verified(identity_id: &IdentityId) -> bool {
            VerifiedIdentities::<T>::contains_key(identity_id)
        }

        /// Get verification result for an identity
        pub fn get_verification(identity_id: &IdentityId) -> Option<VerificationResult> {
            VerifiedIdentities::<T>::get(identity_id)
        }

        /// Get verifier record
        pub fn get_verifier(verifier: &T::AccountId) -> Option<VerifierRecord<T>> {
            Verifiers::<T>::get(verifier)
        }

        /// Get proof by ID
        pub fn get_proof(proof_id: &ProofId) -> Option<IdentityProof<T>> {
            IdentityProofs::<T>::get(proof_id)
        }
    }

    // ============================================================================
    // KeyringVerifier Trait Implementation
    // ============================================================================

    /// Implement the `KeyringVerifier` trait from x3-primitives so that the
    /// runtime can query identity verification status through a unified interface.
    impl<T: Config> KeyringVerifier<T::AccountId, BlockNumberFor<T>> for Pallet<T> {
        fn verify_keyring(keyring_id: &[u8; 32]) -> bool {
            let identity_id = IdentityId::from_slice(keyring_id);
            Self::is_identity_verified(&identity_id)
        }

        fn is_attestor(account: &T::AccountId) -> bool {
            Verifiers::<T>::contains_key(account)
        }

        fn active_attestor_count() -> u32 {
            // Count active verifiers
            Verifiers::<T>::iter()
                .filter(|(_, record)| record.active)
                .count() as u32
        }
    }

    // ============================================================================
    // KeyringAttestorRegistry Trait Implementation
    // ============================================================================

    /// Implement the `KeyringAttestorRegistry` trait from x3-primitives so that
    /// the runtime can manage verifier registration through a unified interface.
    impl<T: Config> KeyringAttestorRegistry<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>
        for Pallet<T>
    {
        fn register_attestor(account: &T::AccountId, stake: BalanceOf<T>) -> DispatchResult {
            ensure!(
                !Verifiers::<T>::contains_key(account),
                Error::<T>::VerifierAlreadyRegistered
            );

            ensure!(
                stake >= T::MinVerifierStake::get(),
                Error::<T>::InsufficientStake
            );

            T::Currency::reserve(account, stake)?;

            let record = VerifierRecord {
                account: account.clone(),
                stake,
                proofs_verified: 0,
                proofs_rejected: 0,
                reputation: 50,
                active: true,
            };

            Verifiers::<T>::insert(account, record);
            TotalVerifiers::<T>::mutate(|n| *n = n.saturating_add(1));

            Self::deposit_event(Event::VerifierRegistered {
                verifier: account.clone(),
                stake,
            });

            Ok(())
        }

        fn deregister_attestor(account: &T::AccountId) -> DispatchResult {
            Verifiers::<T>::try_mutate(account, |maybe_record| -> DispatchResult {
                let record = maybe_record.as_mut().ok_or(Error::<T>::VerifierNotFound)?;

                // Return reserved stake
                T::Currency::unreserve(account, record.stake);

                *maybe_record = None;
                TotalVerifiers::<T>::mutate(|n| *n = n.saturating_sub(1));

                Self::deposit_event(Event::VerifierDeactivated {
                    verifier: account.clone(),
                });

                Ok(())
            })
        }

        fn is_registered(account: &T::AccountId) -> bool {
            Verifiers::<T>::contains_key(account)
        }

        fn total_attestors() -> u64 {
            TotalVerifiers::<T>::get()
        }
    }
}