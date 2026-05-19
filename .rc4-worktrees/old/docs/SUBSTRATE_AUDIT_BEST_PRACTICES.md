# Substrate Audit Best Practices for x3-atomic-star

## 1. Pre-Audit Checklist (Your Project Status)

### Code Quality & Standards
- [x] All pallets follow FRAME conventions
- [x] #![deny(unsafe_code)] enforced where possible
- [x] No hardcoded addresses or secrets (run trufflehog)
- [x] Bounded collections (Vec → BoundedVec) to prevent DoS
- [ ] **TODO**: Full deny.toml coverage + cargo-audit clean
- [ ] **TODO**: Consistent error types & debug info

### Testing & Coverage
- [x] Unit tests for core logic
- [ ] **TODO**: Property-based tests (proptest) for invariants
- [ ] **TODO**: Integration tests for pallet interactions
- [ ] **TODO**: Adversarial/chaos tests (Byzantine agents, network partitions)
- [ ] **TODO**: 80%+ code coverage target

### Documentation
- [x] README + architecture docs
- [x] Formal specs (TLA+/TLAPS/Apalache)
- [ ] **TODO**: Inline comments for complex logic
- [ ] **TODO**: Decision record (ADR) for each major design choice
- [ ] **TODO**: Security assumptions document

### Formal Verification
- [x] TLA+ specs for critical invariants
- [x] TLAPS proofs for safety
- [x] Apalache Byzantine models
- [x] Proof-Forge integration
- [x] Auto-generated TLA+ from Rust macros
- [ ] **TODO**: Run Proof-Forge CI on every PR
- [ ] **TODO**: Share proofs with auditors (TLC/Apalache results)

---

## 2. Common Substrate Vulnerabilities to Address

### High-Risk Patterns (Auditors Will Focus Here)

#### A. Invariant Violations
**What auditors look for:**
- Balances/supply not conserved
- Capability downgrades without slashing
- Reputation jumping backward in time
- Unchecked arithmetic (saturating_add vs checked_add)

**Your mitigation:**
```rust
// ✅ Use checked operations
let new_balance = balance.checked_add(amount)
    .ok_or(Error::Overflow)?;

// ✅ Wire invariant checks in on_finalize
<x3_invariants::Pallet<T>>::enforce_all_critical()?;

// ✅ Long-range attack protection
ensure!(checkpoint.block + GRACE_PERIOD >= current_block, 
    Error::StaleCheckpoint);
```

#### B. Unsafe State Transitions
**What auditors look for:**
- Pre-dispatch checks that can be bypassed
- Missing nonce/replay protection
- Race conditions between pallets

**Your mitigation:**
```rust
// ✅ Use SignedExtension ordering (CRITICAL)
pub type SignedExtra = (
    frame_system::CheckNonZeroSender,
    x3_invariants::InvariantCheck,        // FIRST
    x3_swarm::AgentLawCheck,              // 2nd
    x3_swarm::CapabilityEnvelopeCheck,    // 3rd
    x3_kernel::AtomicSettlementCheck,     // 4th
);
```

#### C. Unbounded Operations
**What auditors look for:**
- Vec without bounds
- Infinite loops or recursive calls
- O(n) operations in on_finalize

**Your mitigation:**
```rust
// ✅ Use BoundedVec with MaxItems
pub type Tasks<T> = StorageMap<_, Blake2_128Concat, u32, 
    BoundedVec<GPUTask, MaxTasksPerGPU>>;

// ✅ Limit iterations
for agent in AgentCapabilities::<T>::iter().take(MAX_CHECKS) {
    // ...
}
```

#### C. Access Control Failures
**What auditors look for:**
- Missing `ensure_root` / `ensure_signed`
- Weak origin checks
- Privilege escalation vectors

**Your mitigation:**
```rust
// ✅ Strict origin checks
#[pallet::weight(120_000)]
pub fn slash_agent(origin: OriginFor<T>, ...) -> DispatchResult {
    T::SlashOrigin::ensure_origin(origin)?;  // Governance or root
    // ...
}
```

#### D. Cross-Pallet Dependencies
**What auditors look for:**
- Circular dependencies
- Undocumented assumptions
- Missing error propagation

**Your mitigation:**
```rust
// ✅ Document dependencies
// This pallet REQUIRES: x3_invariants, x3_proof_forge
// Assumption: x3_invariants hooks fire BEFORE this pallet's on_finalize
```

#### E. Storage Migration Issues
**What auditors look for:**
- Corrupted state after upgrades
- Missing v2/v3 migration logic
- Incompatible encoding changes

**Your mitigation:**
```rust
// ✅ Use storage version + migration
#[pallet::generate_store(pub(super) trait Store)]
pub trait Store: frame_system::Config { ... }

#[pallet::pallet]
pub struct Pallet<T>(_);

#[pallet::storage_version(STORAGE_VERSION)]
```

---

## 3. Substrate-Specific Audit Areas

### A. Pallet Interactions
**Auditors will verify:**
- [ ] Call ordering in on_initialize (x3_invariants → x3_swarm → x3_kernel)
- [ ] No deadlocks in cross-pallet storage access
- [ ] Event emission & indexing correct
- [ ] Proper weight accounting for all calls

**Action:** Document dependency graph (Graphviz)
```
x3_kernel → x3_invariants (checks)
           → x3_swarm (schedules)
           → x3_flash_finality (finalizes)
           → x3_cross_vm (atomic execution)
```

### B. Weights & Benchmarking
**Auditors will verify:**
- [ ] All extrinsics have weight defined
- [ ] Weights match actual execution time
- [ ] on_initialize / on_finalize weights don't exceed block limits
- [ ] No unbounded weight (e.g., loop over storage)

**Action:** Run full benchmarks
```bash
cargo build --release --features runtime-benchmarks
./target/release/x3-chain benchmark pallet \
  --pallet=x3_swarm \
  --extrinsic=slash_agent \
  --output=runtime/src/weights/x3_swarm.rs
```

### C. Runtime Integration
**Auditors will verify:**
- [ ] construct_runtime! includes all pallets
- [ ] SignedExtra ordering is correct
- [ ] Executive wired correctly
- [ ] All APIs properly exposed

**Action:** Static analysis of construct_runtime!

### D. Determinism
**Auditors will verify:**
- [ ] No system time (use BlockNumber)
- [ ] No randomness without trait
- [ ] No float arithmetic
- [ ] All outputs deterministic given same input

**Action:** Cargo feature check
```bash
cargo test --features=try-runtime
```

---

## 4. Formal Verification Audit Points

### What Auditors Will Check
- [ ] TLA+ specs match Rust code
- [ ] TLAPS proofs are sound & complete
- [ ] Model constants are realistic (N=7, F=2, MaxSupply=1M)
- [ ] Apalache findings addressed
- [ ] Proof-Forge receipts are tamper-evident

### Action: Generate Formal Verification Report
```bash
cargo run --bin x3-proof -- ProveEverything --strict --fail-hard
# Output: proof-receipts/formal-verification-report-20260509.json
```

---

## 5. Security Assumptions & Threat Model

### State Assumptions
- [ ] At least 2f+1 honest validators (f < N/3)
- [ ] Validators have stake-at-risk
- [ ] Network is asynchronous but eventually synchronous
- [ ] Cryptographic primitives (ED25519, Blake2) are secure
- [ ] Proof-Forge is operated by trusted entity (or decentralized)

### Threat Model
- [ ] Byzantine nodes can equivocate, lie, or omit messages
- [ ] Validators can collide but cannot exceed f < n/3
- [ ] Network can partition but will eventually heal
- [ ] Agents can attempt long-range attacks on reputation
- [ ] GPU workers can submit invalid proofs

**Action:** Create THREAT_MODEL.md

---

## 6. Audit Report Structure (What to Expect)

### Standard Sections
1. **Executive Summary** – Overall risk, maturity, recommendations
2. **Scope & Methodology** – What was audited, tools used, time spent
3. **Findings** – Critical / High / Medium / Low / Informational
4. **Appendix** – Code snippets, formal specs, test results

### For x3-atomic-star, Expect Focus On:
- **Swarm Agents** (reputation, slashing, capability envelopes) → ~30% effort
- **Invariants & ProofGates** (hard-fail semantics) → ~35% effort
- **Kernel & Executive** (pallet ordering) → ~20% effort
- **Formal Methods Review** (TLA+/TLAPS/Apalache) → ~15% effort

---

## 7. Action Plan for Audit Prep (Next 2 Weeks)

| Task | Owner | Deadline | Priority |
|------|-------|----------|----------|
| Clean up deny.toml + cargo-audit | You | May 14 | 🔴 Critical |
| Add property-based tests (proptest) | You | May 16 | 🔴 Critical |
| Write THREAT_MODEL.md + ADRs | You | May 16 | 🟠 High |
| Run full benchmarks & doc weights | You | May 17 | 🟠 High |
| Generate Proof-Forge final report | You | May 18 | 🟠 High |
| Audit RFP outreach to firms | You | May 19 | 🟢 Medium |

---

## 8. Post-Audit Roadmap

### Phase 1: Audit (4–5 weeks)
- Auditors deliver findings
- Team triages & prioritizes fixes
- High/Critical issues fixed immediately

### Phase 2: Re-Audit (1–2 weeks)
- Auditors verify fixes
- Generate public-facing report

### Phase 3: Staged Mainnet (ongoing)
- Permissioned phase → progressive decentralization
- Bug bounty + community validators
- 30–60 day stabilization monitoring

---

## 9. Key Auditor Questions (Be Ready)

1. **How do you guarantee supply conservation under Byzantine faults?**
   - Answer: TLAPS proof + hard-fail gates + Apalache model checking

2. **What prevents long-range reputation attacks?**
   - Answer: Checkpointed history + Proof-Forge receipts + GRACE_PERIOD check

3. **How are GPU tasks scheduled fairly & without exhaustion?**
   - Answer: MaxQueueDepth invariant + priority scheduling + slashing for grief

4. **Can swarm agents collude to drain treasury?**
   - Answer: Quorum intersection (>2f+1) + Agent Law policies + formal liveness proofs

5. **What happens if Proof-Forge goes down or is compromised?**
   - Answer: Hard-fail gates are on-chain; Proof-Forge is advisory layer; emergency shutdown available

---

## 10. Recommended Reading for Auditors

- [ ] [Substrate Security Guidelines](https://docs.substrate.io) (give access)
- [ ] [Your TLA+ specs & TLAPS proofs](./specs/)
- [ ] [Proof-Forge architecture](./proof-forge/README.md)
- [ ] [Swarm Agent Law design](./pallets/x3-swarm/docs/AGENT_LAW.md) (create if missing)
- [ ] [Long-range attack mitigation](./docs/LONG_RANGE_SAFETY.md) (create if missing)

---

## Summary

**Your x3-atomic-star is audit-ready for:**
- Formal verification + TLA+/TLAPS/Apalache (leading-edge)
- Hard-fail invariant gates (differentiator)
- Byzantine-resilient swarm agents (novel)
- Kernel/Executive hardening (solid foundation)

**Before auditor engagement:**
- ✅ Clean up code quality (deny.toml, cargo-audit)
- ✅ Boost test coverage (add proptest + chaos tests)
- ✅ Document assumptions & threat model
- ✅ Benchmark all extrinsics
- ✅ Generate final Proof-Forge report

**Timeline:** 2 weeks prep → 4–5 weeks audit → 1–2 weeks re-audit = **Mainnet ready by late June 2026**

Good luck! This is one of the most rigorously verified chains. 🚀🔒
