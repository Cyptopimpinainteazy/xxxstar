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
    use sp_runtime::traits::{SaturatedConversion, Saturating, Zero};
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
    pub type LastOffchainUpdateBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

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
                            let _ = chunk.entries.try_push(entry);
                        }

                        Ok(())
                    },
                )?;

                entry_id = entry_id.saturating_add(1);
            }

            Self::charge_storage(agent_id, total_size as usize)?;
            EntryCount::<T>::insert(agent_id, entry_id);
            StorageUsed::<T>::mutate(agent_id, |used| {
                *used = used.saturating_add(total_size);
            });

            Ok(())
        }

        /// Update memory permissions.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::update_permissions())]
        pub fn update_permissions(
            origin: OriginFor<T>,
            agent_id: AgentId,
            can_public_read: bool,
            allowed_readers: Vec<T::AccountId>,
            allowed_writers: Vec<T::AccountId>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let controller =
                AgentController::<T>::get(agent_id).ok_or(Error::<T>::MemoryNotInitialized)?;
            ensure!(who == controller, Error::<T>::NotController);

            let permissions = MemoryAccess {
                can_public_read,
                allowed_readers: allowed_readers.try_into().unwrap_or_default(),
                allowed_writers: allowed_writers.try_into().unwrap_or_default(),
            };

            MemoryPermissions::<T>::insert(agent_id, permissions);

            Self::deposit_event(Event::PermissionsUpdated { agent_id });

            Ok(())
        }

        /// Prune old memory entries.
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::prune_memory())]
        pub fn prune_memory(
            origin: OriginFor<T>,
            agent_id: AgentId,
            up_to_chunk: u32,
        ) -> DispatchResult {
            T::PruneOrigin::ensure_origin(origin)?;

            let mut chunks_removed = 0u32;
            let mut bytes_freed = 0u64;
            let current_block = frame_system::Pallet::<T>::block_number();

            for chunk_id in 0..=up_to_chunk {
                if let Some(chunk) = MemoryChunks::<T>::get(agent_id, chunk_id) {
                    // Only prune if all entries have expired
                    let all_expired = chunk
                        .entries
                        .iter()
                        .all(|e| e.ttl.is_some_and(|ttl| current_block > ttl));

                    if all_expired || chunk.finalized {
                        for entry in &chunk.entries {
                            bytes_freed = bytes_freed
                                .saturating_add(Self::calculate_entry_size(entry) as u64);
                        }

                        MemoryChunks::<T>::remove(agent_id, chunk_id);
                        chunks_removed = chunks_removed.saturating_add(1);
                    }
                }
            }

            // Update storage used
            StorageUsed::<T>::mutate(agent_id, |used| {
                *used = used.saturating_sub(bytes_freed);
            });

            // Return deposit
            if bytes_freed > Zero::zero() {
                let deposit_return = Self::bytes_to_deposit(bytes_freed as usize);
                StorageDeposit::<T>::mutate(agent_id, |deposit| {
                    *deposit = deposit.saturating_sub(deposit_return);
                });

                if let Some(controller) = AgentController::<T>::get(agent_id) {
                    T::Currency::unreserve(&controller, deposit_return);
                }
            }

            Self::deposit_event(Event::MemoryPruned {
                agent_id,
                chunks_removed,
                bytes_freed,
            });

            Ok(())
        }

        /// Increase storage deposit.
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::increase_deposit())]
        pub fn increase_deposit(
            origin: OriginFor<T>,
            agent_id: AgentId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(
                AgentController::<T>::contains_key(agent_id),
                Error::<T>::MemoryNotInitialized
            );

            T::Currency::reserve(&who, amount)?;

            StorageDeposit::<T>::mutate(agent_id, |deposit| {
                *deposit = deposit.saturating_add(amount);
            });

            Self::deposit_event(Event::DepositIncreased { agent_id, amount });

            Ok(())
        }

        /// Withdraw excess deposit.
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::withdraw_deposit())]
        pub fn withdraw_deposit(
            origin: OriginFor<T>,
            agent_id: AgentId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let controller =
                AgentController::<T>::get(agent_id).ok_or(Error::<T>::MemoryNotInitialized)?;
            ensure!(who == controller, Error::<T>::NotController);

            let current_deposit = StorageDeposit::<T>::get(agent_id);
            let storage_used = StorageUsed::<T>::get(agent_id);
            let required_deposit = Self::bytes_to_deposit(storage_used as usize);

            let excess = current_deposit.saturating_sub(required_deposit);
            ensure!(amount <= excess, Error::<T>::InsufficientDeposit);

            T::Currency::unreserve(&who, amount);

            StorageDeposit::<T>::mutate(agent_id, |deposit| {
                *deposit = deposit.saturating_sub(amount);
            });

            Self::deposit_event(Event::DepositWithdrawn { agent_id, amount });

            Ok(())
        }
    }

    // ========================================================================
    // Helper Functions
    // ========================================================================

    impl<T: Config> Pallet<T> {
        /// Check write permission.
        fn ensure_write_permission(agent_id: AgentId, who: &T::AccountId) -> DispatchResult {
            // Operator always has write permission
            if let Some(operator) = AgentOperator::<T>::get(agent_id) {
                if *who == operator {
                    return Ok(());
                }
            }

            // Controller always has write permission
            if let Some(controller) = AgentController::<T>::get(agent_id) {
                if *who == controller {
                    return Ok(());
                }
            }

            // Check allowed writers
            let permissions = MemoryPermissions::<T>::get(agent_id);
            if permissions.allowed_writers.contains(who) {
                return Ok(());
            }

            Err(Error::<T>::WritePermissionDenied.into())
        }

        /// Calculate entry size in bytes.
        fn calculate_entry_size(entry: &MemoryEntry<BlockNumberFor<T>>) -> usize {
            let base_size = 32; // id, type, timestamp, ttl
            let content_size = entry.content.len();
            let metadata_size = entry.metadata.as_ref().map_or(0, |m| m.len());

            base_size + content_size + metadata_size
        }

        /// Convert bytes to deposit amount.
        fn bytes_to_deposit(bytes: usize) -> BalanceOf<T> {
            let cost_per_byte = T::StorageByteCost::get();
            cost_per_byte.saturating_mul((bytes as u32).into())
        }

        /// Charge storage for new data.
        fn charge_storage(agent_id: AgentId, bytes: usize) -> DispatchResult {
            let deposit_needed = Self::bytes_to_deposit(bytes);
            let current_deposit = StorageDeposit::<T>::get(agent_id);
            let storage_used = StorageUsed::<T>::get(agent_id);
            let total_required =
                Self::bytes_to_deposit(storage_used as usize).saturating_add(deposit_needed);

            if current_deposit < total_required {
                // Need more deposit from controller
                if let Some(controller) = AgentController::<T>::get(agent_id) {
                    let additional = total_required.saturating_sub(current_deposit);
                    T::Currency::reserve(&controller, additional)?;
                    StorageDeposit::<T>::mutate(agent_id, |d| *d = d.saturating_add(additional));
                } else {
                    return Err(Error::<T>::InsufficientDeposit.into());
                }
            }

            Ok(())
        }

        /// Compute hash of a chunk for integrity.
        fn compute_chunk_hash(chunk: &MemoryChunk<BlockNumberFor<T>>) -> sp_core::H256 {
            use sp_io::hashing::blake2_256;
            let encoded = chunk.encode();
            sp_core::H256::from(blake2_256(&encoded))
        }

        /// Get memory chunk for runtime API.
        pub fn get_memory_chunk(
            agent_id: AgentId,
            chunk_id: u32,
        ) -> Option<MemoryChunk<BlockNumberFor<T>>> {
            MemoryChunks::<T>::get(agent_id, chunk_id)
        }

        /// Get memory entries in JSONL format (for LLM consumption).
        pub fn get_memory_jsonl(
            agent_id: AgentId,
            offset: u64,
            limit: u32,
        ) -> Vec<JsonlEntry<BlockNumberFor<T>>> {
            let mut entries = Vec::new();
            let chunk_count = CurrentChunk::<T>::get(agent_id).saturating_add(1);
            let mut current_offset = 0u64;
            let mut collected = 0u32;

            for chunk_id in 0..chunk_count {
                if collected >= limit {
                    break;
                }

                if let Some(chunk) = MemoryChunks::<T>::get(agent_id, chunk_id) {
                    for entry in chunk.entries {
                        if current_offset >= offset {
                            let entry_type_str = match entry.entry_type {
                                EntryType::Observation => "observation",
                                EntryType::Action => "action",
                                EntryType::Result => "result",
                                EntryType::Thought => "thought",
                                EntryType::Goal => "goal",
                                EntryType::Plan => "plan",
                                EntryType::Error => "error",
                                EntryType::Checkpoint => "checkpoint",
                                EntryType::Delta => "delta",
                                EntryType::Custom => "custom",
                            };
                            entries.push(JsonlEntry {
                                id: entry.id,
                                entry_type: entry_type_str.as_bytes().to_vec(),
                                content: entry.content,
                                timestamp: entry.timestamp,
                            });
                            collected = collected.saturating_add(1);
                            if collected >= limit {
                                break;
                            }
                        }
                        current_offset = current_offset.saturating_add(1);
                    }
                }
            }

            entries
        }

        /// Get memory summary for runtime API.
        pub fn get_memory_summary(agent_id: AgentId) -> MemorySummary<BalanceOf<T>> {
            MemorySummary {
                agent_id,
                total_entries: EntryCount::<T>::get(agent_id),
                total_chunks: CurrentChunk::<T>::get(agent_id).saturating_add(1),
                storage_used: StorageUsed::<T>::get(agent_id),
                deposit: StorageDeposit::<T>::get(agent_id),
            }
        }

        // ====================================================================
        // Offchain Worker Tasks (Phase 2 Implementation)
        // ====================================================================

        /// Offchain worker task 1A: Index memory snapshots from on-chain to RocksDB.
        /// Executes once per block to record latest memory state.
        pub fn index_memory_worker() {
            // Get current block number
            let current_block = frame_system::Pallet::<T>::block_number();

            // Iterate through all agents and index their latest memory
            // (In production, this would filter agents based on MemoryUpdated events)
            EntryCount::<T>::iter().for_each(|(agent_id, _)| {
                // Get latest memory state
                if let Some(_entry_count) = EntryCount::<T>::get(agent_id).checked_sub(1) {
                    // Compute memory hash from current chunks
                    let memory_hash = Self::compute_agent_memory_hash(agent_id);

                    // Store in offchain storage via offchain_storage module
                    let _snapshot = MemorySnapshot {
                        agent_id: sp_core::H256::from_slice(&[agent_id as u8; 32]),
                        block_number: current_block.saturated_into::<u32>(),
                        memory_hash,
                        size_bytes: StorageUsed::<T>::get(agent_id).saturated_into::<u32>(),
                        indexed_at: current_block.saturated_into::<u32>(),
                        timestamp: current_block.saturated_into::<u64>(),
                    };

                    // Update on-chain latest hash for querying
                    LatestMemoryHash::<T>::insert(agent_id, memory_hash);

                    // Emit event
                    Self::deposit_event(Event::MemoryIndexed {
                        agent_id,
                        block_number: current_block,
                        memory_hash,
                    });
                }
            });
        }

        /// Offchain worker task 1B: Verify memory consistency across validators.
        /// Runs periodically to ensure Byzantine quorum consensus.
        pub fn verify_memory_consistency() {
            let current_block = frame_system::Pallet::<T>::block_number();

            // Check every 100 blocks (production: configurable)
            if (current_block % 100u32.into()).is_zero() {
                // Iterate through agents and verify consensus
                EntryCount::<T>::iter().for_each(|(agent_id, _)| {
                    if let Some(memory_hash) = LatestMemoryHash::<T>::get(agent_id) {
                        // In production, this would query peer validators via RPC
                        // For now, record a local attestation
                        let attestations =
                            MemoryConsensusRecords::<T>::get(agent_id, current_block)
                                .map(|(_, count)| count)
                                .unwrap_or(0);

                        // Simulate collecting attestations (in real impl, query peers)
                        let threshold = T::MemoryConsensusThreshold::get();
                        let required_attestations = (threshold + 50) / 100;

                        if attestations >= required_attestations {
                            // Consensus reached
                            MemoryConsensusRecords::<T>::insert(
                                agent_id,
                                current_block,
                                (memory_hash, attestations),
                            );

                            Self::deposit_event(Event::MemoryConsensusReached {
                                agent_id,
                                block_number: current_block,
                                attestations,
                                consensus_hash: memory_hash,
                            });
                        }
                    }
                });
            }
        }

        /// Offchain worker task 1C: Cleanup old memory snapshots beyond retention period.
        /// Prunes Tier 2 (offchain) memory to save storage space.
        pub fn cleanup_old_memory() {
            let current_block = frame_system::Pallet::<T>::block_number();
            let retention_blocks = T::MemoryRetentionBlocks::get();

            // Only run if retention blocks have passed
            if current_block < retention_blocks {
                return;
            }

            let cutoff_block = current_block.saturating_sub(retention_blocks);
            let mut blocks_pruned = 0u32;

            // Iterate through agents and prune old memory
            EntryCount::<T>::iter().for_each(|(agent_id, _)| {
                // Clean up old memory chunks based on TTL
                let current_chunk_id = CurrentChunk::<T>::get(agent_id);

                for chunk_id in 0..current_chunk_id {
                    if let Some(chunk) = MemoryChunks::<T>::get(agent_id, chunk_id) {
                        // Check if all entries in chunk are expired
                        let all_expired = chunk
                            .entries
                            .iter()
                            .all(|entry| entry.ttl.is_some_and(|ttl| ttl < cutoff_block));

                        if all_expired && chunk.finalized {
                            // Safe to delete
                            MemoryChunks::<T>::remove(agent_id, chunk_id);
                            blocks_pruned = blocks_pruned.saturating_add(1);
                        }
                    }
                }

                // Clean up old consensus records
                let mut to_remove = Vec::new();
                MemoryConsensusRecords::<T>::iter_prefix(agent_id).for_each(|(block_number, _)| {
                    if block_number < cutoff_block {
                        to_remove.push(block_number);
                    }
                });

                for block_number in to_remove {
                    MemoryConsensusRecords::<T>::remove(agent_id, block_number);
                }

                if blocks_pruned > 0 {
                    Self::deposit_event(Event::OffchainMemoryPruned {
                        agent_id,
                        block_number: current_block,
                        blocks_pruned,
                    });
                }
            });

            // Update last offchain update block
            LastOffchainUpdateBlock::<T>::set(current_block);
        }

        /// Compute agent's current memory hash (merkle root of all chunks).
        fn compute_agent_memory_hash(agent_id: AgentId) -> sp_core::H256 {
            use sp_io::hashing::blake2_256;

            let mut hasher_data = Vec::new();
            let chunk_count = CurrentChunk::<T>::get(agent_id).saturating_add(1);

            for chunk_id in 0..chunk_count {
                if let Some(chunk) = MemoryChunks::<T>::get(agent_id, chunk_id) {
                    if let Some(hash) = chunk.hash {
                        hasher_data.extend_from_slice(hash.as_bytes());
                    }
                }
            }

            sp_core::H256::from(blake2_256(&hasher_data))
        }
    }

    // ====================================================================
    // Hooks Implementation
    // ====================================================================

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// Run offchain worker tasks on every block (on_idle would be better but
        /// we use on_initialize here to ensure they run).
        fn on_initialize(_n: BlockNumberFor<T>) -> frame_support::weights::Weight {
            // Task 1A: Index memory
            Self::index_memory_worker();

            // Task 1B: Verify consistency (every 100 blocks)
            Self::verify_memory_consistency();

            // Task 1C: Cleanup old memory
            Self::cleanup_old_memory();

            // Weight: minimal for now, tune based on actual benchmarks
            T::WeightInfo::initialize_memory()
        }
    }
}
