# Proposal: Jury Blockchain Anchoring (Phase 5)

**ID:** jury-blockchain-anchoring  
**Status:** DRAFT → APPROVED (PENDING)  
**Authors:** GitHub Copilot  
**Date:** 2026-02-08  
**Priority:** HIGH (Completes jury system)  

---

## Problem Statement

The jury governance system (Phase 1-4) currently operates off-chain with immutable audit logging but **no persistence to blockchain**. This creates:

1. **Trust Gap** - Jury decisions exist only in PostgreSQL, vulnerable to database compromise
2. **Auditability Gap** - No cross-chain verification; governance decisions not cryptographically anchored
3. **Integration Gap** - Jury decisions cannot trigger on-chain actions (token transfers, parameter changes, etc.)
4. **Compliance Gap** - Regulatory requirements for immutable decision records unmet

### Constraints
- Must not break existing jury system (backward compatible)
- Must support multiple jury decisions per block
- Must scale to 1000+ decisions/day
- Must preserve vote privacy (off-chain votes, on-chain only hash)
- Must work with existing consensus mechanism

---

## Solution Overview

**Implement Jury Blockchain Anchoring**: Persist jury decision hashes to blockchain, creating:

1. **On-Chain Decision Record** - SHA256 hash of jury session + votes on immutable ledger
2. **Cross-Chain Verification** - External systems can verify jury decisions via RPC
3. **Governance Actions** - On-chain contracts trigger based on jury verdicts
4. **Audit Trail** - Complete governance lineage (off-chain audit logs → on-chain hash anchor)

### Key Design Decisions

| Component | Approach | Rationale |
|-----------|----------|-----------|
| **Data Model** | Jury session → SHA256(votes + metadata) | Privacy-preserving; immutable |
| **Runtime Pallet** | x3-jury-anchor pallet | Integrated with consensus |
| **Signature** | Jury manager signs anchor | Authority-based; prevents forgery |
| **Storage** | JuryDecisions map (session_id → hash) | O(1) lookup; efficient queries |
| **Events** | JuryDecisionAnchored event | Chain-observable; indexable |

---

## Scope

### In Scope (Phases 1-4)
- ✅ Jury blockchain anchor runtime pallet (Rust)
- ✅ Off-chain jury → on-chain hook
- ✅ RPC method for verification (jury_decisionStatus)
- ✅ Blockchain adapter integration (TypeScript)
- ✅ Tests (16+ tests)
- ✅ Documentation (guides + examples)
- ✅ E2E examples (deploy + vote + anchor)

### Out of Scope (Future)
- ❌ On-chain voting (off-chain remains)
- ❌ Jury member rotation (separate change)
- ❌ Parameter governance (separate change)
- ❌ Cross-shard jury (future scaling)

---

## Implementation Plan

### Phase 1: Specification
1. Write design.md with pallet architecture
2. Create spec.md with technical details
3. Define RPC interfaces and events
4. Estimate completion: 2 hours

### Phase 2: Core Implementation
1. Create x3-jury-anchor pallet (Rust)
2. Integrate with jury service (Python)
3. Add RPC handlers
4. Implement tests (16+ cases)
5. Estimate completion: 4 hours

### Phase 3: Integration & Deployment
1. Update docker-compose with chain node
2. Create migration scripts
3. Write deployment guide
4. Estimate completion: 2 hours

### Phase 4: Frontend & Examples
1. Update blockchain adapter (TypeScript)
2. Build dashboard example
3. Create E2E example workflows
4. Write comprehensive guides
5. Estimate completion: 3 hours

**Total Estimate:** 11 hours (fits "ship tomorrow" timeline)

---

## Success Criteria

### Functional
- [ ] Jury decisions anchor to blockchain deterministically
- [ ] RPC method returns correct decision hash and block number
- [ ] Off-chain audit logs match on-chain hash
- [ ] Multiple sessions per block supported

### Non-Functional
- [ ] No jury perf degradation (latency < 100ms)
- [ ] On-chain storage < 100KB per session
- [ ] Transaction finality < 6s
- [ ] RPC latency < 200ms

### Testing
- [ ] 16+ unit tests for pallet logic
- [ ] E2E test: create session → vote → anchor → verify
- [ ] Cross-chain verification test
- [ ] Scale test: 100 sessions/sec

### Documentation
- [ ] Technical design (design.md)
- [ ] API reference (RPC methods)
- [ ] Deployment guide
- [ ] Developer examples
- [ ] Troubleshooting guide

---

## Risk Analysis

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|-----------|
| Breaking jury service | Low | High | Backward compatible; feature flag |
| Storage bloat | Low | Medium | Compression; archival strategy |
| RPC performance | Low | Medium | Efficient indexing; caching |
| Vote privacy leak | Low | Critical | Hash only; no vote data on-chain |

---

## Rollout Plan

### Staging (Day 1)
1. Deploy pallet to staging testnet
2. Run E2E tests
3. Verify RPC methods
4. Internal validation

### Production (Day 2)
1. Deploy to mainnet (via governance)
2. Monitor for 24h
3. Update clients (dashboards, adapters)
4. Public announcement

### Rollback
- Feature flag: `JURY_ANCHORING_ENABLED` (default: on)
- Can disable without data loss
- Storage backward compatible

---

## Open Questions

1. **Storage Strategy** - Compress old anchors after N blocks?
   - **Answer:** Yes, implement compression phase in Phase 4.3

2. **Vote Data Storage** - Store full votes or hash only?
   - **Answer:** Hash only (privacy); off-chain audit logs contain full data

3. **Jury Authority** - Single signer or multi-sig?
   - **Answer:** Start with jury manager account; extend to multi-sig in Phase 6

4. **Cross-Chain** - Support other chains immediately?
   - **Answer:** No; single-chain for Phase 5; bridge in Phase 7

---

## References

- [Jury Governance System (Phase 1-4)](../add-offchain-jury/)
- [Blockchain Adapter](../../packages/blockchain-adapter/)
- [Runtime Pallet Structure](../../runtime/)
- [RPC Architecture](../../node/src/rpc.rs)

---

## Approval

**This proposal is ready for Phase 1 implementation.**

Next: Create design.md with full technical specification.

