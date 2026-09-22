# `local-ci`: all gates pass on master — 2026-09-22

Run: `bash scripts/local-ci.sh --testnet` on `bae40d46c8` (merged master), 20:33Z. Summary JSON
`local-ci-20260922T203300Z-summary.json` (gitignored; the per-gate logs sit beside it).

```
local-ci: all gates passed
```

30 gates, 0 failures, including every gate this session's work touched:

| gate | result | seconds | why it is in this note |
| --- | --- | --- | --- |
| format check | PASS | 6 | failed on the genesis-anchor commit; fixed in #463 |
| clippy workspace | PASS | 147 | failed on the same commit (`s.len() % 2 == 0`); fixed in #463 |
| nested workspaces | PASS | 185 | failed on master before this session (`crates/x3-sidecar` stale lockfile); fixed in #464, TICKET-096 |
| test settlement-engine | PASS | 34 | 152 + 23 tests, including the four on real Bitcoin regtest bytes and the four genesis pinning/refusal tests |
| test node | PASS | 231 | the node crate, with the new `dev` feature |
| runtime hash freshness | PASS | 0 | the record moved with every runtime-affecting change |
| testnet ceremony drill | PASS | 244 | a launch can be recorded and verified |
| btc checkpoint genesis | PASS | 392 | **new this session**: a chain starts with Bitcoin's checkpoint pinned, and a spec that lies about it is refused |

The run took ~38 minutes on a box at load ~16–30.

## What this is worth, and what it is not

It is the repository's own gate set, run on the merge of everything landed today, with no
failures — the strongest automated statement available here. It is **not** evidence that the
chain is ready for mainnet: no gate in this list boots two independent operators, none of them
runs a public network, the BTC anchor has never been pinned on a public chain, and the
two-hour soak still fails under load (TICKET-094). Green here means "nothing in the repository
is known-broken", which is a floor, not a ceiling.

Two gates in this list exist because work earlier today found them necessary: the two-hour soak
found the peer-ban feedback loop, and the first genesis drill found that the node had no `dev`
runtime.
