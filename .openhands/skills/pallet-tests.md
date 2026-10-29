---
name: pallet-tests
description: How to write real tests for a FRAME pallet in this repo.
triggers:
  - pallet test
  - write tests
  - add tests
  - mock runtime
  - test coverage
---

# Writing pallet tests

Tests must prove behaviour, not exist. A test like `assert_eq!(2 + 2, 4)` is worse than
none.

## Mock runtime

If the pallet has no `src/mock.rs`, build one from `pallets/x3-verifier/src/tests.rs` or
`pallets/x3-automation/src/mock.rs`. The shape that works on this repo's polkadot-sdk
(`stable2512`):

- `frame_support::construct_runtime!` with `System`, `Balances` and the pallet.
- `#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]` on
  `frame_system::Config`, overriding only `AccountId`, `Lookup`, `Block`, `AccountData`.
- `pallet_balances::Config` with `Balance = u128` (or `u64`), `RuntimeHoldReason`,
  `RuntimeFreezeReason`, `DoneSlashHandler = ()`.
- Use `AccountId32` when the pallet derives accounts from 32-byte hashes or keys.
- Add test-only crates (for example `pallet-balances = { workspace = true,
  features = ["std"] }`) under `[dev-dependencies]` only.

## What to cover, for every extrinsic

1. The success path, asserting the **storage and balances** it changes, not just `Ok`.
2. Every `ensure!` / `Error::<T>::…` it can return, using `assert_noop!` so the test
   also proves nothing was written.
3. Origin checks: an unsigned, wrong-signer, or non-root origin is refused.
4. Money: reserved and free balances before and after, and that total issuance does not
   change unless the code mints or burns on purpose.
5. Repeats: calling twice (double pay, double release, duplicate ids).

## Rules

- Never weaken, delete, or `#[ignore]` a test to make it pass. If the code is wrong, say
  so: either fix the code in the same change and explain the fix, or leave a failing
  test and report it.
- Keep mocks inside test modules.
- Run `cargo test -p <pallet>` and
  `cargo clippy -p <pallet> --all-targets -- -D warnings` before reporting done.
