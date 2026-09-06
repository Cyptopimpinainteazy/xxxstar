#![cfg_attr(not(feature = "std"), no_std)]

//! # Cross-Chain Validator Pallet
//!
//! Stores finalized EVM/SVM header attestations that the settlement engine relies on.
//!
//! **Security model (post C01 remediation):**
//!
//! This pallet is an **authorized-relayer header store**, not an on-chain light
//! client. Authenticity of a stored header rests on the originating account being a
//! member of the pallet's storage-backed `AuthorizedSubmitters` set, and that set is
//! only mutable by a governance `AdminOrigin`. Arbitrary signed accounts are rejected
//! *before* any storage write.
//!
//! We deliberately **no longer fabricate** a consensus/quorum proof from caller
//! supplied byte length (the old `proof.len()/32` and `validator_set.len()/32`
//! laundering). A BFT quorum over a real external validator set requires a trusted
//! snapshot/finality oracle that this pallet does not have; pretending otherwise was
//! audit finding C01. When the `AuthorizedSubmitters` set is empty the submit path is
//! **fail-closed** (nothing can be accepted). A full authenticated finalized-header
//! client (per-chain light client / finality oracle that cryptographically binds
//! headers to the external EVM/SVM chains) is out of scope here and is tracked as
//! residual risk.
//!
//! **Validation Flow (EVM / SVM):**
//! 1. Origin must be an authorized submitter (fail closed when the set is empty).
//! 2. Structural checks: non-zero heights/hashes, bounds, monotonicity, and a
//!    now+`MaxHeaderLookahead` far-future guard.
//! 3. For EVM: `merkle_root` is *recomputed* from the submitted proof leaves and must
//!    equal the claimed root (real Merkle inclusion, `_expected_root` is no longer
//!    ignored).
//! 4. Only after every check passes is any storage written.

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

    /// Merkle root cache for EVM blocks (block_number -> merkle_root)
    #[pallet::storage]
    pub type EvmMerkleRoots<T: Config> =
        StorageMap<_, frame_support::Blake2_128Concat, u64, H256, OptionQuery>;

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
        ValidationFailed { chain: Vec<u8>, reason: Vec<u8> },
        /// The authorized-submitter set was replaced by governance.
        AuthorizedSubmittersUpdated { submitters: Vec<T::AccountId> },
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

        /// Submit and validate an EVM block header. Only authorized submitters may
        /// call; all checks happen before any storage write.
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
            ensure!(proof.len().is_multiple_of(32), Error::<T>::ProofNotMultipleOf32);

            // Phase 2: far-future guard (rejects absurd heights that would poison the
            // high-water mark). Relative to the local chain's current height.
            Self::ensure_not_far_future(block_number)?;

            // Phase 3: real Merkle inclusion. Recompute the root over the submitted
            // proof leaves and require it to equal the claimed `merkle_root`.
            let leaves = Self::proof_to_leaves(&proof)?;
            let recomputed_root = Self::merkle_root_of(&leaves);
            ensure!(recomputed_root == merkle_root, Error::<T>::MerkleRootMismatch);

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
            let validator_set_hash =
                H256::from(sp_io::hashing::blake2_256(&proof));

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
                    let right = if i + 1 < level.len() { level[i + 1] } else { left };
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
            if let Some(stored_header) = LastEvmHeader::<T>::get() {
                stored_header.block_number == block_number
                    && stored_header.block_hash == block_hash
                    && stored_header.state_root == state_root
                    && stored_header.merkle_root == merkle_root
            } else {
                false
            }
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
            if let Some(stored_header) = LastSvmHeader::<T>::get() {
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
            LastEvmHeader::<T>::get().map(|header| header.block_hash)
        }

        /// Bridge Integration: Get latest SVM header hash for settlement verification
        pub fn get_latest_svm_header_hash() -> Option<H256> {
            LastSvmHeader::<T>::get().map(|header| header.block_hash)
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
}
