# The atomic kernel's bundle finalization is not authorized, and its finality gate is self-satisfying

Date: 2026-09-22
Scope: `pallets/x3-atomic-kernel` — `submit_finalization_result` (unsigned), `record_flash_finality_anchor`
(unsigned), `do_finalize_bundle`
Severity: P0 for mainnet. No fund path in this repository releases value on the event, so this is
bundle-integrity and liveness, not a direct drain — but it makes a core pallet's central claim false
and it permanently consumes bundles.

## What the code says it does

`record_flash_finality_anchor` documents itself as the thing that stops fabricated certificates:

> Once anchored, `do_finalize_bundle` uses this to verify the `finality_cert` supplied via the signed
> `finalize_atomic_bundle` extrinsic, **preventing submission of fabricated cert hashes**.

`submit_finalization_result`'s `ValidateUnsigned` says the requirement that a bundle be `Executing`
with an assigned executor "**prevents anonymous peers from finalizing bundles they never claimed**".

Both statements are false against the current code.

## Why the finality gate proves nothing

The gate has two halves, and **the same caller controls both**:

1. The anchor is written by an **unsigned** call (`ensure_none`, `Call::record_flash_finality_anchor`,
   `call_index 5`) whose only checks are `cert != 0` and a block-recency window
   (`ValidateUnsigned`: `block_num <= current + 5`, `current - 50 <= block_num`). The first non-zero
   cert for a height wins:
   ```rust
   if !FinalityCertAnchors::<T>::contains_key(block_num) {
       FinalityCertAnchors::<T>::insert(block_num, cert);
   }
   ```
   Nothing binds `cert` to the block, to a certificate, or to an authority. Any value works.
2. Finalization then requires `finality_cert == FinalityCertAnchors[block_num]`. Since the caller
   planted that value in step 1, the check is a comparison of the caller's input against the
   caller's earlier input.

Reproduction, end to end, by any account that can pay no fee at all:

```
1. attacker: record_flash_finality_anchor(block_num = current, cert = 0x…deadbeef)   // unsigned
   → anchor stored, because nothing else had claimed that height
2. attacker: submit_finalization_result(bundle_id, receipt_root, cert = 0x…deadbeef)  // unsigned
   → passes "Always require valid finality certificate" and the anchor match
```

## Why the executor requirement does not authorize the caller

`assign_bundle_executor` requires a signed origin (`T::X3LangOrigin`) and records the executor on the
bundle. That gates **assignment**, not finalization. Once a bundle is `Executing` — which it must be
for an honest executor to be waiting on results — *anyone* may call `submit_finalization_result` for
it: the call is unsigned, and its `ValidateUnsigned` arm reads only the bundle's status and that an
executor is set. The assigned executor's identity is never compared to the caller, because there is
no caller identity to compare.

Consequence: an attacker front-running a legitimate executor's off-chain worker marks the bundle
`Finalized` with a receipt root of their choosing and permanently blocks the honest result —
`do_finalize_bundle` refuses a second finalization (`ProofAlreadyExists`). Every such bundle is
consumed with a proof nobody produced.

## The dispatch path is weaker than the validation path

`ValidateUnsigned` requires `BundleStatus::Executing`. The dispatch the extrinsic actually runs,
`do_finalize_bundle`, accepts **`Pending` as well**:

```rust
ensure!(
    record.status == BundleStatus::Pending || record.status == BundleStatus::Executing,
    Error::<T>::InvalidBundleState
);
```

A block author can include an unsigned extrinsic without the pool's validation, so the weaker of the
two checks is the one that holds. A bundle that was never assigned to anybody can be finalized.

## Also true

* The dev/testnet builds compile out the `receipt_root` commitment entirely
  (`#[cfg(not(any(feature = "dev", feature = "testnet")))]`), and on mainnet-rc1 the commitment is
  computed from public bundle fields — so it is a deterministic function of on-chain state that any
  observer can compute. It binds the recorded proof to the bundle; it does not prove execution.
* The node's `run_grandpa_finality_anchor` (`node/src/service.rs`) logs
  `cert anchored for block N` **unconditionally**, after a failed `submit_one`, and advances its
  cursor before the submit — so a block whose anchor was rejected is never retried and the log says
  it succeeded. Its doc comment also claims it writes off-chain storage; it does not.
* Nothing in the repository calls `submit_finalization_result` except the pallet's own off-chain
  worker, and there is **no test** for the call at all (`grep submit_finalization_result
  pallets/x3-atomic-kernel/src/tests.rs` → nothing).

## Fix options (owner decision, like the relayer authority)

1. **Authorize the result.** Make `submit_finalization_result` signed by the bundle's assigned
   executor (`record.executor == Some(who)`), or replace the unsigned relay with a committee
   quorum. This is the only option that actually closes the hole, and it changes the off-chain
   worker's submission path.
2. **Make the anchor mean something.** Require an on-chain-verifiable GRANDPA justification, or a
   finalized-head tracker written by a signed authority, so "finality_cert" is not a value the
   caller chooses. The runtime currently has no finality oracle, which is why the check degenerated
   into a value comparison.
3. **Enforce the documented invariants in dispatch**, not only in `ValidateUnsigned`: reject
   `Pending`, reject a second finalization, and keep the two paths in one place.

TICKET-097. Whatever is chosen, the two claims quoted at the top of this file come out of the code
until it is true.
