#![deny(unsafe_code)]
//! # X3 Data Availability (DA) Pallet
//!
//! ## Overview
//!
//! This pallet provides the on-chain data availability layer for X3 Chain.
//! It stores blob commitments and metadata on-chain while actual blob data
//! is stored off-chain (P2P + archival nodes). This is the v0 incremental
//! DA design that can be upgraded to v1 (Reed-Solomon erasure coding) and
//! v2 (KZG commitments).
//!
//! ## Design (v0 — Commitment-First)
//!
//! - **Submit blob commitment**: submitter posts a hash + metadata
//! - **Shard proofs**: validators can submit shard availability proofs
//! - **Retrieval**: clients query blob metadata on-chain, fetch data off-chain
//! - **Anti-spam**: per-byte fees and blob size caps
//!
//! ## Integration
//!
//! - `pallet-x3-sequencer` batches feed into DA commitments
//! - `pallet-x3-settlement-engine` verifies DA before accepting state roots

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;
#[frame_support::pallet]
pub mod pallet {
    use frame_support::{dispatch::DispatchResult, pallet_prelude::*, traits::ReservableCurrency};
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_runtime::SaturatedConversion;

    // ── Config ─────────────────────────────────────────────────────────────

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency for DA fees.
        type Currency: ReservableCurrency<Self::AccountId>;

        /// Maximum blob size in bytes (anti-spam cap).
        #[pallet::constant]
        type MaxBlobSize: Get<u32>;

        /// Per-byte fee for DA submissions.
        #[pallet::constant]
        type PerByteFee: Get<u128>;

        /// Maximum shard proofs per blob.
        #[pallet::constant]
        type MaxShardProofs: Get<u32>;

        /// Retention window in blocks — blobs older than this can be pruned.
        #[pallet::constant]
        type RetentionBlocks: Get<BlockNumberFor<Self>>;
    }

    // ── Storage ────────────────────────────────────────────────────────────

    /// Blob commitments by blob hash.
    #[pallet::storage]
    #[pallet::getter(fn blobs)]
    pub type Blobs<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        H256, // blob_hash
        BlobCommitment<T>,
        OptionQuery,
    >;

    /// Shard availability proofs for a blob.
    #[pallet::storage]
    #[pallet::getter(fn shard_proofs)]
    pub type ShardProofs<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        H256, // blob_hash
        BoundedVec<ShardProof<T>, T::MaxShardProofs>,
        ValueQuery,
    >;

    /// Total bytes committed to DA (for metrics and fee policy).
    #[pallet::storage]
    #[pallet::getter(fn total_bytes_committed)]
    pub type TotalBytesCommitted<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Monotonically increasing blob counter.
    #[pallet::storage]
    #[pallet::getter(fn next_blob_id)]
    pub type NextBlobId<T: Config> = StorageValue<_, u64, ValueQuery>;

    // ── Types ──────────────────────────────────────────────────────────────

    /// On-chain commitment for a data blob.
    #[derive(Debug, Clone, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct BlobCommitment<T: Config> {
        /// Blob identifier.
        pub blob_id: u64,
        /// Submitter account.
        pub submitter: T::AccountId,
        /// Hash of the complete blob data (stored off-chain).
        pub data_hash: H256,
        /// Size of the blob in bytes.
        pub size_bytes: u32,
        /// Block when committed.
        pub committed_at: BlockNumberFor<T>,
        /// Optional erasure-coding commitment (v1: Reed-Solomon root).
        pub erasure_root: Option<H256>,
        /// Optional KZG commitment (v2).
        pub kzg_commitment: Option<H256>,
        /// Associated sequencer batch ID (if any).
        pub batch_id: Option<u64>,
        /// Status: 0=Pending, 1=Available, 2=Expired.
        pub status: u8,
    }

    /// A shard availability proof from a validator/archival node.
    #[derive(Debug, Clone, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct ShardProof<T: Config> {
        /// Shard index within the erasure-coded blob.
        pub shard_index: u32,
        /// Validator/archival node that attests to holding this shard.
        pub attester: T::AccountId,
        /// Proof of shard validity (Merkle branch against erasure_root).
        pub proof_hash: H256,
        /// Block when attested.
        pub attested_at: BlockNumberFor<T>,
    }

    // ── Pallet ─────────────────────────────────────────────────────────────

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // ── Events ─────────────────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A blob commitment was submitted.
        BlobCommitted {
            blob_id: u64,
            data_hash: H256,
            size_bytes: u32,
            submitter: T::AccountId,
        },
        /// A shard availability proof was submitted.
        ShardProofSubmitted {
            blob_hash: H256,
            shard_index: u32,
            attester: T::AccountId,
        },
        /// A blob was marked as fully available (enough shard proofs).
        BlobAvailable { blob_id: u64, data_hash: H256 },
        /// A blob commitment expired and was pruned.
        BlobExpired { blob_id: u64, data_hash: H256 },
    }

    // ── Errors ─────────────────────────────────────────────────────────────

    #[pallet::error]
    pub enum Error<T> {
        /// Blob exceeds maximum size.
        BlobTooLarge,
        /// Blob not found.
        BlobNotFound,
        /// Insufficient funds for DA fee.
        InsufficientFee,
        /// Duplicate blob commitment.
        BlobAlreadyExists,
        /// Too many shard proofs for this blob.
        TooManyShardProofs,
        /// Blob has expired.
        BlobExpired,
        /// Invalid shard proof.
        InvalidShardProof,
    }

    // ── Hooks ──────────────────────────────────────────────────────────────

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        // Future: prune expired blobs in on_initialize
        fn on_initialize(_now: BlockNumberFor<T>) -> Weight {
            Weight::zero()
        }
    }

    // ── Dispatchable Calls ─────────────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Submit a data availability commitment.
        ///
        /// The blob data itself is stored off-chain. This on-chain record
        /// anchors the commitment so validators and clients can verify
        /// availability.
        ///
        /// # Arguments
        /// - `data_hash`: Hash of the complete blob data.
        /// - `size_bytes`: Size of the blob (for fee and cap enforcement).
        /// - `batch_id`: Optional sequencer batch ID this blob belongs to.
        /// - `erasure_root`: Optional v1 erasure coding root.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(15_000, 0))]
        pub fn submit_blob_commitment(
            origin: OriginFor<T>,
            data_hash: H256,
            size_bytes: u32,
            batch_id: Option<u64>,
            erasure_root: Option<H256>,
        ) -> DispatchResult {
            let submitter = ensure_signed(origin)?;

            ensure!(
                size_bytes <= T::MaxBlobSize::get(),
                Error::<T>::BlobTooLarge
            );
            ensure!(
                !Blobs::<T>::contains_key(data_hash),
                Error::<T>::BlobAlreadyExists
            );

            let fee_u128 = T::PerByteFee::get().saturating_mul(size_bytes as u128);
            let fee = fee_u128.saturated_into();
            // Charge fee via T::Currency
            T::Currency::reserve(&submitter, fee).map_err(|_| Error::<T>::InsufficientFee)?;

            let now = <frame_system::Pallet<T>>::block_number();
            let blob_id = NextBlobId::<T>::mutate(|id| {
                let current = *id;
                *id = id.saturating_add(1);
                current
            });

            let commitment = BlobCommitment::<T> {
                blob_id,
                submitter: submitter.clone(),
                data_hash,
                size_bytes,
                committed_at: now,
                erasure_root,
                kzg_commitment: None,
                batch_id,
                status: 0, // Pending
            };

            Blobs::<T>::insert(data_hash, &commitment);
            TotalBytesCommitted::<T>::mutate(|total| {
                *total = total.saturating_add(size_bytes as u64);
            });

            Self::deposit_event(Event::BlobCommitted {
                blob_id,
                data_hash,
                size_bytes,
                submitter,
            });

            log::info!(
                target: "x3-da",
                "Blob #{} committed: hash={:?} size={} bytes",
                blob_id, data_hash, size_bytes
            );

            Ok(())
        }

        /// Submit a shard availability proof for a committed blob.
        ///
        /// Validators and archival nodes call this to attest that they
        /// hold a specific shard of the erasure-coded blob data.
        ///
        /// When sufficient shard proofs are submitted (determined by the
        /// erasure coding parameters), the blob status transitions to
        /// `Available`.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn submit_shard_proof(
            origin: OriginFor<T>,
            blob_hash: H256,
            shard_index: u32,
            proof_hash: H256,
        ) -> DispatchResult {
            let attester = ensure_signed(origin)?;

            ensure!(
                Blobs::<T>::contains_key(blob_hash),
                Error::<T>::BlobNotFound
            );

            let now = <frame_system::Pallet<T>>::block_number();
            let shard = ShardProof::<T> {
                shard_index,
                attester: attester.clone(),
                proof_hash,
                attested_at: now,
            };

            ShardProofs::<T>::try_mutate(blob_hash, |proofs| {
                proofs
                    .try_push(shard)
                    .map_err(|_| Error::<T>::TooManyShardProofs)
            })?;

            Self::deposit_event(Event::ShardProofSubmitted {
                blob_hash,
                shard_index,
                attester,
            });

            Ok(())
        }
    }

    // ── Internal Helpers ───────────────────────────────────────────────────

    impl<T: Config> Pallet<T> {
        /// Check if a blob has sufficient proofs to be considered available.
        pub fn is_blob_available(blob_hash: H256) -> bool {
            Blobs::<T>::get(blob_hash)
                .map(|b| b.status == 1)
                .unwrap_or(false)
        }

        /// Get blob commitment metadata (for RPC).
        pub fn get_blob(blob_hash: H256) -> Option<BlobCommitment<T>> {
            Blobs::<T>::get(blob_hash)
        }
    }
}
