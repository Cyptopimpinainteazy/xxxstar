# Branch consolidation — everything on master (2026-09-24)

Request: "merge all branches and get everything on the master".

## Result

| ref set | total | contained in master | unmerged |
|---|---|---|---|
| `origin/*` (GitHub) | 19 | **19** | none |
| local `refs/heads` | 370 | 364 | 6 (all lineage-separate snapshots — see below) |

`master` == `origin/master` == `b4ee9bac4` when this was measured.

## What was merged in this pass

1. **`feat/x3-prelaunch-economics-x3lang-cutover`** (16 commits) — the pre-launch
   economics baseline, the April 2026 cross-VM audit set, the X3 Security Swarm pack, the
   `crates/gpu-swarm` workspace member, the `x3_operator` CLI and Python GPU-validator
   stack, the salvaged ops assets, and the settlement/coordinator audit fixes.
   `import/x3-chain-master-salvage` is an ancestor of this branch (its 8 commits are the
   first 8 of the 16), so one merge landed both.
   Applied with **zero conflicts**.
2. **`docs/public-testnet-alpha-execution-plan`** (5 commits, docs only) — the Alpha
   execution plan, the runbook link, and `docs/x3-lang/PRODUCTION_CUTOVER_GATE.md`, which
   makes the X3Lang production cutover a launch blocker. This was the last unmerged ref on
   GitHub.
3. **A second merge of the cutover branch** after it gained one commit (`ed95e7d2f`, the
   readiness-matrix generator + its three derived artifacts + the local-ci freshness gate).

## Two defects the merges exposed, both fixed here

- **The root rustfmt gate was already red on master** before any merge: `cargo fmt --all
  -- --check` failed on three files in `crates/x3-state-snapshot` (landed by the
  snapshot-restore PR the same day). Fixed in `9a58c7495`; the gate is green now.
- **The readiness artifacts landed stale.** `scripts/x3_audit_matrix.py --check` — the
  freshness gate the same commit wired into `scripts/local-ci.sh` — failed immediately on
  master: the committed artifacts record `Source digest: bf2f92fd…` while master's
  sources digest to `a4f506f9…`. They had been generated from a tree that also carried
  registry/matrix edits that are still uncommitted in the main worktree, so the derived
  artifact landed without its inputs. Regenerated in `b4ee9bac4`; the check passes
  (`x3-audit-matrix check PASS: artifacts match their sources`).

## The six local branches that cannot be merged, and why

None of them shares a **common ancestor** with master (`git merge-base` is empty), so
there is no merge to perform: their histories are a different lineage (the pre-master
rewrite and two snapshot branches). Merging one wholesale would replace master's tree with
an older snapshot. Measured per branch:

| branch | commits not in master | what it is |
|---|---|---|
| `heads/archive/pr126-pre-master-rewrite-20260909` | 218 | archived pre-rewrite lineage |
| `docs/grant-readiness-truth-20260908` | 179 | pre-rewrite lineage |
| `wip/consolidation-20260917/recovered-usb-clone` | 179 | 88,443 files, 66,548 of them under `vendor/` — a recovery snapshot |
| `fix-x3lang-python` | 69 | `git cherry` shows 68 of its 69 patches already equivalent on master; the 69th is a 21,687-file "baseline snapshot of unversioned tree" |
| `your-task-branch` | 14 | 71,965 files, 69,838 under `vendor/` — a snapshot |
| `t5/fix-annotations-20260522-1458` | 13 | 14 files, **0 of them absent from master** — fully superseded |

Verdicts: `t5/...` has nothing master lacks. `fix-x3lang-python` is patch-equivalent
except for a bulk snapshot commit. The two `vendor/`-dominated branches are snapshots, not
work. The two pre-rewrite ones are the lineage master was rewritten away from. None is a
candidate for merging; deleting them is a separate, destructive decision for the owner.

## Verification performed

- `SKIP_WASM_BUILD=1 cargo check --workspace --locked` on the merged tree: clean.
- The full `cargo check --workspace --locked` (without `SKIP_WASM_BUILD`) fails in this
  container **identically on pristine master** — the runtime wasm build cannot find `std`
  for `wasm32v1-none` (`crypto-common 0.1.6`). Pre-existing and environmental; the pinned
  toolchain (1.90.0) has both wasm targets installed, so this is not a merge regression.
- `cargo fmt --all -- --check`: green after `9a58c7495`.
- `scripts/x3_audit_matrix.py --check`: PASS after `b4ee9bac4`.
- `x3-lang` on the merged tree: `cargo test --workspace` -> **1315 passed / 0 failed**.
- Merge conflict count: 0 for both merges.

## Not done, and named rather than implied

- The 12 files of uncommitted work in the main worktree (a registry/matrix edit among
  them) were **not** touched, committed, or merged: they belong to work in flight by
  another agent, and committing someone else's half-finished edit is how a merge turns
  into a regression. Their registry edits are exactly what the regenerated artifacts will
  pick up when they land.
- The imported `crates/gpu-swarm` carries a 2.2 MB `ed25519_batch.ptx` build artifact
  beside its `.cu` sources and build scripts. Kept, because the import is faithful and the
  file is referenced by the crate's own kernel build path; it is a candidate for a
  `.gitignore` + regenerate decision, not for a silent delete during a merge.
