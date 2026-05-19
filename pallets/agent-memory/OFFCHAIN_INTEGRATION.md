# Agent Memory Offchain Indexing

## Overview

Agent Memory stores sensitive execution state for autonomous agents running on X3 Chain. This document specifies the complete offchain indexing strategy, data retention policy, consistency model, and validator participation requirements.

**Status:** ✅ SPECIFICATION COMPLETE | Implementation in progress

## Data Classification & Tier System

### Tier 1: Public State (On-Chain)
**Storage:** `pallet_agent_memory` storage maps  
**Accessibility:** All validators, RPC nodes  
**Retention:** Configurable via SettlementTimeoutBlocks  
**Examples:**
- Agent metadata (name, owner, version)
- Entry count and chunk structure
- Storage deposit tracking
- Permission metadata

### Tier 2: Private State (Offchain Storage)  
**Storage:** Validator RocksDB indexes (local)  
**Accessibility:** Via RPC API (query API)  
**Retention:** 1-7 days (configurable)  
**Sensitivity:** HIGH - Contains execution traces, local state, computation results  
**Examples:**
- Agent memory snapshots
- Execution traces with call stacks
- Local variables and registers
- Sensitive computation results
- API keys and secrets (if stored)

**Data Replication Strategy:**
- Mandatory: Index all Tier 2 data in local RocksDB
- Optional: Cross-validator replication via gossip protocol
- Fallback: Request memory from peers via RPC if local cache miss

### Tier 3: Archive State (Historical)  
**Storage:** Archive nodes only (external storage)  
**Retention:** 7-90 days  
**Purpose:** Historical audit trail, compliance
**Recovery:** Via archive node queries or manual recovery

## Offchain Indexing Strategy

### Requirement 1: Offchain Worker Tasks

Every validator runs three background tasks:

#### Task 1A: Memory Indexing Worker
**Trigger:** On every block finalization  
**Frequency:** Every 1-2 blocks  
**Operation:**
```
1. Query pallet::Agents storage for all agents
2. For each agent with MemoryUpdated event this block:
   a. Fetch latest memory chunks from on-chain storage
   b. Compute merkle root: blake2_256(chunk_data)
   c. Write to RocksDB: agent_memory_index(agent_id, block_num, root, data)
   d. Record: indexed_at timestamp
3. Emit: MemoryIndexed event with agent_id, block_number, validator_id
```

**RocksDB Schema:**
```sql
TABLE: agent_memory_index
  - agent_id (String): Primary key part 1
  - block_number (u32): Primary key part 2
  - memory_hash (H256): Merkle root of memory snapshot
  - memory_snapshot (Vec<u8>): Serialized agent memory (optional, compress if >1MB)
  - indexed_at (BlockNumber): Block when indexed
  - size_bytes (u32): Uncompressed size of snapshot
  INDEX: idx_agent_memory_latest (agent_id DESC, block_number DESC)
  INDEX: idx_agent_memory_timestamp (indexed_at DESC)
```

#### Task 1B: Consistency Verification Worker  
**Trigger:** Every 100 blocks  
**Frequency:** Periodic validation  
**Operation:**
```
1. Sample 10% of indexed agents (random selection)
2. For each sampled agent:
   a. Get latest memory hash from on-chain storage
   b. Compare with local RocksDB memory_hash
   c. If mismatch:
      - Query peer validators for same (agent_id, block_number)
      - Request memory snapshot over RPC
      - Re-verify merkle root
      - Update RocksDB if peer has correct hash
3. If 2/3 of peers have same hash → Consensus reached
4. Emit: MemoryConsensusReached event with block_number, attestations
```

**Consensus Logic:**
```rust
validators_total = pallet_session::Validators::<T>::get().len()
required_attestations = (validators_total * 2 / 3) + 1
current_attestations = consensus_table.count_matches(agent_id, block, memory_hash)
consensus_reached = current_attestations >= required_attestations
```

#### Task 1C: Retention & Cleanup Worker  
**Trigger:** Every 1000 blocks  
**Frequency:** Retention policy enforcement  
**Operation:**
```
1. Read MemoryRetentionBlocks from config (default: 432k blocks ≈ 24 hours)
2. Calculate cutoff_block = current_block - retention_blocks
3. Query RocksDB for records with block_number < cutoff_block
4. For each expired record:
   a. Delete from agent_memory_index
   b. Delete from agent_memory_consistency
   c. Increment bytes_freed counter
5. Emit: MemoryPruned event with agent_id, blocks_removed, bytes_freed
6. Optional: Archive to external storage (if configured)
```

**Retention Tiers:**
```
Tier 1 - Hot (0-24 hours):
  - Storage: All validators, RocksDB
  - Queries: Full RPC support
  - Consistency: Real-time verification

Tier 2 - Warm (24 hours - 7 days):
  - Storage: Archive nodes only
  - Queries: Archive RPC endpoint
  - Consistency: Eventual consistency

Tier 3 - Cold (7+ days):
  - Storage: External archives (S3, IPFS, etc.)
  - Queries: Manual request required
  - Consistency: No verification
```

### Requirement 2: Data Retention Policy

```yaml
MemoryRetentionBlocks: 432000  # ~24 hours (200ms/block * 432k blocks)
HotDataBlocks: 28800           # ~2 hours - keep in memory cache
WarmDataBlocks: 288000         # ~16 hours - keep in RocksDB
ColdDataBlocks: 432000         # ~24 hours - eligible for archival

# Example timeline:
Block 0:     Memory update for agent-123
Block 1-5:   Offchain workers index memory (Task 1A)
Block 100:   Consistency verification (Task 1B)
Block 101:   If 2/3 match -> MemoryConsensusReached event
Block 432k:  Memory eligible for pruning (Task 1C)
Block 432k+1: Memory archived or deleted
```

### Requirement 3: Consistency Model

**Model:** Eventual Consistency with Byzantine Fault Tolerance

**Guarantees:**
1. **Availability:** Memory indexed within 2 blocks of update
2. **Correctness:** Verified via 2/3+ validator attestations  
3. **Durability:** Replicated across minimum 3 validators (per Byzantine assumptions)
4. **Timeliness:** Consensus reached within 100 blocks

**Consistency Flow:**
```
Timeline:
  Block N:     Agent calls write_memory() → MemoryUpdated event
  Block N+1:   Offchain workers index memory
  Block N+2:   RPC queries available with new memory
  Block N+3:   Optional: query peers for consistency
  ...
  Block N+100: Consistency verification runs
  Block N+101: If 2/3 peers verified → MemoryConsensusReached
  ...
  Block N+432k: Memory eligible for deletion
```

**Validation Procedure:**
```rust
// On-chain verification for critical operations
pub fn verify_memory_consensus(
  agent_id: H256,
  block_number: u32,
  required_consensus: u32,  // e.g., 2/3 of validators
) -> DispatchResult {
  let attestations = MemoryConsensusTable::get(&agent_id, block_number);
  ensure!(
    attestations.len() as u32 >= required_consensus,
    Error::<T>::InsufficientMemoryAttestation
  );
  Ok(())
}
```

### Requirement 4: Validator Participation

**Mandatory:**
- ✅ Index all Tier 1 (public) memory state
- ✅ Respond to RPC queries for indexed memory
- ✅ Participate in consistency verification polling

**Optional (Incentivized):**
- ⚪ Index Tier 2 (private) memory snapshots
- ⚪ Provide Tier 2 memory via RPC API
- ⚪ Host archive data beyond retention period
- ⚪ Provide peer-to-peer memory replication

**Configuration:**
```yaml
# In node config
[offchain_indexing]
enabled: true
participate_in_indexing: true  # Index Tier 2
participate_in_replication: false  # Optional peer replication
provide_rpc_queries: true
retention_hours: 24
max_memory_cache_mb: 1024
archive_enabled: false
archive_backend: "s3"  # "s3", "ipfs", "local_disk"
```

## RPC API Specification

### Method 1: `agent_memory_hash`
**Description:** Get latest memory hash for an agent  
**Parameters:**
- `agent_id: H256` - The agent identifier

**Response:**
```json
{
  "memory_hash": "0x5f3c...",
  "block_number": 12345,
  "indexed_at": 12344,
  "consensus_reached": true,
  "attestations": 3
}
```

**Error Cases:**
- `AgentNotFound` - Agent doesn't exist
- `MemoryNotIndexed` - No offchain index available (query peer validators)

### Method 2: `agent_memory_at_block`
**Description:** Get agent memory snapshot at specific block  
**Parameters:**
- `agent_id: H256` - The agent identifier
- `block_number: u32` - Block to query

**Response:**
```json
{
  "agent_id": "0xabc...",
  "block_number": 12000,
  "memory_data": "0xdef...",  // base64-encoded
  "size_bytes": 4096,
  "verified": true,
  "verification_block": 12100
}
```

**Error Cases:**
- `BlockTooOld` - Block older than retention period
- `MemoryNotFound` - No index for this agent/block combo
- `ConsensusNotReached` - Unverified snapshot (use with caution)

### Method 3: `agent_query`
**Description:** Execute readonly query against agent memory  
**Parameters:**
- `agent_id: H256` - The agent identifier
- `block_number: u32` - Block context
- `function_name: String` - Query function name
- `params: Vec<u8>` - Query parameters (CBOR-encoded)

**Response:**
```json
{
  "success": true,
  "result": "0xabc...",  // base64-encoded result
  "executed_block": 12000,
  "query_latency_ms": 45
}
```

**Error Cases:**
- `QueryFailed` - Function threw error
- `MemoryNotIndexed` - Can't execute query

### Method 4: `agent_memory_consensus`
**Description:** Get consensus status for memory snapshot  
**Parameters:**
- `agent_id: H256`
- `block_number: u32`

**Response:**
```json
{
  "agent_id": "0xabc...",
  "block_number": 12000,
  "memory_hash": "0x5f3c...",
  "attestations_received": [
    { "validator": "0x123...", "verified": true },
    { "validator": "0x456...", "verified": true },
    { "validator": "0x789...", "verified": true }
  ],
  "attestations_required": 3,
  "consensus_reached": true,
  "consensus_reached_at_block": 12101
}
```

## Event Specifications

### Event 1: `MemoryUpdated`
```rust
pub struct MemoryUpdated {
    pub agent_id: H256,
    pub block: BlockNumber,
    pub new_hash: H256,           // merkle root
    pub size_bytes: u32,
    pub timestamp: u64,
}
```

### Event 2: `MemoryIndexed`
```rust
pub struct MemoryIndexed {
    pub agent_id: H256,
    pub block: BlockNumber,
    pub validator: AccountId,     // which validator indexed it
    pub indexed_at: BlockNumber,
}
```

### Event 3: `MemoryConsensusReached`
```rust
pub struct MemoryConsensusReached {
    pub agent_id: H256,
    pub block: BlockNumber,
    pub attestations: u32,        // how many validators agreed
    pub consensus_hash: H256,     // the agreed-upon hash
}
```

### Event 4: `MemoryPruned`
```rust
pub struct MemoryPruned {
    pub agent_id: H256,
    pub block: BlockNumber,
    pub bytes_freed: u64,
    pub pruned_at: BlockNumber,
}
```

## Implementation Checklist

### Phase 1: Core Infrastructure (DONE)
- ✅ Pallet storage types (Agents, MemorySnapshots, LatestMemoryHash)
- ✅ Event types (MemoryUpdated, MemoryIndexed, etc.)
- ✅ Error enum
- ✅ Config trait with constants

### Phase 2: Offchain Workers (TODO)
- ⬜ Implement memory indexing worker (Task 1A)
- ⬜ Implement consistency verification worker (Task 1B)
- ⬜ Implement retention cleanup worker (Task 1C)
- ⬜ Hook workers into pallet on_idle()

### Phase 3: RocksDB & Query API (TODO)
- ⬜ Create offchain_storage module with RocksDB schema
- ⬜ Implement agent_memory_hash() RPC method
- ⬜ Implement agent_memory_at_block() RPC method
- ⬜ Implement agent_query() RPC method
- ⬜ Implement agent_memory_consensus() RPC method

### Phase 4: Integration & Testing (TODO)
- ⬜ Integrate with pallet_session for validator access
- ⬜ Create 15+ comprehensive unit tests
- ⬜ Add integration tests with multi-validator consensus verification
- ⬜ Test retention policy and cleanup
- ⬜ Test RPC API responses
- ⬜ Target 90%+ code coverage

### Phase 5: Monitoring & Metrics (TODO)
- ⬜ Add metrics: agent_memory_snapshots_total
- ⬜ Add metrics: indexing_latency_blocks
- ⬜ Add metrics: consistency_success_rate
- ⬜ Add metrics: query_latency_ms

## Security Considerations

### Data Privacy
- Tier 2 (private) memory never transmitted on network unless explicitly queried
- RPC queries should require authentication if memory is sensitive
- Consider encryption at rest for RocksDB

### Byzantine Resilience
- Consensus requires 2/3+ validator agreement (standard BFT assumption)
- Validators that return incorrect hashes are detectable via consistency verification
- Non-participating validators are excluded from consensus calculation

### Retention & Compliance
- All data automatically pruned after MemoryRetentionBlocks
- Archive nodes may retain longer for compliance
- Immutable audit trail: MemoryConsensusReached events prove verification

## Migration Strategy

For existing chains upgrading from v1 to v2:

1. **Phase 1:** Deploy pallet_agent_memory without breaking changes
2. **Phase 2:** Enable offchain workers (optional for operators)
3. **Phase 3:** Implement RPC API (backward compatible)
4. **Phase 4:** Deprecate old memory API (if applicable)

## Testing Strategy

```bash
# Unit tests for offchain workers
cargo test -p pallet-agent-memory test_indexing_worker

# Integration tests with multi-validator setup
cargo test -p pallet-agent-memory test_consensus_verification

# RPC API tests
cargo test -p pallet-agent-memory test_rpc_query_api

# Retention policy tests
cargo test -p pallet-agent-memory test_retention_cleanup

# Byzantine fault tolerance tests
cargo test -p pallet-agent-memory test_consensus_with_faulty_validator
```

## Performance Targets

| Metric | Target | Actual |
|--------|--------|--------|
| Indexing latency | < 2 blocks | TBD |
| Consensus verification | 50-100 blocks | TBD |
| RPC query response | < 100ms | TBD |
| Memory cache hit rate | > 95% | TBD |
| Consistency verification accuracy | 99.9% | TBD |

## References

- [Substrate Offchain Workers](https://docs.substrate.io/build/runtime-storage/#offchain-storage)
- [RocksDB Documentation](https://github.com/facebook/rocksdb/wiki)
- [Byzantine Fault Tolerance](https://en.wikipedia.org/wiki/Byzantine_fault)
- [Pallet Agent Memory Runtime API](./src/runtime_api.rs)
- [Pallet Agent Memory Tests](./src/tests.rs)

---

**Document Version:** 1.0  
**Last Updated:** 2025-04-24  
**Status:** ✅ SPECIFICATION COMPLETE  
**Next Step:** Implement Phase 2 (Offchain Workers)
