# `arb` scope bounds run against the graph — 2026-09-19

Branch `wip/x3lang-arb-graph-filter-20260919`, commit `bdc83ba7f`, based on
`origin/master` `44a12ba94` (PHASE 22 `99c122b5b`, PHASE 37 `db536ab4c`, PHASE 39
`44a12ba94`).

## Why this exists

PHASE 37 landed with the scope and risk policy decided from the declaration's own
numbers (`compiler/src/arb.rs`), and its own pipeline starts at the opportunity
graph — but nothing asked whether the graph can satisfy those bounds. A scope with
`chains = [x3, ethereum, solana]`, a `500_000 USDC` floor and an `8bps` slippage
ceiling is a search space; if no declared venue is inside it, the scope says "no
opportunity exists" while looking like a strategy, and the search reports an empty
plan.

## What was added

- `arb::search_constraints(&ArbPolicy) -> OpportunityConstraints` — the mapping the
  phase asks for, in the canonical types: the chain *set*, the hop bound, the fee
  ceiling as a path bound, the slippage ceiling and the liquidity floor.
- `arb::admitted_venues(program, policy)` — runs the same two functions the search
  runs (`reject_reason` per venue, `path_reject_reason` on a one-hop path) and
  returns the venues that survive, with the reason each of the rest was refused.
- `arb::scope_admits_a_venue` wired into `arb::verify` — refuses a scope that admits
  nothing, naming every declared venue and the bound that removed it; a program with
  no venue at all is refused as an empty graph. The refusal separates the three
  cases an author fixes differently: no venue, no venue on the scope's chains, and
  venues that break a bound.
- `OpportunityConstraints::allowed_chains` + `RejectionReason::ChainNotAllowed` +
  the path check — `chains = [...]` names chains while `max_chains` counts them, and
  only the set makes a scope's chain list something the search enforces. Comparison
  is case-insensitive, as every other chain reference in this compiler is.
- Tests: `test_arb.rs` (26 -> 29; the fixture now declares the venues the scope
  searches, plus an admitted-set test, a "no venue survives" test and a
  "scope over a chain with no venue" test), `test_opportunity_graph.rs` (+1 for the
  chain set at the search level), and the CLI fixture for the pipeline refusal gains
  its two venues (the scope is now refused one layer earlier otherwise).

## What is deliberately not compared

The deadline. The policy keeps it in blocks, a venue declares its latency in
milliseconds, and `expression_to_blocks` rounds up — the safe direction for a window
and the permissive one for a filter. Deriving one figure from the other would admit a
venue the declared budget excludes, so that comparison belongs to the plan generator
(TICKET-073). `max_finality_blocks`, `max_risk` and `require_proof` stay unset
because the scope states nothing about them.

## Duplicate withdrawn

Before `origin/master` was inspected, an hour went into a second PHASE 37
implementation in the *local* clone (`compiler/src/arbitrage.rs`, an unnamed
`Item::Arb`, `Operation::ArbPlan`, checks against the graph). The local clone was
three commits behind `origin/master` at the time, so the phase looked unclaimed. It
was withdrawn: the files were removed from the local tree and the full patch + both
new files are preserved at `/tmp/root-withdrawn-phase37/`. The idea worth keeping
(run the declared bounds through the canonical filter) is what this branch
implements on top of the landed `arb.rs`.

## Proof

Run in `/tmp/x3-arb-ext` (worktree at `bdc83ba7f`):

- `cargo test --workspace --offline --no-fail-fast` -> **949 passed, 0 failed**
- `cargo clippy --workspace --all-targets --offline -- -D warnings` -> clean
- `cargo fmt --all -- --check` -> clean
- `.venv/bin/python -m pytest -q x3-lang/tests` -> 16 passed
- example sweep `x3c build` over `examples/*.x3` -> 17/23 build (unchanged; no
  example uses `arb`)
- fake-code scan over the changed files -> no matches

Not run here: the repo-root workspace gates (`cargo check/test --workspace` at the
repository root), which compile the whole Substrate-based workspace; this change is
confined to the `x3-lang` workspace and lands through integration.

## Next tasks

- TICKET-073 is still the phase's real gap: nothing turns the decided scope into
  legs. The check added here is what a generator must satisfy, not the generator.
- The deadline comparison belongs in that generator, where the exact millisecond
  figure can be carried beside the block figure instead of derived from it.
- The local clone's `master` is three commits behind `origin/master`; the working
  tree holds another agent's uncommitted PHASE 29 work, so the fast-forward has to
  wait for that to be committed.
