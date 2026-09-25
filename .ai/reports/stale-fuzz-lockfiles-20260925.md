# The fuzz workspaces' lockfiles are all stale, and no gate compiles them

Found on 2026-09-25 while landing the EVM interpreter change
(`6b3e241eb`), which invalidated three nested lockfiles. One of them was a real
consequence of that change; the other two turned out to be stale for reasons
that predate it.

## What was measured

`cargo metadata --locked --manifest-path <dir>/Cargo.toml`, run against the tree
at `0899fb645` (the commit *before* the evm change) for the three nested
workspaces that mention the changed crate:

| workspace | `--locked` at `0899fb645` |
| --- | --- |
| `crates/x3-sidecar` | OK |
| `pallets/atomic-trade-engine/fuzz` | **exit 101** — "the lock file … needs to be updated but `--locked` was passed" |
| `pallets/x3-coin/fuzz` | **exit 101** — same |

So the two fuzz locks were already stale before the evm change; the sidecar lock
was current and had to move.

Widening the measurement to every tracked fuzz workspace, run against the current
tree:

```
OK      pallets/agent-accounts/fuzz
STALE   pallets/agent-memory/fuzz
STALE   pallets/atomic-trade-engine/fuzz
STALE   pallets/cross-chain-validator/fuzz
STALE   pallets/depin-marketplace/fuzz
STALE   pallets/evolution-core/fuzz
STALE   pallets/fraud-proofs/fuzz
STALE   pallets/governance/fuzz
STALE   pallets/meme-overlord/fuzz
STALE   pallets/private-execution/fuzz
STALE   pallets/svm-runtime/fuzz
STALE   pallets/swarm/fuzz
STALE   pallets/treasury/fuzz
STALE   pallets/x3-account-registry/fuzz
STALE   pallets/x3-asset-registry/fuzz
STALE   pallets/x3-atomic-kernel/fuzz
STALE   pallets/x3-automation/fuzz
STALE   pallets/x3-coin/fuzz
STALE   pallets/x3-cross-vm-router/fuzz
STALE   pallets/x3-da/fuzz
STALE   pallets/x3-dex/fuzz
STALE   pallets/x3-domain-registry/fuzz
STALE   pallets/x3-invariants/fuzz
STALE   pallets/x3-inventory/fuzz
STALE   pallets/x3-jury-anchor/fuzz
STALE   pallets/x3-kernel/fuzz
STALE   pallets/x3-oracle/fuzz
STALE   pallets/x3-reservation/fuzz
STALE   pallets/x3-sequencer/fuzz
STALE   pallets/x3-settlement-engine/fuzz
STALE   pallets/x3-slash/fuzz
STALE   pallets/x3-solvency/fuzz
STALE   pallets/x3-supply-ledger/fuzz
STALE   pallets/x3-token-factory/fuzz
STALE   pallets/x3-verifier/fuzz
STALE   pallets/x3-vrf/fuzz
STALE   pallets/x3-wallet-pallet/fuzz
```

**36 of 37.** `pallets/agent-accounts/fuzz` is the only one whose lock still
resolves.

## Why it has gone unnoticed

No gate builds a fuzz target. `GATES_FAST` has a `test` entry per pallet but
nothing for `*/fuzz`, and `nested workspaces` names four sidecar/swarm workspaces
plus `crates/x3-sidecar` — the fuzz trees are not in it. A stale lock is
therefore invisible until someone runs `cargo fuzz`, at which point cargo
re-resolves the whole graph and picks whatever is newest.

That matters more here than for an ordinary dev target: these are the adversarial
harnesses, and a harness that silently rebuilds against a different dependency
set than the pallet it is fuzzing is not evidence about that pallet.

## Why this was not fixed in the evm change

Repairing one lock is not a lock repair. `cargo update --manifest-path
pallets/x3-coin/fuzz/Cargo.toml -p evm@0.39.1` — the narrowest command that
targets the stale entry — moved that lock from **525 to 637 packages**, adding
`yoke`, `zerovec`, `zerotrie`, `zerofrom` and ~100 others. That is a dependency
refresh, not a repair of the evm entry, and 36 of them would be 36 dependency
refreshes inside a change about one interpreter. The two fuzz locks were
therefore reverted to their committed state and recorded here instead.

## Acceptance criteria for the follow-up

1. Decide the policy first, because the two options lead different places:
   keep the fuzz workspaces and gate them, or delete the ones nobody runs.
2. If they are kept: `cargo metadata --locked --manifest-path
   pallets/<name>/fuzz/Cargo.toml` exits 0 for **all** tracked fuzz workspaces,
   and a gate covers that so the next dependency change cannot leave them
   behind. `pallets/agent-accounts/fuzz` is the reference for what a current
   lock looks like.
3. If they are deleted: the tracked `*/fuzz` trees that no target builds are
   removed in one change, with a list of what was removed and why, rather than
   left to rot.
4. Either way the refresh is its own change, with the dependency delta stated
   (for `pallets/x3-coin/fuzz`: 525 -> 637 packages) rather than buried in an
   unrelated commit.
