# ⚡ SPRINT 0 IMMEDIATE EXECUTION PLAN

**Status:** 🔴 ACTIVE (Starting Now: April 26, 2026)  
**Duration:** 1 week (Apr 26 - May 2, 2026)  
**Goal:** Foundation audit + readiness infrastructure + sprint operations setup  
**Team:** 1 engineer (lead: @lojak)

---

## 🎯 THIS WEEK'S MISSION

Get kernel audit complete, establish sprint infrastructure, prepare for Sprint 1 (Packet Standard).

**Deliverables:**
- [ ] Kernel invariant audit complete (canonical supply verified)
- [ ] Emergency halt path tested + documented
- [ ] Mint/burn permissions validated
- [ ] Cross-domain balance reconciliation proven
- [ ] Readiness report crate scaffolded
- [ ] All Sprint 0 tasks in GitHub Projects
- [ ] Sprint 0 tests passing (100%)

---

## 📅 DAILY BREAKDOWN

### Friday, April 26 (TODAY — 4 Hours)
**Goal:** Setup + Initial Audit

```bash
# 1. Create feature branch
git checkout develop
git pull origin develop
git checkout -b sprint-0/foundation/kernel-audit

# 2. Review kernel code
# File: pallets/x3-kernel/src/lib.rs (lines 1-100)
# Focus: Canonical supply invariant (SupplyLedger::check_invariant())

# 3. Start Phase 0.1 task
# Create: tasks/sprint-0/phase-0.1-canonical-supply.md
```

**Task File: `tasks/sprint-0/phase-0.1-canonical-supply.md`**
```markdown
# Phase 0.1: Canonical Supply Invariant Audit

## Acceptance Criteria
- [ ] SupplyLedger::check_invariant() reviewed for correctness
- [ ] 1,000+ random mint/burn operations tested
- [ ] No invariant violations found
- [ ] Fuzz test harness created

## Test Cases
1. Sequential mints: 100 → 100 → 100 (total 300)
2. Sequential burns: 100 → burn 50 → burn 30 (total -80)
3. Interleaved: mint 10, burn 5, mint 20, burn 5 (total +20)
4. Edge case: Burn 0 (no-op)
5. Edge case: Mint on zero account (first deposit)
6. Parallel deposits (10 different accounts, 100 each)
7. Stress test: 1,000 random ops in 60 seconds

## Implementation Plan
- [ ] Modify: `pallets/x3-kernel/src/tests.rs`
- [ ] Add: `#[test] fn test_canonical_supply_invariant() { ... }`
- [ ] Add: `#[test] fn fuzz_supply_ledger_1000ops() { ... }`
- [ ] Verify: `cargo test -p x3-kernel` passes

## Success
All tests passing. No panics. Invariant maintained.
```

**Friday EOD:**
- [ ] Branch created
- [ ] Code reviewed
- [ ] Phase 0.1 task file created
- [ ] Commit: `feat(sprint-0): create phase 0.1 kernel audit task`

---

### Monday, April 29 (Phase 0.1 Execution — 6 Hours)
**Goal:** Canonical supply invariant proven

```bash
# Session start
git fetch origin
git checkout sprint-0/foundation/kernel-audit

# Phase 0.1 Work:
# 1. Add fuzz test harness
# 2. Run 1,000 random ops
# 3. Verify no invariant violations
# 4. Document results

# After work:
cargo test -p x3-kernel
git add pallets/x3-kernel/src/tests.rs
git commit -m "test(kernel): add canonical supply invariant audit (1000 ops)"
git push origin sprint-0/foundation/kernel-audit
```

**Specific Code to Add:**

In `pallets/x3-kernel/src/tests.rs`, add:

```rust
#[test]
fn test_canonical_supply_invariant_sequential() {
    // Setup
    let ledger = SupplyLedger::new();
    
    // Sequential mints
    for i in 0..100 {
        ledger.mint(i, 100);
        ledger.check_invariant().expect("Invariant violated after mint");
    }
    
    // Sequential burns
    for i in 0..50 {
        ledger.burn(i, 50);
        ledger.check_invariant().expect("Invariant violated after burn");
    }
}

#[test]
fn fuzz_canonical_supply_1000_random_ops() {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let ledger = SupplyLedger::new();
    
    for op_num in 0..1000 {
        let account = rng.gen_range(0..100);
        let amount = rng.gen_range(1..1000);
        let is_mint = rng.gen_bool(0.5);
        
        if is_mint {
            ledger.mint(account, amount);
        } else {
            let _ = ledger.try_burn(account, amount);
        }
        
        if op_num % 100 == 0 {
            ledger.check_invariant()
                .expect(&format!("Invariant violated at op {}", op_num));
        }
    }
}
```

**Monday EOD:**
- [ ] Phase 0.1 tests passing
- [ ] Results documented
- [ ] PR ready for review

---

### Tuesday, April 30 (Phase 0.2 — 5 Hours)
**Goal:** Emergency halt path verified

**Task File: `tasks/sprint-0/phase-0.2-emergency-halt.md`**
```markdown
# Phase 0.2: Emergency Halt Path Verification

## Acceptance Criteria
- [ ] Emergency halt callable by governance
- [ ] All transfers blocked when halted
- [ ] All mints blocked when halted
- [ ] All burns blocked when halted
- [ ] Recovery from halt possible
- [ ] Halt state persisted correctly

## Test Cases
1. Call emergency_halt() → verify halted = true
2. Transfer attempt while halted → rejected
3. Mint attempt while halted → rejected
4. Burn attempt while halted → rejected
5. Call recovery_from_halt() → verify halted = false
6. Transfer now works → accepted

## Code Location
pallets/x3-kernel/src/emergency.rs (may need to create)

## Implementation
- [ ] Add emergency_halt() extrinsic
- [ ] Add is_halted() check to all transfer paths
- [ ] Add recovery_from_halt() extrinsic
- [ ] Add tests (6 test cases above)
```

**Tuesday Work:**
```bash
# Continue on same branch
# Implement Phase 0.2
# Add emergency halt tests
# Run: cargo test -p x3-kernel
# Commit: feat(kernel): add emergency halt verification tests
```

**Tuesday EOD:**
- [ ] Phase 0.2 tests passing
- [ ] Emergency halt documented
- [ ] Ready for integration review

---

### Wednesday, May 1 (Phases 0.3 & 0.4 Parallel — 8 Hours)
**Goal:** Mint/burn permissions + balance reconciliation

**Phase 0.3 Task:**
```markdown
# Phase 0.3: Mint/Burn Permission Guards

## Acceptance Criteria
- [ ] Only approved roles can mint
- [ ] Only approved roles can burn
- [ ] Unauthorized mint rejected
- [ ] Unauthorized burn rejected
- [ ] Permission changes logged

## Test Cases
1. Admin mints → accepted
2. User tries mint → rejected
3. Admin burns → accepted
4. User tries burn → rejected
5. Grant mint permission → User can now mint
```

**Phase 0.4 Task:**
```markdown
# Phase 0.4: Cross-Domain Balance Reconciliation

## Acceptance Criteria
- [ ] Balances match across EVM/SVM/X3VM domains
- [ ] No balance leaks detected
- [ ] Supply invariant maintained globally
- [ ] Reconciliation can be triggered on-demand

## Test Cases
1. Mint 100 on EVM domain
2. Transfer 50 to SVM via bridge
3. Transfer 30 to X3VM via bridge
4. Verify: EVM=50, SVM=50, X3VM=30, Total=130 ✓
5. Run reconciliation → no leaks found
```

**Wednesday Work:**
```bash
# Phase 0.3: Implement mint/burn guards
# Phase 0.4: Implement cross-domain balance check
cargo test -p x3-kernel --all
git commit -m "test(kernel): add permission guards + cross-domain reconciliation"
```

**Wednesday EOD:**
- [ ] Phases 0.3 & 0.4 complete
- [ ] All tests passing
- [ ] 4/5 phases done

---

### Thursday, May 2 (Phase 0.5 + Readiness Crate — 7 Hours)
**Goal:** Readiness infrastructure scaffolded + sprint metrics

**Phase 0.5 Task:**
```markdown
# Phase 0.5: Readiness Report Crate Scaffolding

## Deliverable
New crate: `crates/x3-readiness-report/`

## Structure
```
crates/x3-readiness-report/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── collector.rs        # Collect metrics
│   ├── formatter.rs        # Format results
│   └── tests.rs
└── README.md
```

## Collector Features
- Kernel audit status
- Emergency halt path state
- Permission guards state
- Balance reconciliation state
- Test pass/fail counts
- Coverage percentages

## Output Format
Text report + JSON export
```

**Thursday Work:**
```bash
# Create readiness crate
cargo new --lib crates/x3-readiness-report

# Add to Cargo.toml workspace members

# Implement collector
# Implement formatter
# Add tests

git add .
git commit -m "feat(sprint-0): add readiness report infrastructure"
git push origin sprint-0/foundation/kernel-audit
```

**Thursday EOD:**
- [ ] Readiness crate created
- [ ] All 5 phases complete
- [ ] Ready for PR review

---

### Friday, May 3 (Review + Merge — 4 Hours)
**Goal:** Sprint 0 complete, ready for Sprint 1

```bash
# Final verification
cargo test -p x3-kernel
cargo test -p x3-readiness-report
cargo fmt --all
cargo clippy --all -- -D warnings

# Create Pull Request
# Title: "feat: Sprint 0 kernel audit + readiness infrastructure"
# Description: Reference tasks/sprint-0/
# Link to all phase task files

# After 2 approvals + tests passing:
git checkout develop
git pull origin develop
git merge --no-ff sprint-0/foundation/kernel-audit
git push origin develop

# Create release candidate
git checkout -b release/v0.4.0-s0.1
# Update Cargo.toml version: 0.4.0
git commit -am "chore: release v0.4.0-s0.1"
git push origin release/v0.4.0-s0.1

# Open PR to main
# Get 2 approvals, merge to main
# Tag: git tag v0.4.0-s0.1 && git push origin v0.4.0-s0.1
```

**Friday EOD:**
- [ ] All tests passing
- [ ] Merged to `develop`
- [ ] Tagged as v0.4.0-s0.1
- [ ] Sprint 0 COMPLETE ✅

---

## 📊 DEFINITION OF DONE: SPRINT 0

- [ ] Kernel audit 100% complete (all 5 phases)
- [ ] All tests passing (cargo test -p x3-kernel)
- [ ] Code coverage >90% (cargo tarpaulin)
- [ ] Readiness crate scaffolded + working
- [ ] Code reviewed (2 approvals)
- [ ] Merged to `develop`
- [ ] Tagged release (v0.4.0-s0.1)
- [ ] Documentation updated
- [ ] Metrics recorded
- [ ] Sprint review completed

---

## 🎯 SUCCESS METRICS

| Metric | Target | Status |
|--------|--------|--------|
| Phase 0.1 tests passing | 100% | ⏳ |
| Phase 0.2 tests passing | 100% | ⏳ |
| Phase 0.3 tests passing | 100% | ⏳ |
| Phase 0.4 tests passing | 100% | ⏳ |
| Phase 0.5 working | Yes | ⏳ |
| Code coverage | >90% | ⏳ |
| Build time | <5 min | ⏳ |
| Review time | <24h | ⏳ |

---

## 🚨 BLOCKERS & RESOLUTIONS

### If Kernel Code Changes Needed
- [ ] Check git blame for original author
- [ ] Review comment/PR history
- [ ] Ask in #x3-dev if unclear
- [ ] Document any changes made

### If Test Compilation Fails
```bash
# Clean build
cargo clean
cargo build -p x3-kernel

# If still fails:
# Check Rust version: rustup show
# Should be 1.89.0
```

### If Invariant Violations Found
- [ ] Stop and report immediately
- [ ] Post in #blockers
- [ ] Don't continue until fixed
- [ ] Security implications?

---

## 🎓 RUNNING SPRINT 0

### Pre-Week Checklist
- [ ] Feature branch created
- [ ] Task files written
- [ ] Phase breakdown understood
- [ ] Local build works

### Daily (Each Morning)
```bash
git fetch origin
git checkout sprint-0/foundation/kernel-audit
git pull origin sprint-0/foundation/kernel-audit
# (work on phase tasks)
# (commit frequently)
# (push before EOD)
```

### Daily (EOD)
```bash
git push origin sprint-0/foundation/kernel-audit
# Update task status in GitHub Projects
# Post Slack status: "On track" or "Blocked"
```

---

## ✅ IMMEDIATE ACTIONS (NEXT 30 MIN)

- [ ] Read this plan completely
- [ ] Create feature branch: `git checkout -b sprint-0/foundation/kernel-audit`
- [ ] Create `tasks/sprint-0/` directory
- [ ] Create phase task files (copy templates from this doc)
- [ ] Review kernel code for 30 min
- [ ] Make first commit: `feat(sprint-0): initialize kernel audit`
- [ ] Push: `git push -u origin sprint-0/foundation/kernel-audit`

🔥 **GO TIME** 🔥

