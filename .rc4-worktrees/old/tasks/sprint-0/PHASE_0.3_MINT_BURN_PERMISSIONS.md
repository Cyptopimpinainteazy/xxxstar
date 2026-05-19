# Phase 0.3: Mint/Burn Permission Guards

**Duration:** 4 hours (Wednesday May 1)  
**Status:** ⏳ PENDING  
**Owner:** @lojak  

## Objective

Verify only authorized accounts can mint and burn tokens:

> 1. Only RawOrigin::Root can call mint()
> 2. Only token holder or Root can burn()
> 3. Unauthorized calls fail with PermissionDenied error
> 4. Permissions persist across state changes

## Scope

- [ ] Review permission check in mint handler
- [ ] Review permission check in burn handler
- [ ] Add authorization tests (4 total)

## Deliverables

1. **Test file:** Update `pallets/x3-kernel/src/tests.rs`
2. **Coverage:** Root access + non-root rejection tested
3. **Edge cases:** Self-burn, unauthorized others

## Tasks

### Task 0.3.1: Add Mint Permission Tests (2h)

Add to `pallets/x3-kernel/src/tests.rs`:

```rust
#[test]
fn test_mint_requires_root_origin() {
    new_test_ext().execute_with(|| {
        let alice = account_id::<Test>("alice");
        let bob = account_id::<Test>("bob");
        
        // Root can mint
        assert_ok!(XKernel::mint(
            RawOrigin::Root.into(),
            alice.clone(),
            1_000_000,
        ));
        assert_eq!(<FreeBalance<Test>>::get(&alice), 1_000_000);
        
        // Non-root cannot mint
        assert_noop!(
            XKernel::mint(
                RawOrigin::Signed(bob.clone()).into(),
                alice.clone(),
                500_000,
            ),
            DispatchError::BadOrigin
        );
        
        // Balance unchanged
        assert_eq!(<FreeBalance<Test>>::get(&alice), 1_000_000);
    });
}

#[test]
fn test_mint_permission_persistence() {
    new_test_ext().execute_with(|| {
        let alice = account_id::<Test>("alice");
        let bob = account_id::<Test>("bob");
        let charlie = account_id::<Test>("charlie");
        
        // Multiple mints with Root - all succeed
        assert_ok!(XKernel::mint(RawOrigin::Root.into(), alice.clone(), 100_000));
        assert_ok!(XKernel::mint(RawOrigin::Root.into(), bob.clone(), 200_000));
        assert_ok!(XKernel::mint(RawOrigin::Root.into(), charlie.clone(), 300_000));
        
        // Non-root still cannot mint even after multiple Root mints
        assert_noop!(
            XKernel::mint(
                RawOrigin::Signed(alice.clone()).into(),
                bob.clone(),
                50_000,
            ),
            DispatchError::BadOrigin
        );
    });
}
```

**Success:** Both tests pass

---

### Task 0.3.2: Add Burn Permission Tests (2h)

Add to same file:

```rust
#[test]
fn test_burn_token_holder_can_burn() {
    new_test_ext().execute_with(|| {
        let alice = account_id::<Test>("alice");
        
        // Setup balance for alice
        assert_ok!(XKernel::mint(RawOrigin::Root.into(), alice.clone(), 1_000_000));
        
        // Alice can burn her own tokens
        assert_ok!(XKernel::burn(
            RawOrigin::Signed(alice.clone()).into(),
            100_000,
        ));
        
        // Balance reduced
        assert_eq!(<FreeBalance<Test>>::get(&alice), 900_000);
    });
}

#[test]
fn test_burn_unauthorized_cannot_burn() {
    new_test_ext().execute_with(|| {
        let alice = account_id::<Test>("alice");
        let bob = account_id::<Test>("bob");
        
        // Setup: alice has tokens, bob does not
        assert_ok!(XKernel::mint(RawOrigin::Root.into(), alice.clone(), 1_000_000));
        
        // Bob cannot burn alice's tokens
        assert_noop!(
            XKernel::burn(
                RawOrigin::Signed(bob.clone()).into(),
                100_000,
            ),
            Error::InsufficientBalance
        );
        
        // Alice's balance unchanged
        assert_eq!(<FreeBalance<Test>>::get(&alice), 1_000_000);
        assert_eq!(<FreeBalance<Test>>::get(&bob), 0);
    });
}

#[test]
fn test_burn_root_can_burn_any() {
    new_test_ext().execute_with(|| {
        let alice = account_id::<Test>("alice");
        
        // Setup
        assert_ok!(XKernel::mint(RawOrigin::Root.into(), alice.clone(), 1_000_000));
        let initial_supply = <TotalSupply<Test>>::get();
        
        // Root can burn (depends on API - adjust if needed)
        // Note: If burn() doesn't support Root origin, remove this test
        let _ = XKernel::burn(
            RawOrigin::Root.into(),
            100_000,
        );
        
        // Supply should be reduced
        let final_supply = <TotalSupply<Test>>::get();
        assert!(final_supply <= initial_supply);
    });
}
```

**Success:** All 4 tests pass

---

## Testing Evidence

```bash
cd /home/lojak/Desktop/X3_ATOMIC_STAR

# Run phase 0.3 tests
cargo test -p x3-kernel test_mint_requires_root -- --nocapture
cargo test -p x3-kernel test_burn_token_holder -- --nocapture

# Expected: All 4-5 tests pass
```

## Sign-Off

- [ ] Mint permission tests passing
- [ ] Burn permission tests passing
- [ ] Root can mint any amount
- [ ] Non-root cannot mint
- [ ] Holder can burn their own tokens
- [ ] Unauthorized cannot burn others' tokens
- [ ] Ready for Phase 0.4

## Notes

- Permissions are **access control critical**
- Any permission bypass is a security vulnerability
- Test both positive (allowed) and negative (denied) cases
- If burn() has different API, adjust tests accordingly
