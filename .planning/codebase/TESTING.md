# Testing Patterns

**Analysis Date:** 2026-05-19

## Test Framework

**Runner:**
- `cargo test` for all Rust crates
- Config: no `jest.config.*` or `vitest.config.*` at root; pallets use inline `#[cfg(test)]` modules

**Assertion Library:**
- Rust: built-in `assert!`, `assert_eq!`, `assert_ne!`
- FRAME: `assert_ok!`, `assert_err!`, `assert_noop!`, `assert_storage_noop!` from `frame-support`

**Run Commands:**
```bash
cargo test                                     # Run all workspace tests
cargo test -p pallet-x3-cross-vm-router        # Single pallet tests (critical path)
cargo test -p pallet-x3-supply-ledger          # Supply ledger tests
cargo test -p pallet-x3-settlement-engine      # Settlement engine tests
cargo test -p pallet-x3-atomic-kernel          # Atomic kernel tests
cargo test -p x3-gateway                       # Gateway tests (requires DATABASE_URL)
cargo test -- --nocapture                      # Show stdout during test run
```

## Test File Organization

**Location:** Co-located with source code inside each pallet's `src/`

**Naming:**
- `pallets/<name>/src/mock.rs` — test mock runtime
- `pallets/<name>/src/tests.rs` — unit tests

**Structure:**
```
pallets/x3-cross-vm-router/
├── src/
│   ├── lib.rs         # Pallet implementation
│   ├── mock.rs        # Test mock runtime (construct_runtime! for tests)
│   ├── tests.rs       # Unit tests
│   └── weights.rs     # Benchmark weights
```

## Test Structure

**Suite Organization:**
```rust
// pallets/*/src/tests.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::*;
    use frame_support::{assert_ok, assert_err, assert_noop};

    #[test]
    fn test_happy_path() {
        new_test_ext().execute_with(|| {
            // Arrange
            let caller = 1u64;
            let amount: u128 = 1_000_000;

            // Act
            assert_ok!(PalletX3::some_extrinsic(
                RuntimeOrigin::signed(caller),
                amount,
            ));

            // Assert
            assert_eq!(Storage::<Test>::get(caller), expected_value);
        });
    }
}
```

**Patterns:**
- Setup: `new_test_ext().execute_with(|| { ... })` wraps every test
- Happy path: `assert_ok!(call)` verifies extrinsic succeeds
- Error path: `assert_err!(call, Error::<Test>::ErrorVariant)` verifies correct error
- No-mutation check: `assert_noop!(call, Error::<Test>::SomeError)` — verifies storage unchanged
- Event check: `System::assert_has_event(Event::SomeEvent { ... }.into())`

## Mock Runtime Pattern

```rust
// pallets/*/src/mock.rs
use frame_support::construct_runtime;

// Construct a minimal test runtime wiring only what the pallet needs
construct_runtime!(
    pub enum Test {
        System: frame_system,
        Balances: pallet_balances,
        PalletUnderTest: crate,  // The pallet being tested
    }
);

// Implement minimal Config traits for Test
impl crate::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    // ... other minimal implementations
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}
```

## Mocking

**Framework:** FRAME's test runtime system (no external mock crate)

**Patterns:**
```rust
// Replace trait implementations with minimal test versions
impl pallet_balances::Config for Test {
    type Balance = u128;
    // ...
}

// Use () for WeightInfo in tests (no actual weights)
type WeightInfo = ();

// Use u64 as AccountId in tests (smaller than production AccountId32)
type AccountId = u64;
```

**What to Mock:**
- `WeightInfo`: always `()` in tests
- External pallets: use real FRAME pallets (pallet-balances, frame-system) in mock
- Trait types: use primitive types (u64, u128) for simpler test data

**What NOT to Mock:**
- Storage: never mock storage — use real `TestExternalities`
- Invariant logic: never stub the supply ledger invariant check
- Core business logic: the pallet under test must execute its real implementation

## Test Coverage (Current State)

**High coverage (with test counts):**
- `pallet-x3-atomic-kernel`: 231 tests (largest test suite)
- `pallet-x3-settlement-engine`: 78 tests
- `pallet-x3-cross-vm-router`: 63 tests
- `pallet-x3-inventory`: 52 tests
- `pallet-atomic-trade-engine`: 46 tests
- `pallet-agent-accounts`: 44 tests
- `pallet-x3-supply-ledger`: 38 tests
- `pallet-x3-launchpad`: 30 tests
- `pallet-x3-dex`: 27 tests
- `pallet-x3-domain-registry`: 27 tests

**Sparse coverage (critical pallets with very few tests):**
- `pallet-evolution-core`: 1 test
- `pallet-meme-overlord`: 1 test
- `pallet-x3-verifier`: 1 test
- `pallet-x3-slash`: 3 tests

**Zero tests (requires immediate action):**
- `pallet-fraud-proofs`: 0 tests
- `pallet-northern-swarm`: 0 tests
- `pallet-x3-control`: 0 tests

## Integration Tests

**Location:** `integration-tests/` (4 loose `.rs` files)

**Status: NOT WIRED INTO WORKSPACE**
- Files: `cross-vm-atomic-test.rs`, `cross-vm-pallet-test.rs`, `parallel-proposer-integration.rs`, `svm-counter-test/`
- No `Cargo.toml` in `integration-tests/` — these are standalone programs
- Not included in `cargo test` run
- Must be explicitly added to workspace or run manually

**Additional test directories:**
- `tests/` — workspace-level tests (may require missing crates)
- `tests_core/` — core test suite
- `tests_phase4/` — phase 4 tests

## Gateway Tests (x3-gateway)

**Location:** `crates/x3-gateway/src/db.rs` (11 tests), `crates/x3-gateway/src/rest.rs` (3 integration tests)

**Run:**
```bash
cargo test -p x3-gateway  # requires DATABASE_URL env var pointing to test DB
```

## CI-Level Tests

**Active in `.github/workflows/ci.yml`:**
- `cargo test -p pallet-x3-cross-vm-router` (critical path)
- `cargo test -p pallet-x3-supply-ledger` (critical path)
- `cargo test -p pallet-x3-settlement-engine` (critical path)
- `cargo test -p pallet-x3-atomic-kernel` (critical path)

**Additional CI workflows:**
- `proof-gates.yml` — launch gate proof execution
- `economic-attack-tests.yml` — economic security validation
- `formal-verification.yml` — formal proofs
- `v04-ship-gate.yml` — v0.4 release gate

**WARNING:** All CI workflows will fail until the ~90 missing crates from root Cargo.toml are resolved. The workspace manifest fails to parse, causing `cargo check` to abort before any tests run.

## Coverage

**Requirements:** No enforced coverage threshold found in CI or config files
**View Coverage:**
```bash
cargo llvm-cov --workspace      # requires cargo-llvm-cov installed
cargo tarpaulin --workspace     # alternative coverage tool
```

## Common Patterns

**Async Testing (in gateway):**
```rust
#[tokio::test]
async fn test_endpoint() {
    let app = build_test_app().await;
    let response = app.oneshot(Request::builder()...build().unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
```

**Error Testing (in pallets):**
```rust
#[test]
fn rejects_duplicate_nonce() {
    new_test_ext().execute_with(|| {
        // First call succeeds
        assert_ok!(Router::initiate_transfer(origin.clone(), params.clone()));
        // Second call with same nonce fails
        assert_noop!(
            Router::initiate_transfer(origin, params),
            Error::<Test>::NonceAlreadyUsed
        );
    });
}
```

**Supply Invariant Testing:**
```rust
#[test]
fn invariant_preserved_after_transfer() {
    new_test_ext().execute_with(|| {
        let before = SupplyLedger::canonical_supply();
        assert_ok!(Router::initiate_transfer(...));
        let after = SupplyLedger::canonical_supply();
        assert_eq!(before, after, "supply invariant violated");
    });
}
```

---

*Testing analysis: 2026-05-19*
