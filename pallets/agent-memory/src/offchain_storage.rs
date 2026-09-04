//! Offchain storage layer for Agent Memory indexing.
//!
//! This module handles RocksDB-based indexing of agent memory snapshots,
//! consistency verification, and query execution.

use sp_core::H256;
use sp_std::prelude::*;

/// Offchain storage key prefix for agent memory index
const _MEMORY_INDEX_PREFIX: &[u8] = b"agent_memory:index:";
const _MEMORY_CONSENSUS_PREFIX: &[u8] = b"agent_memory:consensus:";

/// Agent memory snapshot metadata for offchain indexing
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct MemorySnapshot {
    /// Agent identifier
    pub agent_id: H256,
    /// Block number when memory was updated
    pub block_number: u32,
    /// Merkle root hash of memory data
    pub memory_hash: H256,
    /// Size of memory snapshot in bytes
    pub size_bytes: u32,
    /// Block number when indexed in offchain storage
    pub indexed_at: u32,
    /// Timestamp of indexing
    pub timestamp: u64,
}

/// Consensus attestation for memory snapshot
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct MemoryAttestation {
    /// Validator account ID (serialized as Vec<u8>)
    pub validator: Vec<u8>,
    /// Memory hash the validator attested to
    pub attested_hash: H256,
    /// Block where attestation was recorded
    pub attested_at_block: u32,
    /// Whether this attestation has been verified
    pub verified: bool,
}

/// Consensus status for a memory snapshot
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ConsensusStatus {
    /// Agent identifier
    pub agent_id: H256,
    /// Block number of memory snapshot
    pub block_number: u32,
    /// Final agreed-upon memory hash
    pub consensus_hash: H256,
    /// List of validators who attested
    pub attestations: Vec<MemoryAttestation>,
    /// Total attestations required for consensus (2/3 + 1)
    pub required_attestations: u32,
    /// Block where consensus was reached (None if not reached)
    pub consensus_reached_at_block: Option<u32>,
}

/// Query result for agent memory
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct QueryResult {
    /// Whether query execution succeeded
    pub success: bool,
    /// Query result bytes (if successful)
    pub result: Option<Vec<u8>>,
    /// Error message (if failed)
    pub error: Option<Vec<u8>>,
    /// Block context used for query
    pub executed_block: u32,
    /// Query execution latency in milliseconds
    pub latency_ms: u32,
}

impl MemorySnapshot {
    /// Create a new memory snapshot
    pub fn new(
        agent_id: H256,
        block_number: u32,
        memory_hash: H256,
        size_bytes: u32,
        indexed_at: u32,
        timestamp: u64,
    ) -> Self {
        Self {
            agent_id,
            block_number,
            memory_hash,
            size_bytes,
            indexed_at,
            timestamp,
        }
    }

    /// Encode snapshot for RocksDB storage
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        // Simple binary encoding: agent_id(32) + block(4) + hash(32) + size(4) + idx(4) + ts(8)
        buf.extend_from_slice(self.agent_id.as_bytes());
        buf.extend_from_slice(&self.block_number.to_le_bytes());
        buf.extend_from_slice(self.memory_hash.as_bytes());
        buf.extend_from_slice(&self.size_bytes.to_le_bytes());
        buf.extend_from_slice(&self.indexed_at.to_le_bytes());
        buf.extend_from_slice(&self.timestamp.to_le_bytes());
        buf
    }

    /// Decode snapshot from RocksDB storage
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 84 {
            // 32 + 4 + 32 + 4 + 4 + 8
            return None;
        }

        let mut offset = 0;

        // Read agent_id (32 bytes)
        let agent_id = H256::from_slice(&bytes[offset..offset + 32]);
        offset += 32;

        // Read block_number (4 bytes)
        let block_number = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += 4;

        // Read memory_hash (32 bytes)
        let memory_hash = H256::from_slice(&bytes[offset..offset + 32]);
        offset += 32;

        // Read size_bytes (4 bytes)
        let size_bytes = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += 4;

        // Read indexed_at (4 bytes)
        let indexed_at = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += 4;

        // Read timestamp (8 bytes)
        let timestamp = u64::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]);

        Some(Self {
            agent_id,
            block_number,
            memory_hash,
            size_bytes,
            indexed_at,
            timestamp,
        })
    }
}

impl MemoryAttestation {
    /// Create a new attestation
    pub fn new(
        validator: Vec<u8>,
        attested_hash: H256,
        attested_at_block: u32,
        verified: bool,
    ) -> Self {
        Self {
            validator,
            attested_hash,
            attested_at_block,
            verified,
        }
    }
}

impl ConsensusStatus {
    /// Create new consensus status
    pub fn new(
        agent_id: H256,
        block_number: u32,
        consensus_hash: H256,
        required_attestations: u32,
    ) -> Self {
        Self {
            agent_id,
            block_number,
            consensus_hash,
            attestations: Vec::new(),
            required_attestations,
            consensus_reached_at_block: None,
        }
    }

    /// Add an attestation
    pub fn add_attestation(&mut self, attestation: MemoryAttestation) {
        self.attestations.push(attestation);
    }

    /// Check if consensus is reached
    pub fn is_consensus_reached(&self) -> bool {
        let verified_count = self
            .attestations
            .iter()
            .filter(|a| a.verified && a.attested_hash == self.consensus_hash)
            .count() as u32;
        verified_count >= self.required_attestations
    }

    /// Get count of verified attestations
    pub fn verified_count(&self) -> u32 {
        self.attestations.iter().filter(|a| a.verified).count() as u32
    }
}

impl QueryResult {
    /// Create successful query result
    pub fn success(result: Vec<u8>, executed_block: u32, latency_ms: u32) -> Self {
        Self {
            success: true,
            result: Some(result),
            error: None,
            executed_block,
            latency_ms,
        }
    }

    /// Create failed query result
    pub fn error(error: Vec<u8>, executed_block: u32, latency_ms: u32) -> Self {
        Self {
            success: false,
            result: None,
            error: Some(error),
            executed_block,
            latency_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_snapshot_encode_decode() {
        let original = MemorySnapshot::new(
            H256::from([1u8; 32]),
            100,
            H256::from([2u8; 32]),
            4096,
            99,
            1234567890,
        );

        let encoded = original.encode();
        assert_eq!(encoded.len(), 84);

        let decoded = MemorySnapshot::decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_memory_snapshot_decode_invalid() {
        let too_short = vec![0u8; 50];
        assert!(MemorySnapshot::decode(&too_short).is_none());
    }

    #[test]
    fn test_consensus_status_verification() {
        let mut consensus = ConsensusStatus::new(
            H256::from([1u8; 32]),
            100,
            H256::from([2u8; 32]),
            2, // require 2 attestations
        );

        // Add unverified attestation
        consensus.add_attestation(MemoryAttestation::new(
            b"validator1".to_vec(),
            H256::from([2u8; 32]),
            100,
            false,
        ));
        assert!(!consensus.is_consensus_reached());

        // Add verified attestation
        consensus.add_attestation(MemoryAttestation::new(
            b"validator2".to_vec(),
            H256::from([2u8; 32]),
            100,
            true,
        ));

        // Still not reached (need 2 verified)
        assert_eq!(consensus.verified_count(), 1);
        assert!(!consensus.is_consensus_reached());

        // Add second verified attestation
        consensus.add_attestation(MemoryAttestation::new(
            b"validator3".to_vec(),
            H256::from([2u8; 32]),
            100,
            true,
        ));

        assert_eq!(consensus.verified_count(), 2);
        assert!(consensus.is_consensus_reached());
    }

    #[test]
    fn test_query_result_success() {
        let result = QueryResult::success(b"test_result".to_vec(), 100, 50);
        assert!(result.success);
        assert_eq!(result.result.unwrap(), b"test_result");
        assert_eq!(result.executed_block, 100);
    }

    #[test]
    fn test_query_result_error() {
        let result = QueryResult::error(b"query_failed".to_vec(), 100, 25);
        assert!(!result.success);
        assert_eq!(result.error.unwrap(), b"query_failed");
        assert_eq!(result.latency_ms, 25);
    }
}
