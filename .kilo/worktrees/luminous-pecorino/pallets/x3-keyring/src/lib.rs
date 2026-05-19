#![deny(unsafe_code)]
//! # X3 Keyring Pallet
//!
//! The X3 Keyring pallet provides on-chain keyring verification for cross-VM
//! operations. It manages a registry of verified keyring attestors and enforces
//! a quorum-based verification model for keyring proofs.
//!
//! ## Overview
//!
//! This pallet enables:
//! - Registration of keyring attestors (with stake)
//! - Submission of keyring verification proofs
//! - Quorum-based verification with configurable thresholds
//! - Reward distribution for successful verifiers
//! - Slashing for malicious or incorrect attestors
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     X3 KEYRING PALLET                        │
//! │                                                             │
//! │  ┌──────────────┐   ┌──────────────┐   ┌────────────────┐  │
//! │  │  Attestor    │──▶│  Proof       │──▶│  Quorum        │  │
//! │  │  Registry    │   │  Submission  │   │  Verification  │  │
//! │  └──────────────┘   └──────────────┘   └────────────────┘  │
//! │         │                    │                    │          │
//! │         ▼                    ▼                    ▼          │
//! │  ┌──────────────┐   ┌──────────────┐   ┌────────────────┐  │
//! │  │ Registered   │   │ Pending      │   │ Verified       │  │
//! │  │ Attestors    │   │ Proofs       │   │ Keyrings       │  │
//! │  └──────────────┘   └──────────────┘   └────────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//! ```

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

    /// Attestor identifier (32 bytes)
    pub type AttestorId = H256;

    /// Proof identifier (32 bytes)
    pub type ProofId = H256;

    /// Keyring identifier (32 bytes)
    pub type KeyringId = H256;

    /// Agent identifier
    pub type AgentId = u32;

    /// Proof verification status
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
    pub enum ProofStatus {
        #[default]
        Pending,
        Verified,
        Rejected,
        Expired,
    }

    /// Registered keyring attestor
    #[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Debug)]
    #[scale_info(skip_type_params(T))]
    pub struct AttestorRecord<T: Config> {
        /// Attestor account
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

    /// Keyring verification proof
    #[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Debug)]
    #[scale_info(skip_type_params(T))]
    pub struct KeyringProof<T: Config> {
        /// Unique proof identifier
        pub proof_id: ProofId,
        /// Keyring being verified
        pub keyring_id: KeyringId,
        /// Attestor who submitted the proof
        pub attestor: T::AccountId,
        /// Proof data (signature, challenge response, etc.)
        pub proof_data: BoundedVec<u8, T::MaxProofSize>,
        /// Associated attestation hash
        pub attestation_hash: H256,
        /// Block when proof was submitted
        pub submitted_at: BlockNumberFor<T>,
        /// Verification status
        pub status: ProofStatus,
        /// Number of confirmations received
        pub confirmations: u32,
    }

    /// Keyring verification result
    #[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq)]
    pub struct VerificationResult {
        /// Whether the keyring was verified
        pub verified: bool,
        /// The verified keyring identifier
        pub keyring_id: KeyringId,
        /// Number of attestors who confirmed
        pub confirmation_count: u32,
        /// Timestamp of verification
        pub timestamp: u64,
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

        /// Origin that can register attestors
        type AttestorRegistrar: EnsureOrigin<Self::RuntimeOrigin>;

        /// Minimum stake to become an attestor
        #[pallet::constant]
        type MinAttestorStake: Get<BalanceOf<Self>>;

        /// Maximum size of proof data
        #[pallet::constant]
        type MaxProofSize: Get<u32>;

        /// Maximum keyring data size
        #[pallet::constant]
        type MaxKeyringSize: Get<u32>;

        /// Maximum keyring data size
        #[pallet::constant]
        type MaxKeyringsPerAttestor: Get<u32>;

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
        type AttestationSlashAmount: Get<BalanceOf<Self>>;

        /// Maximum number of agents to verify in a single batch
        #[pallet::constant]
        type MaxAgentsPerVerify: Get<u32>;

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

    /// Registered attestors
    #[pallet::storage]
    #[pallet::getter(fn attestors)]
    pub type Attestors<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, AttestorRecord<T>, OptionQuery>;

    /// Active attestors list
    #[pallet::storage]
    #[pallet::getter(fn active_attestors)]
    pub type ActiveAttestors<T: Config> =
        StorageValue<_, BoundedVec<T::AccountId, T::MaxKeyringsPerAttestor>, ValueQuery>;

    /// Pending keyring proofs
    #[pallet::storage]
    #[pallet::getter(fn keyring_proofs)]
    pub type KeyringProofs<T: Config> =
        StorageMap<_, Blake2_128Concat, ProofId, KeyringProof<T>, OptionQuery>;

    /// Verified keyrings (keyring_id -> verification result)
    #[pallet::storage]
    #[pallet::getter(fn verified_keyrings)]
    pub type VerifiedKeyrings<T: Config> =
        StorageMap<_, Blake2_128Concat, KeyringId, VerificationResult, OptionQuery>;

    /// Keyring proof confirmations (proof_id -> set of attestor accounts who confirmed)
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

    /// Total attestors registered
    #[pallet::storage]
    pub type TotalAttestors<T: Config> = StorageValue<_, u64, ValueQuery>;

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
        pub initial_attestors: Vec<T::AccountId>,
        pub initial_stake: BalanceOf<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            for attestor in &self.initial_attestors {
                let record = AttestorRecord {
                    account: attestor.clone(),
                    stake: self.initial_stake,
                    proofs_verified: 0,
                    proofs_rejected: 0,
                    reputation: 50,
                    active: true,
                };
                Attestors::<T>::insert(attestor, record);
            }

            // Initialize active attestors list
            let _ = ActiveAttestors::<T>::try_append(|_| BoundedVec::new());
        }
    }

    // ============================================================================
    // Events
    // ============================================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New attestor registered
        AttestorRegistered {
            attestor: T::AccountId,
            stake: BalanceOf<T>,
        },
        /// Attestor deactivated
        AttestorDeactivated {
            attestor: T::AccountId,
        },
        /// Attestor reactivated
        AttestorReactivated {
            attestor: T::AccountId,
        },
        /// Keyring proof submitted
        KeyringProofSubmitted {
            proof_id: ProofId,
            keyring_id: KeyringId,
            attestor: T::AccountId,
        },
        /// Keyring proof confirmed by another attestor
        KeyringProofConfirmed {
            proof_id: ProofId,
            keyring_id: KeyringId,
            confirmator: T::AccountId,
            total_confirmations: u32,
        },
        /// Keyring verification completed
        KeyringVerified {
            keyring_id: KeyringId,
            proof_id: ProofId,
            result: VerificationResult,
        },
        /// Keyring proof rejected
        KeyringProofRejected {
            proof_id: ProofId,
            keyring_id: KeyringId,
            reason: BoundedVec<u8, ConstU32<128>>,
        },
        /// Attestor slashed for malicious behavior
        AttestorSlashed {
            attestor: T::AccountId,
            amount: BalanceOf<T>,
        },
        /// Reward distributed to attestor
        RewardDistributed {
            attestor: T::AccountId,
            amount: BalanceOf<T>,
        },
        /// Multiple agents verified in a single batch
        AgentsVerified {
            agent_ids: BoundedVec<AgentId, T::MaxKeyringsPerAttestor>,
            attestor: T::AccountId,
            count: u32,
        },
    }

    // ============================================================================
    // Errors
    // ============================================================================

    #[pallet::error]
    pub enum Error<T> {
        /// Attestor not found
        AttestorNotFound,
        /// Attestor already registered
        AttestorAlreadyRegistered,
        /// Attestor not active
        AttestorNotActive,
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
        /// Keyring already verified
        KeyringAlreadyVerified,
        /// Invalid proof format
        InvalidProofFormat,
        /// Attestor already confirmed this proof
        AlreadyConfirmed,
        /// Minimum confirmations not reached
        InsufficientConfirmations,
        /// No active attestors
        NoActiveAttestors,
    }

    // ============================================================================
    // Hooks
    // ============================================================================

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_finalize(block: BlockNumberFor<T>) {
            // Expire old proofs that have timed out
            Self::expire_old_proofs(block);
        }
    }

    // ============================================================================
    // Extrinsics
    // ============================================================================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register as a keyring attestor (requires stake)
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_attestor())]
        pub fn register_attestor(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(
                !Attestors::<T>::contains_key(&who),
                Error::<T>::AttestorAlreadyRegistered
            );

            let stake = T::MinAttestorStake::get();
            T::Currency::reserve(&who, stake)?;

            let record = AttestorRecord {
                account: who.clone(),
                stake,
                proofs_verified: 0,
                proofs_rejected: 0,
                reputation: 50,
                active: true,
            };

            Attestors::<T>::insert(&who, record);
            TotalAttestors::<T>::mutate(|n| *n = n.saturating_add(1));

            Self::deposit_event(Event::AttestorRegistered {
                attestor: who,
                stake,
            });

            Ok(())
        }

        /// Submit a keyring verification proof
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::submit_keyring_proof())]
        pub fn submit_keyring_proof(
            origin: OriginFor<T>,
            keyring_id: KeyringId,
            proof_data: BoundedVec<u8, T::MaxProofSize>,
            attestation_hash: H256,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify attestor is registered and active
            let attestor_record =
                Attestors::<T>::get(&who).ok_or(Error::<T>::AttestorNotFound)?;
            ensure!(attestor_record.active, Error::<T>::AttestorNotActive);

            // Generate proof ID
            let proof_id = Self::generate_proof_id(&who, &keyring_id, &attestation_hash);

            ensure!(
                !KeyringProofs::<T>::contains_key(&proof_id),
                Error::<T>::ProofNotFound
            );

            let current_block = frame_system::Pallet::<T>::block_number();

            let proof = KeyringProof {
                proof_id,
                keyring_id,
                attestor: who.clone(),
                proof_data,
                attestation_hash,
                submitted_at: current_block,
                status: ProofStatus::Pending,
                confirmations: 0,
            };

            KeyringProofs::<T>::insert(&proof_id, proof);
            TotalProofsSubmitted::<T>::mutate(|n| *n = n.saturating_add(1));

            Self::deposit_event(Event::KeyringProofSubmitted {
                proof_id,
                keyring_id,
                attestor: who,
            });

            Ok(())
        }

        /// Confirm a keyring proof (attest that the proof is valid)
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::confirm_keyring_proof())]
        pub fn confirm_keyring_proof(
            origin: OriginFor<T>,
            proof_id: ProofId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify attestor is registered and active
            ensure!(
                Attestors::<T>::contains_key(&who),
                Error::<T>::AttestorNotFound
            );
            let attestor_record = Attestors::<T>::get(&who).unwrap();
            ensure!(attestor_record.active, Error::<T>::AttestorNotActive);

            // Verify proof exists and is pending
            KeyringProofs::<T, _>::try_mutate(
                proof_id,
                |maybe_proof| -> DispatchResult {
                    let proof = maybe_proof.as_mut().ok_or(Error::<T>::ProofNotFound)?;

                    ensure!(
                        proof.status == ProofStatus::Pending,
                        Error::<T>::ProofAlreadyVerified
                    );

                    // Check if this attestor already confirmed
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
                        proof.status = ProofStatus::Verified;

                        // Store verified keyring
                        let verification_result = VerificationResult {
                            verified: true,
                            keyring_id: proof.keyring_id,
                            confirmation_count: proof.confirmations,
                            timestamp: sp_io::offchain::timestamp().unix_millis() as u64,
                        };
                        VerifiedKeyrings::<T>::insert(&proof.keyring_id, &verification_result);

                        // Update attestor stats and reward
                        Self::reward_attestor(&proof.attestor, &proof.keyring_id)?;

                        // Update proof submitter stats
                        Attestors::<T>::try_mutate(&proof.attestor, |maybe_record| {
                            if let Some(record) = maybe_record {
                                record.proofs_verified = record.proofs_verified.saturating_add(1);
                            }
                            Ok::<(), Error<T>>(())
                        })?;

                        Self::deposit_event(Event::KeyringVerified {
                            keyring_id: proof.keyring_id,
                            proof_id,
                            result: verification_result,
                        });
                    } else {
                        Self::deposit_event(Event::KeyringProofConfirmed {
                            proof_id,
                            keyring_id: proof.keyring_id,
                            confirmator: who.clone(),
                            total_confirmations: proof.confirmations,
                        });
                    }

                    Ok(())
                },
            )
        }

        /// Reject a keyring proof
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::reject_keyring_proof())]
        pub fn reject_keyring_proof(
            origin: OriginFor<T>,
            proof_id: ProofId,
            reason: BoundedVec<u8, ConstU32<128>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify attestor is registered and active
            ensure!(
                Attestors::<T>::contains_key(&who),
                Error::<T>::AttestorNotFound
            );
            let attestor_record = Attestors::<T>::get(&who).unwrap();
            ensure!(attestor_record.active, Error::<T>::AttestorNotActive);

            KeyringProofs::<T, _>::try_mutate(proof_id, |maybe_proof| -> DispatchResult {
                let proof = maybe_proof.as_mut().ok_or(Error::<T>::ProofNotFound)?;

                ensure!(
                    proof.status == ProofStatus::Pending,
                    Error::<T>::ProofAlreadyVerified
                );

                proof.status = ProofStatus::Rejected;

                // Slash the submitter for invalid proof
                Self::slash_attestor(&proof.attestor)?;

                // Update submitter stats
                Attestors::<T>::try_mutate(&proof.attestor, |maybe_record| {
                    if let Some(record) = maybe_record {
                        record.proofs_rejected = record.proofs_rejected.saturating_add(1);
                    }
                    Ok::<(), Error<T>>(())
                })?;

                Self::deposit_event(Event::KeyringProofRejected {
                    proof_id,
                    keyring_id: proof.keyring_id,
                    reason,
                });

                Ok(())
            })
        }

        /// Deactivate an attestor
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::deactivate_attestor())]
        pub fn deactivate_attestor(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            Attestors::<T>::try_mutate(&who, |maybe_record| -> DispatchResult {
                let record = maybe_record.as_mut().ok_or(Error::<T>::AttestorNotFound)?;
                ensure!(record.active, Error::<T>::AttestorNotActive);

                record.active = false;
                // Unreserve stake
                T::Currency::unreserve(&who, record.stake);

                Self::deposit_event(Event::AttestorDeactivated { attestor: who });
                Ok(())
            })
        }

        /// Reactivate a deactivated attestor
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::reactivate_attestor())]
        pub fn reactivate_attestor(
            origin: OriginFor<T>,
            attestor: T::AccountId,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            let registrar = ensure_signed(origin)?;
            let _ = registrar;

            // Ensure caller has registrar permissions
            T::AttestorRegistrar::ensure_origin(origin)?;

            Attestors::<T>::try_mutate(&attestor, |maybe_record| -> DispatchResult {
                let record = maybe_record.as_mut().ok_or(Error::<T>::AttestorNotFound)?;
                ensure!(!record.active, Error::<T>::AttestorNotActive);

                record.active = true;
                // Re-reserve stake
                T::Currency::reserve(&attestor, record.stake)?;

                Self::deposit_event(Event::AttestorReactivated { attestor });
                Ok(())
            })
        }

        /// Verify multiple agents' keyrings in a single batch transaction.
        ///
        /// This extrinsic allows a registered, active attestor to simultaneously
        /// verify multiple agents. For each agent ID provided, the pallet checks
        /// that the agent exists in the AgentAccounts pallet and that their
        /// keyring has been verified on-chain. This enables efficient batch
        /// verification of agents for use in the X3 network.
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::verify_agents(agent_ids.len() as u32))]
        pub fn verify_agents(
            origin: OriginFor<T>,
            agent_ids: BoundedVec<AgentId, T::MaxAgentsPerVerify>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify attestor is registered and active
            let attestor_record =
                Attestors::<T>::get(&who).ok_or(Error::<T>::AttestorNotFound)?;
            ensure!(attestor_record.active, Error::<T>::AttestorNotActive);

            // Ensure attestor has registered their keyring (keyring verification required)
            // Use the attestor's AccountId as keyring_id for self-verification check
            let keyring_id = KeyringId::from_slice(&who.encode()[..32]);
            ensure!(
                Self::is_keyring_verified(keyring_id),
                Error::<T>::ProofNotFound
            );

            // Ensure we don't exceed the limit
            ensure!(
                agent_ids.len() <= T::MaxAgentsPerVerify::get() as usize,
                Error::<T>::InvalidProofFormat
            );

            // Verify at least one agent
            ensure!(!agent_ids.is_empty(), Error::<T>::InvalidProofFormat);

            let mut verified_count: u32 = 0;
            let mut verified_agents: BoundedVec<AgentId, T::MaxAgentsPerVerify> =
                BoundedVec::new();

            for &agent_id in agent_ids.iter() {
                // Check agent exists in AgentAccounts pallet via runtime API
                if Self::agent_exists(agent_id) {
                    // Check agent's keyring is verified
                    // Derive keyring ID from agent_id
                    let agent_keyring_id = Self::agent_keyring_id(agent_id);
                    if Self::is_keyring_verified(&agent_keyring_id) {
                        verified_count = verified_count.saturating_add(1);
                        verified_agents
                            .try_push(agent_id)
                            .map_err(|_| Error::<T>::InvalidProofFormat)?;
                    }
                }
            }

            ensure!(verified_count > 0, Error::<T>::InvalidProofFormat);

            // Update attestor stats
            Attestors::<T>::try_mutate(&who, |maybe_record| {
                if let Some(record) = maybe_record.as_mut() {
                    record.proofs_verified = record.proofs_verified.saturating_add(verified_count);
                }
                Ok::<(), Error<T>>(())
            })?;

            // Reward the attestor for batch verification
            Self::reward_attestor_batch(&who, verified_count)?;

            Self::deposit_event(Event::AgentsVerified {
                agent_ids: verified_agents,
                attestor: who,
                count: verified_count,
            });

            Ok(())
        }
    }

    // ============================================================================
    // Internal Functions
    // ============================================================================

    impl<T: Config> Pallet<T> {
        /// Generate unique proof ID
        fn generate_proof_id(
            attestor: &T::AccountId,
            keyring_id: &KeyringId,
            attestation_hash: &H256,
        ) -> ProofId {
            let block = frame_system::Pallet::<T>::block_number();
            let mut data = attestor.encode();
            data.extend(keyring_id.as_bytes());
            data.extend(attestation_hash.as_bytes());
            data.extend(block.encode());
            H256::from(blake2_256(&data))
        }

        /// Reward an attestor for successful verification
        fn reward_attestor(
            attestor: &T::AccountId,
            keyring_id: &KeyringId,
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
            .map_err(|_| Error::<T>::AttestorNotFound)?;

            T::Currency::transfer(
                &Self::account_id(),
                attestor,
                reward,
                frame_support::traits::ExistenceRequirement::KeepAlive,
            )?;

            // Update attestor stats
            Attestors::<T>::try_mutate(attestor, |maybe_record| {
                if let Some(record) = maybe_record.as_mut() {
                    record.reputation = record.reputation.saturating_add(5).min(100);
                }
            });

            Self::deposit_event(Event::RewardDistributed {
                attestor: attestor.clone(),
                amount: reward,
            });

            Ok(())
        }

        /// Slash an attestor for malicious behavior
        fn slash_attestor(attestor: &T::AccountId) -> DispatchResult {
            let slash_amount = T::AttestationSlashAmount::get();

            Attestors::<T>::try_mutate(attestor, |maybe_record| -> DispatchResult {
                let record = maybe_record.as_mut().ok_or(Error::<T>::AttestorNotFound)?;

                let actual_slash = slash_amount.min(record.stake);
                let _ = T::Currency::slash_reserved(attestor, actual_slash);
                record.stake = record.stake.saturating_sub(actual_slash);
                record.reputation = record.reputation.saturating_sub(20);

                // Deactivate if stake too low
                if record.stake < T::MinAttestorStake::get() {
                    record.active = false;
                }

                Self::deposit_event(Event::AttestorSlashed {
                    attestor: attestor.clone(),
                    amount: actual_slash,
                });

                Ok(())
            })
        }

        /// Expire proofs that have timed out
        fn expire_old_proofs(current_block: BlockNumberFor<T>) {
            let timeout = T::ProofTimeout::get();

            KeyringProofs::<T>::iter().for_each(|(proof_id, mut proof)| {
                if proof.status == ProofStatus::Pending {
                    if current_block.saturating_sub(proof.submitted_at) >= timeout {
                        proof.status = ProofStatus::Expired;
                        KeyringProofs::<T>::insert(&proof_id, proof);

                        // Return any reserved funds
                        let _ = T::Currency::unreserve(&proof.attestor, T::MinAttestorStake::get());
                    }
                }
            });
        }

        /// Get the pallet's account ID (treasury)
        pub fn account_id() -> T::AccountId {
            T::PalletId::get().into_account_truncating()
        }

        /// Check if a keyring is verified
        pub fn is_keyring_verified(keyring_id: &KeyringId) -> bool {
            VerifiedKeyrings::<T>::contains_key(keyring_id)
        }

        /// Get verification result for a keyring
        pub fn get_verification(keyring_id: &KeyringId) -> Option<VerificationResult> {
            VerifiedKeyrings::<T>::get(keyring_id)
        }

        /// Get attestor record
        pub fn get_attestor(attestor: &T::AccountId) -> Option<AttestorRecord<T>> {
            Attestors::<T>::get(attestor)
        }

        /// Get proof by ID
        pub fn get_proof(proof_id: &ProofId) -> Option<KeyringProof<T>> {
            KeyringProofs::<T>::get(proof_id)
        }
    }

    // ============================================================================
    // KeyringVerifier Trait Implementation
    // ============================================================================

    /// Implement the `KeyringVerifier` trait from x3-primitives so that the
    /// runtime can query keyring verification status through a unified interface.
    impl<T: Config> KeyringVerifier<T::AccountId, BlockNumberFor<T>> for Pallet<T> {
        fn verify_keyring(keyring_id: &[u8; 32]) -> bool {
            let keyring_id = KeyringId::from_slice(keyring_id);
            Self::is_keyring_verified(&keyring_id)
        }

        fn is_attestor(account: &T::AccountId) -> bool {
            Attestors::<T>::contains_key(account)
        }

        fn active_attestor_count() -> u32 {
            ActiveAttestors::<T>::get().len() as u32
        }
    }

    // ============================================================================
    // KeyringAttestorRegistry Trait Implementation
    // ============================================================================

    /// Implement the `KeyringAttestorRegistry` trait from x3-primitives so that
    /// the runtime can manage attestor registration through a unified interface.
    impl<T: Config> KeyringAttestorRegistry<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>
        for Pallet<T>
    {
        fn register_attestor(account: &T::AccountId, stake: BalanceOf<T>) -> DispatchResult {
            ensure!(
                !Attestors::<T>::contains_key(account),
                Error::<T>::AttestorAlreadyRegistered
            );

            ensure!(
                stake >= T::MinAttestorStake::get(),
                Error::<T>::InsufficientStake
            );

            T::Currency::reserve(account, stake)?;

            let record = AttestorRecord {
                account: account.clone(),
                stake,
                proofs_verified: 0,
                proofs_rejected: 0,
                reputation: 50,
                active: true,
            };

            Attestors::<T>::insert(account, record);
            TotalAttestors::<T>::mutate(|n| *n = n.saturating_add(1));

            Self::deposit_event(Event::AttestorRegistered {
                attestor: account.clone(),
                stake,
            });

            Ok(())
        }

        fn deregister_attestor(account: &T::AccountId) -> DispatchResult {
            Attestors::<T>::try_mutate(account, |maybe_record| -> DispatchResult {
                let record = maybe_record.as_mut().ok_or(Error::<T>::AttestorNotFound)?;

                // Return reserved stake
                T::Currency::unreserve(account, record.stake);

                *maybe_record = None;
                TotalAttestors::<T>::mutate(|n| *n = n.saturating_sub(1));

                Self::deposit_event(Event::AttestorDeactivated {
                    attestor: account.clone(),
                });

                Ok(())
            })
        }

        fn is_registered(account: &T::AccountId) -> bool {
            Attestors::<T>::contains_key(account)
        }

        fn total_attestors() -> u64 {
            TotalAttestors::<T>::get()
        }
    }
}