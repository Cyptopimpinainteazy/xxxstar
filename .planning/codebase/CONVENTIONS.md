# Coding Conventions

**Analysis Date:** 2026-05-19

## Naming Patterns

**Files:**
- Rust pallet entry point: always `src/lib.rs` (never `src/<pallet-name>.rs`)
- Rust test mock: `src/mock.rs` (co-located with tests)
- Rust unit tests: `src/tests.rs`
- Rust weights: `src/weights.rs`
- Rust pallet Cargo.toml: package name uses `pallet-` prefix (`pallet-x3-cross-vm-router`)

**Functions:**
- Extrinsics (callable): snake_case verbs (`initiate_transfer`, `settle_transfer`, `open_launch`, `close_launch`)
- Internal helpers: snake_case with `do_` prefix for side-effectful helpers (`do_initiate_transfer`, `do_settle_transfer`)
- Trait methods: snake_case following FRAME conventions

**Variables:**
- snake_case throughout Rust code
- Type aliases in runtime: PascalCase (`AccountId`, `BlockNumber`, `Balance`)
- Storage value names: PascalCase (`NextNonce`, `UsedMessages`, `PendingTransfers`)

**Types:**
- Error variants: PascalCase; doc comments include error code format `/// LAUNCH-001: ...`
- Event variants: PascalCase; matches extrinsic name where applicable (`TransferInitiated`, `TransferSettled`)
- Config trait types: PascalCase with `T::` prefix when used

## Code Style

**Formatting:**
- Tool: `cargo fmt` (rustfmt)
- Key settings: standard rustfmt defaults; `cargo fmt --all -- --check` enforced in CI (ci.yml)

**Linting:**
- Tool: `cargo clippy`
- Key rules: `-D warnings` in CI (RUSTFLAGS = "-D warnings" in ci.yml) — all warnings are errors
- `#![deny(unsafe_code)]` in every pallet — no unsafe Rust permitted
- `#![cfg_attr(not(feature = "std"), no_std)]` in every pallet — ensures WASM compatibility

## Pallet Boilerplate Pattern

Every pallet follows this structure in `src/lib.rs`:

```rust
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type WeightInfo: WeightInfo;
        // ... domain-specific associated types
    }

    #[pallet::storage]
    pub type SomeStorage<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        SomeType,
    >;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::extrinsic_name())]
        pub fn extrinsic_name(origin: OriginFor<T>, ...) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            Self::do_extrinsic_name(caller, ...)?;
            Ok(())
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        SomeEvent { field: T::AccountId },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Error description
        SomeError,
    }
}
```

## Import Organization

**Order (observed in pallets):**
1. `use super::*;` (inside pallet module)
2. `use frame_support::...`
3. `use frame_system::...`
4. `use sp_runtime::...` / `use sp_std::...`
5. Local crate imports (`use crate::...`)
6. External crate imports (alphabetical)

**Path Aliases:**
- `sp_std::prelude::*` often used for no_std-compatible Vec, BTreeMap, etc.
- Feature-gated std imports: `#[cfg(feature = "std")] use std::...`

## Error Handling

**Pallets:**
- All extrinsics return `DispatchResult` or `DispatchResultWithPostInfo`
- `ensure!(condition, Error::<T>::ErrorVariant)` for precondition checks
- `checked_add` / `checked_sub` for arithmetic; `saturating_add` / `saturating_mul` for metrics
- Propagate errors with `?` operator
- Never use `unwrap()` or `expect()` in pallet code (3 found in workspace total — see CONCERNS.md)

**Off-chain services:**
- `anyhow::Result<T>` for error propagation
- `?` operator throughout
- `thiserror` for typed errors in crates where needed

## Logging

**Framework:** `log` crate in pallets; `tracing` in node binary and services

**Patterns:**
- Pallets: `log::info!("pallet-name: message {:?}", data)` — prefix with pallet name
- Node: `tracing::info!(target: "module", "message")` — use `target:` field
- Use `debug!` for verbose data; `warn!` for unexpected-but-recoverable; `error!` for failures

## Comments

**When to Comment:**
- Every `#[pallet::error]` variant MUST have a doc comment explaining cause
- Every storage item should have a doc comment explaining purpose and invariants
- Error codes follow format: `/// PALLET-NNN: human readable description` (observed in x3-launchpad)
- Complex business logic: inline comment explaining the invariant being preserved

**TSDoc / RustDoc:**
- Public types and trait methods should have `///` doc comments
- Module-level doc comments (`//!`) on pallet modules

## Function Design

**Size:** `do_*` internal helpers handle all business logic; public extrinsics are thin wrappers
**Parameters:** Prefer typed parameters over primitives; use `T::AccountId`, `T::Balance` not raw `u128`
**Return Values:** `DispatchResult` for state-changing calls; `T::SomeType` for reads

## Module Design

**Exports:**
- Each pallet exports `pub use pallet::*;` at crate root
- Weights: `pub mod weights; pub use weights::WeightInfo;`

**Barrel Files:**
- No barrel files — use direct paths in runtime `use` statements

## Invariant Documentation

**Pattern observed in critical pallets (x3-supply-ledger, x3-cross-vm-router):**
```rust
// INVARIANT: canonical_supply == sum(all_domain_totals) + pending_transfers
// This invariant is checked after every state-changing operation.
// Violation: call Halt::halt() and emit HaltEvent
```

---

*Convention analysis: 2026-05-19*
