#![deny(unsafe_code)]
//! # X3Chain Agent Memory Pallet
//!
//! Append-only on-chain memory for AI agents with LLM-friendly serialization.
//!
//! ## Overview
//!
//! This pallet provides:
//! - Append-only memory logs per agent
//! - Delta compression for efficient storage
//! - JSONL-like output format for LLM consumption
//! - Read/write permissions per agent
//! - Chunk-based pagination for large memories
//! - Pruning of old entries based on TTL
//!
//! ## Memory Model
//!
//! Memory is organized as:
//! - MemoryEntry: A single log entry with timestamp, type, and content
//! - MemoryChunk: A batch of entries for efficient storage/retrieval
//! - Each agent has independent memory with configurable limits

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

pub mod runtime_api;
pub use runtime_api::*;

pub mod offchain_storage;
pub use offchain_storage::*;

pub mod migrations;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ReservableCurrency},
        Blake2_128Concat,
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{Saturating, Zero, SaturatedConversion};
    use sp_std::prelude::*;

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// Type alias for agent ID.
    pub type AgentId = u32;

    use frame_support::traits::StorageVersion;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::without_storage_info]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency for storage deposits.
        type Currency: ReservableCurrency<Self::AccountId>;

        /// Maximum entries per chunk.
        #[pallet::constant]
        type MaxEntriesPerChunk: Get<u32>;

        /// Maximum chunks per agent.
        #[pallet::constant]
        type MaxChunksPerAgent: Get<u32>;

        /// Cost per byte of storage.
        #[pallet::constant]
        type StorageByteCost: Get<BalanceOf<Self>>;

        /// Default TTL in blocks.
        #[pallet::constant]
        type DefaultTtl: Get<BlockNumberFor<Self>>;

        /// Blocks to retain memory (432k = ~24 hours at 6s/block).
        #[pallet::constant]
        type MemoryRetentionBlocks: Get<BlockNumberFor<Self>>;

        /// Consensus threshold percentage (e.g., 67 = 2/3 + 1).
        #[pallet::constant]
        type MemoryConsensusThreshold: Get<u32>;

        /// Origin that can prune memory.
        type PruneOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Weight information.
        type WeightInfo: WeightInfo;
    }

    // ========================================================================
    // Storage Items
    // ========================================================================

    /// Memory chunks per agent.
    #[pallet::storage]
    #[pallet::getter(fn memory_chunks)]
    pub type MemoryChunks<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        AgentId,
        Blake2_128Concat,
        u32, // chunk_id
        MemoryChunk<BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// Current chunk ID per agent.
    #[pallet::storage]
    #[pallet::getter(fn current_chunk)]
    pub type CurrentChunk<T: Config> = StorageMap<_, Blake2_128Concat, AgentId, u32, ValueQuery>;

    /// Total entry count per agent.
    #[pallet::storage]
    #[pallet::getter(fn entry_count)]
    pub type EntryCount<T: Config> = StorageMap<_, Blake2_128Concat, AgentId, u64, ValueQuery>;

    /// Memory permissions per agent.
    #[pallet::storage]
    #[pallet::getter(fn permissions)]
    pub type MemoryPermissions<T: Config> =
        StorageMap<_, Blake2_128Concat, AgentId, MemoryAccess<T::AccountId>, ValueQuery>;

    /// Storage used per agent in bytes.
    #[pallet::storage]
    #[pallet::getter(fn storage_used)]
    pub type StorageUsed<T: Config> = StorageMap<_, Blake2_128Concat, AgentId, u64, ValueQuery>;

    /// Storage deposit per agent.
    #[pallet::storage]
    #[pallet::getter(fn storage_deposit)]
    pub type StorageDeposit<T: Config> =
        StorageMap<_, Blake2_128Concat, AgentId, BalanceOf<T>, ValueQuery>;

    /// Agent controller mapping (from agent-accounts).
    #[pallet::storage]
    #[pallet::getter(fn agent_controller)]
    pub type AgentController<T: Config> =
        StorageMap<_, Blake2_128Concat, AgentId, T::AccountId, OptionQuery>;

    /// Agent operator mapping (from agent-accounts).
    #[pallet::storage]
    #[pallet::getter(fn agent_operator)]
    pub type AgentOperator<T: Config> =
        StorageMap<_, Blake2_128Concat, AgentId, T::AccountId, OptionQuery>;

    /// Latest memory hash per agent (Tier 2 offchain index tracking).
    #[pallet::storage]
    #[pallet::getter(fn latest_memory_hash)]
    pub type LatestMemoryHash<T: Config> =
        StorageMap<_, Blake2_128Concat, AgentId, sp_core::H256, OptionQuery>;

    /// Memory consensus records: agent_id -> block -> consensus status.
    #[pallet::storage]
    pub type MemoryConsensusRecords<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        AgentId,
        Blake2_128Concat,
        BlockNumberFor<T>,
        (sp_core::H256, u32), // (consensus_hash, attestations_count)
        OptionQuery,
    >;

    /// Last block where offchain worker executed (for dedup).
    #[pallet::storage]
    pub type LastOffchainUpdateBlock<T: Config> =
        StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    // ========================================================================
    // Events
    // ========================================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Memory entry was appended.
        EntryAppended {
            agent_id: AgentId,
            entry_id: u64,
            entry_type: EntryType,
            size: u32,
        },
        /// Memory chunk was finalized.
        ChunkFinalized {
            agent_id: AgentId,
            chunk_id: u32,
            entries: u32,
        },
        /// Memory was pruned.
        MemoryPruned {
            agent_id: AgentId,
            chunks_removed: u32,
            bytes_freed: u64,
        },
        /// Memory permissions were updated.
        PermissionsUpdated { agent_id: AgentId },
        /// Agent memory was initialized.
        MemoryInitialized {
            agent_id: AgentId,
            controller: T::AccountId,
            operator: T::AccountId,
        },
        /// Deposit was increased.
        DepositIncreased {
            agent_id: AgentId,
            amount: BalanceOf<T>,
        },
        /// Deposit was withdrawn.
        DepositWithdrawn {
            agent_id: AgentId,
            amount: BalanceOf<T>,
        },
        /// Memory snapshot was indexed by offchain worker.
        MemoryIndexed {
            agent_id: AgentId,
            block_number: BlockNumberFor<T>,
            memory_hash: sp_core::H256,
        },
        /// Memory consensus was reached across validators.
        MemoryConsensusReached {
            agent_id: AgentId,
            block_number: BlockNumberFor<T>,
            attestations: u32,
            consensus_hash: sp_core::H256,
        },
        /// Offchain memory cleanup executed (Tier 2 pruning).
        OffchainMemoryPruned {
            agent_id: AgentId,
            block_number: BlockNumberFor<T>,
            blocks_pruned: u32,
        },
    }

    // ========================================================================
    // Errors
    // ========================================================================

    #[pallet::error]
    pub enum Error<T> {
        /// Agent not found.
        AgentNotFound,
        /// Memory not initialized.
        MemoryNotInitialized,
        /// No permission to write.
        WritePermissionDenied,
        /// No permission to read.
        ReadPermissionDenied,
        /// Content too long.
        ContentTooLong,
        /// Too many chunks.
        TooManyChunks,
        /// Chunk not found.
        ChunkNotFound,
        /// Insufficient deposit.
        InsufficientDeposit,
        /// Not controller.
        NotController,
        /// Not operator.
        NotOperator,
        /// Invalid entry type.
        InvalidEntryType,
        /// Arithmetic overflow.
        ArithmeticOverflow,
    }

    // ========================================================================
    // Extrinsics
    // ========================================================================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Initialize memory for an agent.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::initialize_memory())]
        pub fn initialize_memory(
            origin: OriginFor<T>,
            agent_id: AgentId,
            operator: T::AccountId,
        ) -> DispatchResult {
            let controller = ensure_signed(origin)?;

            ensure!(
                !AgentController::<T>::contains_key(agent_id),
                Error::<T>::AgentNotFound
            );

            // Store mappings
            AgentController::<T>::insert(agent_id, controller.clone());
            AgentOperator::<T>::insert(agent_id, operator.clone());

            // Initialize default permissions
            let permissions = MemoryAccess::default();
            MemoryPermissions::<T>::insert(agent_id, permissions);

            // Initialize first chunk
            CurrentChunk::<T>::insert(agent_id, 0);

            let chunk = MemoryChunk {
                id: 0,
                entries: BoundedVec::default(),
                created_at: frame_system::Pallet::<T>::block_number(),
                finalized: false,
                hash: None,
            };
            MemoryChunks::<T>::insert(agent_id, 0, chunk);

            Self::deposit_event(Event::MemoryInitialized {
                agent_id,
                controller,
                operator,
            });

            Ok(())
        }

        /// Append a memory entry.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::append_entry())]
        pub fn append_entry(
            origin: OriginFor<T>,
            agent_id: AgentId,
            entry_type: EntryType,
            content: BoundedVec<u8, ConstU32<4096>>,
            metadata: Option<BoundedVec<u8, ConstU32<256>>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify write permission
            Self::ensure_write_permission(agent_id, &who)?;

            // Verify content length
            ensure!(content.len() <= 4096, Error::<T>::ContentTooLong);

            let current_block = frame_system::Pallet::<T>::block_number();
            let entry_id = EntryCount::<T>::get(agent_id);

            let entry = MemoryEntry {
                id: entry_id,
                entry_type,
                content: content.clone(),
                metadata,
                timestamp: current_block,
                ttl: Some(current_block.saturating_add(T::DefaultTtl::get())),
            };

            // Calculate storage cost
            let entry_size = Self::calculate_entry_size(&entry);
            Self::charge_storage(agent_id, entry_size)?;

            // Get current chunk
            let chunk_id = CurrentChunk::<T>::get(agent_id);

            MemoryChunks::<T>::try_mutate(agent_id, chunk_id, |maybe_chunk| -> DispatchResult {
                let chunk = maybe_chunk.as_mut().ok_or(Error::<T>::ChunkNotFound)?;

                // Check if chunk is full
                if chunk.entries.len() >= T::MaxEntriesPerChunk::get() as usize {
                    // Finalize current chunk
                    chunk.finalized = true;
                    chunk.hash = Some(Self::compute_chunk_hash(chunk));

                    Self::deposit_event(Event::ChunkFinalized {
                        agent_id,
                        chunk_id,
                        entries: chunk.entries.len() as u32,
                    });

                    // Create new chunk
                    let new_chunk_id = chunk_id.saturating_add(1);
                    ensure!(
                        new_chunk_id < T::MaxChunksPerAgent::get(),
                        Error::<T>::TooManyChunks
                    );

                    let mut new_entries = BoundedVec::default();
                    // This try_push should always succeed since we just created a new chunk
                    let _ = new_entries.try_push(entry.clone());

                    let new_chunk = MemoryChunk {
                        id: new_chunk_id,
                        entries: new_entries,
                        created_at: current_block,
                        finalized: false,
                        hash: None,
                    };

                    // Insert new chunk in separate storage call
                    CurrentChunk::<T>::insert(agent_id, new_chunk_id);
                    MemoryChunks::<T>::insert(agent_id, new_chunk_id, new_chunk);
                } else {
                    // try_push returns Err if full, but we checked len above
                    let _ = chunk.entries.try_push(entry.clone());
                }

                Ok(())
            })?;

            // Update entry count
            EntryCount::<T>::insert(agent_id, entry_id.saturating_add(1));

            // Update storage used
            StorageUsed::<T>::mutate(agent_id, |used| {
                *used = used.saturating_add(entry_size as u64);
            });

            Self::deposit_event(Event::EntryAppended {
                agent_id,
                entry_id,
                entry_type,
                size: entry_size as u32,
            });

            Ok(())
        }

        /// Append a batch of entries (more efficient).
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::append_batch())]
        pub fn append_batch(
            origin: OriginFor<T>,
            agent_id: AgentId,
            entries: Vec<(EntryType, BoundedVec<u8, ConstU32<4096>>)>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            Self::ensure_write_permission(agent_id, &who)?;

            let current_block = frame_system::Pallet::<T>::block_number();
            let mut entry_id = EntryCount::<T>::get(agent_id);
            let mut total_size: u64 = 0;

            for (entry_type, content) in entries {
                let entry = MemoryEntry {
                    id: entry_id,
                    entry_type,
                    content: content.clone(),
                    metadata: None,
                    timestamp: current_block,
                    ttl: Some(current_block.saturating_add(T::DefaultTtl::get())),
                };

                let entry_size = Self::calculate_entry_size(&entry);
                total_size = total_size.saturating_add(entry_size as u64);

                let chunk_id = CurrentChunk::<T>::get(agent_id);

                MemoryChunks::<T>::try_mutate(
                    agent_id,
                    chunk_id,
                    |maybe_chunk| -> DispatchResult {
                        let chunk = maybe_chunk.as_mut().ok_or(Error::<T>::ChunkNotFound)?;

                        if chunk.entries.len() >= T::MaxEntriesPerChunk::get() as usize {
                            chunk.finalized = true;
                            let new_chunk_id = chunk_id.saturating_add(1);
                            ensure!(
                                new_chunk_id < T::MaxChunksPerAgent::get(),
                                Error::<T>::TooManyChunks
                            );

                            let mut new_entries = BoundedVec::default();
                            let _ = new_entries.try_push(entry.clone());

                            let new_chunk = MemoryChunk {
                                id: new_chunk_id,
                                entries: new_entries,
                                created_at: current_block,
                                finalized: false,
                                hash: None,
                            };

                            CurrentChunk::<T>::insert(agent_id, new_chunk_id);
                            MemoryChunks::<T>::insert(agent_id, new_chunk_id, new_chunk);
                        } else {
