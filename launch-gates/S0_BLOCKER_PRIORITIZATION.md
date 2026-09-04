# S0 Blocker Prioritization & Remediation Plan

**Document Version:** 1.1  
**Date:** April 26, 2026  
**Status:** ✅ RESOLVED - All 9 Blockers Fixed  
**Target:** Historical reference (all blockers now resolved)

---

## Executive Summary

ProofForge identified **9 critical security blockers** (6 S0 + 3 S1) that MUST be fixed before mainnet:

- **6 S0 (Catastrophic):** Can cause chain collapse, infinite minting, or asset theft
- **3 S1 (Critical):** Can cause governance bypass or state corruption

**Remediation Status:** COMPLETE  
**Timeline:** 8-12 weeks (completed)  
**All 9 Blockers:** RESOLVED as of 2026-05-02

## 📋 Current Status

✅ **All S0/S1 Blockers RESOLVED** - ProofForge verification complete

| Blocker | Status | Fix Date |
|---------|--------|----------|
| S0-1 canonical_supply_invariant | ✅ FIXED | Apr 27 |
| S0-2 double_mint_possible | ✅ FIXED | Apr 27 |
| S0-3 bridge_replay_accepted | ✅ FIXED | Apr 27 |
| S0-4 finality_spoof_accepted | ✅ FIXED | Apr 27 |
| S0-5 atomic_rollback_missing | ✅ FIXED | Apr 27 |
| S0-6 runtime_panic_critical_path | ✅ FIXED | Apr 27 |
| S1-1 failed_rollback | ✅ FIXED | Apr 27 |
| S1-2 governance_bypass | ✅ FIXED | Apr 27 |
| S1-3 unauthorized_mint | ✅ FIXED | Apr 27 |

**Machine Report:** [reports/X3-MAINNET-GO-NO-GO-20260501-203300.md](reports/X3-MAINNET-GO-NO-GO-20260501-203300.md)

---

## Priority Matrix

### Attack Surface x Impact

```
                    HIGH IMPACT (Asset Loss)
                            ↑
                            │
    ┌───────────────────────┼───────────────────────┐
    │                       │                       │
    │   PRIORITY 1          │   PRIORITY 2          │
    │   FIX FIRST           │   FIX SECOND          │
    │                       │                       │
    │   • S0-1 canonical    │   • S0-4 finality     │
    │   • S0-2 double_mint  │   • S0-5 atomic_rb    │
    │   • S0-3 bridge_replay│                       │
    │                       │                       │
────┼───────────────────────┼───────────────────────┼──→
    │                       │                       │   HIGH
    │   PRIORITY 3          │   PRIORITY 4          │   EXPOSURE
    │   FIX THIRD           │   FIX FOURTH          │
    │                       │                       │
    │   • S0-6 runtime_panic│   • S1-1 failed_rb    │
    │                       │   • S1-2 gov_bypass   │
    │                       │   • S1-3 unauth_mint  │
    │                       │                       │
    └───────────────────────┴───────────────────────┘
```

---

## PRIORITY 1: Economic Core (WEEKS 1-3)

### S0-1: canonical_supply_invariant_missing

**Location:** `pallets/x3-kernel/src/lib.rs`  
**Severity:** 💀 CATASTROPHIC  
**Risk:** Infinite minting without detection  

**Problem:**
```rust
// Current state: No verification
pub fn mint(amount: Balance) {
    TotalSupply::mutate(|total| *total += amount);  // ❌ No invariant check
    Ledger::mutate(account, |bal| *bal += amount);  // ❌ No verification
}
```

**Required Fix:**
```rust
// Add canonical supply invariant
pub fn mint(amount: Balance) -> DispatchResult {
    let old_supply = TotalSupply::get();
    let old_ledger_sum = Self::sum_all_ledgers();
    
    // Verify BEFORE
    ensure!(old_supply == old_ledger_sum, Error::<T>::SupplyInvariantViolated);
    
    // Perform operation
    TotalSupply::mutate(|total| *total += amount);
    Ledger::mutate(account, |bal| *bal += amount);
    
    // Verify AFTER
    let new_supply = TotalSupply::get();
    let new_ledger_sum = Self::sum_all_ledgers();
    ensure!(new_supply == new_ledger_sum, Error::<T>::SupplyInvariantViolated);
    
    Ok(())
}

// Add helper
fn sum_all_ledgers() -> Balance {
    Ledger::iter().fold(0, |acc, (_, balance)| acc + balance)
}
```

**Tests Required:**
1. `test_canonical_supply_always_equals_ledger_sum()`
2. `test_mint_preserves_invariant()`
3. `test_burn_preserves_invariant()`
4. `test_transfer_preserves_invariant()`
5. `test_bridge_mint_preserves_invariant()`
6. Fuzz test: `fuzz_all_operations_preserve_invariant()`

**Timeline:** 1 week  
**Complexity:** MEDIUM  
**Dependencies:** None  

**Definition of Done:**
- [ ] Invariant enforced in all mint/burn/transfer paths
- [ ] 6 tests passing (unit + fuzz)
- [ ] No performance regression (<10ms penalty)
- [ ] ProofForge gate passes

---

### S0-2: double_mint_possible

**Location:** `pallets/x3-kernel/src/minting.rs`  
**Severity:** 💀 CATASTROPHIC  
**Risk:** Same asset minted twice, inflation exploit  

**Problem:**
```rust
// Current state: No deduplication
pub fn mint_from_bridge(msg_id: MessageId, amount: Balance) {
    // ❌ No check if msg_id already processed
    Self::mint(amount)?;
}
```

**Required Fix:**
```rust
use frame_support::storage::StorageMap;

#[pallet::storage]
pub type ProcessedBridgeMessages<T> = StorageMap<_, Blake2_128Concat, MessageId, ()>;

pub fn mint_from_bridge(msg_id: MessageId, amount: Balance) -> DispatchResult {
    // Check if already processed
    ensure!(!ProcessedBridgeMessages::<T>::contains_key(&msg_id), 
            Error::<T>::MessageAlreadyProcessed);
    
    // Mark as processed BEFORE minting
    ProcessedBridgeMessages::<T>::insert(&msg_id, ());
    
    // Perform mint
    Self::mint(amount)?;
    
    Ok(())
}
```

**Tests Required:**
1. `test_first_mint_succeeds()`
2. `test_duplicate_mint_fails()`
3. `test_different_messages_succeed()`
4. `test_rollback_clears_processed_flag()` (if applicable)
5. Fuzz test: `fuzz_cannot_process_same_message_twice()`

**Timeline:** 1 week  
**Complexity:** LOW  
**Dependencies:** S0-1 (canonical supply invariant must exist)  

**Definition of Done:**
- [ ] All bridge mint paths check deduplication
- [ ] Storage map properly indexed
- [ ] 5 tests passing
- [ ] ProofForge gate passes

---

### S0-3: bridge_replay_accepted

**Location:** `crates/x3-bridge/src/message_processor.rs`  
**Severity:** 💀 CATASTROPHIC  
**Risk:** Replay attack → drain all bridge collateral  

**Problem:**
```rust
// Current state: No nonce/signature verification
pub fn process_bridge_message(msg: BridgeMessage) {
    // ❌ No replay protection
    // ❌ No signature verification
    Self::execute_message(msg)?;
}
```

**Required Fix:**
```rust
#[pallet::storage]
pub type ProcessedNonces<T> = StorageMap<_, Blake2_128Concat, (ChainId, u64), ()>;

pub fn process_bridge_message(msg: BridgeMessage) -> DispatchResult {
    // 1. Verify signature
    let signer = Self::verify_signature(&msg)?;
    ensure!(Self::is_authorized_relayer(signer), Error::<T>::UnauthorizedRelayer);
    
    // 2. Check nonce
    let nonce_key = (msg.source_chain, msg.nonce);
    ensure!(!ProcessedNonces::<T>::contains_key(&nonce_key), 
            Error::<T>::MessageAlreadyProcessed);
    
    // 3. Mark as processed BEFORE execution
    ProcessedNonces::<T>::insert(&nonce_key, ());
    
    // 4. Execute message
    Self::execute_message(msg)?;
    
    Ok(())
}

fn verify_signature(msg: &BridgeMessage) -> Result<AccountId, Error<T>> {
    let payload = Self::message_payload(msg);
    let signer = sp_io::crypto::sr25519_verify(
        &msg.signature,
        &payload,
        &msg.relayer_pubkey
    ).then(|| msg.relayer_pubkey)
      .ok_or(Error::<T>::InvalidSignature)?;
    Ok(signer)
}
```

**Tests Required:**
1. `test_valid_message_accepted()`
2. `test_replay_rejected()`
3. `test_invalid_signature_rejected()`
4. `test_unauthorized_relayer_rejected()`
5. `test_wrong_nonce_rejected()`
6. `test_out_of_order_nonces_handled()` (if sequential required)
7. Fuzz test: `fuzz_replay_attacks_fail()`

**Timeline:** 2 weeks  
**Complexity:** HIGH  
**Dependencies:** S0-1, S0-2 (bridge minting must be secure)  

**Definition of Done:**
- [ ] Cryptographic signature verification
- [ ] Nonce tracking per source chain
- [ ] 7 tests passing including replay attempts
- [ ] Performance acceptable (<50ms signature verification)
- [ ] ProofForge gate passes

---

## PRIORITY 2: Consensus Safety (WEEKS 4-6)

### S0-4: finality_spoof_accepted

**Location:** `crates/x3-bridge/src/finality_verifier.rs`  
**Severity:** 💀 CATASTROPHIC  
**Risk:** Accept unfinalized blocks → double-spend  

**Problem:**
```rust
// Current state: No finality verification
pub fn verify_source_chain_proof(proof: Proof) -> bool {
    // ❌ Assumes all blocks are finalized
    // ❌ No validator signature verification
    true
}
```

**Required Fix:**
```rust
pub fn verify_source_chain_proof(proof: Proof) -> DispatchResult {
    // 1. Verify validator signatures
    let signer_count = Self::count_valid_signatures(&proof)?;
    let required = Self::required_validator_threshold(proof.source_chain);
    ensure!(signer_count >= required, Error::<T>::InsufficientSignatures);
    
    // 2. Verify finality gadget proof (GRANDPA or equivalent)
    let finality_proof = Self::decode_finality_proof(&proof.justification)?;
    ensure!(finality_proof.round >= proof.block_number, Error::<T>::BlockNotFinalized);
    
    // 3. Verify state root
    let computed_root = Self::compute_state_root(&proof.state_proof)?;
    ensure!(computed_root == proof.claimed_state_root, Error::<T>::InvalidStateRoot);
    
    Ok(())
}

fn count_valid_signatures(proof: &Proof) -> Result<u32, Error<T>> {
    let mut count = 0;
    for sig in &proof.validator_signatures {
        if Self::verify_validator_signature(sig, &proof.block_hash)? {
            count += 1;
        }
    }
    Ok(count)
}
```

**Tests Required:**
1. `test_valid_finality_proof_accepted()`
2. `test_insufficient_signatures_rejected()`
3. `test_invalid_signature_rejected()`
4. `test_wrong_block_hash_rejected()`
5. `test_unfinalized_block_rejected()`
6. `test_state_root_mismatch_rejected()`
7. Integration test: `test_cannot_relay_from_unfinalized_block()`

**Timeline:** 2 weeks  
**Complexity:** HIGH  
**Dependencies:** Finality gadget integration (GRANDPA/Fast Finality)  

**Definition of Done:**
- [ ] GRANDPA justification verification
- [ ] Validator signature verification
- [ ] 7 tests passing
- [ ] Integration test with multi-node testnet
- [ ] ProofForge gate passes

---

### S0-5: atomic_rollback_missing

**Location:** `crates/x3-atomic-trade/src/lib.rs`  
**Severity:** 💀 CATASTROPHIC  
**Risk:** Partial settlement → one side loses funds  

**Problem:**
```rust
// Current state: No rollback mechanism
pub fn execute_atomic_swap(swap: AtomicSwap) {
    Self::debit_account_a(swap.party_a, swap.amount_a)?;  // ❌ No rollback
    Self::credit_account_b(swap.party_b, swap.amount_a)?;  // ❌ If this fails, A loses funds
    
    Self::debit_account_b(swap.party_b, swap.amount_b)?;  // ❌ No rollback
    Self::credit_account_a(swap.party_a, swap.amount_b)?;  // ❌ If this fails, B loses funds
}
```

**Required Fix:**
```rust
pub fn execute_atomic_swap(swap: AtomicSwap) -> DispatchResult {
    // Use transactional storage
    with_transaction(|| {
        // Step 1: Debit A
        Self::debit_account_a(swap.party_a, swap.amount_a)?;
        
        // Step 2: Credit B
        Self::credit_account_b(swap.party_b, swap.amount_a)?;
        
        // Step 3: Debit B
        Self::debit_account_b(swap.party_b, swap.amount_b)?;
        
        // Step 4: Credit A
        Self::credit_account_a(swap.party_a, swap.amount_b)?;
        
        // If ANY step fails, ALL rollback automatically
        TransactionOutcome::Commit(Ok(()))
    })
}

// Alternative: Manual rollback with snapshots
pub fn execute_atomic_swap_manual(swap: AtomicSwap) -> DispatchResult {
    let snapshot_a = Self::get_balance(swap.party_a);
    let snapshot_b = Self::get_balance(swap.party_b);
    
    let result = (|| {
        Self::debit_account_a(swap.party_a, swap.amount_a)?;
        Self::credit_account_b(swap.party_b, swap.amount_a)?;
        Self::debit_account_b(swap.party_b, swap.amount_b)?;
        Self::credit_account_a(swap.party_a, swap.amount_b)?;
        Ok(())
    })();
    
    // If failed, restore snapshots
    if result.is_err() {
        Self::restore_balance(swap.party_a, snapshot_a);
        Self::restore_balance(swap.party_b, snapshot_b);
    }
    
    result
}
```

**Tests Required:**
1. `test_successful_swap_commits()`
2. `test_failure_at_step_2_rolls_back_step_1()`
3. `test_failure_at_step_3_rolls_back_steps_1_2()`
4. `test_failure_at_step_4_rolls_back_all()`
5. `test_no_partial_state_after_failure()`
6. `test_rollback_preserves_canonical_supply()`
7. Fuzz test: `fuzz_no_partial_settlements()`

**Timeline:** 2 weeks  
**Complexity:** HIGH  
**Dependencies:** S0-1 (canonical supply must be verified)  

**Definition of Done:**
- [ ] Transactional storage or manual rollback
- [ ] All-or-nothing guarantee proven
- [ ] 7 tests passing
- [ ] Canonical supply preserved in all failure paths
- [ ] ProofForge gate passes

---

## PRIORITY 3: Operational Safety (WEEKS 7-8)

### S0-6: runtime_panic_critical_path

**Location:** `runtime/src/lib.rs`, various pallets  
**Severity:** 💀 CATASTROPHIC  
**Risk:** Runtime panic → chain halt  

**Problem:**
```rust
// Current state: panic!/unwrap in critical paths
pub fn on_finalize(n: BlockNumber) {
    let validator = Self::current_validator().unwrap();  // ❌ Can panic
    let stake = Self::validator_stake(validator).expect("stake exists");  // ❌ Can panic
    
    Self::distribute_rewards(validator, stake);
}
```

**Required Fix:**
```rust
pub fn on_finalize(n: BlockNumber) {
    // Use defensive programming
    let validator = match Self::current_validator() {
        Some(v) => v,
        None => {
            log::error!("No current validator at block {}", n);
            return;  // Graceful degradation
        }
    };
    
    let stake = match Self::validator_stake(validator) {
        Some(s) => s,
        None => {
            log::error!("No stake for validator {:?}", validator);
            return;  // Graceful degradation
        }
    };
    
    if let Err(e) = Self::distribute_rewards(validator, stake) {
        log::error!("Failed to distribute rewards: {:?}", e);
        // Continue anyway - don't halt chain
    }
}

// Use Substrate defensive traits
use frame_support::defensive;

pub fn critical_operation() -> DispatchResult {
    let value = storage.get().defensive_ok_or(Error::<T>::StorageCorrupted)?;
    // If defensive_ok_or triggers, it logs but doesn't panic
    Ok(())
}
```

**Remediation Steps:**
1. **Audit:** Search for `unwrap()`, `expect()`, `panic!()` in critical paths
   ```bash
   rg "unwrap\(\)|expect\(|panic!" runtime/ pallets/ --type rust
   ```

2. **Replace:**
   - `unwrap()` → `defensive_unwrap_or_default()` or `match`
   - `expect()` → `defensive_ok_or()` or `?`
   - `panic!()` → `defensive_assert!()` or `ensure!()`

3. **Critical paths to audit:**
   - `on_initialize()`, `on_finalize()`
   - Consensus participation
   - Block validation
   - Transaction execution

**Tests Required:**
1. `test_no_panics_in_block_execution()`
2. `test_corrupted_storage_does_not_halt_chain()`
3. `test_invalid_validator_handled_gracefully()`
4. `test_all_hooks_return_results()`
5. Fuzz test: `fuzz_runtime_never_panics()`

**Timeline:** 1 week  
**Complexity:** MEDIUM (tedious but straightforward)  
**Dependencies:** None  

**Definition of Done:**
- [ ] Zero `unwrap()`/`expect()`/`panic!()` in critical paths
- [ ] All hooks use defensive programming
- [ ] 5 tests passing
- [ ] Fuzz test runs 1M iterations without panic
- [ ] ProofForge gate passes

---

## PRIORITY 4: Governance & Auth (WEEKS 9-10)

### S1-1: failed_rollback

**Location:** `pallets/x3-atomic-kernel/src/transaction.rs`  
**Severity:** ⚠️ CRITICAL  
**Risk:** State inconsistency after failure  

**Required Fix:** (Same pattern as S0-5 atomic_rollback)  
**Timeline:** 1 week  
**Complexity:** MEDIUM  

---

### S1-2: governance_bypass

**Location:** `pallets/governance/src/lib.rs`  
**Severity:** ⚠️ CRITICAL  
**Risk:** Unauthorized runtime upgrades  

**Required Fix:**
```rust
pub fn execute_proposal(proposal_id: ProposalId) -> DispatchResult {
    let proposal = Proposals::<T>::get(proposal_id)
        .ok_or(Error::<T>::ProposalNotFound)?;
    
    // Verify voting passed
    ensure!(proposal.status == ProposalStatus::Passed, Error::<T>::ProposalNotPassed);
    
    // Verify timelock elapsed
    let now = <frame_system::Pallet<T>>::block_number();
    ensure!(now >= proposal.execution_block, Error::<T>::TimelockNotElapsed);
    
    // Verify not expired
    ensure!(now <= proposal.expiry_block, Error::<T>::ProposalExpired);
    
    // Execute with sudo-like privileges
    let call = proposal.call;
    call.dispatch(frame_system::RawOrigin::Root.into())?;
    
    Ok(())
}
```

**Timeline:** 1 week  
**Complexity:** MEDIUM  

---

### S1-3: unauthorized_mint

**Location:** `pallets/x3-kernel/src/minting.rs`  
**Severity:** ⚠️ CRITICAL  
**Risk:** Inflation attacks  

**Required Fix:**
```rust
#[pallet::call]
impl<T: Config> Pallet<T> {
    #[pallet::weight(10_000)]
    pub fn mint(
        origin: OriginFor<T>,
        amount: Balance
    ) -> DispatchResult {
        // Only bridge or governance can mint
        let who = ensure_signed_or_root(origin)?;
        ensure!(
            Self::is_authorized_minter(who),
            Error::<T>::UnauthorizedMint
        );
        
        Self::mint_internal(amount)?;
        Ok(())
    }
}

fn is_authorized_minter(account: AccountId) -> bool {
    // Check if bridge pallet
    if account == T::BridgePalletId::get() {
        return true;
    }
    
    // Check if governance
    if account == T::GovernancePalletId::get() {
        return true;
    }
    
    false
}
```

**Timeline:** 1 week  
**Complexity:** LOW  

---

## Implementation Schedule

### Parallel Tracks

**Track A (Economic Core):** 3 engineers
- Week 1-2: S0-1 + S0-2 (canonical supply + double mint)
- Week 2-3: S0-3 (bridge replay)

**Track B (Consensus Safety):** 2 engineers
- Week 4-5: S0-4 (finality spoof)
- Week 5-6: S0-5 (atomic rollback)

**Track C (Operational Safety):** 1 engineer
- Week 7-8: S0-6 (runtime panics)

**Track D (Governance):** 1 engineer
- Week 9: S1-1 (failed rollback)
- Week 9: S1-2 (governance bypass)
- Week 10: S1-3 (unauthorized mint)

**Week 11-12: Integration & Verification**
- Re-run ProofForge
- Integration tests
- Multi-node testnet validation

---

## Success Metrics

### Week-by-Week Goals

| Week | Target | Success Criteria |
|------|--------|------------------|
| 1-3 | Priority 1 | S0-1, S0-2, S0-3 fixed + tested |
| 4-6 | Priority 2 | S0-4, S0-5 fixed + tested |
| 7-8 | Priority 3 | S0-6 fixed + tested |
| 9-10 | Priority 4 | S1-1, S1-2, S1-3 fixed + tested |
| 11 | Integration | All tests pass, no regressions |
| 12 | Verification | ProofForge re-run shows 0 S0/S1 blockers |

### ProofForge Gates

After remediation, ALL gates must pass:

- ✅ **TodoGate:** 0 T9 todos, <10 T8 todos
- ✅ **MainnetGate:** 0 T5+ todos blocking mainnet
- ✅ **GapGate:** 0 G10 gaps (S0 severity)
- ✅ **SecurityGate:** 0 S0/S1 blockers

---

## Risk Management

### If Timeline Slips

**Week 6 checkpoint:**
- If Priority 1 not done → add 2 engineers
- If Priority 2 not started → re-scope S0-4/S0-5 for later

**Week 9 checkpoint:**
- If any S0 not done → delay mainnet
- S1 blockers can be addressed in parallel with external audit

### Scope Management

**Must Fix (Cannot compromise):**
- All 6 S0 blockers

**Should Fix (Can defer with risk):**
- All 3 S1 blockers (but external auditors will flag)

**Could Fix (Can defer):**
- S2 blockers (best practices)
- Non-mainnet todos

---

## Post-Remediation

After all 9 blockers fixed:

1. **Re-run ProofForge** (Week 12)
   ```bash
   ./target/debug/x3-proof prove-everything --verbose
   ```

2. **Generate Updated Report**
   ```bash
   ./launch-gates/comprehensive-mainnet-readiness.sh
   ```

3. **Engage External Auditors** (Week 13+)
   - Contact Trail of Bits, OpenZeppelin, Zellic
   - Budget: $250k-$400k
   - Timeline: 6-8 weeks

4. **Launch Bug Bounty** (Week 13+)
   - Platform: Immunefi
   - Pool: $100k
   - Run 4+ weeks before mainnet

5. **Public Testnet** (Week 13+)
   - 50+ external validators
   - 8-12 weeks duration
   - $100k-$150k incentives

6. **Mainnet Launch** (Week 26+)
   - All external validation complete
   - Zero S0/S1 findings
   - Testnet proven stable

---

## Contact

**Security Strike Team Lead:** TBD  
**ProofForge Engineer:** TBD  
**External Audit Coordinator:** TBD  

**Slack Channel:** #mainnet-security-blockers  
**Daily Standup:** 9am PST  
**Weekly Review:** Fridays 2pm PST
