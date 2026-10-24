#![cfg_attr(not(feature = "std"), no_std)]

//! # Cross-Chain Validator Pallet
//!
//! Stores external EVM/SVM header attestations and finalized headers that the
//! settlement engine relies on.
//!
//! **Security models:**
//!
//! The legacy `LastEvmHeader` / `LastSvmHeader` path is an **authorized-relayer
//! header store**, not an on-chain light client. Authenticity rests on the
//! originating account being a member of `AuthorizedSubmitters`.
//!
//! The `LastFinalizedEvmHeader` / `LastFinalizedSvmHeader` path verifies a
//! domain-separated Ed25519 certificate from a governance-approved, distinct
//! attester set. The certificate binds the chain, height/slot, block hash, state
//! commitment, and chain-specific auxiliary root. It is the source used by the
//! runtime external-root and settlement verifiers.
//!
//! This is a threshold attester model, not a native Ethereum sync-committee or
//! Solana stake-weighted consensus verifier. Governance configuration and key
//! custody are therefore part of the trust boundary. An empty attester set or a
//! zero threshold fails closed; governance may publish an empty/zero pair to
//! revoke a verifier immediately.
//!
//! **Validation Flow (EVM / SVM):**
//! 1. Origin must be an authorized submitter (fail closed when the set is empty).
//! 2. Structural checks: non-zero heights/hashes, bounds, monotonicity, and a
//!    now+`MaxHeaderLookahead` far-future guard.
//! 3. For EVM: `merkle_root` is *recomputed* from the submitted proof leaves and must
//!    equal the claimed root (real Merkle inclusion, `_expected_root` is no longer
//!    ignored).
//! 4. Only after every check passes is any storage written.
//!
//! ## What `merkle_root` has to be for the settlement engine
//!
//! `pallet-x3-settlement-engine` settles an EVM leg by walking a **Merkle Patricia
//! receipt** proof — `rlp(index)` as the key, an RLP node path, the receipt as the leaf —
//! against this pallet's stored root. This pallet validates the root the other way: as
//! the root of a **flat** Merkle tree over the submitted 32-byte leaves. Those are two
//! different structures over the same 32 bytes, so a submitter who attests a block's
//! leaf-Merkle root makes that block impossible to settle, and nothing here complains:
//! the shape is valid, the root is stored, and only the trie walk refuses.
//!
//! So the root submitted for a block the settlement engine will settle **must be that
//! block's `receiptsRoot`** — and this pallet cannot check that, because it never sees an
//! RLP header. The whole header, hash included, is the authorized submitter's attestation
//! (see the security model above); what this pallet does check is that the submitted
//! `proof` really has the claimed root as its root, so a submitter cannot store a root
//! they cannot open. The *meaning* of the root is their claim.
//!
//! The failure mode is liveness rather than theft - a root of the wrong kind is refused
//! by the walk rather than settled against — which is why it is stated here rather than
//! enforced: there is nothing to enforce against. An operator wiring the settlement path
//! has to set this field to the receipts root, and
//! `pallets/x3-settlement-engine/src/tests.rs` pins the four ways the walk fails, so a
//! wrong root shows up as a refused settlement rather than as a settlement against the
//! wrong tree.

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_runtime::traits::SaturatedConversion;
    use sp_std::vec::Vec;

    /// Max entries in the authorized-submitter set. Kept low/constant so the set is
    /// a small bounded whitelist administered by governance.
    pub const MAX_AUTHORIZED_SUBMITTERS: u32 = 64;
    /// Max raw EVM proof (Merkle leaves) bytes. 32 bytes per leaf, hard cap on work.
    pub const MAX_PROOF_BYTES: u32 = 10 * 1024;
    /// Max SVM "validator set" bytes (concatenated 32-byte entries).
    pub const MAX_VALIDATOR_SET_BYTES: u32 = 400;
    /// Max parent-slot-hash chain length carried on an SVM header.
    pub const MAX_PARENT_SLOT_HASHES: u32 = 16;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type WeightInfo: WeightInfo;

        /// Origin permitted to manage the authorized-submitter set (governance/Root
        /// in production). Header submission itself is gated by membership in the
        /// storage-backed set, *not* by this origin.
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Maximum acceptable distance, in the local chain's block height, between the
        /// current block and a submitted EVM/SVM header height. Rejects absurd
        /// far-future values (e.g. `u64::MAX`) that would otherwise poison the
        /// high-water mark and permanently block legitimate headers (liveness DoS).
        type MaxHeaderLookahead: Get<u64>;
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Storage: Cross-Chain Header State
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Last validated EVM block header
    #[pallet::storage]
    pub type LastEvmHeader<T: Config> = StorageValue<_, EvmHeaderInfo, OptionQuery>;

    /// Last validated SVM (Solana) block header
    #[pallet::storage]
    pub type LastSvmHeader<T: Config> = StorageValue<_, SvmHeaderInfo, OptionQuery>;

    /// Headers whose finality certificate reached the independently configured quorum.
    #[pallet::storage]
    pub type LastFinalizedEvmHeader<T: Config> = StorageValue<_, EvmHeaderInfo, OptionQuery>;

    #[pallet::storage]
    pub type LastFinalizedSvmHeader<T: Config> = StorageValue<_, SvmHeaderInfo, OptionQuery>;

    /// Records which verifier produced `LastFinalizedEvmHeader`.
    #[pallet::storage]
    pub type EvmHeaderFinalitySource<T: Config> =
        StorageValue<_, EvmHeaderFinalitySourceKind, ValueQuery>;

    /// Trusted Ethereum beacon light-client state.
    #[pallet::storage]
    pub type EthereumBeaconState<T: Config> =
        StorageValue<_, x3_verification_router::ethereum_beacon::EthereumBeaconState, OptionQuery>;

    /// Beacon fork schedule bound when the native checkpoint was installed.
    #[pallet::storage]
    pub type EthereumBeaconForks<T: Config> =
        StorageValue<_, x3_verification_router::ethereum_beacon::ForkVersions, OptionQuery>;

    /// Finalized beacon roots retained for execution ancestry proofs.
    #[pallet::storage]
    pub type EthereumFinalizedCheckpoints<T: Config> = StorageMap<
        _,
        frame_support::Blake2_128Concat,
        H256,
        x3_verification_router::ethereum_beacon::EthereumFinalizedCheckpoint,
        OptionQuery,
    >;

    /// Governance-approved Ed25519 attesters for finality certificates.
    #[pallet::storage]
    pub type EvmHeaderAttesters<T: Config> =
        StorageValue<_, BoundedVec<[u8; 32], ConstU32<64>>, ValueQuery>;

    #[pallet::storage]
    pub type SvmHeaderAttesters<T: Config> =
        StorageValue<_, BoundedVec<[u8; 32], ConstU32<64>>, ValueQuery>;

    #[pallet::storage]
    pub type EvmHeaderAttesterThreshold<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    pub type SvmHeaderAttesterThreshold<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Merkle root cache for EVM blocks (block_number -> merkle_root)
    #[pallet::storage]
    pub type EvmMerkleRoots<T: Config> =
        StorageMap<_, frame_support::Blake2_128Concat, u64, H256, OptionQuery>;

    /// Full finalized EVM header evidence retained per block number.
    ///
    /// Receipt verification binds the block hash and state root as well as the
    /// receipts root, so retaining only the newest header would make older
    /// deposits unprovable.
    #[pallet::storage]
    pub type FinalizedEvmHeaders<T: Config> =
        StorageMap<_, frame_support::Blake2_128Concat, u64, EvmHeaderInfo, OptionQuery>;

    /// Validator set cache for SVM slots (slot -> validator_set_hash)
    #[pallet::storage]
    pub type SvmValidatorSets<T: Config> =
        StorageMap<_, frame_support::Blake2_128Concat, u64, H256, OptionQuery>;

    /// Cross-chain validation statistics
    #[pallet::storage]
    pub type ValidationStats<T: Config> = StorageValue<_, ValidationStatistics, ValueQuery>;

    /// Storage-backed set of accounts allowed to submit/attest external headers.
    /// Only mutable via `set_authorized_submitters`, which requires `AdminOrigin`.
    /// When empty the whole header-submission path is disabled (fail closed).
    #[pallet::storage]
    pub type AuthorizedSubmitters<T: Config> =
        StorageMap<_, frame_support::Blake2_128Concat, T::AccountId, (), ValueQuery>;

    // ═══════════════════════════════════════════════════════════════════════════════
    // Types
    // ═══════════════════════════════════════════════════════════════════════════════

    #[derive(Debug, Clone, Encode, Decode, DecodeWithMemTracking, PartialEq, Eq, TypeInfo)]
    pub struct EvmHeaderInfo {
        pub block_number: u64,
        pub block_hash: H256,
        pub state_root: H256,
        /// The root this header was attested with, validated as the root of a flat Merkle
        /// tree over the submitted leaves.
        ///
        /// For a block the settlement engine settles it has to be that block's
        /// **`receiptsRoot`** — the engine walks a Merkle Patricia receipt proof against
        /// it, which is a different structure over the same 32 bytes. See the module
        /// documentation; the wrong kind fails closed at settlement rather than here.
        pub merkle_root: H256,
        pub validator_set_hash: H256,
        pub verified_at_block: u32,
        pub validation_proof: Vec<u8>,
    }

    #[derive(Debug, Clone, Encode, Decode, DecodeWithMemTracking, PartialEq, Eq, TypeInfo)]
    pub struct SvmHeaderInfo {
        pub slot: u64,
        pub block_hash: H256,
        pub state_root: H256,
        pub validator_set_hash: H256,
        pub verified_at_block: u32,
        pub validation_proof: Vec<u8>,
        pub parent_slot_hashes: Vec<H256>,
    }

    #[derive(
        Debug, Clone, Encode, Decode, DecodeWithMemTracking, PartialEq, Eq, TypeInfo, Default,
    )]
    pub struct ValidationStatistics {
        pub evm_headers_validated: u64,
        pub svm_headers_validated: u64,
        pub total_validation_failures: u64,
        pub last_validation_block: u32,
    }

    #[derive(
        Debug, Clone, Copy, Encode, Decode, DecodeWithMemTracking, PartialEq, Eq, TypeInfo, Default,
    )]
    pub enum EvmHeaderFinalitySourceKind {
        #[default]
        Unset,
        AttesterQuorum,
        BeaconSyncCommittee,
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Events
    // ═══════════════════════════════════════════════════════════════════════════════

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// EVM header successfully validated
        EvmHeaderValidated {
            block_number: u64,
            block_hash: H256,
            merkle_root: H256,
        },
        /// SVM header successfully validated
        SvmHeaderValidated {
            slot: u64,
            block_hash: H256,
            validator_set_hash: H256,
        },
        /// Validation failed with reason
        ValidationFailed {
            chain: Vec<u8>,
            reason: Vec<u8>,
        },
        /// The authorized-submitter set was replaced by governance.
        AuthorizedSubmittersUpdated {
            submitters: Vec<T::AccountId>,
        },
        /// The independently signed header-attester sets were replaced.
        HeaderAttestersUpdated {
            evm_count: u32,
            evm_threshold: u32,
            svm_count: u32,
            svm_threshold: u32,
        },
        /// An independently finalized EVM header became the runtime anchor.
        EvmHeaderFinalized {
            block_number: u64,
            block_hash: H256,
            state_root: H256,
            receipts_root: H256,
        },
        /// An independently finalized Solana header became the runtime anchor.
        SvmHeaderFinalized {
            slot: u64,
            block_hash: H256,
            state_root: H256,
            validator_set_hash: H256,
        },
        EthereumBeaconCheckpointSet {
            slot: u64,
            block_root: H256,
        },
        EthereumBeaconUpdateAccepted {
            attested_slot: u64,
            finalized_slot: u64,
            finalized_root: H256,
        },
        EthereumExecutionHeaderFinalized {
            beacon_slot: u64,
            block_number: u64,
            block_hash: H256,
            receipts_root: H256,
        },
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Errors
    // ═══════════════════════════════════════════════════════════════════════════════

    #[pallet::error]
    pub enum Error<T> {
        /// Invalid EVM block header
        InvalidEvmHeader,
        /// Invalid SVM block header
        InvalidSvmHeader,
        /// Merkle root mismatch
        MerkleRootMismatch,
        /// Validator set verification failed
        ValidatorSetVerificationFailed,
        /// Header is too old
        HeaderTooOld,
        /// State root is zero (invalid)
        InvalidStateRoot,
        /// Merkle proof verification failed
        InvalidMerkleProof,
        /// Parent block not found for linking
        ParentBlockNotFound,
        /// Timestamp not monotonically increasing
        NonMonotonicTimestamp,
        /// Insufficient validator quorum
        InsufficientValidatorQuorum,
        /// Proof data malformed
        MalformedProofData,
        /// Caller is not an authorized header submitter (fail closed when set empty)
        NotAuthorizedSubmitter,
        /// Only `AdminOrigin` may manage the authorized-submitter set
        NotAuthorizedAdministrator,
        /// Submission would exceed the `MaxHeaderLookahead` window (far future)
        FarFutureHeader,
        /// Duplicate entries are not allowed in the authorized-submitter set
        DuplicateSubmitter,
        /// Authorized-submitter set exceeds `MAX_AUTHORIZED_SUBMITTERS`
        TooManySubmitters,
        /// Proof data not a multiple of the 32-byte hash width
        ProofNotMultipleOf32,
        /// Duplicate validator/signer entries in the SVM set
        DuplicateValidator,
        /// Submission payload exceeds the configured size bound
        PayloadTooLarge,
        /// Header attester set or threshold is invalid.
        InvalidAttesterSet,
        /// The header certificate did not reach the configured independent quorum.
        InvalidHeaderCertificate,
        /// Native Ethereum beacon client has no trusted checkpoint.
        EthereumBeaconNotBootstrapped,
        /// Native Ethereum beacon consensus update was invalid.
        InvalidEthereumBeaconUpdate,
        /// Native Ethereum execution/ancestry proof was invalid.
        InvalidEthereumExecutionProof,
        /// A weaker attester certificate cannot replace an active native beacon anchor.
        NativeEvmFinalityActive,
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Extrinsics
    // ═══════════════════════════════════════════════════════════════════════════════

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Governance: replace the authorized-submitter set.
        ///
        /// Passing an empty list clears the set and thereby **disables** the header
        /// submission path until a new trusted set is configured (fail closed).
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::set_authorized_submitters())]
        pub fn set_authorized_submitters(
            origin: OriginFor<T>,
            new_submitters: Vec<T::AccountId>,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            ensure!(
                (new_submitters.len() as u32) <= MAX_AUTHORIZED_SUBMITTERS,
                Error::<T>::TooManySubmitters
            );

            // Reject duplicate accounts so the membership predicate stays unambiguous.
            for (i, acc) in new_submitters.iter().enumerate() {
                if new_submitters[..i].contains(acc) {
                    return Err(Error::<T>::DuplicateSubmitter.into());
                }
            }

            // Clear the prior set. The map is unbounded (`without_storage_info`), so
            // drain in `max`-limited batches until the previous membership is fully
            // removed. The set is independently capped at `MAX_AUTHORIZED_SUBMITTERS`
            // (≤64), so this converges in a single bounded batch in practice.
            loop {
                let removed = AuthorizedSubmitters::<T>::clear(u32::MAX, None);
                if removed.unique == 0 && removed.loops == 0 {
                    break;
                }
                if removed.maybe_cursor.is_none() {
                    break;
                }
            }
            for acc in new_submitters.iter() {
                AuthorizedSubmitters::<T>::insert(acc, ());
            }

            Self::deposit_event(Event::AuthorizedSubmittersUpdated {
                submitters: new_submitters,
            });
            Ok(())
        }

        /// Governance: configure independent Ed25519 finality attesters.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::set_header_attesters())]
        pub fn set_header_attesters(
            origin: OriginFor<T>,
            evm_attesters: Vec<[u8; 32]>,
            evm_threshold: u32,
            svm_attesters: Vec<[u8; 32]>,
            svm_threshold: u32,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            ensure!(
                Self::attester_set_is_valid(&evm_attesters, evm_threshold)
                    && Self::attester_set_is_valid(&svm_attesters, svm_threshold),
                Error::<T>::InvalidAttesterSet
            );
            let evm_count = evm_attesters.len() as u32;
            let svm_count = svm_attesters.len() as u32;
            let evm: BoundedVec<[u8; 32], ConstU32<64>> = evm_attesters
                .try_into()
                .map_err(|_| Error::<T>::InvalidAttesterSet)?;
            let svm: BoundedVec<[u8; 32], ConstU32<64>> = svm_attesters
                .try_into()
                .map_err(|_| Error::<T>::InvalidAttesterSet)?;
            EvmHeaderAttesters::<T>::put(evm);
            SvmHeaderAttesters::<T>::put(svm);
            EvmHeaderAttesterThreshold::<T>::put(evm_threshold);
            SvmHeaderAttesterThreshold::<T>::put(svm_threshold);
            Self::deposit_event(Event::HeaderAttestersUpdated {
                evm_count,
                evm_threshold,
                svm_count,
                svm_threshold,
            });
            Ok(())
        }

        /// Governance: install a trusted Ethereum beacon checkpoint and fork schedule.
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::set_ethereum_beacon_checkpoint())]
        pub fn set_ethereum_beacon_checkpoint(
            origin: OriginFor<T>,
            checkpoint: x3_verification_router::ethereum_beacon::EthereumCheckpointUpdate,
            forks: x3_verification_router::ethereum_beacon::ForkVersions,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            let state = x3_verification_router::ethereum_beacon::initialize_ethereum_beacon(
                &checkpoint,
                &forks,
            )
            .map_err(|_| Error::<T>::InvalidEthereumBeaconUpdate)?;
            let block_root = state
                .finalized_header
                .hash_tree_root()
                .map_err(|_| Error::<T>::InvalidEthereumBeaconUpdate)?;

            loop {
                let removed = EthereumFinalizedCheckpoints::<T>::clear(u32::MAX, None);
                if removed.unique == 0 && removed.loops == 0 {
                    break;
                }
                if removed.maybe_cursor.is_none() {
                    break;
                }
            }
            loop {
                let removed = EvmMerkleRoots::<T>::clear(u32::MAX, None);
                if removed.unique == 0 && removed.loops == 0 {
                    break;
                }
                if removed.maybe_cursor.is_none() {
                    break;
                }
            }
            loop {
                let removed = FinalizedEvmHeaders::<T>::clear(u32::MAX, None);
                if removed.unique == 0 && removed.loops == 0 {
                    break;
                }
                if removed.maybe_cursor.is_none() {
                    break;
                }
            }
            LastFinalizedEvmHeader::<T>::kill();
            EvmHeaderFinalitySource::<T>::put(EvmHeaderFinalitySourceKind::Unset);
            EthereumFinalizedCheckpoints::<T>::insert(
                block_root,
                x3_verification_router::ethereum_beacon::EthereumFinalizedCheckpoint {
                    slot: state.finalized_header.slot,
                    block_roots_root: state.block_roots_root,
                },
            );
            EthereumBeaconForks::<T>::put(forks);
            EthereumBeaconState::<T>::put(state);
            Self::deposit_event(Event::EthereumBeaconCheckpointSet {
                slot: checkpoint.header.slot,
                block_root,
            });
            Ok(())
        }

        /// Submit a native Ethereum sync-committee finality update.
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::submit_ethereum_beacon_update())]
        pub fn submit_ethereum_beacon_update(
            origin: OriginFor<T>,
            update: x3_verification_router::ethereum_beacon::EthereumUpdate,
        ) -> DispatchResult {
            ensure_signed(origin)?;
            let state =
                EthereumBeaconState::<T>::get().ok_or(Error::<T>::EthereumBeaconNotBootstrapped)?;
            let forks =
                EthereumBeaconForks::<T>::get().ok_or(Error::<T>::EthereumBeaconNotBootstrapped)?;
            let next_state =
                x3_verification_router::ethereum_beacon::apply_ethereum_finality_update(
                    &state, &update, &forks,
                )
                .map_err(|_| Error::<T>::InvalidEthereumBeaconUpdate)?;
            let finalized_root = next_state
                .finalized_header
                .hash_tree_root()
                .map_err(|_| Error::<T>::InvalidEthereumBeaconUpdate)?;
            EthereumFinalizedCheckpoints::<T>::insert(
                finalized_root,
                x3_verification_router::ethereum_beacon::EthereumFinalizedCheckpoint {
                    slot: next_state.finalized_header.slot,
                    block_roots_root: next_state.block_roots_root,
                },
            );
            EthereumBeaconState::<T>::put(next_state.clone());
            Self::deposit_event(Event::EthereumBeaconUpdateAccepted {
                attested_slot: update.attested_header.slot,
                finalized_slot: next_state.finalized_header.slot,
                finalized_root,
            });
            Ok(())
        }

        /// Submit a native beacon execution proof and bind it to the EVM anchor.
        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::submit_ethereum_execution_proof())]
        pub fn submit_ethereum_execution_proof(
            origin: OriginFor<T>,
            proof: x3_verification_router::ethereum_beacon::EthereumExecutionProof,
        ) -> DispatchResult {
            ensure_signed(origin)?;
            let state =
                EthereumBeaconState::<T>::get().ok_or(Error::<T>::EthereumBeaconNotBootstrapped)?;
            let trusted_root = state
                .finalized_header
                .hash_tree_root()
                .map_err(|_| Error::<T>::InvalidEthereumExecutionProof)?;
            let trusted = x3_verification_router::ethereum_beacon::EthereumFinalizedCheckpoint {
                slot: state.finalized_header.slot,
                block_roots_root: state.block_roots_root,
            };
            let verified =
                x3_verification_router::ethereum_beacon::verify_ethereum_execution_proof(
                    trusted_root,
                    &trusted,
                    &proof,
                    EthereumFinalizedCheckpoints::<T>::get,
                )
                .map_err(|_| Error::<T>::InvalidEthereumExecutionProof)?;

            if let Some(existing) = FinalizedEvmHeaders::<T>::get(verified.block_number) {
                ensure!(
                    existing.block_hash == verified.block_hash
                        && existing.state_root == verified.state_root
                        && existing.merkle_root == verified.receipts_root,
                    Error::<T>::InvalidEvmHeader
                );
            }
            let proof_bytes = proof.encode();
            let header_info = EvmHeaderInfo {
                block_number: verified.block_number,
                block_hash: verified.block_hash,
                state_root: verified.state_root,
                merkle_root: verified.receipts_root,
                validator_set_hash: trusted_root,
                verified_at_block: frame_system::Pallet::<T>::block_number()
                    .saturated_into::<u32>(),
                validation_proof: proof_bytes,
            };
            if LastFinalizedEvmHeader::<T>::get()
                .is_none_or(|last| verified.block_number > last.block_number)
            {
                LastFinalizedEvmHeader::<T>::put(header_info.clone());
            }
            FinalizedEvmHeaders::<T>::insert(verified.block_number, header_info);
            EvmHeaderFinalitySource::<T>::put(EvmHeaderFinalitySourceKind::BeaconSyncCommittee);
            EvmMerkleRoots::<T>::insert(verified.block_number, verified.receipts_root);
            ValidationStats::<T>::mutate(|stats| {
                stats.evm_headers_validated = stats.evm_headers_validated.saturating_add(1);
                stats.last_validation_block =
                    frame_system::Pallet::<T>::block_number().saturated_into::<u32>();
            });
            Self::deposit_event(Event::EthereumExecutionHeaderFinalized {
                beacon_slot: verified.beacon_slot,
                block_number: verified.block_number,
                block_hash: verified.block_hash,
                receipts_root: verified.receipts_root,
            });
            Ok(())
        }

        /// Accept an independently finalized EVM header.
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::validate_finalized_evm_header())]
        pub fn validate_finalized_evm_header(
            origin: OriginFor<T>,
            block_number: u64,
            block_hash: H256,
            state_root: H256,
            receipts_root: H256,
            certificate: Vec<u8>,
        ) -> DispatchResult {
            ensure_signed(origin)?;
            ensure!(
                EvmHeaderFinalitySource::<T>::get()
                    != EvmHeaderFinalitySourceKind::BeaconSyncCommittee,
                Error::<T>::NativeEvmFinalityActive
            );
            ensure!(
                block_hash != H256::zero()
                    && state_root != H256::zero()
                    && receipts_root != H256::zero(),
                Error::<T>::InvalidEvmHeader
            );
            ensure!(
                certificate.len()
                    <= x3_verification_router::header_finality::MAX_HEADER_CERTIFICATE_BYTES,
                Error::<T>::PayloadTooLarge
            );
            let expected = x3_verification_router::header_finality::FinalizedHeader {
                chain: x3_verification_router::header_finality::HEADER_FINALITY_ETHEREUM,
                height: block_number,
                block_hash: block_hash.0,
                state_root: state_root.0,
                auxiliary_root: receipts_root.0,
            };
            x3_verification_router::header_finality::verify_header_certificate(
                expected.chain,
                expected,
                &EvmHeaderAttesters::<T>::get(),
                EvmHeaderAttesterThreshold::<T>::get(),
                &certificate,
            )
            .map_err(|_| Error::<T>::InvalidHeaderCertificate)?;
            if let Some(existing) = FinalizedEvmHeaders::<T>::get(block_number) {
                ensure!(
                    existing.block_hash == block_hash
                        && existing.state_root == state_root
                        && existing.merkle_root == receipts_root,
                    Error::<T>::InvalidEvmHeader
                );
            }
            let header_info = EvmHeaderInfo {
                block_number,
                block_hash,
                state_root,
                merkle_root: receipts_root,
                validator_set_hash: H256::from(sp_io::hashing::blake2_256(&certificate)),
                verified_at_block: frame_system::Pallet::<T>::block_number()
                    .saturated_into::<u32>(),
                validation_proof: certificate,
            };
            if LastFinalizedEvmHeader::<T>::get()
                .is_none_or(|last| block_number > last.block_number)
            {
                LastFinalizedEvmHeader::<T>::put(header_info.clone());
            }
            FinalizedEvmHeaders::<T>::insert(block_number, header_info);
            EvmHeaderFinalitySource::<T>::put(EvmHeaderFinalitySourceKind::AttesterQuorum);
            EvmMerkleRoots::<T>::insert(block_number, receipts_root);
            ValidationStats::<T>::mutate(|stats| {
                stats.evm_headers_validated = stats.evm_headers_validated.saturating_add(1);
                stats.last_validation_block =
                    frame_system::Pallet::<T>::block_number().saturated_into::<u32>();
            });
            Self::deposit_event(Event::EvmHeaderFinalized {
                block_number,
                block_hash,
                state_root,
                receipts_root,
            });
            Ok(())
        }

        /// Accept an independently finalized Solana header.
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::validate_finalized_svm_header())]
        pub fn validate_finalized_svm_header(
            origin: OriginFor<T>,
            slot: u64,
            block_hash: H256,
            state_root: H256,
            validator_set_hash: H256,
            certificate: Vec<u8>,
        ) -> DispatchResult {
            ensure_signed(origin)?;
            ensure!(
                block_hash != H256::zero()
                    && state_root != H256::zero()
                    && validator_set_hash != H256::zero(),
                Error::<T>::InvalidSvmHeader
            );
            ensure!(
                certificate.len()
                    <= x3_verification_router::header_finality::MAX_HEADER_CERTIFICATE_BYTES,
                Error::<T>::PayloadTooLarge
            );
            let expected = x3_verification_router::header_finality::FinalizedHeader {
                chain: x3_verification_router::header_finality::HEADER_FINALITY_SOLANA,
                height: slot,
                block_hash: block_hash.0,
                state_root: state_root.0,
                auxiliary_root: validator_set_hash.0,
            };
            x3_verification_router::header_finality::verify_header_certificate(
                expected.chain,
                expected,
                &SvmHeaderAttesters::<T>::get(),
                SvmHeaderAttesterThreshold::<T>::get(),
                &certificate,
            )
            .map_err(|_| Error::<T>::InvalidHeaderCertificate)?;
            if let Some(last) = LastFinalizedSvmHeader::<T>::get() {
                ensure!(slot > last.slot, Error::<T>::InvalidSvmHeader);
            }
            LastFinalizedSvmHeader::<T>::put(SvmHeaderInfo {
                slot,
                block_hash,
                state_root,
                validator_set_hash,
                verified_at_block: frame_system::Pallet::<T>::block_number()
                    .saturated_into::<u32>(),
                validation_proof: certificate,
                parent_slot_hashes: Vec::new(),
            });
            SvmValidatorSets::<T>::insert(slot, validator_set_hash);
            ValidationStats::<T>::mutate(|stats| {
                stats.svm_headers_validated = stats.svm_headers_validated.saturating_add(1);
                stats.last_validation_block =
                    frame_system::Pallet::<T>::block_number().saturated_into::<u32>();
            });
            Self::deposit_event(Event::SvmHeaderFinalized {
                slot,
                block_hash,
                state_root,
                validator_set_hash,
            });
            Ok(())
        }

        /// Submit and validate an EVM block header. Only authorized submitters may
        /// call; all checks happen before any storage write.
        ///
        /// `merkle_root` is checked to be the root of a **flat** Merkle tree over the
        /// `proof`'s leaves, and for a block the settlement engine will settle it must be
        /// that block's **`receiptsRoot`** — the engine walks a Merkle Patricia receipt
        /// proof against it, which is a different structure. See the module documentation:
        /// this pallet cannot tell the two apart, so the meaning of the root is the
        /// submitter's claim and the wrong choice fails closed at settlement.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::validate_evm_header())]
        pub fn validate_evm_header(
            origin: OriginFor<T>,
            block_number: u64,
            block_hash: H256,
            state_root: H256,
            merkle_root: H256,
            proof: Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Phase 0: authentication. Reject non-authorized origins *and* fail closed
            // when no trusted submitter set has been configured.
            Self::ensure_authorized_submitter(&who)?;

            // Phase 1: structural bounds & size bounds, still before any write.
            ensure!(block_number > 0, Error::<T>::InvalidEvmHeader);
            ensure!(block_hash != H256::zero(), Error::<T>::InvalidEvmHeader);
            ensure!(state_root != H256::zero(), Error::<T>::InvalidStateRoot);
            ensure!(!proof.is_empty(), Error::<T>::InvalidEvmHeader);
            ensure!(
                (proof.len() as u32) <= MAX_PROOF_BYTES,
                Error::<T>::PayloadTooLarge
            );
            ensure!(
                proof.len().is_multiple_of(32),
                Error::<T>::ProofNotMultipleOf32
            );

            // Phase 2: far-future guard (rejects absurd heights that would poison the
            // high-water mark). Relative to the local chain's current height.
            Self::ensure_not_far_future(block_number)?;

            // Phase 3: real Merkle inclusion. Recompute the root over the submitted
            // proof leaves and require it to equal the claimed `merkle_root`.
            let leaves = Self::proof_to_leaves(&proof)?;
            let recomputed_root = Self::merkle_root_of(&leaves);
            ensure!(
                recomputed_root == merkle_root,
                Error::<T>::MerkleRootMismatch
            );

            // Phase 4: parent / monotonicity.
            if let Some(last_header) = LastEvmHeader::<T>::get() {
                ensure!(
                    block_number > last_header.block_number,
                    Error::<T>::NonMonotonicTimestamp
                );
            }

            // All checks passed → now write.
            // `validator_set_hash` is an internal identifier commitment of the proof
            // leaves. It is NOT presented as an independently-verified BFT quorum; the
            // trust anchor is the authorized submitter (see module docs).
            let validator_set_hash = H256::from(sp_io::hashing::blake2_256(&proof));

            let header_info = EvmHeaderInfo {
                block_number,
                block_hash,
                state_root,
                merkle_root,
                validator_set_hash,
                verified_at_block: frame_system::Pallet::<T>::block_number()
                    .saturated_into::<u32>(),
                validation_proof: proof,
            };

            LastEvmHeader::<T>::put(header_info.clone());
            EvmMerkleRoots::<T>::insert(block_number, merkle_root);

            // Update statistics
            ValidationStats::<T>::mutate(|stats| {
                stats.evm_headers_validated = stats.evm_headers_validated.saturating_add(1);
                stats.last_validation_block =
                    frame_system::Pallet::<T>::block_number().saturated_into::<u32>();
            });

            Self::deposit_event(Event::EvmHeaderValidated {
                block_number,
                block_hash,
                merkle_root,
            });

            Ok(())
        }

        /// Submit and validate an SVM (Solana) block header. Only authorized
        /// submitters may call; all checks happen before any storage write.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::validate_svm_header())]
        pub fn validate_svm_header(
            origin: OriginFor<T>,
            slot: u64,
            block_hash: H256,
            state_root: H256,
            validator_set: Vec<u8>,
            parent_slot_hashes: Vec<H256>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Phase 0: authentication (fail closed when set empty).
            Self::ensure_authorized_submitter(&who)?;

            // Phase 1: structural + size bounds, before any write.
            ensure!(slot > 0, Error::<T>::InvalidSvmHeader);
            ensure!(block_hash != H256::zero(), Error::<T>::InvalidSvmHeader);
            ensure!(state_root != H256::zero(), Error::<T>::InvalidStateRoot);
            ensure!(
                (validator_set.len() as u32) <= MAX_VALIDATOR_SET_BYTES,
                Error::<T>::PayloadTooLarge
            );
            ensure!(
                !validator_set.is_empty(),
                Error::<T>::ValidatorSetVerificationFailed
            );
            ensure!(
                validator_set.len().is_multiple_of(32),
                Error::<T>::MalformedProofData
            );
            ensure!(
                !parent_slot_hashes.is_empty(),
                Error::<T>::MalformedProofData
            );
            ensure!(
                (parent_slot_hashes.len() as u32) <= MAX_PARENT_SLOT_HASHES,
                Error::<T>::PayloadTooLarge
            );

            // Reject zero or duplicate validator/signer entries -> honest non-empty,
            // self-selected/duplicate set rejection.
            ensure!(
                Self::validator_entries_well_formed(&validator_set)?,
                Error::<T>::DuplicateValidator
            );

            // Phase 2: far-future guard.
            Self::ensure_not_far_future(slot)?;

            // Phase 3: monotonic slot progression + non-zero parent anchor.
            if let Some(last_svm_header) = LastSvmHeader::<T>::get() {
                ensure!(slot > last_svm_header.slot, Error::<T>::InvalidSvmHeader);
                ensure!(
                    parent_slot_hashes[0] != H256::zero(),
                    Error::<T>::NonMonotonicTimestamp
                );
            }
            ensure!(
                parent_slot_hashes[0] != H256::zero(),
                Error::<T>::NonMonotonicTimestamp
            );

            // All checks passed → now write. `validator_set_hash` is an internal
            // identifier of the (well-formed, non-duplicated) set, not a BFT proof.
            let validator_set_hash = H256::from(sp_io::hashing::blake2_256(&validator_set));

            let header_info = SvmHeaderInfo {
                slot,
                block_hash,
                state_root,
                validator_set_hash,
                verified_at_block: frame_system::Pallet::<T>::block_number()
                    .saturated_into::<u32>(),
                validation_proof: validator_set,
                parent_slot_hashes,
            };

            LastSvmHeader::<T>::put(header_info.clone());
            SvmValidatorSets::<T>::insert(slot, validator_set_hash);

            // Update statistics
            ValidationStats::<T>::mutate(|stats| {
                stats.svm_headers_validated = stats.svm_headers_validated.saturating_add(1);
                stats.last_validation_block =
                    frame_system::Pallet::<T>::block_number().saturated_into::<u32>();
            });

            Self::deposit_event(Event::SvmHeaderValidated {
                slot,
                block_hash,
                validator_set_hash,
            });

            Ok(())
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Helper Functions
    // ═══════════════════════════════════════════════════════════════════════════════

    impl<T: Config> Pallet<T> {
        fn attester_set_is_valid(keys: &[[u8; 32]], threshold: u32) -> bool {
            if keys.len() > x3_verification_router::header_finality::MAX_ATTESTERS {
                return false;
            }
            if keys.is_empty() {
                return threshold == 0;
            }
            if threshold == 0 || threshold as usize > keys.len() {
                return false;
            }
            for (index, key) in keys.iter().enumerate() {
                if key.iter().all(|byte| *byte == 0) || keys[..index].contains(key) {
                    return false;
                }
            }
            true
        }

        /// True iff the caller is in the storage-backed authorized-submitter set.
        /// An empty set means nobody is authorized → header path is fail-closed.
        fn ensure_authorized_submitter(who: &T::AccountId) -> DispatchResult {
            if !AuthorizedSubmitters::<T>::contains_key(who) {
                return Err(Error::<T>::NotAuthorizedSubmitter.into());
            }
            Ok(())
        }

        /// Reject heights beyond the current chain height + `MaxHeaderLookahead`.
        /// Guards against spectacular far-future values used to poison the monotonic
        /// high-water mark. NOTE: without a per-chain clock/light client the unit is
        /// the local chain height; see module docs for the residual-risk caveat.
        fn ensure_not_far_future(height: u64) -> DispatchResult {
            let now = frame_system::Pallet::<T>::block_number().saturated_into::<u64>();
            let horizon = now.saturating_add(T::MaxHeaderLookahead::get());
            if height > horizon {
                return Err(Error::<T>::FarFutureHeader.into());
            }
            Ok(())
        }

        /// Split raw proof bytes into 32-byte Merkle leaves.
        fn proof_to_leaves(proof: &[u8]) -> Result<sp_std::vec::Vec<H256>, Error<T>> {
            let mut leaves = sp_std::vec::Vec::with_capacity(proof.len() / 32);
            for chunk in proof.chunks_exact(32) {
                let mut hash = [0u8; 32];
                hash.copy_from_slice(chunk);
                // A leaf that is entirely zero cannot be a meaningful commitment; the
                // Merkle layer below would otherwise accept a blank tree.
                if hash.iter().all(|b| *b == 0) {
                    return Err(Error::<T>::MalformedProofData);
                }
                leaves.push(H256::from(hash));
            }
            Ok(leaves)
        }

        /// Standard balanced binary Merkle root over the given 32-byte leaves. For a
        /// single leaf the root is that leaf. Odd nodes are carried up (clone). This
        /// is real recomputation and is bound to the claimed root by the caller of the
        /// extrinsic (an `_expected_root`-style mismatch is rejected).
        fn merkle_root_of(leaves: &[H256]) -> H256 {
            use sp_std::vec;
            if leaves.is_empty() {
                // Callers reject empty/`<32` vectors earlier; keep total function for
                // internal use but this arm should be unreachable in production paths.
                return H256::zero();
            }
            let mut level: sp_std::vec::Vec<H256> = leaves.to_vec();
            while level.len() > 1 {
                let mut next: sp_std::vec::Vec<H256> = vec::Vec::with_capacity(level.len() / 2 + 1);
                let mut i = 0;
                while i < level.len() {
                    let left = level[i];
                    let right = if i + 1 < level.len() {
                        level[i + 1]
                    } else {
                        left
                    };
                    let mut concat = [0u8; 64];
                    concat[..32].copy_from_slice(left.as_bytes());
                    concat[32..].copy_from_slice(right.as_bytes());
                    next.push(H256::from(sp_io::hashing::blake2_256(&concat)));
                    i += 2;
                }
                level = next;
            }
            level[0]
        }

        /// Validate SVM set entries: each 32-byte entry must be non-zero and the set
        /// must contain no duplicates (no self-selected/duplicate "validators").
        fn validator_entries_well_formed(set: &[u8]) -> Result<bool, Error<T>> {
            let count = set.len() / 32;
            if count == 0 {
                return Ok(false);
            }
            let entries: sp_std::vec::Vec<[u8; 32]> = set
                .chunks_exact(32)
                .map(|c| {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(c);
                    arr
                })
                .collect();
            for (i, e) in entries.iter().enumerate() {
                if e.iter().all(|b| *b == 0) {
                    return Ok(false); // zero = blank/self-unset signer
                }
                if entries[..i].contains(e) {
                    return Ok(false); // duplicate signer
                }
            }
            Ok(true)
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // RPC Queries
    // ═══════════════════════════════════════════════════════════════════════════════

    impl<T: Config> Pallet<T> {
        /// Query EVM header validation status
        pub fn query_evm_header_status() -> Option<EvmHeaderInfo> {
            LastEvmHeader::<T>::get()
        }

        /// Query SVM header validation status
        pub fn query_svm_header_status() -> Option<SvmHeaderInfo> {
            LastSvmHeader::<T>::get()
        }

        /// Query cross-chain validation statistics
        pub fn query_validation_statistics() -> ValidationStatistics {
            ValidationStats::<T>::get()
        }

        /// Check if an EVM merkle root is stored and verified
        pub fn is_evm_merkle_root_verified(block_number: u64, merkle_root: H256) -> bool {
            EvmMerkleRoots::<T>::get(block_number)
                .is_some_and(|stored_root| stored_root == merkle_root)
        }

        /// Check if an SVM validator set is stored and verified
        pub fn is_svm_validator_set_verified(slot: u64, validator_set_hash: H256) -> bool {
            SvmValidatorSets::<T>::get(slot)
                .is_some_and(|stored_hash| stored_hash == validator_set_hash)
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Bridge Integration (Phase 2)
    // ═══════════════════════════════════════════════════════════════════════════════
    // Settlement engine calls these methods to verify cross-chain headers before finalization

    impl<T: Config> Pallet<T> {
        /// Bridge Integration: Verify EVM header for settlement finality
        /// Settlement engine calls this before finalizing an EVM-leg settlement
        pub fn verify_settlement_evm_header(
            block_number: u64,
            block_hash: H256,
            state_root: H256,
            merkle_root: H256,
        ) -> bool {
            // Check if header exists and all fields match
            if EvmHeaderFinalitySource::<T>::get()
                != EvmHeaderFinalitySourceKind::BeaconSyncCommittee
            {
                return false;
            }
            LastFinalizedEvmHeader::<T>::get().is_some_and(|stored_header| {
                stored_header.block_number == block_number
                    && stored_header.block_hash == block_hash
                    && stored_header.state_root == state_root
                    && stored_header.merkle_root == merkle_root
            })
        }

        /// Bridge Integration: Verify SVM header for settlement finality
        /// Settlement engine calls this before finalizing an SVM-leg settlement
        pub fn verify_settlement_svm_header(
            slot: u64,
            block_hash: H256,
            state_root: H256,
            validator_set_hash: H256,
        ) -> bool {
            // Check if header exists and all fields match
            if let Some(stored_header) = LastFinalizedSvmHeader::<T>::get() {
                stored_header.slot == slot
                    && stored_header.block_hash == block_hash
                    && stored_header.state_root == state_root
                    && stored_header.validator_set_hash == validator_set_hash
            } else {
                false
            }
        }

        /// Bridge Integration: Get latest EVM header hash for settlement verification
        pub fn get_latest_evm_header_hash() -> Option<H256> {
            (EvmHeaderFinalitySource::<T>::get()
                == EvmHeaderFinalitySourceKind::BeaconSyncCommittee)
                .then(|| LastFinalizedEvmHeader::<T>::get().map(|header| header.block_hash))
                .flatten()
        }

        /// Bridge Integration: Get latest SVM header hash for settlement verification
        pub fn get_latest_svm_header_hash() -> Option<H256> {
            LastFinalizedSvmHeader::<T>::get().map(|header| header.block_hash)
        }

        /// Bridge Integration: Deposit settlement verification event
        /// Called by settlement engine after successful cross-chain validation
        pub fn deposit_settlement_verification_event(
            chain: Vec<u8>,
            _block_or_slot: u64,
            verified: bool,
        ) {
            let reason = if verified {
                b"settlement_verified".to_vec()
            } else {
                b"settlement_verification_failed".to_vec()
            };

            if !verified {
                Self::deposit_event(Event::ValidationFailed {
                    chain: chain.clone(),
                    reason,
                });
            }
        }
    }

    /// This pallet is the chain's store of finalized external headers, which is
    /// what an EVM receipt proof has to be checked against: the proof carries a
    /// header of its own, so a verifier without an anchor would be checking the
    /// receipt against the root the prover chose.
    ///
    /// Both answers come from storage written only after the configured
    /// independent attester quorum verifies a chain-bound certificate, and both
    /// fail closed when no such header exists:
    ///
    /// - `anchored_header` reads the receipts root recorded for a height. That
    ///   root is the one the settlement engine walks a receipt proof to, so a
    ///   proof for a height with no recorded root is refused rather than checked
    ///   against the one it carries.
    /// - `attested_head` is the newest finalized header's height, and it is what
    ///   confirmation depth is measured from. The proof's own idea of the head
    ///   (`current_block_number` in the wire format) is not consulted.
    impl<T: Config> x3_verification_router::evm_receipt::EvmHeaderAnchor for Pallet<T> {
        fn anchored_header(
            block_number: u64,
        ) -> Option<x3_verification_router::evm_receipt::AnchoredEvmHeader> {
            if EvmHeaderFinalitySource::<T>::get()
                != EvmHeaderFinalitySourceKind::BeaconSyncCommittee
            {
                return None;
            }
            let finalized = FinalizedEvmHeaders::<T>::get(block_number)?;
            Some(x3_verification_router::evm_receipt::AnchoredEvmHeader {
                number: block_number,
                receipts_root: finalized.merkle_root.0,
                state_root: finalized.state_root.0,
                block_hash: finalized.block_hash.0,
            })
        }

        fn attested_head() -> Option<u64> {
            (EvmHeaderFinalitySource::<T>::get()
                == EvmHeaderFinalitySourceKind::BeaconSyncCommittee)
                .then(|| LastFinalizedEvmHeader::<T>::get().map(|header| header.block_number))
                .flatten()
        }
    }
}
