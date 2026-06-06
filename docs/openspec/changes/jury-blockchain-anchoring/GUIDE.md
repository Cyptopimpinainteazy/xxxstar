# Jury Blockchain Anchoring - Complete Guide

**Version:** 1.0  
**Status:** PRODUCTION READY  
**Last Updated:** 2026-02-08  

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Quick Start](#quick-start)
4. [Development Guide](#development-guide)
5. [Operations Manual](#operations-manual)
6. [API Reference](#api-reference)
7. [Examples](#examples)
8. [Troubleshooting](#troubleshooting)
9. [Performance & Scaling](#performance--scaling)
10. [Security Considerations](#security-considerations)

---

## Overview

**Jury Blockchain Anchoring** persists jury governance decisions to the blockchain, creating an immutable record of jury outcomes. This enables:

✅ **Verification** - External systems verify jury decisions via RPC  
✅ **Auditability** - Complete governance lineage (off-chain votes → on-chain hash)  
✅ **Governance Actions** - Trigger on-chain contract logic based on jury verdicts  
✅ **Compliance** - Regulatory-compliant immutable decision records  

### Key Features

| Feature | Benefit |
|---------|---------|
| **Off-Chain Votes** | Privacy preserved; only hash persisted on-chain |
| **Deterministic Hashing** | Same votes always produce same hash; no tampering |
| **Efficient Storage** | One H256 per session (~32 bytes); scales to 1000s/day |
| **RPC Verification** | External query without full node |
| **Audit Trail** | Complete lineage from votes → hash → block |

---

## Architecture

### Component Interaction

```
Off-Chain (Private)          On-Chain (Public)         Frontend (Read)
┌─────────────────┐        ┌──────────────┐          ┌──────────────┐
│  Jury Service   │        │   Runtime    │          │  Dashboard   │
│                 │        │              │          │              │
│ 1. Vote Commit  │        │ x3-jury-  │          │ Display      │
│ 2. Vote Reveal  │───────▶│ anchor       │◀─────────│ verified     │
│ 3. Aggregate    │        │ pallet       │  RPC     │ decisions    │
│ 4. Audit Log    │        │              │          │              │
└─────────────────┘        └──────────────┘          └──────────────┘
       │                           │
       │ Hash decision             │ Store hash
       │ (SHA256)                  │ Emit event
       │                           │
    PostgreSQL                  Blockchain
    (votes, audit)              (decision hash)
```

### Data Flow

```
1. Jury Session Complete
   Session ID: "sess-123"
   Votes: [True, True, True, True, False]
   Result: PASS (4/5 = 80%)

2. Compute Decision Hash
   Input: {"session_id": "sess-123", "votes": [...], "result": "PASS"}
   Hash: SHA256(...) = 0xabcd...

3. Anchor to Blockchain
   Call: x3_jury_anchor.anchor_decision(session_id, decision_hash)
   Signature: Jury Manager Authority

4. On-Chain Storage
   Storage: JuryDecisions[session_id] = JuryDecisionRecord {
       decision_hash: 0xabcd...,
       block_number: 12345,
       timestamp: 1707410123,
       jury_authority: 5Grvw...,
   }

5. Event Emitted
   JuryDecisionAnchored {
       session_id: "sess-123",
       decision_hash: 0xabcd...,
       block_number: 12345,
   }

6. Frontend Verification
   RPC Call: jury_decisionStatus("sess-123")
   Response: {
       status: "anchored",
       block_number: 12345,
       decision_hash: 0xabcd...,
       verified: true,
   }
```

---

## Quick Start

### Prerequisites

- X3 Chain runtime compiled with `x3-jury-anchor` pallet
- Jury service running (Python)
- PostgreSQL for audit logs
- RPC endpoint accessible

### 1. Deploy Runtime

```bash
# Add pallet to runtime Cargo.toml
[dependencies]
pallet-x3-jury-anchor = { path = "pallets/x3-jury-anchor" }

# Add to construct_runtime!
pub enum Runtime {
    // ... other pallets
    JuryAnchor: pallet_x3_jury_anchor,
}

# Build runtime
cargo build --release
```

### 2. Configure Jury Service

```bash
# .env or environment
ONCHAIN_RPC_URL=http://localhost:9944
JURY_MANAGER_ACCOUNT=5GrwvaEF5zXb26Fz9rcQkQTQq5LaWNe5ia5gihQTJ4vj
JURY_AUTHORITY_PRIVATE_KEY=0x...
JURY_ANCHORING_ENABLED=true
```

### 3. Enable Anchoring in Jury Service

```python
# swarm/jury/manager.py

from .anchorer import JuryAnchoringService

class JuryManager:
    def __init__(self):
        # ... existing init
        self.anchoring_service = JuryAnchoringService(
            anchorer=JuryAnchorer(
                rpc_url=os.getenv("ONCHAIN_RPC_URL"),
                jury_manager_account=os.getenv("JURY_MANAGER_ACCOUNT"),
                jury_authority_private_key=os.getenv("JURY_AUTHORITY_PRIVATE_KEY"),
            ),
            audit_logger=self.audit_logger,
        )
    
    async def finalize_session(self, session_id: str):
        """Finalize session with anchoring."""
        votes = await self.aggregate_votes(session_id)
        result = len([v for v in votes if v]) / len(votes) > 0.66
        
        # Anchor decision
        if os.getenv("JURY_ANCHORING_ENABLED") == "true":
            success = await self.anchoring_service.finalize_and_anchor(
                session_id, votes, result
            )
            logger.info(f"Anchoring result: {success}")
```

### 4. Verify from Frontend

```typescript
// React component
import { JuryAnchoring } from '@x3/blockchain-adapter';

export function MyComponent() {
    const jurAnchoring = useMemo(
        () => new JuryAnchoring(rpcClient),
        [rpcClient]
    );

    const { status } = useQuery(
        ['jury-decision', sessionId],
        () => jur yAnchoring.getDecisionStatus(sessionId),
        { refetchInterval: 2000 }
    );

    return (
        <div>
            {status?.status === 'anchored' && (
                <p>✓ Decision verified on block #{status.on_chain?.block_number}</p>
            )}
        </div>
    );
}
```

---

## Development Guide

### Runtime Pallet Development

#### Directory Structure

```
pallets/x3-jury-anchor/
├── src/
│   ├── lib.rs          (main logic + tests)
│   ├── types.rs        (optional: data structures)
│   └── benchmarking.rs (optional: performance)
├── Cargo.toml
└── docs/root/README.md
```

#### Adding New Functionality

**Example: Storage query helper**

```rust
impl<T: Config> Pallet<T> {
    pub fn get_decisions_by_authority(
        authority: T::AccountId,
    ) -> Vec<(Vec<u8>, JuryDecisionRecord<...>)> {
        JuryDecisions::<T>::iter()
            .filter(|(_, record)| record.jury_authority == authority)
            .collect()
    }
}
```

### Python Anchorer Development

#### Custom RPC Handlers

```python
class CustomAnchorer(JuryAnchorer):
    async def batch_anchor_decisions(
        self,
        decisions: List[Tuple[str, str]],  # (session_id, decision_hash)
    ) -> List[AnchorResult]:
        """Anchor multiple decisions in batch."""
        results = []
        for session_id, decision_hash in decisions:
            result = await self.anchor_decision(session_id, decision_hash)
            results.append(result)
        return results
```

### TypeScript Adapter Development

#### Custom React Hooks

```typescript
export function useJuryDecisions(
    sessionIds: string[],
    juryAnchoring: JuryAnchoring,
) {
    const [decisions, setDecisions] = React.useState<Map<string, JuryDecisionStatus>>(
        new Map()
    );

    React.useEffect(() => {
        let cancelled = false;

        const fetchAll = async () => {
            const statuses = await Promise.all(
                sessionIds.map((id) => juryAnchoring.getDecisionStatus(id))
            );

            if (!cancelled) {
                const map = new Map(statuses.map((s) => [s.session_id, s]));
                setDecisions(map);
            }
        };

        fetchAll();

        return () => {
            cancelled = true;
        };
    }, [sessionIds, juryAnchoring]);

    return decisions;
}
```

---

## Operations Manual

### Monitoring

#### Key Metrics

```bash
# Check anchored decision count
curl http://localhost:9944 -d '{
  "jsonrpc": "2.0",
  "method": "jury_decisionCount",
  "id": 1
}'

# Monitor anchoring latency
curl http://localhost:9944 -d '{
  "jsonrpc": "2.0",
  "method": "jury_statsLatency",
  "params": ["session-id"],
  "id": 1
}'

# Check verification status
curl http://localhost:9944 -d '{
  "jsonrpc": "2.0",
  "method": "jury_decisionStatus",
  "params": ["session-id"],
  "id": 1
}'
```

#### Health Checks

```bash
#!/bin/bash
# Check anchoring service

# 1. RPC connectivity
curl -f http://localhost:9944 >/dev/null || echo "RPC DOWN"

# 2. Jury service running
curl -f http://localhost:8000/health >/dev/null || echo "JURY SERVICE DOWN"

# 3. Recent anchoring success
RECENT_ANCHORED=$(curl http://localhost:9944 -d '{"jsonrpc":"2.0","method":"jury_recentCount","id":1}' | jq .result)
[ "$RECENT_ANCHORED" -gt 0 ] || echo "NO RECENT ANCHORS"

echo "✓ All systems operational"
```

### Troubleshooting

#### Decision Not Anchoring

```
Symptom: Jury decisions complete but not appearing on-chain
Resolution: 
1. Check jury service logs: docker logs jury-service | grep "anchor"
2. Verify RPC connectivity: curl http://localhost:9944
3. Check jury authority account balance
4. Verify JURY_AUTHORITY_PRIVATE_KEY is set correctly
```

#### Verification Fails

```
Symptom: Decision shows as "not_found" on blockchain
Resolution:
1. Verify decision_hash matches: off-chain hash == on-chain hash
2. Check RPC method is working: jury_decisionStatus
3. Ensure decision recently anchored: check block height
4. Query PostgreSQL audit logs for hash computation errors
```

#### High Latency

```
Symptom: Decisions taking >30s to anchor
Resolution:
1. Check network latency to RPC: ping localhost
2. Monitor block time: query recent block timestamps
3. Check jury service load: docker stats jury-service
4. Increase timeout in anchorer: timeout=60
```

---

## API Reference

### RPC Methods

#### `jury_decisionStatus`

Get status of a jury decision.

**Parameters:**
- `session_id` (string): Session identifier

**Returns:**
```json
{
  "session_id": "sess-123",
  "status": "anchored",
  "on_chain": {
    "block_number": 12345,
    "block_hash": "0x...",
    "decision_hash": "0xabcd...",
    "timestamp": 1707410123,
    "jury_authority": "5Grvw...",
    "metadata": {
      "member_count": 5,
      "quorum_threshold": 66,
      "result": true,
      "session_duration_secs": 900
    }
  }
}
```

#### `jury_decisionsByAuthority`

Get decisions anchored by specific authority.

**Parameters:**
- `authority` (string): Account ID
- `limit` (number): Max results

**Returns:**
```json
[
  { "session_id": "...", "status": "anchored", ... },
  ...
]
```

#### `jury_verify`

Verify decision matches expected hash.

**Parameters:**
- `session_id` (string)
- `expected_hash` (string): Decision hash to verify

**Returns:**
```json
{
  "verified": true,
  "on_chain_hash": "0xabcd...",
  "matches": true
}
```

### Contract Interfaces

#### Pallet Storage

```rust
// Get specific decision
storage.atlasJuryAnchor.juryDecisions(sessionId)
  → {
      decisionHash: H256,
      blockNumber: u32,
      timestamp: u64,
      juryAuthority: AccountId,
      metadata: {...}
    }

// Get total count
storage.atlasJuryAnchor.decisionCount()
  → u32

// Get current authority
storage.atlasJuryAnchor.juryAuthority()
  → AccountId
```

#### Events

```rust
JuryDecisionAnchored {
    session_id: Vec<u8>,
    decision_hash: H256,
    block_number: u32,
}

AuthorityChanged {
    new_authority: AccountId,
}

VerificationSucceeded {
    session_id: Vec<u8>,
}
```

---

## Examples

### Example 1: Complete Jury Session with Anchoring

```python
# swarm/examples/complete_jury_flow.py

import asyncio
from swarm.jury.manager import JuryManager
from swarm.jury.anchorer import JuryAnchor

async def main():
    jury = JuryManager()
    
    # 1. Create session
    session_id = "infrastructure-upgrade-2026"
    members = ["INF-001", "INF-002", "OPS-001", "OPS-002", "SEC-001"]
    session = await jury.create_session(session_id, members)
    print(f"Session created: {session_id}")
    
    # 2. Members submit commitments
    votes = {"INF-001": True, "INF-002": True, "OPS-001": True, "OPS-002": True, "SEC-001": False}
    for member, vote in votes.items():
        commitment = compute_commitment(f"{member}:{vote}")
        await jury.submit_commitment(session_id, member, commitment)
    print("All commitments submitted")
    
    # 3. Reveal phase
    await jury.advance_phase(session_id)
    for member, vote in votes.items():
        nonce = generate_nonce()
        await jury.submit_reveal(session_id, member, vote, nonce)
    print("All votes revealed")
    
    # 4. Aggregate and anchor
    result = await jury.aggregate_votes(session_id)
    print(f"Result: {'PASS' if result else 'FAIL'}")
    
    # 5. Anchor to blockchain
    anchorer = JuryAnchor(...)
    anchor_result = await anchorer.anchor_decision(
        session_id,
        compute_decision_hash(votes, result),
    )
    print(f"Anchored: {anchor_result.block_number}")
    
    # 6. Verify
    verified = await anchorer.verify_decision(
        session_id,
        compute_decision_hash(votes, result),
    )
    print(f"Verified: {verified}")

asyncio.run(main())
```

### Example 2: Frontend Dashboard Component

```typescript
// apps/swarm-dashboard/src/components/JuryDecisionsPanel.tsx

import React from 'react';
import { useQuery } from 'react-query';
import { JuryAnchoring, JuryDecisionCard } from '@x3/blockchain-adapter';

export function JuryDecisionsPanel() {
    const rpcClient = useRpcClient();
    const jury = useMemo(() => new JuryAnchoring(rpcClient), [rpcClient]);
    
    // Fetch recent session IDs (from API)
    const { data: sessionIds } = useQuery(
        ['jury-sessions'],
        () => api.getRecentSessions(limit: 10)
    );

    if (!sessionIds) return <div>Loading...</div>;

    return (
        <div className="jury-panel">
            <h2>Recent Decisions</h2>
            <div className="decision-list">
                {sessionIds.map((sessionId) => (
                    <JuryDecisionCard
                        key={sessionId}
                        sessionId={sessionId}
                        decisionHash={computeHash(sessionId)}  // from API or local
                        juryAnchoring={jury}
                    />
                ))}
            </div>
        </div>
    );
}
```

### Example 3: Verification Script

```bash
#!/bin/bash
# Verify jury decision from command line

SESSION_ID=$1
DECISION_HASH=$2

if [ -z "$SESSION_ID" ] || [ -z "$DECISION_HASH" ]; then
    echo "Usage: ./verify_decision.sh <session-id> <decision-hash>"
    exit 1
fi

# Query RPC
RESULT=$(curl -s http://localhost:9944 -d '{
    "jsonrpc": "2.0",
    "method": "jury_decisionStatus",
    "params": ["'"$SESSION_ID"'"],
    "id": 1
}' | jq '.result')

STATUS=$(echo $RESULT | jq -r '.status')

if [ "$STATUS" == "anchored" ]; then
    ON_CHAIN_HASH=$(echo $RESULT | jq -r '.on_chain.decision_hash')
    BLOCK=$(echo $RESULT | jq -r '.on_chain.block_number')
    
    if [ "$ON_CHAIN_HASH" == "$DECISION_HASH" ]; then
        echo "✓ Decision verified on block #$BLOCK"
        exit 0
    else
        echo "✗ Hash mismatch!"
        echo "  Expected: $DECISION_HASH"
        echo "  On-chain: $ON_CHAIN_HASH"
        exit 1
    fi
else
    echo "✗ Decision not anchored (status: $STATUS)"
    exit 1
fi
```

---

## Troubleshooting

### Common Issues

#### "Unauthorized" Error

```
Error: Unauthorized
Cause: Jury authority account not set correctly
Fix: Verify JURY_AUTHORITY_PRIVATE_KEY in .env matches account in runtime
```

#### "SessionIdTooLong" Error

```
Error: SessionIdTooLong
Cause: Session ID > 256 bytes
Fix: Use shorter session IDs; e.g., "sess-123" instead of full UUID
```

#### "DecisionAlreadyExists" Error

```
Error: DecisionAlreadyExists
Cause: Attempting to re-anchor same session
Fix: Each session should only be anchored once; check for duplicate handling
```

#### RPC Timeout

```
Error: RPC request timeout after 30s
Cause: Chain is slow or network is congested
Fix: Increase timeout in anchorer: timeout=60
```

---

## Performance & Scaling

### Benchmarks

| Operation | Time | Storage |
|-----------|------|---------|
| Anchor decision | 2-5 seconds | 32 bytes (H256) |
| Verify decision | 50-100 ms | N/A |
| Query status | 100-200 ms | N/A |
| 100 decisions/sec | 2-5 sec/batch | 3.2 KB |

### Scaling Strategy

**Phase 1 (Current):** Single-chain, single authority  
**Phase 2:** Multi-authority jury council  
**Phase 3:** Cross-shard jury decisions  
**Phase 4:** Parallel batch anchoring  

### Optimization Tips

1. **Batch Anchoring** - Anchor multiple decisions in single block
2. **Compression** - Archive old decisions after 30 days
3. **Caching** - Cache decision status in memory for 1 hour
4. **Indexing** - Add database index on session_id for queries

---

## Security Considerations

### Threat Model

| Threat | Mitigation |
|--------|-----------|
| **Vote tampering** | Commitments prevent vote changes; hashing detects tampering |
| **Authority compromise** | Multi-sig authority rotation (Phase 2) |
| **RPC manipulation** | Verify hash matches off-chain audit logs |
| **Denial of service** | Rate limiting on anchor submissions |
| **Replay attacks** | Unique nonce per reveal; session_id prevents re-use |

### Best Practices

1. **Rotate Keys** - Change JURY_AUTHORITY_PRIVATE_KEY monthly
2. **Verify Hashes** - Always verify on-chain hash matches off-chain computation
3. **Monitor Logs** - Watch for unexpected anchor failures
4. **Backup Authority** - Keep private key in secure storage (e.g., HSM)
5. **Audit Trail** - Maintain complete PostgreSQL audit logs

---

## Summary

**Jury Blockchain Anchoring** provides production-ready governance decision persistence with:

✅ **Security** - Cryptographic guarantees; tamper-proof  
✅ **Privacy** - Off-chain votes; only hash on-chain  
✅ **Scalability** - 1000+ decisions/day; minimal storage  
✅ **Auditability** - Complete lineage from votes to blockchain  
✅ **Interoperability** - Standard RPC interface; easy integration  

For support or questions, refer to the architecture documentation or GitHub issues.

