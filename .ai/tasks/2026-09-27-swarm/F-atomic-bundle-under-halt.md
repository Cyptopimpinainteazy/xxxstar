# Workstream F — a bundle **in flight** when the halt lands, on three validators (P3)

You are a lane of the X3 engineering team. Repo: `/home/lojak/Desktop/xxxstar-main`. Read `AGENTS.md`
first, then this brief. This is priority **P3 (Atomic Kernel)** of the operator's burn-down order.

## The recorded gap you are closing

`scripts/drills/halt_recovery_live.sh` (gate `halt recovery on a live chain`) trips the economic halt
on three real validators and proves the remedy works — but it halts an **idle** chain. Every row that
touches this says so:

* `FEATURE_REGISTRY.toml` → `[atomic_kernel]`: *"the halt and rollback paths have never been exercised
  on a multi-validator network"*.
* matrix `X3-RT-005` (Atomic bundle submission, tested 55): *"Needs multi-validator adversarial
  lifecycle proof"*.
* `X3-RT-002`: *"the signed path's certificate comes from an unsigned anchor any peer can write first
  (TICKET-107), and the halt/rollback paths have never been exercised on a multi-validator network"*.

The specific hole: **nothing proves what happens to a bundle that is already in flight when the halt
lands.** If its bond is trapped until the halt is cleared, the safety valve has taken funds hostage;
if a *new* bundle is still accepted while halted, the valve leaks.

## What to build

Extend the existing drill rather than starting a second one. Files:
`scripts/drills/halt_recovery_live.sh` (+ its driver `scripts/drills/halt_recovery_live_driver.cjs`),
and the gate that runs it (`bash scripts/local-ci.sh --only halt-recovery-on-a-live-chain`, check the
exact slug with `--list`). Do not touch `pallets/**` or `runtime/**` unless you find a real defect —
and if you do, that is a finding worth its own commit and its own message.

New phase, on the same three-validator chain the drill already boots:

1. **Fund and submit a real bundle.** `submitAtomicBundle(legs, deadlineBlocks, chainId, nonce)` is
   gated on `T::X3LangOrigin`, which this runtime wires to `EnsureX3LangGateway` — on a dev/local
   chain the gateway is `//x3-atomic-gateway` (`dev_gateway_genesis()`), and it is endowed on `local3`
   since `681e2e260`. `BundleLeg` lives in `pallets/x3-atomic-kernel/src/proof.rs`:
   `{ vm_type, token_in: H256, token_out: H256, amount_in: u128, min_amount_out: u128, deadline: u64,
   access: DeclaredAccess }` — let polkadot-js encode it from plain JS values. Copy the leg shape
   from a passing pallet test (`pallets/x3-atomic-kernel/src/tests.rs`) rather than inventing one.
   Record the submitter's free and reserved balance, and the bundle's status, at a finalized block.
2. **Trip the halt** exactly as the drill already does (council motion → `emergencyHalt`), and require
   both flags set.
3. **While halted, a *new* bundle must be refused** with `EconomicHaltActive`, and the refusal must
   not consume a nonce, reserve a bond, or create a bundle. Assert that on the chain, not by reading
   code.
4. **While halted, the in-flight bundle must still be rollbackable.** `rollbackAtomicBundle(bundleId,
   reason)` is `ensure_signed` and the reason must be authorised for the caller
   (`rollback_rejects_callers_who_are_not_authorised_for_the_reason` shows the rule), and
   `rollback_atomic_bundle` is on the halt's exemption list (`6d7bfc540`) — that is the property this
   phase exists to measure. Require: the extrinsic is **included** (not refused by the pool), the
   bundle ends `RolledBack`, the submitter's reserved balance returns to its pre-bundle value, and
   total issuance is unchanged on **every** validator at one finalized block.
5. **Then clear the halt** (the drill already does) and require traffic and a fresh bundle submission
   to work again.

Your negative controls matter more than the happy path: the *new-bundle-refused* call must be measured
to leave no state behind, and the rollback must be measured to actually move the bond (a rollback that
silently does nothing would make step 4 pass vacuously).

## Rules

- Do not weaken, skip, delete or `#[ignore]` any existing test or gate. The drill must still prove
  everything it proves today; you are adding a phase, not replacing one.
- Real chain state only: no mocked chain-success claim, no fabricated tx id, no hard-coded block hash.
- Run it: `bash scripts/drills/halt_recovery_live.sh` (three nodes, ~5–10 min; the box may be busy —
  give it a generous timeout and re-run rather than declaring a flake).
- Commit only your own paths. **Never `git add -A`. Do not push** — message me (`/root`) with the
  commit hash, the exact commands, the observed output, and what you could not prove.
