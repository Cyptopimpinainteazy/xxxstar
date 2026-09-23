# The unsigned finalization extrinsic is gone

Date: 2026-09-23. Closes TICKET-097's safety half.

## What it was

```rust
#[pallet::call_index(4)]
pub fn submit_finalization_result(origin, bundle_id, receipt_root, finality_cert, _committed_at_ns)
    -> DispatchResult
{
    ensure_none(origin)?;                       // no caller identity at all
    let now = <frame_system::Pallet<T>>::block_number();
    Self::do_finalize_bundle(bundle_id, receipt_root, finality_cert, now)
}
```

`do_finalize_bundle` requires `finality_cert == FinalityCertAnchors[finalized_block]`, and
`FinalityCertAnchors` is written by `record_flash_finality_anchor` — also `ensure_none`, also
reachable by anyone, storing the first non-zero certificate for a height within `[now − 50, now + 5]`.
On a production build the receipt root must commit to the bundle's own fields, but those fields are
public, so the caller can compute it. In one block, any account could therefore:

1. `record_flash_finality_anchor { block_num: now, cert: X }` with an `X` of its choosing,
2. `submit_finalization_result { bundle_id, receipt_root: commit(X), finality_cert: X }` for any
   bundle in `Executing`,

and the bundle was `Finalized` with a proof the chain would show to external verifiers. The
`ValidateUnsigned` guard the pallet's comment credits with preventing this ("prevents anonymous peers
from finalizing bundles they never claimed") checks bundle state and executor, which assignment
already set — it never looked at the caller, because an unsigned extrinsic has no caller to look at.

## Why it was removed instead of repaired

Its only intended producer does not exist. The pallet's OCW read a finalization record from off-chain
local storage under `b"x3fin:" + bundle_id`; the settlement engine's OCW read `b"x3settle:" +
intent_id`. In this repository `sp_io::offchain::local_storage_set` appears in exactly one place —
`node/src/service.rs`, writing `b"x3ff:"` — and the strings `x3fin:` and `x3settle:` appear only in
the readers, in tests of the key format, and in the orchestrator's documentation of it. No component
ever wrote either record, on any chain, in any feature configuration.

Meanwhile the chain already has two signed, authorized finalization entry points that cover the same
ground, and the node's atomic gateway service has always used the signed one:

| entry point | origin | who runs it |
| --- | --- | --- |
| `finalize_atomic_bundle` | `X3LangOrigin` — a genesis-named gateway account in the custody registry (since spec 19; before that, a compiled-in dev key, see `.ai/reports/gateway-origin-registry-20260923.md`) | the node's atomic gateway service, which signs with its own key |
| `finalize_with_settlement` | `SettlementOrigin` — the settlement account | the settlement path |

So the unauthenticated surface was pure attack surface with no producer, and it was deleted rather
than given a signature that nothing could ever produce.

## What was deleted

* the call and its weight entry, its benchmark, and its `ValidateUnsigned` branch;
* the pallet OCW branch that consumed `x3fin:` (the OCW keeps the `x3ff:` anchor submission);
* the settlement engine's OCW hook and its `decode_settlement_finalization_marker` decoder, plus the
  test for that decoder — the settlement engine's bridge to the kernel is `finalize_with_settlement`;
* the orchestrator's two tests and its documentation of the `x3fin:` protocol;
* four pallet tests that pinned the `x3fin:` key format.

## Coverage that was kept

The four checks the removed call exercised are now driven through `finalize_atomic_bundle`, so they
still run against the same core in `do_finalize_bundle`:

```
finalization_refuses_a_bundle_nobody_has_been_assigned_to
finalization_requires_the_chain_to_have_anchored_the_certificate
finalization_requires_the_receipt_root_the_bundle_commits_to
finalization_happens_once
finalization_has_no_unsigned_entry_point      # new: RuntimeOrigin::none() -> BadOrigin, bundle stays Executing
```

## A flaky suite found on the way

`pallet-x3-atomic-kernel`'s mock kept its economic-halt flag in a process-wide `AtomicBool` behind a
mutex. The mutex serialised the halt tests against each other, but **every other test in the crate
read the same flag**, so a test that never touched halting could observe `true` mid-flight:

```
thread 'tests::finalization_refuses_a_bundle_nobody_has_been_assigned_to' panicked:
  Expected Ok(_). Got Err(Module { index: 2, error: [11, 0, 0, 0] })   # EconomicHaltActive
```

Measured: **2 of 6** whole-suite runs failed before the change, **0 of 10** after it. The flag is a
`thread_local` `Cell` now — each test thread gets its own economy, which removes the race instead of
scheduling around it. This matters beyond this pallet: `cargo test --workspace` is one of the
repository's own proof commands, and a suite that fails one run in three cannot support a claim.

## What this does not fix

**TICKET-107.** The signed path still takes its certificate from the per-block anchor, and the anchor
is still written unsigned, first-write-wins. An attacker can plant a certificate for block *N*; the
honest service reads the anchor, builds the commitment around that certificate and **signs it**, so a
bundle can still be finalized with a certificate no voter produced. The fix is client-side and small:
`node/src/atomic_service.rs::finalize_bundle` should finalize with the certificate *its own* finality
voter observed (the same value it would write under `x3ff:`), and treat the chain's anchor as a
cross-check rather than the source — which turns the attack from forgery into a bounded liveness
failure. Until then the anchor is a relay, not an authorization, and this report says so rather than
implying the removal closed more than it did.

## Evidence

```
cargo test -p pallet-x3-atomic-kernel                        66 passed (10 consecutive green runs)
cargo test -p pallet-x3-settlement-engine                    157 + 23 passed
cargo test -p atomic-swap-orchestrator                       23 passed
cargo check -p x3-chain-node                                 ok
cargo clippy -p pallet-x3-atomic-kernel -p pallet-x3-settlement-engine --all-targets -- -D warnings
                                                             clean
rg "sp_io::offchain::local_storage_set"  -> node/src/service.rs only
rg "x3fin:|x3settle:"                    -> no readers left
```
