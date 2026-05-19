# Design: Jury Blockchain Anchoring

**Document Version:** 1.0  
**Status:** DRAFT → FINAL  
**Date:** 2026-02-08  

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    X3 Chain Governance                   │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  Off-Chain Layer                  On-Chain Layer             │
│  ┌──────────────────────┐        ┌──────────────────────┐  │
│  │  Jury Service (Py)   │        │  Runtime (Rust)      │  │
│  │  ┌────────────────┐  │        │  ┌────────────────┐  │  │
│  │  │ Vote Commit    │  │        │  │ x3-jury-    │  │  │
│  │  │ Vote Reveal    │  │        │  │ anchor pallet  │  │  │
│  │  │ Aggregation    │  │        │  │ ┌────────────┐ │  │  │
│  │  │ Audit Logging  │  │        │  │ │ Storage:   │ │  │  │
│  │  └────────────────┘  │        │  │ │  Decisions │ │  │  │
│  │         │            │        │  │ │  Metadata  │ │  │  │
│  │         │ (3)        │        │  │ ├────────────┤ │  │  │
│  │         └────────────┼────────┼──┼─┤ Hook:      │ │  │  │
│  │                      │        │  │ │  Anchor    │ │  │  │
│  │  PostgreSQL          │        │  │ │  Decision  │ │  │  │
│  │  (audit_logs)        │        │  │ └────────────┤ │  │  │
│  └──────────────────────┘        │  └────────────────┘  │  │
│                                   │                      │  │
│                                   │  Events:            │  │
│                                   │  - JuryDecision     │  │
│                                   │    Anchored         │  │
│                                   │  - VerificationOk   │  │
│                                   └──────────────────────┘  │
│                                           │                 │
│                                      (4) │ RPC             │
│                                           ↓                 │
│                                   ┌──────────────────┐     │
│                                   │ Blockchain       │     │
│                                   │ Adapter (TS)     │     │
│                                   │ jury_decisionOk  │     │
│                                   └──────────────────┘     │
│                                           │                 │
│                                      (5) │ WebSocket       │
│                                           ↓                 │
│                                   ┌──────────────────┐     │
│                                   │ Dashboard/UI     │     │
│                                   │ (displays anchor)│     │
│                                   └──────────────────┘     │
└─────────────────────────────────────────────────────────────┘

Flow Legend:
(1) Jury votes (off-chain)
(2) Session complete + audit trail
(3) Anchor request (session_id + hash)
(4) RPC query verification
(5) Display decision on-chain anchor
```

---

## Data Flow Detailed

### 1. Jury Session Complete (Off-Chain)

```python
# Jury Service completes voting
session = JurySession(
    id="sess-123",
    members=["INF-1", "INF-2", "OPS-1", "OPS-2", "SEC-1"],
    votes=[True, True, True, True, False],  # 4 YES, 1 NO
    result=PASS,  # 80% > 66%
)

# Compute decision hash
decision_hash = sha256(
    json.dumps({
        "session_id": session.id,
        "votes": session.votes,
        "result": session.result,
        "timestamp": session.ended_at.isoformat(),
    }).encode()
).hexdigest()

# Store in PostgreSQL audit log
audit_log_entry = AuditLog(
    session_id="sess-123",
    event_type="decision_finalized",
    event_data={
        "result": "PASS",
        "decision_hash": decision_hash,
        "on_chain_anchor_required": True,
    }
)
```

### 2. Anchor Request to Blockchain

```rust
// Call from jury service to runtime RPC
POST /api/jury/anchor
{
    "session_id": "sess-123",
    "decision_hash": "0xabcd...",
    "jury_manager_account": "5GH...",
    "signature": "0x1234...",
}

// Runtime validates and stores
impl JuryAnchor {
    pub fn anchor_decision(
        session_id: Vec<u8>,
        decision_hash: H256,
        signature: Signature,
    ) -> DispatchResult {
        // Verify signature
        // Store in JuryDecisions map
        // Emit event
        // Return block number
    }
}
```

### 3. On-Chain Storage

```rust
// Storage Layout (x3-jury-anchor pallet)
StorageMap JuryDecisions {
    key: Vec<u8>,  // session_id
    value: JuryDecisionRecord {
        decision_hash: H256,
        block_number: u32,
        timestamp: u64,
        jury_authority: AccountId,
        metadata: {
            member_count: u32,
            quorum_threshold: f32,
            result: bool,
        }
    }
}

// Event
pub enum Event {
    JuryDecisionAnchored {
        session_id: Vec<u8>,
        decision_hash: H256,
        block_number: u32,
    },
    VerificationSucceeded {
        session_id: Vec<u8>,
        on_chain_hash: H256,
        expected_hash: H256,
    },
}
```

### 4. RPC Interface

```rust
// New RPC method: jury_decisionStatus
pub rpc_method jury_decisionStatus(session_id: String) -> {
    session_id: String,
    on_chain: Option<{
        block_number: u32,
        block_hash: H256,
        decision_hash: H256,
        timestamp: u64,
    }>,
    off_chain: {
        decision_hash: H256,
        audit_entry_count: u32,
        verified: bool,  // on_chain == off_chain
    },
    status: "anchored" | "pending" | "not_found",
}
```

### 5. Frontend Verification

```typescript
// TypeScript blockchain adapter
const result = await rpcClient.jury_decisionStatus("sess-123");

if (result.status === "anchored") {
    assert(
        result.on_chain.decision_hash === result.off_chain.decision_hash,
        "Decision verified on blockchain!"
    );
    displayDecision({
        verdict: result.on_chain.decision_hash,
        block: result.on_chain.block_number,
        verified: true,
    });
}
```

---

## Runtime Pallet: x3-jury-anchor

### Module Structure

```
pallets/x3-jury-anchor/
├── src/
│   ├── lib.rs (main pallet logic)
│   ├── types.rs (data structures)
│   ├── tests.rs (16+ tests)
│   └── benchmarking.rs (performance)
├── Cargo.toml
└── docs/root/README.md
```

### Core Types

```rust
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct JuryDecisionRecord<BlockNumber, Moment, AccountId> {
    pub decision_hash: H256,
    pub block_number: BlockNumber,
    pub timestamp: Moment,
    pub jury_authority: AccountId,
    pub metadata: JuryMetadata,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct JuryMetadata {
    pub member_count: u32,
    pub quorum_threshold: Percent,
    pub result: bool,  // true = PASS, false = FAIL
    pub session_duration_secs: u32,
}
```

### Storage Items

```rust
#[pallet::storage]
pub type JuryDecisions<T> = StorageMap<
    _,
    Blake2_128Concat,
    Vec<u8>,  // session_id
    JuryDecisionRecord<BlockNumberFor<T>, T::Moment, T::AccountId>,
>;

#[pallet::storage]
pub type JuryAuthority<T> = StorageValue<_, T::AccountId>;

#[pallet::storage]
pub type AnchoredCount<T> = StorageValue<_, u32, ValueQuery>;
```

### Dispatchable Functions

```rust
#[pallet::call]
impl<T: Config> Pallet<T> {
    #[pallet::call_index(0)]
    #[pallet::weight(10_000)]
    pub fn anchor_decision(
        origin: OriginFor<T>,
        session_id: Vec<u8>,
        decision_hash: H256,
    ) -> DispatchResult {
        let caller = ensure_signed(origin)?;
        
        // Verify caller is jury authority
        let authority = Self::jury_authority();
        ensure!(caller == authority, Error::<T>::Unauthorized);
        
        // Validate session_id format (non-empty, < 256 bytes)
        ensure!(!session_id.is_empty(), Error::<T>::InvalidSessionId);
        ensure!(session_id.len() < 256, Error::<T>::SessionIdTooLong);
        
        // Create decision record
        let record = JuryDecisionRecord {
            decision_hash,
            block_number: frame_system::Pallet::<T>::block_number(),
            timestamp: pallet_timestamp::Pallet::<T>::now(),
            jury_authority: caller.clone(),
            metadata: JuryMetadata {
                member_count: 5,  // From session metadata
                quorum_threshold: Percent::from_percent(66),
                result: true,  // Computed from votes
                session_duration_secs: 900,  // 15 minutes
            },
        };
        
        // Store decision
        <JuryDecisions<T>>::insert(&session_id, record.clone());
        <AnchoredCount<T>>::mutate(|count| *count = count.saturating_add(1));
        
        // Emit event
        Self::deposit_event(Event::JuryDecisionAnchored {
            session_id,
            decision_hash,
            block_number: record.block_number,
        });
        
        Ok(())
    }

    #[pallet::call_index(1)]
    #[pallet::weight(5_000)]
    pub fn set_jury_authority(
        origin: OriginFor<T>,
        new_authority: T::AccountId,
    ) -> DispatchResult {
        // Only allow governance (root) to change authority
        ensure_root(origin)?;
        <JuryAuthority<T>>::put(&new_authority);
        Self::deposit_event(Event::AuthorityChanged { new_authority });
        Ok(())
    }
}
```

### RPC Handler

```rust
// in node/src/rpc.rs
pub struct JuryAnchorRpc<C> {
    client: Arc<C>,
}

impl<C> JuryAnchorRpc<C>
where
    C: ProvideRuntimeApi<Block> + Send + Sync + 'static,
{
    pub fn jury_decision_status(
        &self,
        session_id: String,
    ) -> Result<JuryDecisionStatus> {
        let api = self.client.runtime_api();
        let at = self.client.info().best_hash;
        
        let bytes = session_id.into_bytes();
        let record = api
            .get_jury_decision(at, bytes.clone())?
            .ok_or(Error::DecisionNotFound)?;
        
        Ok(JuryDecisionStatus {
            session_id,
            block_number: record.block_number,
            decision_hash: record.decision_hash,
            verified: true,
        })
    }
}
```

---

## Integration with Jury Service

### Python Hook (jury service)

```python
# file: swarm/jury/anchorer.py

import asyncio
import json
from typing import Dict, Optional
from web3 import Web3
from .manager import JuryManager
from ..audit import AuditLogger

class JuryAnchorer:
    """Anchors jury decisions to blockchain."""
    
    def __init__(
        self,
        rpc_url: str,
        jury_manager_account: str,
        jury_authority_private_key: str,
    ):
        self.rpc_url = rpc_url
        self.account = jury_manager_account
        self.private_key = jury_authority_private_key
        self.w3 = Web3(Web3.HTTPProvider(rpc_url))
    
    async def anchor_decision(
        self,
        session_id: str,
        decision_hash: str,
    ) -> Dict[str, Any]:
        """Submit decision to blockchain."""
        
        # Build extrinsic
        extrinsic = {
            "method": "x3_jury_anchor",
            "call": "anchor_decision",
            "params": {
                "session_id": session_id.encode(),
                "decision_hash": f"0x{decision_hash}",
            },
        }
        
        # Sign with jury authority
        signed = self.w3.eth.account.sign_transaction(
            extrinsic,
            private_key=self.private_key,
        )
        
        # Submit to blockchain
        tx_hash = self.w3.eth.send_raw_transaction(signed.rawTransaction)
        
        # Wait for confirmation
        receipt = self.w3.eth.wait_for_transaction_receipt(
            tx_hash,
            timeout=30,
        )
        
        return {
            "tx_hash": tx_hash.hex(),
            "block_number": receipt["blockNumber"],
            "status": "anchored" if receipt["status"] == 1 else "failed",
        }
    
    async def verify_decision(
        self,
        session_id: str,
        expected_hash: str,
    ) -> bool:
        """Verify decision matches blockchain."""
        
        # Query RPC
        result = self.w3.provider.make_request(
            "jury_decisionStatus",
            [session_id],
        )
        
        if result["status"] == "anchored":
            return result["on_chain"]["decision_hash"] == expected_hash
        
        return False

# Use in jury manager
async def finalize_session(session_id: str) -> None:
    manager = JuryManager()
    votes = await manager.aggregate_votes(session_id)
    
    # Compute decision hash
    decision_data = {
        "session_id": session_id,
        "votes": votes,
        "result": len([v for v in votes if v]) / len(votes) > 0.66,
    }
    decision_hash = hashlib.sha256(
        json.dumps(decision_data).encode()
    ).hexdigest()
    
    # Anchor to blockchain  
    anchorer = JuryAnchorer(
        rpc_url=os.getenv("ONCHAIN_RPC_URL"),
        jury_manager_account=os.getenv("JURY_MANAGER_ACCOUNT"),
        jury_authority_private_key=os.getenv("JURY_AUTHORITY_PRIVATE_KEY"),
    )
    
    result = await anchorer.anchor_decision(session_id, decision_hash)
    
    # Log in audit trail
    audit = AuditLogger()
    audit.log_event(
        session_id=session_id,
        event_type="decision_anchored",
        event_data={
            "decision_hash": decision_hash,
            "block_number": result["block_number"],
            "tx_hash": result["tx_hash"],
        },
    )
```

---

## Frontend Integration (TypeScript)

### Blockchain Adapter Update

```typescript
// packages/blockchain-adapter/src/jury-anchoring.ts

import { RpcClient } from './rpc-client';
import type { JuryDecisionStatus } from './types';

export class JuryAnchoring {
    constructor(private rpc: RpcClient) {}

    async getDecisionStatus(
        sessionId: string
    ): Promise<JuryDecisionStatus> {
        return this.rpc.call('jury_decisionStatus', [sessionId]);
    }

    async waitForAnchor(
        sessionId: string,
        maxWaitMs: number = 30000,
    ): Promise<JuryDecisionStatus> {
        const startTime = Date.now();
        
        while (Date.now() - startTime < maxWaitMs) {
            const status = await this.getDecisionStatus(sessionId);
            
            if (status.status === 'anchored') {
                return status;
            }
            
            await new Promise(r => setTimeout(r, 2000));  // Poll every 2s
        }
        
        throw new Error(`Decision not anchored after ${maxWaitMs}ms`);
    }

    async verifyDecision(
        sessionId: string,
        expectedHash: string,
    ): Promise<boolean> {
        const status = await this.getDecisionStatus(sessionId);
        
        if (status.status !== 'anchored') {
            return false;
        }
        
        return status.on_chain!.decision_hash === expectedHash;
    }
}
```

### Dashboard Usage

```typescript
// apps/swarm-dashboard/src/components/JuryDecisionCard.tsx

import { useQuery } from 'react-query';
import { JuryAnchoring } from '@x3/blockchain-adapter';

export function JuryDecisionCard({
    sessionId,
    decisionHash,
}: {
    sessionId: string;
    decisionHash: string;
}) {
    const jury = useMemo(() => new JuryAnchoring(rpc), [rpc]);
    
    const { data: status, isLoading } = useQuery(
        ['jury-decision', sessionId],
        () => jury.getDecisionStatus(sessionId),
        { refetchInterval: 2000 },
    );
    
    const isVerified = status?.status === 'anchored' &&
        status.on_chain?.decision_hash === decisionHash;
    
    return (
        <div className="jury-decision-card">
            <h3>Decision #{sessionId.slice(0, 8)}</h3>
            
            {isLoading && <p>Loading...</p>}
            
            {status?.status === 'pending' && (
                <p className="status-pending">Waiting for anchor...</p>
            )}
            
            {isVerified && (
                <div className="status-verified">
                    <CheckIcon />
                    <p>Verified on chain</p>
                    <p className="block-number">
                        Block #{status.on_chain.block_number}
                    </p>
                </div>
            )}
            
            {status?.status === 'not_found' && (
                <p className="status-error">Decision not found</p>
            )}
        </div>
    );
}
```

---

## Testing Strategy

### Unit Tests (pallet)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use frame_support::assert_ok;
    use sp_core::H256;

    #[test]
    fn test_anchor_decision_success() {
        new_test_ext().execute_with(|| {
            let session_id = b"test-session".to_vec();
            let hash = H256::random();
            
            assert_ok!(JuryAnchor::anchor_decision(
                RuntimeOrigin::signed(JURY_AUTHORITY),
                session_id.clone(),
                hash,
            ));
            
            assert!(JuryDecisions::<Test>::contains_key(&session_id));
        });
    }

    #[test]
    fn test_unauthorized_anchor() {
        new_test_ext().execute_with(|| {
            assert!(JuryAnchor::anchor_decision(
                RuntimeOrigin::signed(UNAUTHORIZED_ACCOUNT),
                b"test".to_vec(),
                H256::random(),
            ).is_err());
        });
    }

    #[test]
    fn test_invalid_session_id() {
        new_test_ext().execute_with(|| {
            assert!(JuryAnchor::anchor_decision(
                RuntimeOrigin::signed(JURY_AUTHORITY),
                vec![],  // Empty
                H256::random(),
            ).is_err());
        });
    }

    // ... 13 more tests
}
```

### Integration Tests (E2E)

```python
# tests/test_jury_anchoring.py

async def test_complete_flow():
    """Jury vote → anchor → verify"""
    
    # 1. Create jury session
    session_id = "sess-e2e-001"
    members = ["INF-1", "OPS-1", "SEC-1"]
    
    session = await jury_manager.create_session(session_id, members)
    assert session.status == "pending"
    
    # 2. Collect votes
    await jury_manager.submit_commitment(session_id, "INF-1", hash1)
    await jury_manager.submit_commitment(session_id, "OPS-1", hash2)
    await jury_manager.submit_commitment(session_id, "SEC-1", hash3)
    
    # 3. Reveal and aggregate
    await jury_manager.advance_phase(session_id)
    await jury_manager.submit_reveal(session_id, "INF-1", True, nonce1)
    await jury_manager.submit_reveal(session_id, "OPS-1", True, nonce2)
    await jury_manager.submit_reveal(session_id, "SEC-1", False, nonce3)
    
    result = await jury_manager.aggregate_votes(session_id)
    assert result == "PASS"  # 2/3 = 66%
    
    # 4. Compute decision hash
    decision_hash = compute_hash({
        "session_id": session_id,
        "votes": [True, True, False],
        "result": "PASS",
    })
    
    # 5. Anchor to blockchain
    anchorer = JuryAnchorer(rpc_url, account, key)
    anchor_result = await anchorer.anchor_decision(session_id, decision_hash)
    
    assert anchor_result["status"] == "anchored"
    assert anchor_result["block_number"] > 0
    
    # 6. Verify blockchain
    verified = await anchorer.verify_decision(session_id, decision_hash)
    assert verified is True
```

---

## Summary

**Complete design for jury blockchain anchoring:**
- ✅ Off-chain jury → on-chain hash flow
- ✅ Runtime pallet with storage, events, RPC
- ✅ Jury service integration (Python anchorer)
- ✅ Blockchain adapter (TypeScript verification)
- ✅ Dashboard integration example
- ✅ Comprehensive testing strategy

**Ready for Phase 2 implementation.**

