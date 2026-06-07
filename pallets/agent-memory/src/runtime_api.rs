//! Runtime API for Agent Memory pallet.
//!
//! Provides offchain access to agent memory chunks.

use frame_support::pallet_prelude::{DecodeWithMemTracking, *};
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_std::prelude::*;

/// Memory entry for API response.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug, Default)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct MemoryEntryResponse {
    /// Entry ID.
    pub id: u64,
    /// Entry type (as string).
    pub entry_type: Vec<u8>,
    /// Content (JSON bytes).
    pub content: Vec<u8>,
    /// Metadata.
    pub metadata: Option<Vec<u8>>,
    /// Block timestamp.
    pub timestamp: u64,
    /// TTL (blocks until expiration).
    pub ttl: Option<u64>,
}

/// Memory chunk response for API.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug, Default)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct MemoryChunkResponse {
    /// Chunk index.
    pub chunk_index: u32,
    /// Agent ID.
    pub agent_id: u32,
    /// Whether chunk is finalized.
    pub finalized: bool,
    /// Number of entries.
    pub entry_count: u32,
    /// Entries in this chunk.
    pub entries: Vec<MemoryEntryResponse>,
    /// Start block.
    pub start_block: u64,
    /// End block (if finalized).
    pub end_block: Option<u64>,
}

/// JSONL-formatted memory dump.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug, Default)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct MemoryJsonlResponse {
    /// JSONL lines (each line is a JSON object).
    pub lines: Vec<Vec<u8>>,
    /// Total entries.
    pub total: u32,
    /// Whether there are more entries.
    pub has_more: bool,
    /// Cursor for pagination.
    pub cursor: Option<u64>,
}

/// Response for agent_memory_hash RPC method
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct MemoryHashResponse {
    /// Latest memory hash (merkle root)
    pub memory_hash: Vec<u8>, // H256 as bytes
    /// Block where memory was last updated
    pub block_number: u32,
    /// Block where memory was indexed offchain
    pub indexed_at: u32,
    /// Whether consensus has been reached
    pub consensus_reached: bool,
    /// Number of validator attestations received
    pub attestations: u32,
}

/// Response for agent_memory_at_block RPC method
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct MemorySnapshotResponse {
    /// Agent ID
    pub agent_id: Vec<u8>, // H256 as bytes
    /// Block number of snapshot
    pub block_number: u32,
    /// Memory data (serialized)
    pub memory_data: Vec<u8>,
    /// Size in bytes
    pub size_bytes: u32,
    /// Whether snapshot is verified by consensus
    pub verified: bool,
    /// Block where verification completed
    pub verification_block: u32,
}

/// Response for agent_query RPC method
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct QueryResponse {
    /// Whether query succeeded
    pub success: bool,
    /// Query result (if successful)
    pub result: Option<Vec<u8>>,
    /// Error message (if failed)
    pub error: Option<Vec<u8>>,
    /// Block context where query executed
    pub executed_block: u32,
    /// Query execution latency in milliseconds
    pub latency_ms: u32,
}

/// Attestation entry in consensus status
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct AttestationEntry {
    /// Validator account ID
    pub validator: Vec<u8>,
    /// Whether attestation is verified
    pub verified: bool,
}

/// Response for agent_memory_consensus RPC method
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ConsensusStatusResponse {
    /// Agent ID
    pub agent_id: Vec<u8>, // H256 as bytes
    /// Block number of memory snapshot
    pub block_number: u32,
    /// Memory hash being attested
    pub memory_hash: Vec<u8>, // H256 as bytes
    /// List of validator attestations received
    pub attestations_received: Vec<AttestationEntry>,
    /// Number of attestations required for consensus
    pub attestations_required: u32,
    /// Whether consensus has been reached
    pub consensus_reached: bool,
    /// Block where consensus was reached
    pub consensus_reached_at_block: u32,
}

sp_api::decl_runtime_apis! {
    /// Agent Memory Runtime API.
    pub trait AgentMemoryApi {
        /// Get a memory chunk by agent and index.
        fn get_memory_chunk(agent_id: u32, chunk_index: u32) -> Option<MemoryChunkResponse>;

        /// Get latest N entries for an agent.
        fn get_latest_entries(agent_id: u32, count: u32) -> Vec<MemoryEntryResponse>;

        /// Get entries as JSONL format (LLM-friendly).
        fn get_memory_jsonl(agent_id: u32, from_id: u64, limit: u32) -> MemoryJsonlResponse;

        /// Get total chunk count for agent.
        fn get_chunk_count(agent_id: u32) -> u32;

        /// Get entry count for agent.
        fn get_entry_count(agent_id: u32) -> u64;

            // ════════════════════════════════════════════════════════════════════════════════
            // Phase 3: Offchain Memory RPC Methods
            // ════════════════════════════════════════════════════════════════════════════════
            /// Get latest memory hash and consensus status for an agent.
        fn agent_memory_hash(agent_id: Vec<u8>) -> MemoryHashResponse;

        /// Get agent memory snapshot at specific block.
        fn agent_memory_at_block(agent_id: Vec<u8>, block_number: u32) -> MemorySnapshotResponse;

        /// Execute readonly query against agent memory.
        fn agent_query(agent_id: Vec<u8>, block_number: u32, function_name: Vec<u8>, params: Vec<u8>) -> QueryResponse;

        /// Get consensus status for memory snapshot.
        fn agent_memory_consensus(agent_id: Vec<u8>, block_number: u32) -> ConsensusStatusResponse;
    }
}
