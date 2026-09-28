# Workstream D — PR #519, the Northern Swarm / Reactor on-chain subsystem

You are a lane of the X3 engineering team. Repo: `/home/lojak/Desktop/xxxstar-main`. Read `AGENTS.md`
first, then this brief. The operator named this workstream explicitly: *"get another agent working on
the 519 branch with the newly tracked Northern Swarm/Reactor subsystem."*

## What "519" is

PR **#519**, branch `release-gate/swarm-reactor-onchain-v1`, head `2cf1d792e` (fetched; it is in your
FETCH_HEAD / `git log origin/master..FETCH_HEAD`). It is **not** merged: it is a draft that adds the
Northern Swarm pallet's on-chain story —

```
 pallets/northern-swarm/{Cargo.toml,src/{benchmarking,lib,mock,tests,types,weights}.rs}   ~+900
 runtime/src/lib.rs (+28)   scripts/mainnet/swarm_reactor_gate.py (new, 301 lines)
 scripts/mainnet_release_gate.py (-/+)   scripts/run-frame-benchmarks.sh
```

22 files, +1881/-323. The headline claim is *measured FRAME weights* for a subsystem that was
previously tracked only in `FEATURE_REGISTRY.toml` as `[x3_reactor]` (`crate_or_service =
"crates/x3-bench"`, tauri_app "Reactor", readiness 40, no matrix row), while `crates/northern-swarm`
and `pallets/northern-swarm` have no tracking at all.

## Your job: verify it, then land it — or say precisely why not

1. **Inspect before trusting.** `git diff origin/master...FETCH_HEAD` for the whole branch, commit by
   commit (`git log --oneline origin/master..FETCH_HEAD`). Note that master has moved a long way since
   the branch was cut — including `runtime/src/lib.rs` gaining `<crate>::weights::SubstrateWeight<Runtime>`
   wiring for 18 pallets tonight, and a new local-ci gate `runtime weights wired`
   (`scripts/check-runtime-weights-wired.py`). A naive merge will conflict there; port the intent.
2. **Re-run its claims on this box.** Whatever the branch says it proved, prove it here:
   `cargo test -p pallet-northern-swarm` (and the crate), `cargo clippy ... --all-targets -- -D warnings`,
   `python3 scripts/mainnet/swarm_reactor_gate.py` (or whatever invocation its header documents), and
   the repo-wide gates it touches. If it claims measured weights, check the numbers came from
   `scripts/run-frame-benchmarks.sh`-shaped generation rather than hand-written constants — the
   repository has been burned by "generated" weight files that were stubs (`pallets/*/src/weights.rs`
   all open with "Auto-generated weight stubs").
3. **Land it on master honestly.** Bring the branch's content onto current master in focused commits:
   the pallet, the runtime wiring, the gate. Resolve the runtime conflict by keeping both intents
   (Northern Swarm's real weights *and* the `SubstrateWeight` wiring). Keep `scripts/mainnet_release_gate.py`
   working — it is the gate that runs tonight's green `make mainnet-check`.
4. **Track it.** Add matrix rows for the Northern Swarm pallet/crate and for Reactor
   (`crates/x3-bench`), with named `required_tests` that exist, honest scores justified by your
   measurements, and blockers that say what is still missing. Reconcile `[x3_reactor]` in
   `FEATURE_REGISTRY.toml` if its 40 and its blockers no longer describe the tree.
5. **Runtime record.** The pallet and `runtime/src/lib.rs` are in the runtime graph, so this changes
   the WASM. Do **not** run `./scripts/update-runtime-hashes.sh` yourself — there are other lanes in
   flight and the record must be refreshed once, on a quiet tree, by the primary agent. Say in your
   report exactly which files of yours are runtime-graph so the refresh covers them.

## Rules

- No fake green. A benchmark you did not run is not measured; a weight file you did not generate is
  not measured; a gate that passes because it does not look is not a gate.
- Do not weaken, skip or delete tests. Prove each new check load-bearing (break it, watch it go red,
  restore, watch it go green) and record the evidence.
- **Commit only your own paths. Never `git add -A`. Do not push** — message me (`/root`) with hashes,
  measurements, and what you could not land. If the branch turns out to be unsound, say so with the
  evidence rather than merging it.
