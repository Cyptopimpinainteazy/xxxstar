#![deny(unsafe_code)]
//! # X3 Sequencer Pallet
//!
//! ## Overview
//!
//! This pallet implements a **shared sequencer** for the X3 Chain. It accepts
//! transactions from rollups and external chains, batches them with a Merkle
//! root, and emits finalization events that settlement and DA layers can anchor.
//!
//! ## Design
//!
//! - **Submission**: `submit_transaction(payload, source_chain)` stores a
//!   `SequencedTx` in the current pending batch.
//! - **Batching**: On each block, pending transactions are batched into a
//!   `SequencingBatch` with a deterministic Merkle root.
//! - **Finalization**: When the batch is sealed (by block finalization),
//!   `BatchFinalized` event is emitted with the root.
//! - **Fees**: Per-byte fee enforcement prevents DA spam.
//!
//! ## Integration
//!
//! The sequencer pallet is designed to be composed with:
//! - `pallet-x3-settlement-engine` for state root verification
//! - `pallet-x3-da` for data availability commitments

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

        /// Currency for sequencing fees.
        type Currency: ReservableCurrency<Self::AccountId>;

        /// Maximum transactions per batch.
        #[pallet::constant]
        type MaxTxsPerBatch: Get<u32>;

        /// Maximum payload size per sequenced transaction (bytes).
        #[pallet::constant]
        type MaxPayloadSize: Get<u32>;

        /// Per-byte fee for sequencing (anti-spam).
        #[pallet::constant]
        type PerByteFee: Get<u128>;

        /// Minimum base fee per transaction.
        #[pallet::constant]
        type BaseFee: Get<u128>;
    }

    // ── Storage ────────────────────────────────────────────────────────────

    /// Current pending batch — transactions waiting to be sealed.
    #[pallet::storage]
    #[pallet::getter(fn pending_txs)]
    pub type PendingTxs<T: Config> =
        StorageValue<_, BoundedVec<SequencedTx, T::MaxTxsPerBatch>, ValueQuery>;

    /// Monotonically increasing batch counter.
    #[pallet::storage]
    #[pallet::getter(fn next_batch_id)]
    pub type NextBatchId<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Finalized batches by batch ID.
    /// Uses `OptionQuery` with no `MaxEncodedLen` constraint by setting
    /// `#[pallet::without_storage_info]`.
    #[pallet::storage]
    #[pallet::getter(fn batches)]
    pub type Batches<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u64, // batch_id
        SequencingBatch,
        OptionQuery,
    >;

    /// Transaction count across all batches (global sequence number).
    #[pallet::storage]
    #[pallet::getter(fn global_sequence)]
    pub type GlobalSequence<T: Config> = StorageValue<_, u64, ValueQuery>;

    // ── Types ──────────────────────────────────────────────────────────────

    /// A single sequenced transaction.
    #[derive(Debug, Clone, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo)]
    pub struct SequencedTx {
        /// Global sequence number (monotonically increasing).
        pub sequence: u64,
        /// Source chain identifier (0 = X3 native, 1+ = rollup IDs).
        pub source_chain: u32,
        /// Hash of the transaction payload.
        pub payload_hash: H256,
        /// Payload size in bytes (for fee calculation).
        pub payload_size: u32,
        /// Block number when submitted.
        pub submitted_at: u32,
    }

    /// A finalized batch of sequenced transactions.
    /// Uses concrete types to enable `MaxEncodedLen` derivation.
    #[derive(Debug, Clone, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo)]
    pub struct SequencingBatch {
        /// Batch identifier.
        pub batch_id: u64,
        /// Merkle root of all sequenced tx hashes in this batch.
        pub merkle_root: H256,
        /// Number of transactions in this batch.
        pub tx_count: u32,
        /// Block number where this batch was sealed.
        pub sealed_at: u32,
        /// Total bytes sequenced in this batch.
        pub total_bytes: u64,
    }

    // ── Pallet ─────────────────────────────────────────────────────────────

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // ── Events ─────────────────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A transaction was submitted for sequencing.
        TransactionSequenced {
            sequence: u64,
            source_chain: u32,
            payload_hash: H256,
            submitter: T::AccountId,
        },
        /// A batch was sealed and finalized.
        BatchFinalized {
            batch_id: u64,
            merkle_root: H256,
            tx_count: u32,
            sealed_at: u32,
        },
    }

    // ── Errors ─────────────────────────────────────────────────────────────

    #[pallet::error]
    pub enum Error<T> {
        /// Batch is full — wait for next block.
        BatchFull,
        /// Payload exceeds maximum size.
        PayloadTooLarge,
        /// Insufficient funds for sequencing fee.
        InsufficientFee,
        /// Invalid source chain identifier.
        InvalidSourceChain,
    }

    // ── Hooks ──────────────────────────────────────────────────────────────

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// At end of each block, seal the pending batch into a finalized batch.
        fn on_finalize(now: BlockNumberFor<T>) {
            let pending = PendingTxs::<T>::take();
            if pending.is_empty() {
                return;
            }

            let batch_id = NextBatchId::<T>::mutate(|id| {
                let current = *id;
                *id = id.saturating_add(1);
                current
            });

            // Compute Merkle root of all tx hashes in the batch
            let hashes: sp_std::vec::Vec<H256> = pending.iter().map(|tx| tx.payload_hash).collect();
            let merkle_root = Self::compute_merkle_root(&hashes);
            let total_bytes: u64 = pending.iter().map(|tx| tx.payload_size as u64).sum();
            let sealed_at: u32 = now.try_into().unwrap_or(0u32);

            let batch = SequencingBatch {
                batch_id,
                merkle_root,
                tx_count: pending.len() as u32,
                sealed_at,
                total_bytes,
            };

            Batches::<T>::insert(batch_id, &batch);

            Self::deposit_event(Event::BatchFinalized {
                batch_id,
                merkle_root,
                tx_count: pending.len() as u32,
                sealed_at,
            });

            log::info!(
                target: "x3-sequencer",
                "Batch #{} sealed: {} txs, root={:?}",
                batch_id, pending.len(), merkle_root
            );
        }
    }

    // ── Dispatchable Calls ─────────────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Submit a transaction for sequencing.
        ///
        /// The caller pays `BaseFee + payload_size * PerByteFee`.
        ///
        /// # Arguments
        /// - `payload_hash`: Hash of the transaction payload (stored off-chain).
        /// - `payload_size`: Size of the payload in bytes (for fee + DA accounting).
        /// - `source_chain`: Rollup/chain ID (0 = native X3).
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn submit_transaction(
            origin: OriginFor<T>,
            payload_hash: H256,
            payload_size: u32,
            source_chain: u32,
        ) -> DispatchResult {
            let submitter = ensure_signed(origin)?;

            ensure!(
                payload_size <= T::MaxPayloadSize::get(),
                Error::<T>::PayloadTooLarge
            );

            // Fee calculation: base + per-byte
            let fee_u128 = T::BaseFee::get()
                .saturating_add(T::PerByteFee::get().saturating_mul(payload_size as u128));
            let fee = fee_u128.saturated_into();

            // Charge fee via T::Currency
            T::Currency::reserve(&submitter, fee).map_err(|_| Error::<T>::InsufficientFee)?;

            let now = <frame_system::Pallet<T>>::block_number();
            let sequence = GlobalSequence::<T>::mutate(|seq| {
                let current = *seq;
                *seq = seq.saturating_add(1);
                current
            });

            let tx = SequencedTx {
                sequence,
                source_chain,
                payload_hash,
                payload_size,
                submitted_at: now.try_into().unwrap_or(0u32),
            };

            PendingTxs::<T>::try_mutate(|pending| {
                pending.try_push(tx).map_err(|_| Error::<T>::BatchFull)
            })?;

            Self::deposit_event(Event::TransactionSequenced {
                sequence,
                source_chain,
                payload_hash,
                submitter,
            });

            Ok(())
        }
    }

    // ── Internal Helpers ───────────────────────────────────────────────────

    impl<T: Config> Pallet<T> {
        /// Compute a simple binary Merkle root over ordered hashes.
        fn compute_merkle_root(hashes: &[H256]) -> H256 {
            if hashes.is_empty() {
                return H256::zero();
            }
            if hashes.len() == 1 {
                return hashes[0];
            }

            let mut current: sp_std::vec::Vec<H256> = hashes.to_vec();
            while current.len() > 1 {
                if current.len() % 2 == 1 {
                    let last = *current.last().unwrap();
                    current.push(last);
                }
                current = current
                    .chunks(2)
                    .map(|pair| {
                        let mut data = [0u8; 64];
                        data[..32].copy_from_slice(pair[0].as_bytes());
                        data[32..].copy_from_slice(pair[1].as_bytes());
                        H256(sp_io::hashing::blake2_256(&data))
                    })
                    .collect();
            }
            current[0]
        }

        /// Get a batch by ID (for RPC).
        pub fn get_batch(batch_id: u64) -> Option<SequencingBatch> {
            Batches::<T>::get(batch_id)
        }

        /// Get current pending transaction count.
        pub fn pending_count() -> u32 {
            PendingTxs::<T>::get().len() as u32
        }
    }
}
