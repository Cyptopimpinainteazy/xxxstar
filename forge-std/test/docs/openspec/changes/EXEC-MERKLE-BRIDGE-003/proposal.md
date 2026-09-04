# Gap #3: Cross-VM Bridge Integration with State Merkle Proofs

**Proposal ID:** `EXEC-MERKLE-BRIDGE-003`

**Status:** APPROVED (Implementing)

**Authors:** X3 Chain Engineering

**Date Created:** 2026-04-06

---

## Summary

Integrate state merkle proofs (Gap #2) into the cross-VM bridge system to enable cryptographically verified settlement of atomic operations across EVM and SVM. The bridge will validate state claims using compact merkle proofs before committing to canonical ledger changes, eliminating the need to trust external chain state synchronizers and providing gas-efficient on-chain verification.

---

## Motivation

### Problem Statement

The current cross-VM bridge (`cross-vm-coordinator` and `cross-vm-bridge`) lacks cryptographic verification that execution state on one VM matches claims from another VM. This creates a trust gap:

1. **EVM→SVM transfers** rely on `EVM state monitor` to report accurate balance changes
2. **SVM→EVM transfers** rely on `SVM state indexer` to report accurate account state
3. **No proof of settlement** — Bridge accepts state claims without cryptographic backing
4. **Trust assumption** — Requires external chain monitors to be honest

### Why Gap #3 Matters

By integrating merkle proofs:
- **Eliminate external trust** — Proofs are self-contained and cryptographically verifiable
- **Gas-efficient settlement** — Compact proofs (O(log n)) reduce on-chain verification cost
- **Deterministic cross-VM atomicity** — State changes are verifiable before commitment
- **Foundation for multi-chain** — Pattern scales to Solana, other L2s

---

## Design

### Architecture Overview

```
┌────────────────────────────────────────────────────────┐
│          CROSS-VM ATOMIC SWAP WITH MERKLE PROOFS       │
├────────────────────────────────────────────────────────┤
│                                                        │
│  1. EVM Execution                2. SVM Execution    │
│  ┌──────────────────┐            ┌─────────────────┐ │
│  │ • Transfer 100   │            │ • Receive 100   │ │
│  │ • Output hash₀   │            │ • Output hash₁  │ │
│  └──────────┬───────┘            └────────┬────────┘ │
│             │                             │          │
│             └─────────────────┬───────────┘          │
│                               │                      │
│          3. GPU Validator Proves Execution           │
│          ┌────────────────────────────────┐          │
│          │ Merkle Root:                   │          │
│          │ ├─ hash₀ (EVM output)          │          │
│          │ ├─ hash₁ (SVM output)          │          │
│          │ └─ UnifiedProof with merkle    │          │
│          │   • Byzantine attestations     │          │
│          │   • MerkleProofPath for each   │          │
│          │     execution output           │          │
│          └─────────────┬──────────────────┘          │
│                        │                             │
│          4. Bridge Verifies Merkle Proofs            │
│          ┌────────────────────────────────┐          │
│          │ For each execution:             │          │
│          │ ├─ Verify merkle path          │          │
│          │ ├─ Check state root matches    │          │
│          │ └─ Validate Byzantine quorum   │          │
│          └─────────────┬──────────────────┘          │
│                        │                             │
│          5. Atomic Settlement                         │
│          ┌────────────────────────────────┐          │
│          │ Both succeed or both rollback  │          │
│          │ • Ledger state updated        │          │
│          │ • No trust in external chains │          │
│          └────────────────────────────────┘          │
│                                                      │
└────────────────────────────────────────────────────────┘
```

### Integration Points

#### 1. **Cross-VM Coordinator** (`crates/cross-vm-coordinator/src/`)

**Current:** HTLC state machine with `LockedPhase`, `RevealedPhase`, `SettledPhase`

**Gap #3 adds:** Merkle proof validation in settlement logic

```rust
// New type in coordinator
pub struct MerkleProofSettlement {
    pub merkle_proof: StateMerkleProof,
    pub execution_index: usize,       // Which output in merkle tree
    pub validator_signatures: Vec<(ValidatorId, Signature)>,  // Byzantine consensus
}

// Modify SettledPhase to require merkle proof
pub struct SettledPhase {
    pub settlement: MerkleProofSettlement,  // NEW: Merkle-backed settlement
    pub evm_receipt: EvmReceipt,
    pub svm_receipt: SvmReceipt,
}
```

#### 2. **Cross-VM Bridge** (`crates/cross-vm-bridge/src/`)

**Current:** Two-phase commit (2PC) protocol with nonce tracking

**Gap #3 adds:** Merkle proof verification before commit

```rust
// New validator trait
pub trait MerkleProofValidator {
    fn verify_settlement_proof(
        &self,
        proof: &StateMerkleProof,
        evm_state_root: [u8; 32],
        svm_state_root: [u8; 32],
    ) -> Result<(), BridgeError>;
}

// Modify bridge settlement
impl CrossVmBridge {
    pub fn finalize_atomic_swap_with_proof(
        &mut self,
        proof: MerkleProofSettlement,
    ) -> Result<(), BridgeError> {
        // 1. Verify merkle proof cryptographically
        self.merkle_validator.verify_settlement_proof(
            &proof.merkle_proof,
            self.evm_state_root,
            self.svm_state_root,
        )?;
        
        // 2. Verify Byzantine consensus
        self.verify_validator_signatures(&proof.validator_signatures)?;
        
        // 3. Atomically update both VM states (commit phase)
        self.commit_settlement(proof)?;
        
        Ok(())
    }
}
```

#### 3. **Bridge Adapters** (`crates/x3-bridge-adapters/src/`)

**Current:** `RuntimeCrossVmDispatcher` with real VM execution (Gap #2 deliverable)

**Gap #3 adds:** Merkle proof collection on execution

```rust
// Modify RuntimeCrossVmDispatcher to track merkle state
impl RuntimeCrossVmDispatcher {
    pub fn execute_atomic_with_merkle_collection(
        &self,
        evm_target: [u8; 20],
        svm_program_id: [u8; 32],
        evm_data: Vec<u8>,
        svm_data: Vec<u8>,
    ) -> Result<(ExecutionReceipt, StateMerkleProof), BridgeError> {
        // Execute both VMs
        let evm_result = self.execute_evm(evm_target, 0, evm_data)?;
        let svm_result = self.execute_svm(svm_program_id, svm_data)?;
        
        // Build merkle proof from outputs
        let outputs = vec![
            &evm_result.state_changes[..],
            &svm_result.state_changes[..],
        ];
        
        let merkle_proof = generate_merkle_proof_from_outputs(outputs)?;
        
        Ok((receipt, merkle_proof))
    }
}
```

#### 4. **Unified Proof Structure** (Extend Gap #1)

**Current:** `UnifiedProof` with optional `state_merkle_proof` field

**Gap #3 adds:** Full integration of merkle field in bridge workflows

```rust
// Already exists from Gap #2, now bridge uses it
pub struct UnifiedProof {
    pub header: ProofHeader,
    pub execution_result: ExecutionResult,
    pub validator_attestations: Vec<GpuValidatorAttestation>,
    pub state_merkle_proof: Option<StateMerkleProof>,  // ← Bridge uses this
    pub proof_aggregation_data: ProofAggregationData,
}
```

---

## Implementation Steps

### Phase 1: Merkle Proof Validator in Bridge
- [ ] Create `MerkleProofValidator` trait in `cross-vm-bridge`
- [ ] Implement SHA-256 based verification
- [ ] Add validator signature verification (Byzantine consensus)
- [ ] Add unit tests for proof validation

### Phase 2: Coordinator Settlement Integration
- [ ] Modify `SettledPhase` to include `MerkleProofSettlement`
- [ ] Add merkle proof validation to `finalize_settlement()` 
- [ ] Update HTLC state machine to require proofs
- [ ] Add integration tests for merkle-backed settlement

### Phase 3: Bridge Adapters Integration
- [ ] Wire merkle proof collection into `RuntimeCrossVmDispatcher`
- [ ] Add merkle proof to atomic swap receipts
- [ ] Update settlement to use merkle-verified state roots
- [ ] Add E2E tests

### Phase 4: Verification & Polish
- [ ] All bridge tests pass
- [ ] All coordinator tests pass
- [ ] E2E cross-VM test with merkle proofs
- [ ] Documentation updates

---

## Integration Points

### Code Changes

| File | Changes |
|------|---------|
| `crates/cross-vm-bridge/src/lib.rs` | Add `MerkleProofValidator` trait, modify settlement logic |
| `crates/cross-vm-bridge/src/proof_validator.rs` | NEW — Merkle proof validation implementation |
| `crates/cross-vm-coordinator/src/lib.rs` | Add `MerkleProofSettlement` type, modify settlement phase |
| `crates/cross-vm-coordinator/src/settlement.rs` | NEW — Merkle-backed settlement logic |
| `crates/x3-bridge-adapters/src/lib.rs` | Wire merkle proof collection into dispatcher |
| `crates/x3-gpu-validator-swarm/src/state_merkle_proof.rs` | Already complete (Gap #2) — reuse |
| `crates/x3-gpu-validator-swarm/src/unified_proof.rs` | Already complete (Gap #1+2) — reuse |

### No Changes Required
- `pallets/x3-atomic-kernel/` — Already handles cross-chain finality
- `runtime/src/` — Bridge adapters abstract away details
- `crates/cross-vm-coordinator/src/persistence.rs` — No changes needed

---

## Invariants

### New Invariants to Register

1. **Merkle Proof Validity**
   - `MERKLE_PROOF_VALIDATION_INVARIANT`: Every cross-VM settlement must have a valid merkle proof path leading to claimed state root
   - Entry point: `MerkleProofValidator::verify_settlement_proof()`
   - Test: `test_merkle_settlement_proof_valid`, `test_merkle_settlement_proof_invalid_rejects`

2. **Byzantine Consensus in Settlement**
   - `BYZANTINE_SETTLEMENT_INVARIANT`: Settlement requires >2/3 validator signatures on merkle root
   - Entry point: `verify_validator_signatures()`
   - Test: `test_settlement_requires_byzantine_quorum`, `test_settlement_rejects_insufficient_validators`

3. **Atomic State Transitions**
   - `ATOMIC_MERKLE_SETTLEMENT_INVARIANT`: Both EVM and SVM state changes commit or both rollback based on merkle proof validity
   - Entry point: `commit_settlement()` in bridge
   - Test: `test_atomic_settlement_both_succeed`, `test_atomic_settlement_both_rollback`

4. **No Trust in External Chains**
   - `SELF_CONTAINED_PROOF_INVARIANT`: Settlement verification requires only local merkle proof + validator signatures, no external chain queries
   - Entry point: `finalize_atomic_swap_with_proof()`
   - Test: `test_settlement_no_external_chain_queries`, `test_settlement_works_offline`

---

## Testing Strategy

### Unit Tests (by component)

**Bridge Merkle Validator** (new file: `test_merkle_proof_validator.rs`)
```rust
#[test]
fn test_merkle_proof_validator_valid_proof() {}

#[test]
fn test_merkle_proof_validator_invalid_proof_rejected() {}

#[test]
fn test_merkle_proof_validator_wrong_state_root_rejected() {}

#[test]
fn test_merkle_proof_validator_byzantine_quorum_required() {}

#[test]
fn test_merkle_proof_validator_tampered_proof_rejected() {}
```

**Coordinator Settlement** (modify: `coordinator/tests/`)
```rust
#[test]
fn test_coordinator_settlement_with_merkle_proof() {}

#[test]
fn test_coordinator_settlement_rejects_invalid_proof() {}

#[test]
fn test_coordinator_settlement_atomic_both_succeed() {}

#[test]
fn test_coordinator_settlement_atomic_both_rollback() {}
```

### Integration Tests (new file: `tests/integration/merkle_bridge_settlement.rs`)

```rust
#[test]
fn test_e2e_atomic_swap_with_merkle_settlement() {
    // 1. Execute on EVM
    // 2. Execute on SVM
    // 3. Generate merkle proof for outputs
    // 4. Bridge verifies merkle proof
    // 5. Both state roots update atomically
}

#[test]
fn test_e2e_atomic_swap_merkle_proof_failure_triggers_rollback() {
    // 1. Execute on both VMs
    // 2. Generate INVALID merkle proof
    // 3. Bridge rejects settlement
    // 4. Both VMs rollback
}

#[test]
fn test_e2e_atomic_swap_byzantine_consensus_required() {
    // 1. Settlement requires >2/3 validator signatures
    // 2. Insufficient validators → rejection
}

#[test]
fn test_e2e_atomic_swap_offline_verification() {
    // 1. Verify merkle proof works without external chain queries
    // 2. Ensure no trust in external monitors
}
```

### Verification Matrix

| Component | Test Count | Status |
|-----------|-----------|--------|
| MerkleProofValidator | 5 unit | Pending |
| CoordinatorSettlement | 4 unit | Pending |
| BridgeIntegration | 4 integration | Pending |
| E2E Atomic Swap | 1 e2e | Pending |
| **Total** | **14** | **Pending** |

---

## Rollout Plan

### Phase 1: Foundation (Days 1-2)
- Implement `MerkleProofValidator` trait
- Add merkle proof validation logic
- Write unit tests for validator

### Phase 2: Coordinator Integration (Days 3-4)
- Modify settlement phase to include merkle proofs
- Update HTLC state machine
- Write coordinator settlement tests

### Phase 3: Bridge Wiring (Days 5-6)
- Wire merkle collection into adapters
- Update settlement to use proofs
- Write E2E tests

### Phase 4: Verification (Day 7)
- All tests pass
- Documentation complete
- Git commit and merge

---

## Risks & Mitigations

| Risk | Severity | Mitigation |
|------|----------|-----------|
| Merkle proof generation is expensive | MEDIUM | Proofs are O(log n); generation happens once per atomic swap |
| Byzantine consensus overhead | MEDIUM | Already required for finality; Gap #3 reuses existing validator set |
| External chain state desync | LOW | Merkle proofs are self-contained; no external chain queries needed |
| Breaking changes to bridge API | HIGH | Merkle proofs are optional initially; add feature flag if needed |
| Performance regression in settlement | MEDIUM | Benchmark before/after; proofs add ~2-3KB per settlement |

---

## Open Questions

1. **Sparse merkle trees for large state?** Current implementation uses dense merkle trees. Should we support sparse trees for partial state proofs?
   - **Answer:** Not in Gap #3; defer to future optimization phase

2. **Multiple hash functions?** Should bridge support BLAKE3, Keccak256 in addition to SHA-256?
   - **Answer:** No; SHA-256 is standard. Add feature flag if needed for other chains

3. **Proof compression?** Can we compress proofs by sharing prefixes across multiple outputs?
   - **Answer:** Possible future optimization. Gap #3 uses uncompressed proofs

4. **On-chain verification cost?** Should we estimate gas cost for EVM to verify merkle proofs?
   - **Answer:** Yes; measure in E2E tests and document in benchmarks

---

## References

### Related Proposals
- `EXEC-PROOF-UNIFY-001` — Gap #1: Unified Proof Format (DONE)
- `EXEC-MERKLE-PROOF-002` — Gap #2: State Merkle Proof Verification (DONE)

### Related Code
- `docs/state-merkle-proof-verification.md` — Merkle proof design
- `docs/gpu-validator-proof-aggregation.md` — Unified proof aggregation
- `crates/cross-vm-coordinator/src/` — HTLC state machine
- `crates/cross-vm-bridge/src/` — 2PC bridge protocol

### Standards
- [RFC 6962](https://tools.ietf.org/html/rfc6962) — Certificate Transparency (merkle trees)
- [FIPS 180-4](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf) — SHA-256

---

## Appendix: Example Code

### Settlement Flow with Merkle Proof

```rust
// Step 1: Bridge receives unified proof with merkle data
let unified_proof: UnifiedProof = receive_from_gpu_validator();

// Step 2: Extract merkle proof
let merkle_settlement = MerkleProofSettlement {
    merkle_proof: unified_proof.state_merkle_proof.unwrap(),
    execution_index: 0,
    validator_signatures: unified_proof.validator_attestations
        .iter()
        .map(|att| (att.validator_id, att.signature.clone()))
        .collect(),
};

// Step 3: Bridge verifies and settles atomically
bridge.finalize_atomic_swap_with_proof(merkle_settlement)?;

// Step 4: Both VM state roots updated
println!("Settlement complete. Both EVM and SVM state verified via merkle proof.");
```

---

**Approval:** Engineering Lead signature here
**Date Approved:** 2026-04-06
**Implementation Start:** 2026-04-06
**Target Completion:** 2026-04-13
