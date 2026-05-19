# Phase 0.1: Canonical Supply Invariant Audit

**Duration:** 6 hours (Monday Apr 29 - Tuesday Apr 30)  
**Status:** 🟡 IN PROGRESS  
**Owner:** @lojak  

## Objective

Verify that the x3-kernel pallet maintains the **canonical supply invariant** under all conditions:

> Total Supply = Sum of all balances + locked amounts

## Scope

- [x] Review kernel code for invariant enforcement
- [ ] Add sequential mutation test (100 operations)
- [ ] Add fuzz test harness (1,000 random operations)
- [ ] Verify test pass with `cargo test`

## Deliverables

1. **Test file:** `pallets/x3-kernel/src/tests.rs` with new test functions
2. **Coverage:** Supply invariant must hold after every mutation
3. **Fuzz depth:** Minimum 1,000 random operation sequences

## Tasks

### Task 0.1.1: Review Kernel Structure (1h)

Read and understand:
- `pallets/x3-kernel/src/lib.rs` - Pallet declaration
- `pallets/x3-kernel/src/types.rs` - Supply types
- `pallets/x3-kernel/src/storage.rs` - Storage invariants
- Look for: Where are balances stored? Where is total supply tracked?

**Success:** Can describe the 3 balance storage locations and total supply location

---

### Task 0.1.2: Add Sequential Mutation Test (2h)

Add to `pallets/x3-kernel/src/tests.rs`:

```rust
#[test]
fn test_canonical_supply_invariant_sequential() {
    new_test_ext().execute_with(|| {
        let accounts = vec![
            account_id::<Test>("alice"),
            account_id::<Test>("bob"),
            account_id::<Test>("charlie"),
        ];
        
        // Initial mint
        let initial_supply = 1_000_000_000;
        for account in &accounts {
            assert_ok!(XKernel::mint(
                RawOrigin::Root.into(),
                account.clone(),
                initial_supply / 3,
            ));
        }
        
        // Verify initial invariant
        assert_supply_invariant("initial state");
        
        // Sequential operations: transfer, lock, unlock, burn
        for op_idx in 0..100 {
            let src_idx = op_idx % 3;
            let dst_idx = (op_idx + 1) % 3;
            let amount = (op_idx as u128 + 1) * 1_000;
            
            // Transfer
            let _ = XKernel::transfer(
                RawOrigin::Signed(accounts[src_idx].clone()).into(),
                accounts[dst_idx].clone(),
                amount,
            );
            assert_supply_invariant(&format!("after transfer op {}", op_idx));
            
            // Lock
            let _ = XKernel::lock_balance(
                RawOrigin::Signed(accounts[dst_idx].clone()).into(),
                amount / 2,
            );
            assert_supply_invariant(&format!("after lock op {}", op_idx));
        }
        
        System::assert_last_event(Event::InvariantHeld.into());
    });
}

fn assert_supply_invariant(context: &str) {
    let total_supply = <TotalSupply<Test>>::get();
    let mut balance_sum = 0u128;
    
    // Sum all free balances
    for account in &[
        account_id::<Test>("alice"),
        account_id::<Test>("bob"),
        account_id::<Test>("charlie"),
    ] {
        balance_sum = balance_sum.saturating_add(
            <FreeBalance<Test>>::get(account)
        );
    }
    
    // Add locked amounts
    let locked_sum: u128 = <LockedBalance<Test>>::iter_values()
        .fold(0u128, |acc, val| acc.saturating_add(val));
    
    assert_eq!(
        total_supply,
        balance_sum.saturating_add(locked_sum),
        "Supply invariant FAILED at {}: {} != {} + {}",
        context,
        total_supply,
        balance_sum,
        locked_sum
    );
}
```

**Success:** Test runs to completion without panic

---

### Task 0.1.3: Add Fuzz Test Harness (2h)

Add to same file:

```rust
#[test]
fn fuzz_canonical_supply_1000_random_ops() {
    use rand::Rng;
    
    new_test_ext().execute_with(|| {
        let accounts = vec![
            account_id::<Test>("alice"),
            account_id::<Test>("bob"),
            account_id::<Test>("charlie"),
        ];
        
        // Initial state
        let initial_per_account = 1_000_000_000;
        for account in &accounts {
            let _ = XKernel::mint(
                RawOrigin::Root.into(),
                account.clone(),
                initial_per_account,
            );
        }
        assert_supply_invariant("fuzz_start");
        
        // 1000 random operations
        let mut rng = rand::thread_rng();
        for fuzz_iteration in 0..1000 {
            let op = rng.gen_range(0..4); // 0=transfer, 1=lock, 2=unlock, 3=burn
            let src_idx = rng.gen_range(0..3);
            let dst_idx = rng.gen_range(0..3);
            let amount = rng.gen_range(1..=10_000_000);
            
            match op {
                0 => { // Transfer
                    let _ = XKernel::transfer(
                        RawOrigin::Signed(accounts[src_idx].clone()).into(),
                        accounts[dst_idx].clone(),
                        amount,
                    );
                },
                1 => { // Lock
                    let _ = XKernel::lock_balance(
                        RawOrigin::Signed(accounts[src_idx].clone()).into(),
                        amount,
                    );
                },
                2 => { // Unlock
                    let _ = XKernel::unlock_balance(
                        RawOrigin::Signed(accounts[src_idx].clone()).into(),
                        amount,
                    );
                },
                3 => { // Burn
                    let _ = XKernel::burn(
                        RawOrigin::Signed(accounts[src_idx].clone()).into(),
                        amount,
                    );
                },
                _ => unreachable!(),
            }
            
            // Verify invariant after EVERY operation
            assert_supply_invariant(&format!("fuzz_op_{}", fuzz_iteration));
        }
        
        println!("✅ 1000 random operations completed - invariant held!");
    });
}
```

**Success:** All 1,000 iterations complete without breaking invariant

---

### Task 0.1.4: Execute & Verify (1h)

```bash
cd /home/lojak/Desktop/X3_ATOMIC_STAR

# Run the new tests
cargo test -p x3-kernel test_canonical_supply_invariant_sequential -- --nocapture
cargo test -p x3-kernel fuzz_canonical_supply_1000_random_ops -- --nocapture

# Check test output
echo "✅ Phase 0.1 complete if both tests show: test result: ok"
```

**Success Criteria:**
- [ ] Both test functions execute without panics
- [ ] All 100 sequential operations maintain invariant
- [ ] All 1,000 fuzz operations maintain invariant
- [ ] No test failures reported

## Testing Evidence

After completing tasks, run:

```bash
cargo test -p x3-kernel --lib 2>&1 | grep -E "test.*canonical|test.*fuzz|test result:"
```

Expected output:
```
test tests::test_canonical_supply_invariant_sequential ... ok
test tests::fuzz_canonical_supply_1000_random_ops ... ok

test result: ok. XX passed; 0 failed; 0 ignored
```

## Sign-Off

- [ ] Code review approved (2 reviewers)
- [ ] All tests passing
- [ ] Coverage >90% for supply module
- [ ] Merged to feature branch
- [ ] Ready for Phase 0.2

## Notes

- Supply invariant is **critical path** for v0.4
- Any failure here blocks all downstream work
- Document any edge cases or assumptions found
- Update test names if they don't match actual function names in codebase
