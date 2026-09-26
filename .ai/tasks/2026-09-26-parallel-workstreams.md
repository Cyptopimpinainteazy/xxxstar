# Parallel workstreams — 2026-09-26

Three agents are working in this tree at once, plus the primary agent. This file is the
authoritative scope for each. If you were spawned and the message that came with you was empty,
**find your workstream here by reading the "Your workstream" section of the message, or take the
first workstream whose files are untouched.**

## The goal being served

Raise X3 from ~61% to **75% public-testnet readiness**: every mainnet-blocking feature at 80%+,
zero P0 defects, zero broken features, and — once the operator's 7 physical servers exist — a real
7-validator testnet surviving a 72-hour soak. The specific bullets these workstreams touch:

* `X3Lang: .x3 → compile → X3VM → finalized block → receipt proven across the validator network`
* `Snapshot/restore: demonstrated`
* `Prometheus/Grafana/logging: live across all validators`

The 7 physical validators are **not available** (the operator is still bringing up the servers).
Everything here has to be provable on this box using the built-in `local3` three-validator chain.

## Ground rules for every agent in this tree

1. **Work on code, not prose.** A commit whose only content is `.md`/`.ai`/`reports` is not a
   completed task here.
2. **Do not commit and do not push.** The primary agent commits. Leave your work in the working
   tree and report exactly which files you touched. If you think a commit is essential, say so in
   your report instead of making one.
3. **Own your files.** Only create/modify the files listed under your workstream. Do not edit
   `runtime/src/lib.rs`, `pallets/x3-kernel/**`, `node/src/**`, or anything another workstream
   names.
3b. **`scripts/local-ci.sh` is the primary agent's file** while three agents run. Write the gate
   line you want and hand it over in your report as text; do not edit the file yourself.
4. **Never touch `reports/rc6/*`** — those are the operator's dirty files. `FETCH_HEAD` and
   `libproto_lib/` are untracked junk; leave them.
5. **No fake green.** A test that asserts a constant, a mock that stands in for the production
   path, or a skipped test is not evidence. If you cannot make something real, make it fail closed
   and say so.
6. **Prove your work with the repository's own commands** and quote the exact output in your report.
   `bash scripts/local-ci.sh --only '<slug>'` is how gates are run; `--list` shows the slugs.
7. Old proofs are not new proofs. Re-run the command and paste what it printed.

## Current tree state (check it yourself before editing)

```
branch  feat/x3-prelaunch-economics-x3lang-cutover
head    25a07000b  (master on origin is the same commit)
dirty   reports/rc6/*  (operator's — do not commit)
```

Live gates that were green on this box immediately before these workstreams started:
`local node smoke`, `local network smoke` (3 validators, GRANDPA finality, canonical-hash
agreement), `EVM contract lifecycle` (anvil), `SVM contract lifecycle`,
`X3-native lifecycles`, `cross-domain EVM`, `cross-domain SVM`, both strict-posture variants,
`runtime variant dry-runs`, `validator failure drill`, `testnet ceremony drill`.

Helper infra you are expected to reuse rather than reinvent:

* `scripts/local-network-smoke.sh` — boots the built-in `local3` chain (Alice/Bob/Charlie) on free
  ports, waits for finality, and requires all three to agree on the canonical hash. Read it before
  writing your own bring-up; it already solves ports, `--base-path`, and teardown.
* `scripts/local-node-smoke.sh` — one node.
* `node/tests/x3vm_live_lifecycle.rs` — boots a real node and proves comit lifecycle claims. The
  `#[ignore]`-marked tests in it are the ones the live/cross gates run.

---

## Workstream A — monitoring live across all validators

**Deliverable:** something that puts metrics and logs from **all three** `local3` validators in
front of an operator, and a runnable check that fails when that is not true.

Concretely:

1. A bring-up path (`scripts/monitoring/…` and/or `monitoring/…`) that starts the three validators
   with Prometheus endpoints enabled and scrapes them. `substrate-prometheus-endpoint` is already a
   dependency of the runtime; the node exposes it via `--prometheus-port` (verify the real flag —
   read `node/src/cli.rs`, do not guess).
2. A check that proves **per-validator** liveness rather than "a port answered": each validator's
   metrics endpoint yields its own identity (peer count, finalized number, role) and the three are
   distinguishable. A single scrape of one node is not the evidence this bullet asks for.
3. A Grafana dashboard definition (JSON) an operator can import, plus a Prometheus scrape config,
   both checked in and both validated by the check (e.g. `jq`/JSON parse) so they cannot rot.
4. A gate entry so this runs by itself: **write the exact line** for the `--live` array in
   `scripts/local-ci.sh` (a name that says what it proves) and hand it over in your report — the
   primary agent applies it, because another workstream is also adding a gate line.

**Files you own:** `scripts/monitoring/**`, `monitoring/**`, and any *new* test file you add for it.

**Acceptance:** you have run the new gate on this box and pasted its output, including the part that
shows three separate validators' metrics; and you have shown it fails when one validator is not
scraped (temporarily, then restored).

---

## Workstream B — X3Lang proven across the validator network

**Deliverable:** `.x3 source → compiler → X3BC → submit_comit_v2 → a *finalized block* → receipt
read back from a node that did not submit it`, on the three-validator `local3` network.

Today this is proven on **one** node (`node/tests/x3vm_live_lifecycle.rs`). The goal bullet says
"proven across the validator network". The gap is specifically:

* the submitting node is not the only node — a *different* validator must be the one the receipt is
  read from, and that node must have finalized the same block containing the extrinsic;
* the receipt must be readable from finalized state on a node that never saw the submitter's local
  view (i.e. it came through consensus).

**Files you own:** a **new** test file under `node/tests/` (do not edit
`x3vm_live_lifecycle.rs`; that one belongs to the existing proofs) and, if you need a runner, a new
script under `scripts/`. Write the gate line for it if it belongs in `--cross`, and hand it over.

**Acceptance:** the ignored/live test run on this box, with its output pasted: three validators
running, the comit finalized, the receipt fetched over RPC from a node other than the submitter,
and the receipt's value/gas/version matching the compiled program. If a genuine gap stops you,
say precisely which call returned what, and fix what you can (fail closed rather than fake it).

---

## Workstream C — snapshot / restore demonstrated on a running chain

**Deliverable:** export the state of a running chain, restore it into a fresh node, and prove the
restored chain is the same chain — not an empty one that merely starts.

Requirements, in the order they matter:

1. The restore must preserve **finalized height**, the **canonical hash at that height**, and a
   piece of application state (the kernel's canonical ledger / supply, or a registered asset).
2. The snapshot must be taken from a chain that is actually running, not from a stopped database
   copied by hand, unless you can show that is the same thing here (`--base-path` + a real DB).
3. Restoring must be shown to be *load-bearing*: a check that would pass for an empty database has
   to fail. The repository already has a `snapshot murder test` gate and a `test state snapshot`
   gate — read them first (`scripts/local-ci.sh --list`, then the script) and extend the real one
   rather than adding a parallel mechanism.

**Files you own:** the snapshot/restore script and test files (search for the existing ones and
work in place), plus — as text in your report — a gate line if the proof is not yet gated.

**Acceptance:** the command you ran, its output, and the specific numbers (height, hash, supply)
before and after. If "restore" is only a file copy of a live database and that cannot be made
safe, say so plainly and show what the tooling actually does today.

---

## Reporting back

End your turn with exactly this, for your workstream only:

```
FIXED:            (code changes, files)
TESTED:           (commands + the result they printed)
COMMIT:           (none — the primary agent commits; say what you would commit)
REMAINING HARD BLOCKERS:
```

If the message that spawned you contained no text at all, say so first, state which workstream you
took from this file, and then do it.
