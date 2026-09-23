# The two "safety" e2e files were never tests; four real ones replace them

Date: 2026-09-23. Closes TICKET-106.

## What was there

`tests/e2e/safety_tests.rs` (291 lines) and `tests/e2e/real_finality_proofs.rs` (89 lines) sat in the
`e2e_tests` workspace member. Neither is declared in `tests/e2e/Cargo.toml`, which lists its five test
targets explicitly, and `safety_tests.rs` opens with `mod mock;` for a file that does not exist — so
neither has ever been compiled by anything.

Reading them is worse than the missing declaration suggests. They are not tests that need a harness;
they are written against an API that never existed:

```rust
let bundle_id = H256::random();                      // the pallet derives bundle ids; there is no random one
assert_ok!(AtomicKernel::submit_atomic_bundle(
    RuntimeOrigin::signed(1), vec![], 200));         // the call takes (legs, deadline, chain_id, nonce)
legs.push(BundleLeg::Lock { amount, asset });        // BundleLeg has no such variant
assert_err!(..., "NonceAlreadyUsed");                // compared against a DispatchError, not a string
FinalityCertAnchors::<Runtime>::insert(finalized_block, finality_cert);   // `_ = ...` dropped, never inserted
```

They also sign with `RuntimeOrigin::signed(1)`, an account the runtime has never authorized for the
atomic gate — so even the origin model they assume is wrong. They are exactly the shape the CRITICAL-TOK-1
audit found on this repository before: a file that reads as coverage and provides none.

## What replaced them

Four tests in `runtime/src/tests.rs`, where the runtime-level harness already exists and compiles
(`RuntimeGenesisConfig::build_storage()`), driven through the real calls:

| test | what it proves |
| --- | --- |
| `the_atomic_kernel_refuses_an_account_the_chain_did_not_authorize` | an account outside the custody registry gets `BadOrigin` from `assign_bundle_executor` and `finalize_atomic_bundle` |
| `the_genesis_authorized_gateway_reaches_the_atomic_kernel` | the account the genesis names gets *past* the origin — the wiring added in spec 19, end to end through the real runtime |
| `a_bundle_finalizes_once_with_the_receipt_root_the_chain_requires` | submit → assign → a root the bundle does not commit to is refused (`InvalidReceiptRoot`) → the computed root finalizes it once, PoAE proof stored, second attempt refused |
| `a_bundle_nonce_cannot_be_replayed` | the same `(chain_id, nonce)` cannot submit twice (`InvalidNonce`) |

The genesis those tests build is the point of the first two: a chain that authorizes nobody admits
nobody, and a chain that names its gateway lets it through — the two halves of the origin change,
now exercised rather than asserted in prose.

## A formatting drift found on the way

Running `cargo fmt --all` after these edits rewrote five files that this change did not touch —
`node/src/rpc.rs`, `pallets/x3-settlement-engine/src/runtime_api.rs` (both from the MCP merge) and
`pallets/x3-atomic-kernel/src/tests.rs`, `node/src/service.rs`, `crates/atomic-swap-orchestrator/src/lib.rs`
(blank lines and wrapping left by the last two PRs). Which means **`cargo fmt --all -- --check` was red
on master**: those PRs ran tests, `cargo check` and clippy, but not the format gate. Fixed in the same
PR, as its own commit, with the reason in the message — a gate that nobody runs is not a gate.

## Evidence

```
cargo test -p x3-chain-runtime --lib                50 passed (4 new)
cargo clippy -p x3-chain-runtime --all-targets -- -D warnings   clean
cargo check -p e2e_tests --all-targets              ok (the package still builds without the two files)
cargo fmt --all -- --check                          clean (was failing before this PR)
```
