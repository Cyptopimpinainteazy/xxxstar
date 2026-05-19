# Dual-VM Implementation Plan
## Multi-Phase Plan for Track 2.1–2.7, 2.10

**Created:** 2026-04-30  
**Context:** Implementation of real Frontier EVM, real BPF SVM runtime, real cross-VM bridge finality verifier, real GPU proof verification, pallet executor authorization audit, benchmark weight runs, crypto signing audit, RBAC + emergency-pause governor + sudo replacement.

This plan uses **tracer-bullet vertical slices** - each phase delivers a working end-to-end increment that touches all layers (runtime, pallets, VMs, bridge, GPU, governance), starting minimal and expanding.

---

## Phase 1: Real VM Foundations (Week 1-2)
**Goal:** Deploy minimal real EVM and SVM implementations that can execute basic transactions.

### Tracer Bullet 1.1: Real Frontier EVM MVP
- **Runtime Layer:** Wire minimal Frontier pallet into runtime
- **VM Layer:** Replace mock EVM with real Frontier executor (basic contract deployment)
- **RPC Layer:** Enable basic EVM RPC endpoints
- **Testing:** Deploy and call a simple Solidity contract
- **Verification:** Confirm EVM state syncs to canonical ledger

### Tracer Bullet 1.2: Real BPF SVM MVP
- **Runtime Layer:** Wire minimal SVM pallet into runtime
- **VM Layer:** Replace mock SVM with real BPF runtime (basic program deployment)
- **RPC Layer:** Enable basic SVM RPC endpoints
- **Testing:** Deploy and execute a simple Solana program
- **Verification:** Confirm SVM state syncs to canonical ledger

---

## Phase 2: Cross-VM Bridge Core (Week 3-4)
**Goal:** Enable atomic cross-VM asset transfers with basic finality verification.

### Tracer Bullet 2.1: Cross-VM Message Passing
- **Bridge Layer:** Implement EVM-to-SVM and SVM-to-EVM message passing
- **Finality Layer:** Basic cross-VM bridge finality verifier (block confirmations)
- **Runtime Layer:** Add cross-VM extrinsic dispatch
- **Testing:** Atomic EVM→SVM and SVM→EVM transfers
- **Verification:** Transactions roll back on failure

### Tracer Bullet 2.2: Bridge Finality Hardening
- **Finality Layer:** Enhanced verifier with multi-block confirmation
- **Security Layer:** Add replay protection and state consistency checks
- **Testing:** Cross-VM transfers under network stress
- **Verification:** Bridge maintains atomicity across VM boundaries

---

## Phase 3: GPU Proof Verification (Week 5-6)
**Goal:** Integrate real GPU-based proof verification for cross-chain operations.

### Tracer Bullet 3.1: GPU Proof Verification MVP
- **GPU Layer:** Connect real GPU validator to runtime
- **Proof Layer:** Implement basic GPU proof verification for bridge transactions
- **Runtime Layer:** Add GPU proof extrinsic validation
- **Testing:** Verify GPU proofs for cross-VM operations
- **Verification:** GPU validation integrates with consensus

### Tracer Bullet 3.2: GPU Proof Optimization
- **Performance Layer:** Optimize GPU proof generation and verification
- **Scalability Layer:** Handle multiple concurrent proof requests
- **Testing:** GPU validation under load
- **Verification:** Proof verification meets latency requirements

---

## Phase 4: Security & Governance (Week 7-8)
**Goal:** Implement comprehensive security audits and governance mechanisms.

### Tracer Bullet 4.1: Executor Authorization Audit
- **Security Layer:** Pallet executor authorization audit (access control)
- **Governance Layer:** Basic RBAC for pallet execution
- **Runtime Layer:** Add authorization checks to all executors
- **Testing:** Authorization prevents unauthorized execution
- **Verification:** Audit logs capture all authorization events

### Tracer Bullet 4.2: Crypto Signing & Governance
- **Security Layer:** Crypto signing audit (key management and verification)
- **Governance Layer:** Emergency-pause governor and sudo replacement
- **Runtime Layer:** Implement governance pallet with multi-sig
- **Testing:** Crypto signing validates all transactions
- **Verification:** Emergency pause halts operations correctly

---

## Phase 5: Performance & Benchmarks (Week 9-10)
**Goal:** Optimize and benchmark all dual-VM operations.

### Tracer Bullet 5.1: Weight Benchmarking
- **Performance Layer:** Benchmark weight runs for all VM operations
- **Optimization Layer:** Tune gas/weight calculations
- **Runtime Layer:** Implement weight-based fee calculation
- **Testing:** Weight benchmarks under various loads
- **Verification:** Weights accurately reflect computational cost

### Tracer Bullet 5.2: End-to-End Performance
- **Integration Layer:** Full dual-VM pipeline benchmarking
- **Monitoring Layer:** Add performance metrics collection
- **Optimization Layer:** Performance tuning based on benchmarks
- **Testing:** Sustained TPS with dual-VM operations
- **Verification:** Meet or exceed performance targets

---

## Success Criteria

### Per Phase
- **Functional:** End-to-end feature works in testnet
- **Secure:** Basic security checks pass
- **Performant:** No obvious bottlenecks
- **Documented:** Implementation documented

### Overall
- Real VMs deployed and operational
- Cross-VM bridge with finality working
- GPU proof verification integrated
- Security audits complete
- Governance mechanisms functional
- Performance benchmarks met

---

## Dependencies & Risks

### Prerequisites
- Node infrastructure operational
- Basic RPC/WebSocket working
- Testnet environment available

### High-Risk Items
- GPU hardware availability
- Cross-VM state consistency
- Governance key management

### Mitigation
- Start with minimal implementations
- Extensive testing at each phase
- Rollback procedures for each tracer bullet

---

## Implementation Notes

- Each tracer bullet should be independently deployable
- Use feature flags to enable/disable new functionality
- Maintain backward compatibility throughout
- Comprehensive logging and monitoring from day 1
- Security review after each phase completion