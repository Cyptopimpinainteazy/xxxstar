# X3Lang intent-guard findings — 2026-09-18

Scope: `x3-lang/compiler` + `x3-lang/vm`. Baseline was `origin/master`
(`b1d0a4ba5`), not local `master` (`3f1176bce`, 15 commits behind).

## Landed and proven

### 1. Cost-kind policy allowlist is now enforced end to end

`CompiledTradingPolicy.allowed_cost_kinds` was declared but never read. A host
could report `CommittedCost { kind: "totally_made_up" }` and the cost was
accepted as an unclassifiable number; receipts wrote the placeholder
`kind: "committed"` for every entry, so the allowlist was unverifiable after
the fact as well as unenforced before it.

Now: `CostKind::as_str`/`from_str` are the single source of truth; unknown kinds
fail closed (`UnknownCostKind`); the VM's own fee accruals are classified
(borrow/repay `FlashLiquidityFee`, swap `LiquidityFee`, bridge
`CrossDomainFee`) and checked against the allowlist so a venue cannot bypass it
by folding an unlisted cost into a leg fee; `TradingState.cost_ledger`
preserves the real category into receipts; and replay rejects any receipt cost
whose kind is unknown or unlisted.

The default allowlist in `trading_lowering.rs` gained `ProofFee` and
`CrossDomainFee`, because a v1 trade body can bridge and a bridge is settled
against a finality/inclusion proof — without them, enabling enforcement would
have made legal trades unpayable.

### 2. The source-finality requirement is a compile error

`verify_finality_explicit` computed exactly the right condition and then called
`add_warning`. `verify_with_config` returns `Ok(())` whenever no *error* was
accumulated, so **warnings in this verifier are silently discarded**. A bridge
with no source-finality requirement compiled clean — the classic shape of a
reorged source lock becoming an unbacked destination mint. Now an error, with a
conformance rejection case.

### 3. A swap leg must declare a slippage bound

New `verify_slippage_explicit`: any program containing `Swap` or `MultiHopSwap`
must declare `require slippage <= N`. `min_output` is not a substitute — it is
one absolute floor fixed at compile time and says nothing about how far the
market may move before the fill. New error, plus a conformance rejection case.

## Findings that need work (ticketed)

### F1 — `get_builtin_invariants()` is unsound for intent programs (HIGH)

The six builtin rules reason about linear position in `ir.operations`. Intent
lowering emits the `from`/`to` endpoints *before* the route body, and it uses
`Release` for the destination endpoint rather than for a source-chain claim.
Measured on `tests/conformance/valid/intents/internal_swap.x3` — a well-formed
intent — the lowered IR is:

```
0 IntentResolve   1 Lock (from)      2 Release (to endpoint)
3 AtomicBegin     4 Bridge           5 Swap            6 AtomicEnd
7 Require nonce   8 Require finality 9 OnTimeout{Refund}
10 Release (refund target)           11 OnFail
```

and four of the six rules fire: `no_double_claim` (ops 2 and 10),
`no_claim_after_refund`, `no_refund_after_claim`,
`destination_fill_before_source_claim`. Promoting them to errors was measured
and reverted — it breaks every correct intent. Because their results were
dropped as warnings, nobody noticed.

Also note the lowering redundancy: the timeout refund emits both
`OnTimeout{action: Refund}` *and* a standalone `Release`.

### F2 — Proof-requirement guard is both dropped and unsatisfiable (HIGH)

Same dropped-warning bug, plus the expected names (`lock_proof`, `fill_proof`,
`claim_proof`) contradict the vocabulary the language's own fixtures use
(`source_lock_proof`, `source_finality_proof`, `destination_fill_proof`,
`destination_finality_proof`, `solver_signature`). Even a correct program
cannot satisfy the current check.

### F3 — Four compiled-policy fields are fabricated (HIGH)

`trading_lowering.rs` aliases `max_total_cost := max_gas`,
`max_price_impact_bps := max_slippage_bps`, `max_mev_leakage_bps :=
max_slippage_bps`, `quote_freshness_blocks := deadline_blocks`. The AST risk
policy declares none of them, so no source program can set them, and nothing
enforces them. They are read only by `EconomicPolicy::validate_not_weaker_than`
— so the "policy strength" comparison compares fabricated duplicates of the
same numbers.

### F4 — Warnings are structurally unobservable (HIGH)

`verify_with_config` returns `Result<(), Vec<X3Error>>` and drops every warning
the accumulator collected. This is why F1 and F2 went unnoticed, and it is the
root cause of the finality bug. Any safety check written as a warning in this
verifier is dead code.

## Reconciliation with other agents' branches

No git isolation is available, so all sessions share this tree. Branches whose
`x3-lang` content is **not** a subset of `origin/master` (commits touching
`x3-lang` not in `origin/master`, and residual content origin/master lacks):

| Branch | x3-lang commits | residual lines only on branch |
| --- | --- | --- |
| `codex/x3-economic-safety-kernel` | 38 | ~319 |
| `codex/x3-trading-core-v1-hardening` | 8 | ~290 |
| `fix-x3lang-python` | 4 | ~194 |
| `reapply-features` | 4 | ~190 |
| `add-slippage` | 1 | ~182 |
| `archive/pr126-pre-master-rewrite-20260909` | 5 | ~193 |
| `wip/consolidation-20260917/recovered-usb-clone` | 17 | not measured |

All sit on pre-trading-core bases, so most of each diff is origin/master moving
ahead. The residual lines are what needs adjudication — see TICKET-006.

## Verification commands

```bash
cd x3-lang
CARGO_TARGET_DIR=/tmp/x3lang-target-main /tmp/x3lang-cargo.sh test --workspace
CARGO_TARGET_DIR=/tmp/x3lang-target-main /tmp/x3lang-cargo.sh clippy --workspace --all-targets -- -D warnings
CARGO_TARGET_DIR=/tmp/x3lang-target-main /tmp/x3lang-cargo.sh fmt --all -- --check
```

Result at time of writing: **485 passed, 0 failed**; clippy exit 0; fmt exit 0.
Logs: `.ai/runlogs/x3lang-final-gates.log`, `x3lang-slippage4.log`,
`x3lang-final-clippy.log`.
