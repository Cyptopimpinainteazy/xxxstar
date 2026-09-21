# Integration pass — 2026-09-19

Root agent, after the model switch. Three things were found and fixed in the local
clone's state rather than in code.

## The local clone was three commits behind `origin/master`

`origin/master` already carried PHASE 22 (`99c122b5b`), PHASE 37 (`db536ab4c`) and
PHASE 39 (`44a12ba94`); local `master` sat at `0a68cb883`, so every agent working in
the main tree was building on a stale base. Consequence: an hour went into a duplicate
PHASE 37 in `compiler/src/arbitrage.rs` before the landed implementation was noticed.
That work was withdrawn (patch and files preserved at `/tmp/root-withdrawn-phase37/`)
and the one genuinely additive idea — run the declared scope through the canonical
opportunity filter — was re-implemented on top of the landed `arb.rs` on branch
`wip/x3lang-arb-graph-filter-20260919` (`bdc83ba7f`, report:
`x3lang-arb-graph-filter-20260919.md`).

## PHASE 29 was finished but uncommitted, and is now committed and rebased

The packets agent's turn ended with its work deliberately left uncommitted in the
shared tree (`vm/src/opportunity_packet.rs`, `vm/tests/opportunity_packets.rs`,
`vm/src/lib.rs`, `x3c.rs`, `cli.rs`). It was committed as `71d8e26e4`, then rebased
onto `origin/master`; the single conflict (`cli.rs`, where PHASE 22's netting tests and
the packet tests were both inserted at the same point) was resolved by keeping both
blocks. Result: local `master` is `920a44775` = `origin/master` + PHASE 29.

## Integrated proof, on the rebased tree

- `cargo test --workspace --offline --no-fail-fast` -> **981 passed, 0 failed**
  (PHASE 22 + 37 + 39 + 29 all in one tree)
- `cargo clippy --workspace --all-targets --offline -- -D warnings` -> clean
- `cargo fmt --all -- --check` -> clean
- `.venv/bin/python -m pytest -q x3-lang/tests` -> 16 passed
- example sweep `x3c build` over `examples/*.x3` -> 17/23 (unchanged)

## Open work

- **TICKET-076** — reconcile the two `arb` implementations into one: `arb.rs` has the
  guard-enforcement check and the stage map; the withdrawn `arbitrage.rs` had a
  graph-grounded constraint mapping, which the branch above now carries into `arb.rs`.
  The branch is the reconciliation, awaiting landing.
- **TICKET-065** — `crates/x3-crosschain-intent/src/proof/evm.rs` advertises MPT
  verification with no trust anchor (`block_hash` unchecked, trie index ignored,
  `verification-router`'s real verifier never called, zero call sites). Re-audited by
  the `arb_ir` agent; unassigned.
- Rows still missing in the phase ledger: 30 (execution lanes), 38 (hyperarb),
  50 (p50/p95/p99 benchmarks), 51 (GPU path + CPU/GPU equality).
- `wip/x3lang-arb-graph-filter-20260919` (`bdc83ba7f`) is based on `44a12ba94`; it
  needs a trivial rebase onto `920a44775` before landing.
