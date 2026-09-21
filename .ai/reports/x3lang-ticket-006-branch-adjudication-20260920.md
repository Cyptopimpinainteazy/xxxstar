# TICKET-006 — residual `x3-lang` content on stale branches, adjudicated

The ticket: six branches carry `x3-lang` content `origin/master` lacks (~180–320 lines
each), all on pre-trading-core bases, "mostly superseded, but not proven so line by
line". Acceptance: for each branch, either the residual content is shown to be present in
a different form on `origin/master`, or it is landed.

The set turned out to be much larger than six, so the adjudication is mechanical rather
than file-by-file reading.

## Method

1. **Which branches carry an `x3-lang` tree at all** — 562 refs.
2. **Is that tree reachable from `origin/master`?** One `git rev-list --objects
   origin/master` (30 684 objects), then a membership test on each branch's `x3-lang`
   tree hash. A tree that is in master's object graph **is** a state master has held, so
   the branch's content is an earlier state of master rather than work master lacks. This
   is what makes the adjudication cheap: it is a property of the tree, not of the diff.
3. For the branches that fail (2), **per file**: is the path on master now, and if not,
   how many commits in master's current lineage ever touched that path? A file whose path
   is absent and whose history count is zero is content master never had — the only
   residue worth reading.

## Result

```
branches with an x3-lang tree: 562
  tree reachable from master: 530      -> superseded (an earlier state of master)
  NOT reachable:               32      -> 15 distinct trees, adjudicated below
```

| branch (representative) | x3-lang files | files master never had | disposition |
|---|---:|---:|---|
| `add-slippage`, `agents/setup-instructions-request`, `archive/stale-x3lang-trading-wip-20260918`, `batch/20260918T2015Z`, `reapply-features`, `salvage/x3lang-intent-bridge`, `wip/consolidation-20260917/main`, `wip/x3lang-arb-graph-filter-20260919`, `fix/master-trading-core-compile-break` | 135–215 each | **0** | every file is on master, renamed there, or removed there |
| `codex/x3-economic-safety-kernel` (+2 `preserve/*` refs) | 156 | 0 | its two unapplied commits' content is on master: `vm/src/economic.rs` exists, `SubmissionProfile` is in `compiler/src/ir.rs` |
| `wip/consolidation-20260917/recovered-usb-clone` | 130 | 8 | none is language content: `.pytest_cache/*` (4), `ralph.py`, `ralph_output.txt`, `ralph_prd_run.txt`, `x3_dashboard.html` |
| `wip/x3lang-preserve-packets-and-arbitrage-20260919` | 214 | 2 | `compiler/src/{arbitrage.rs,tests/test_arbitrage.rs}` — the **older generation of master's `arb.rs`**: 699 lines against master's 921, 44 of its 87 named items present in master's file, and its branch-only names are the pre-rename spellings (`ArbContract`, `Arbitrage`, `CAPITAL`, `DISCOVER`) where master has `ArbDiscover`/`ArbCapital` and the siblings `hyperarb.rs`/`lanes.rs`/`netting.rs` |
| `wip/consolidation-20260917/x3-lang-prototype-20260621` | — | 22 | a separate `prototypes/x3-lang-20260621/` tree (7 crates, June 2026) that master **never had**; superseded by the implementation (`compiler/src/{parser,semantic,emitter}.rs`, `vm/`) |
| `t5/fix-annotations-20260522-1458`, `your-task-branch` | 0 | 0 | no `x3-lang/` at all — unrelated refs |

## What was not landed, and why that is the right answer

- **`arbitrage.rs`** is the only file on any branch that is neither on master nor a
  non-language artifact. It is a renamed, grown module, so landing it would *delete* the
  newer `arb.rs` — the failure the merge-queue doc's `git cherry` rule exists to prevent.
- **The prototype tree** is a June 2026 prototype of the same language. Master keeps no
  `prototypes/` directory, and the prototype's crates are the ancestors of `x3-lang/`'s
  compiler and VM. Landing it would add a second, smaller implementation of the language
  to the tree.
- **The eight files in `recovered-usb-clone`** are a pytest cache, an agent's transcript
  and its driver, and a dashboard page.

Nothing is landed, and the acceptance's other half — "shown to be present in a different
form on `origin/master`" — is the measured disposition of every one:
530 branches by tree reachability, the economic-commitments work by file and symbol, the
arb module by rename and growth, the prototype by its descendants, and the rest by the
file being on master already.

## Reproducing

```bash
git rev-list --objects origin/master | awk '{print $1}' | sort -u > /tmp/master-objects.txt
git rev-parse <branch>:x3-lang            # the branch's tree
grep -qF <tree> /tmp/master-objects.txt   # is that state one master has held?

# for a branch that fails the test, per file:
git cat-file -e origin/master:<path>                      # is it on master now?
git rev-list --count origin/master -- <path>              # did master ever have that path?
```
