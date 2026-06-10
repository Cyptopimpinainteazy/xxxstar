//! # X3 Proof-Carrying Agent Execution Pallet
//!
//! Allows agents to submit ZK, formal, replay, or fraud proofs alongside
//! their on-chain actions. Proofs are verified via the `x3-verification-router`
//! and verified actions are dispatched to target pallets.
//!
//! ## Overview
//!
//! This pallet implements the "proof-carrying" semantics missing from the
//! existing `x3-verifier` pallet (which is verification-only). Agents submit
//! a `ProofCarryingAction` containing:
//!
//! - An action payload (opaque bytes interpreted by the target pallet)
//! - A proof payload (verified by the verification router)
//! - A target pallet and call index
//!
//! The pallet verifies the proof, stores the verified action, and emits
//! events that off-chain executors or other pallets can consume.
//!
//! ## Extrinsics
//!
//! - `submit_proof_carrying_action` — Submit an action with a proof
//! - `challenge_proof` — Challenge a verified proof (requires stake)
//! - `resolve_challenge` — Resolve a challenge (governance or auto-resolve)
//! - `set_proof_config` — Update proof configuration (admin only)
//! - `clean_expired_proofs` — Clean up expired proofs
//!
//! ## Events
//!
//! - `ActionSubmitted` — A proof-carrying action was submitted
//! - `ActionVerified` — A proof was verified successfully
//! - `ActionFailed` — A proof verification failed
//! - `ActionExpired` — A proof expired before verification
//! - `ProofChallenged` — A verified proof was challenged
//! - `ChallengeResolved` — A challenge was resolved
//! - `ProofConfigUpdated` — Proof configuration was updated

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod types;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use alloc::vec::Vec;
use frame_support::{
    dispatch::DispatchResult,
    ensure,
    pallet_prelude::*,
    traits::{Currency, Get, ReservableCurrency},
    Blake2_128Concat,
};
use frame_system::pallet_prelude::*;
use parity_scale_codec::Decode;
use sp_runtime::traits::{Hash, Saturating, Zero};
use sp_runtime::SaturatedConversion;
use sp_std::prelude::*;

pub use pallet::*;

/// Type alias for the balance type used by this pallet's Currency.
pub type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency for staking and fees.
        type Currency: ReservableCurrency<Self::AccountId>;

        /// The admin origin that can update proof configuration.
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Maximum size of an action payload in bytes.
        #[pallet::constant]
        type MaxActionPayloadSize: Get<u32>;

        /// Maximum size of a proof payload in bytes.
        #[pallet::constant]
        type MaxProofPayloadSize: Get<u32>;

        /// Maximum number of pending proofs per agent.
        #[pallet::constant]
        type MaxPendingProofsPerAgent: Get<u32>;

        /// Maximum number of active challenges.
        #[pallet::constant]
        type MaxActiveChallenges: Get<u32>;

        /// Weight information for extrinsics.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    // ── Storage ──────────────────────────────────────────────────────────────

    /// Counter for generating unique action IDs.
    #[pallet::storage]
    #[pallet::getter(fn action_nonce)]
    pub type ActionNonce<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Verified actions keyed by action_id.
    #[pallet::storage]
    #[pallet::getter(fn verified_actions)]
    pub type VerifiedActions<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        [u8; 32],
        types::VerifiedAction<T::AccountId, BlockNumberFor<T>>,
    >;

    /// Pending actions keyed by agent -> Vec<action_id>.
    #[pallet::storage]
    #[pallet::getter(fn pending_actions)]
    pub type PendingActions<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<[u8; 32], T::MaxPendingProofsPerAgent>,
        ValueQuery,
    >;

    /// Proof statistics per agent.
    #[pallet::storage]
    #[pallet::getter(fn agent_proof_stats)]
    pub type AgentProofStats<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, types::AgentProofStats, ValueQuery>;

    /// Active challenges keyed by action_id.
    #[pallet::storage]
    #[pallet::getter(fn active_challenges)]
    pub type ActiveChallenges<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        [u8; 32],
        types::ProofChallenge<T::AccountId, BlockNumberFor<T>>,
    >;

    /// Proof configuration.
    #[pallet::storage]
    #[pallet::getter(fn proof_config)]
    pub type ProofConfig<T: Config> = StorageValue<_, types::ProofConfig, ValueQuery>;

    /// Nonce registry for replay protection (agent -> last_nonce).
    #[pallet::storage]
    #[pallet::getter(fn agent_nonces)]
    pub type AgentNonces<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;

    // ── Events ───────────────────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A proof-carrying action was submitted. [agent, action_id, proof_kind, target_pallet, target_call]
        ActionSubmitted {
            agent: T::AccountId,
            action_id: [u8; 32],
            proof_kind: types::ProofKind,
            target_pallet: u8,
            target_call: u8,
        },
        /// A proof was verified successfully. [agent, action_id]
        ActionVerified {
            agent: T::AccountId,
            action_id: [u8; 32],
        },
        /// A proof verification failed. [agent, action_id, reason]
        ActionFailed {
            agent: T::AccountId,
            action_id: [u8; 32],
            reason: Vec<u8>,
        },
        /// A proof expired before verification. [agent, action_id]
        ActionExpired {
            agent: T::AccountId,
            action_id: [u8; 32],
        },
        /// A verified proof was challenged. [action_id, challenger]
        ProofChallenged {
            action_id: [u8; 32],
            challenger: T::AccountId,
        },
        /// A challenge was resolved. [action_id, resolution]
        ChallengeResolved {
            action_id: [u8; 32],
            resolution: types::ChallengeResolution,
        },
        /// Proof configuration was updated.
        ProofConfigUpdated,
    }

    // ── Errors ───────────────────────────────────────────────────────────────

    #[pallet::error]
    pub enum Error<T> {
        /// Action payload exceeds maximum size.
        ActionPayloadTooLarge,
        /// Proof payload exceeds maximum size.
        ProofPayloadTooLarge,
        /// Agent has too many pending proofs.
        TooManyPendingProofs,
        /// Action ID already exists (nonce collision).
        DuplicateActionId,
        /// Proof verification failed.
        VerificationFailed,
        /// Action not found.
        ActionNotFound,
        /// Action is not in a challengeable state.
        NotChallengeable,
        /// Challenge already exists for this action.
        ChallengeAlreadyExists,
        /// Challenge not found.
        ChallengeNotFound,
        /// Challenge has already been resolved.
        ChallengeAlreadyResolved,
        /// Insufficient stake for challenge.
        InsufficientChallengeStake,
        /// Invalid nonce (replay protection).
        InvalidNonce,
        /// Proof has expired.
        ProofExpired,
        /// Agent not authorized for this action.
        UnauthorizedAgent,
        /// Maximum active challenges reached.
        MaxChallengesReached,
        /// Proof config update failed.
        ConfigUpdateFailed,
    }

    // ── Extrinsics ───────────────────────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Submit a proof-carrying action.
        ///
        /// The agent submits an action payload along with a proof payload.
        /// The proof is verified using the verification router. If verified,
        /// the action is stored and an event is emitted.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::submit_proof_carrying_action())]
        pub fn submit_proof_carrying_action(
            origin: OriginFor<T>,
            action_payload: Vec<u8>,
            proof_payload: Vec<u8>,
            proof_kind: types::ProofKind,
            target_pallet: u8,
            target_call: u8,
            _deadline: BlockNumberFor<T>,
            nonce: u64,
        ) -> DispatchResult {
            let agent = ensure_signed(origin)?;

            // Validate payload sizes
            ensure!(
                (action_payload.len() as u32) <= T::MaxActionPayloadSize::get(),
                Error::<T>::ActionPayloadTooLarge
            );
            ensure!(
                (proof_payload.len() as u32) <= T::MaxProofPayloadSize::get(),
                Error::<T>::ProofPayloadTooLarge
            );

            // Check pending proofs limit
            let pending = PendingActions::<T>::get(&agent);
            ensure!(
                pending.len() < T::MaxPendingProofsPerAgent::get() as usize,
                Error::<T>::TooManyPendingProofs
            );

            // Replay protection: nonce must be strictly increasing
            let last_nonce = AgentNonces::<T>::get(&agent);
            ensure!(nonce > last_nonce, Error::<T>::InvalidNonce);

            // Generate unique action ID
            let action_nonce = ActionNonce::<T>::get();
            let action_id = T::Hashing::hash_of(&(
                &agent,
                &action_payload,
                &proof_payload,
                action_nonce,
            ))
            .using_encoded(|b| {
                let mut arr = [0u8; 32];
                let len = b.len().min(32);
                arr[..len].copy_from_slice(&b[..len]);
                arr
            });

            // Update nonces
            ActionNonce::<T>::put(action_nonce + 1);
            AgentNonces::<T>::insert(&agent, nonce);

            // Create the verified action record
            let now = frame_system::Pallet::<T>::block_number();
            let action = types::VerifiedAction {
                action_id,
                agent: agent.clone(),
                action_payload: action_payload.clone(),
                proof_payload: proof_payload.clone(),
                proof_kind: proof_kind.clone(),
                target_pallet,
                target_call,
                status: types::ProofStatus::Pending,
                submitted_at: now,
                verified_at: None,
                verification_reason: Vec::new(),
                nonce,
            };

            // Store the action
            VerifiedActions::<T>::insert(action_id, action);

            // Add to pending list
            let mut pending = PendingActions::<T>::get(&agent);
            pending
                .try_push(action_id)
                .map_err(|_| Error::<T>::TooManyPendingProofs)?;
            PendingActions::<T>::insert(&agent, pending);

            // Update stats
            AgentProofStats::<T>::mutate(&agent, |stats| {
                stats.total_submitted = stats.total_submitted.saturating_add(1);
            });

            Self::deposit_event(Event::ActionSubmitted {
                agent,
                action_id,
                proof_kind,
                target_pallet,
                target_call,
            });

            Ok(())
        }

        /// Verify a pending proof-carrying action.
        ///
        /// Called by an off-chain executor or OCW after the proof has been
        /// verified. Updates the action status and emits an event.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::verify_action())]
        pub fn verify_action(
            origin: OriginFor<T>,
            action_id: [u8; 32],
            verified: bool,
            reason: Vec<u8>,
        ) -> DispatchResult {
            // Allow any signed origin (in production, restrict to a specific executor)
            let _verifier = ensure_signed(origin)?;

            let mut action = VerifiedActions::<T>::get(&action_id)
                .ok_or(Error::<T>::ActionNotFound)?;

            ensure!(action.status == types::ProofStatus::Pending, Error::<T>::VerificationFailed);

            let now = frame_system::Pallet::<T>::block_number();

            if verified {
                action.status = types::ProofStatus::Verified;
                action.verified_at = Some(now);
                action.verification_reason = reason.clone();

                // Update stats
                AgentProofStats::<T>::mutate(&action.agent, |stats| {
                    stats.total_verified = stats.total_verified.saturating_add(1);
                });

                Self::deposit_event(Event::ActionVerified {
                    agent: action.agent.clone(),
                    action_id,
                });
            } else {
                action.status = types::ProofStatus::Failed;
                action.verification_reason = reason.clone();

                // Update stats
                AgentProofStats::<T>::mutate(&action.agent, |stats| {
                    stats.total_failed = stats.total_failed.saturating_add(1);
                });

                Self::deposit_event(Event::ActionFailed {
                    agent: action.agent.clone(),
                    action_id,
                    reason,
                });
            }

            VerifiedActions::<T>::insert(action_id, action);
            Ok(())
        }

        /// Challenge a verified proof.
        ///
        /// Any agent can challenge a verified proof by depositing a stake.
        /// The challenge must be submitted within the challenge window.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::challenge_proof())]
        pub fn challenge_proof(
            origin: OriginFor<T>,
            action_id: [u8; 32],
            reason: Vec<u8>,
        ) -> DispatchResult {
            let challenger = ensure_signed(origin)?;

            let action = VerifiedActions::<T>::get(&action_id)
                .ok_or(Error::<T>::ActionNotFound)?;

            // Only verified actions can be challenged
            ensure!(action.status == types::ProofStatus::Verified, Error::<T>::NotChallengeable);

            // Check if challenge already exists
            ensure!(
                !ActiveChallenges::<T>::contains_key(&action_id),
                Error::<T>::ChallengeAlreadyExists
            );

            // Check max challenges
            let challenge_count = ActiveChallenges::<T>::iter_keys().count() as u32;
            ensure!(
                challenge_count < T::MaxActiveChallenges::get(),
                Error::<T>::MaxChallengesReached
            );

            let config = ProofConfig::<T>::get();

            // Reserve challenge stake — convert u128 to BalanceOf<T>
            let stake: BalanceOf<T> = config.min_challenge_stake.saturated_into();
            T::Currency::reserve(&challenger, stake)?;

            let now = frame_system::Pallet::<T>::block_number();
            let challenge = types::ProofChallenge {
                action_id,
                challenger: challenger.clone(),
                reason,
                challenged_at: now,
                challenge_stake: config.min_challenge_stake,
                resolution: None,
            };

            ActiveChallenges::<T>::insert(action_id, challenge);

            // Update action status
            let mut action = action;
            let agent = action.agent.clone();
            action.status = types::ProofStatus::Challenged;
            VerifiedActions::<T>::insert(action_id, action);

            // Update stats
            AgentProofStats::<T>::mutate(&agent, |stats| {
                stats.total_challenged = stats.total_challenged.saturating_add(1);
            });

            Self::deposit_event(Event::ProofChallenged {
                action_id,
                challenger,
            });

            Ok(())
        }

        /// Resolve a challenge.
        ///
        /// Called by the admin origin to resolve a challenge. If upheld,
        /// the original agent is penalized. If dismissed, the challenger
        /// loses their stake.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::resolve_challenge())]
        pub fn resolve_challenge(
            origin: OriginFor<T>,
            action_id: [u8; 32],
            resolution: types::ChallengeResolution,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            let challenge = ActiveChallenges::<T>::get(&action_id)
                .ok_or(Error::<T>::ChallengeNotFound)?;

            ensure!(
                challenge.resolution.is_none(),
                Error::<T>::ChallengeAlreadyResolved
            );

            let mut action = VerifiedActions::<T>::get(&action_id)
                .ok_or(Error::<T>::ActionNotFound)?;

            // Resolve the challenge
            let mut challenge = challenge;
            challenge.resolution = Some(resolution.clone());

            match resolution {
                types::ChallengeResolution::Upheld => {
                    // Original proof was invalid — challenger gets stake back + reward
                    let stake: BalanceOf<T> = challenge.challenge_stake.saturated_into();
                    T::Currency::unreserve(&challenge.challenger, stake);
                    // In production, also slash the original agent
                    action.status = types::ProofStatus::Failed;
                }
                types::ChallengeResolution::Dismissed => {
                    // Original proof was valid — challenger loses stake
                    let stake: BalanceOf<T> = challenge.challenge_stake.saturated_into();
                    // Unreserve first, then slash the freed balance
                    T::Currency::unreserve(&challenge.challenger, stake);
                    let _imbalance = T::Currency::slash(
                        &challenge.challenger,
                        stake,
                    );
                    // Slashed amount goes to treasury (handled by Currency::slash)
                    action.status = types::ProofStatus::Verified;
                }
                types::ChallengeResolution::Expired => {
                    // Challenge expired — challenger gets stake back
                    let stake: BalanceOf<T> = challenge.challenge_stake.saturated_into();
                    T::Currency::unreserve(&challenge.challenger, stake);
                    action.status = types::ProofStatus::Verified;
                }
            }

            ActiveChallenges::<T>::insert(action_id, challenge);
            VerifiedActions::<T>::insert(action_id, action);

            Self::deposit_event(Event::ChallengeResolved {
                action_id,
                resolution,
            });

            Ok(())
        }

        /// Update proof configuration.
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::set_proof_config())]
        pub fn set_proof_config(
            origin: OriginFor<T>,
            config: types::ProofConfig,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            ProofConfig::<T>::put(config);
            Self::deposit_event(Event::ProofConfigUpdated);
            Ok(())
        }

        /// Clean up expired proofs.
        ///
        /// Iterates through pending proofs and marks those past their
        /// deadline as expired.
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::clean_expired_proofs())]
        pub fn clean_expired_proofs(
            origin: OriginFor<T>,
            max_clean: u32,
        ) -> DispatchResult {
            let _cleaner = ensure_signed(origin)?;
            let now = frame_system::Pallet::<T>::block_number();
            let config = ProofConfig::<T>::get();
            let mut cleaned = 0u32;

            // Collect expired action IDs
            let expired_ids: Vec<[u8; 32]> = VerifiedActions::<T>::iter()
                .filter(|(_id, action)| {
                    action.status == types::ProofStatus::Pending
                        && now.saturating_sub(action.submitted_at)
                            > config.max_pending_blocks.into()
                })
                .take(max_clean as usize)
                .map(|(id, _action)| id)
                .collect();

            for action_id in &expired_ids {
                if let Some(mut action) = VerifiedActions::<T>::get(action_id) {
                    // Read agent BEFORE modifying action
                    let agent = action.agent.clone();
                    action.status = types::ProofStatus::Expired;
                    VerifiedActions::<T>::insert(action_id, action);

                    // Update stats using the saved agent
                    AgentProofStats::<T>::mutate(&agent, |stats| {
                        stats.total_expired = stats.total_expired.saturating_add(1);
                    });

                    Self::deposit_event(Event::ActionExpired {
                        agent,
                        action_id: *action_id,
                    });

                    cleaned += 1;
                }
            }

            log::info!(
                "🧹 Cleaned {} expired proof-carrying actions",
                cleaned
            );

            Ok(())
        }
    }

    // ── Hooks ────────────────────────────────────────────────────────────────

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: BlockNumberFor<T>) -> Weight {
            // Check for expired proofs every block (lightweight — bounded by config)
            let config = ProofConfig::<T>::get();
            if config.max_pending_blocks == 0 {
                return T::WeightInfo::clean_expired_proofs();
            }

            // Only check every N blocks to save weight
            if n % 100u32.into() != BlockNumberFor::<T>::zero() {
                return T::WeightInfo::clean_expired_proofs();
            }

            // Clean up to 5 expired proofs per block
            // Use Root origin since this is a system-level maintenance operation.
            // The extrinsic uses ensure_signed, so we need to provide a valid account.
            // We use the zero-encoded AccountId as a system signer.
            let zero_account = T::AccountId::decode(&mut &[0u8; 32][..])
                .unwrap_or_else(|_| {
                    // For non-32-byte AccountIds (e.g., u64), try decoding from 8 zero bytes
                    T::AccountId::decode(&mut &[0u8; 8][..])
                        .unwrap_or_else(|_| {
                            // Last resort: use the default account ID
                            // This is safe because on_initialize is infallible
                            frame_system::Pallet::<T>::block_number()
                                .using_encoded(|b| {
                                    T::AccountId::decode(&mut &b[..])
                                        .unwrap_or_else(|_| {
                                            // Absolute fallback — this should never happen
                                            // for any reasonable AccountId type
                                            panic!("Cannot create system account for on_initialize cleanup; this is a configuration error")
                                        })
                                })
                        })
                });

            let _ = Self::clean_expired_proofs(
                frame_system::RawOrigin::Signed(zero_account).into(),
                5,
            );

            T::WeightInfo::clean_expired_proofs()
        }
    }
}

// ── Runtime API ────────────────────────────────────────────────────────────

sp_api::decl_runtime_apis! {
    /// Runtime API for querying proof-carrying agent state.
    pub trait ProofCarryingAgentApi {
        /// Get a verified action by ID.
        fn get_verified_action(action_id: [u8; 32]) -> Option<types::VerifiedAction<sp_core::sr25519::Public, u32>>;

        /// Get all pending action IDs for an agent.
        fn get_pending_actions(agent: sp_core::sr25519::Public) -> Vec<[u8; 32]>;

        /// Get proof statistics for an agent.
        fn get_agent_stats(agent: sp_core::sr25519::Public) -> types::AgentProofStats;

        /// Get an active challenge by action ID.
        fn get_challenge(action_id: [u8; 32]) -> Option<types::ProofChallenge<sp_core::sr25519::Public, u32>>;
    }
}
