# Phase 0.4: Cross-Domain Balance Reconciliation

**Duration:** 4 hours (Wednesday May 1 - Thursday May 2)  
**Status:** ⏳ PENDING  
**Owner:** @lojak  

## Objective

Verify balances remain consistent across all storage domains:

> 1. FreeBalance + LockedBalance = Total per account
> 2. All account totals sum to TotalSupply
> 3. No balance drift across block finalization
> 4. Emergency reconciliation works

## Scope

- [ ] Review storage domains in kernel
- [ ] Add reconciliation test (cross-domain consistency)
- [ ] Add drift detection test
- [ ] Add finalization test

## Deliverables

1. **Test file:** Update `pallets/x3-kernel/src/tests.rs`
2. **Coverage:** Multi-domain consistency verified
3. **Finalization:** Tested across block boundaries

## Tasks

### Task 0.4.1: Add Cross-Domain Consistency Tests (2h)

Add to `pallets/x3-kernel/src/tests.rs`:

```rust
#[test]
fn test_cross_domain_balance_consistency() {
    new_test_ext().execute_with(|| {
        let alice = account_id::<Test>("alice");
        let bob = account_id::<Test>("bob");
        
        // Setup: Create complex state
        assert_ok!(XKernel::mint(RawOrigin::Root.into(), alice.clone(), 1_000_000));
        assert_ok!(XKernel::mint(RawOrigin::Root.into(), bob.clone(), 500_000));
        
        // Alice transfers to bob
        assert_ok!(XKernel::transfer(
            RawOrigin::Signed(alice.clone()).into(),
            bob.clone(),
            100_000,
        ));
        
        // Alice locks some balance
        assert_ok!(XKernel::lock_balance(
            RawOrigin::Signed(alice.clone()).into(),
            200_000,
        ));
        
        // Verify per-account consistency: free + locked = claimed
        let alice_free = <FreeBalance<Test>>::get(&alice);
        let alice_locked = <LockedBalance<Test>>::get(&alice);
        let alice_total = alice_free.saturating_add(alice_locked);
        
        // Original was 1M, transferred 100k, so claimed should be 900k
        assert_eq!(alice_total, 900_000, "Alice: free({}) + locked({}) != 900k", alice_free, alice_locked);
        
        let bob_free = <FreeBalance<Test>>::get(&bob);
        let bob_locked = <LockedBalance<Test>>::get(&bob);
        let bob_total = bob_free.saturating_add(bob_locked);
        
        // Bob started with 500k, got 100k, so total should be 600k
        assert_eq!(bob_total, 600_000, "Bob: free({}) + locked({}) != 600k", bob_free, bob_locked);
        
        println!("✅ Cross-domain consistency verified");
    });
}

#[test]
fn test_global_supply_reconciliation() {
    new_test_ext().execute_with(|| {
        let accounts = vec![
            account_id::<Test>("alice"),
            account_id::<Test>("bob"),
            account_id::<Test>("charlie"),
        ];
        
        // Create distributed state
        for (idx, account) in accounts.iter().enumerate() {
            let amount = (idx as u128 + 1) * 1_000_000;
            assert_ok!(XKernel::mint(RawOrigin::Root.into(), account.clone(), amount));
        }
        
        // Complex operations
        assert_ok!(XKernel::transfer(
            RawOrigin::Signed(accounts[0].clone()).into(),
            accounts[1].clone(),
            250_000,
        ));
        assert_ok!(XKernel::lock_balance(
            RawOrigin::Signed(accounts[1].clone()).into(),
            300_000,
        ));
        assert_ok!(XKernel::lock_balance(
            RawOrigin::Signed(accounts[2].clone()).into(),
            500_000,
        ));
        
        // Calculate total from storage
        let mut sum_free = 0u128;
        let mut sum_locked = 0u128;
        
        for account in &accounts {
            sum_free = sum_free.saturating_add(<FreeBalance<Test>>::get(account));
            sum_locked = sum_locked.saturating_add(<LockedBalance<Test>>::get(account));
        }
        
        let calculated_supply = sum_free.saturating_add(sum_locked);
        let stored_supply = <TotalSupply<Test>>::get();
        
        assert_eq!(
            calculated_supply, stored_supply,
            "Global reconciliation failed: calculated({}) != stored({})",
            calculated_supply, stored_supply
        );
        
        println!("✅ Global supply reconciliation: {} = {}", calculated_supply, stored_supply);
    });
}
```

**Success:** Cross-domain consistency verified

---

### Task 0.4.2: Add Drift Detection Tests (1.5h)

Add to same file:

```rust
#[test]
fn test_no_balance_drift_on_operations() {
    new_test_ext().execute_with(|| {
        let alice = account_id::<Test>("alice");
        let bob = account_id::<Test>("bob");
        
        // Initial supply
        assert_ok!(XKernel::mint(RawOrigin::Root.into(), alice.clone(), 1_000_000));
        let initial_supply = <TotalSupply<Test>>::get();
        
        // Series of operations
        for i in 0..10 {
            let amount = (i as u128 + 1) * 10_000;
            
            let _ = XKernel::transfer(
                RawOrigin::Signed(alice.clone()).into(),
                bob.clone(),
                amount,
            );
            
            let current_supply = <TotalSupply<Test>>::get();
            assert_eq!(
                current_supply, initial_supply,
                "Balance drift detected at iteration {}: {} != {}",
                i, current_supply, initial_supply
            );
        }
        
        println!("✅ No drift over {} operations", 10);
    });
}

#[test]
fn test_balance_after_finalization() {
    new_test_ext().execute_with(|| {
        let alice = account_id::<Test>("alice");
        
        // Mint
        assert_ok!(XKernel::mint(RawOrigin::Root.into(), alice.clone(), 1_000_000));
        let pre_finalize = <FreeBalance<Test>>::get(&alice);
        
        // Simulate finalization (run_to_block)
        // Note: Adjust if your test harness uses different block finalization
        System::finalize();
        System::initialize(
            &2,
            &Default::default(),
            &Default::default(),
            frame_system::InitKind::Full,
        );
        
        let post_finalize = <FreeBalance<Test>>::get(&alice);
        assert_eq!(pre_finalize, post_finalize);
        
        println!("✅ Balance persisted through finalization");
    });
}
```

**Success:** No drift detected, finalization OK

---

### Task 0.4.3: Emergency Reconciliation (0.5h)

If kernel has emergency reconciliation, add:

```rust
#[test]
fn test_emergency_reconciliation() {
    new_test_ext().execute_with(|| {
        // This test is conditional - only if pallet has reconcile() function
        // Placeholder for reconciliation verification
        
        let alice = account_id::<Test>("alice");
        assert_ok!(XKernel::mint(RawOrigin::Root.into(), alice.clone(), 1_000_000));
        
        // If reconcile exists:
        // assert_ok!(XKernel::reconcile(RawOrigin::Root.into()));
        // assert_eq!(<TotalSupply<Test>>::get(), 1_000_000);
        
        println!("✅ Reconciliation verified (or N/A)");
    });
}
```

---

## Testing Evidence

```bash
cd /home/lojak/Desktop/X3_ATOMIC_STAR

# Run phase 0.4 tests
cargo test -p x3-kernel test_cross_domain -- --nocapture
cargo test -p x3-kernel test_global_supply -- --nocapture
cargo test -p x3-kernel test_no_balance_drift -- --nocapture

# Expected: All 5 tests pass
```

## Sign-Off

- [ ] Cross-domain consistency verified
- [ ] Global reconciliation working
- [ ] No drift over 10+ operations
- [ ] Balances persist through finalization
- [ ] Emergency reconciliation (if exists)
- [ ] Ready for Phase 0.5

## Notes

- Balance reconciliation is **critical for data integrity**
- Storage must be consistent across all domains
- Drift detection catches silent balance leaks
- Finalization must preserve state
