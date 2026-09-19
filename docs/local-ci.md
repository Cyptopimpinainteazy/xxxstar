# Local CI

`scripts/local-ci.sh` is the gate runner of record for this repository. Every
gate it runs is a real command with a real exit code, written to its own log
under `.ai/runlogs/`.

## Why local CI is the primary CI here

GitHub-hosted runners are unusable for this account: each hosted run finishes in
2-4 seconds with `"steps": []`, which is the account-level billing lock, not a
test failure. `gh run list` shows it on every push - the `ubuntu-latest` jobs
(`Semgrep`, `OSV-Scan`, `Trivy`, `mainnet-readiness`, `feature-matrix`, ...)
complete instantly with no steps, while the self-hosted jobs actually build.

So the two self-hosted runners (`x3star1`, `x3star2`) plus this script are the
only places a gate executes. A push that skips `.githooks/pre-push` is genuinely
unverified.

## Running it

```bash
scripts/local-ci.sh                 # fast gate set (default)
scripts/local-ci.sh --live          # + EVM/SVM contract lifecycles (anvil, solana-test-validator)
scripts/local-ci.sh --cross         # + X3-native and cross-domain lifecycles
scripts/local-ci.sh --variants      # + runtime migration dry-run, all six variants
scripts/local-ci.sh --loom          # + the loom model checks (needs the pinned nightly)
scripts/local-ci.sh --release       # + make mainnet-check
scripts/local-ci.sh --deep          # + cargo test --workspace (slow, broadest signal)
scripts/local-ci.sh --all           # everything (the release bar)
scripts/local-ci.sh --list          # show the gate list, run nothing
scripts/local-ci.sh --pre-push      # what the hook runs
```

Equivalent make targets: `make local-ci`, `local-ci-live`, `local-ci-cross`,
`local-ci-variants`, `local-ci-loom`, `local-ci-release`, `local-ci-all`,
`local-ci-prepush`, `local-ci-list`, `local-ci-dry-run`, `local-ci-deep`.

`--loom` adds `bash scripts/run-loom-tests.sh`, which runs
`tests/loom-concurrency` the way its header documents:
`RUSTFLAGS="--cfg loom" cargo +nightly-2026-05-01 test --release` from that
directory. The crate is outside the workspace on purpose — it is `#![cfg(loom)]`
end to end, so without the flag it compiles to an empty crate, and as a member
it would be a crate of trivially-passing emptiness. Loom needs a nightly (it
depends on `generator`); on a box without the pinned toolchain the gate reports
**BLOCKED**, not PASS. Override the toolchain with `X3_LOOM_TOOLCHAIN`.

`--deep` adds `env -u SKIP_WASM_BUILD cargo test --workspace` — the broadest
automated check the repository has (all test targets in all workspace members;
5,068 tests across 358 binaries as of 2026-09-18). It is slow, so it is opt-in
and `--all`/`make local-ci-deep` include it for release-candidate runs.

### Scheduling and scoping

| flag | meaning |
| --- | --- |
| `--jobs N` | gates to run at once (default 3, or `X3_LOCAL_CI_JOBS`) |
| `--cargo-jobs N` | `CARGO_BUILD_JOBS` for every gate (default 10) |
| `--only a,b` | run exactly these gate slugs (slugs come from `--list`) |
| `--skip a,b` | drop these gates; the summary records each skip loudly |
| `--changed-from REF` | add the gates the diff `REF...HEAD` implies |
| `--dry-run` | print what would run |
| `--fail-fast` | stop scheduling once a gate has failed |

Default parallelism is deliberately low. Two self-hosted runners already share
this 32-core host; letting every cargo invocation default to one job per core
pushed the load average to 54-67. `--jobs 3` with `CARGO_BUILD_JOBS=10` keeps a
full local run inside the machine's budget while still finishing far sooner than
a sequential run.

## What the fast gate set covers

| gate | proves |
| --- | --- |
| `format check` | `cargo fmt --all -- --check` (both workspaces) |
| `agent guards` | `agent_guard`, `no_stub_guard`, `test_cheat_guard` (`make guard`) |
| `make gate exit codes` | the H13 regression: make test recipes propagate cargo's exit code |
| `script syntax` | every tracked shell/Python/tooling-JS file parses (302 shell, 506 Python as of this writing) |
| `workflow wiring` | every script/action/make target a workflow calls exists, and every workflow can actually fire |
| `test integrity diff` | no `#[ignore]`, `.skip(`, `assert true` added by this change set |
| `readiness consistency` | `FEATURE_REGISTRY.toml` `required_tests` are real test function names |
| `workspace check` | `cargo check --workspace` |
| `clippy workspace`, `clippy runtime rc1`, `clippy node rc1` | the three lint configurations, `-D warnings` |
| `test x3-lang`, `test atomic-kernel`, `test atomic-swap std`, `test settlement-engine`, `test node`, `test cross-vm-coordinator` | the unit suites that gate landing |

The `--live`, `--cross`, `--release`, `--variants` and `--loom` groups add the
gates that need a real chain, a full release build, six runtime compilations, or
a pinned nightly toolchain.

## Results

### Preconditions

`--release` (`make mainnet-check`) needs `srtool` on PATH for its
reproducible-build section, and the box has lost that binary more than once
(something rewrites `~/.cargo/bin`). Re-install the pinned revision — the same
one the self-hosted gate job uses — with:

```bash
make srtool-install     # cargo install --locked --git …/srtool-cli --rev 0485b5507a… srtool-cli
```

Every other gate is self-contained. The run prints `srtool=MISSING` in its
prereq line when the binary is absent, so a `--release` failure is never
ambiguous.

Each run writes:

```
.ai/runlogs/local-ci-<UTC stamp>.log              # aggregate, append-only
.ai/runlogs/local-ci-<UTC stamp>-<slug>.log       # one per gate, full output
.ai/runlogs/local-ci-<UTC stamp>-<slug>.status    # PASS/FAIL, the real exit code
.ai/runlogs/local-ci-<UTC stamp>-<slug>.secs      # wall-clock seconds
.ai/runlogs/local-ci-<UTC stamp>-summary.json     # machine-readable summary
```

The run exits non-zero if any gate failed; the summary is printed even when
gates fail, so a partial run is still readable.

### Verification discipline

`--only <light subset>` is for triage, not for sign-off. A change under
`runtime/` (including `runtime/build.rs`, which is compiled by every clippy
configuration) or in a build script must be verified with the whole fast set —
`--pre-push`, or a plain run — before merging. #235 shipped a `clippy::ptr_arg`
error in `runtime/build.rs` precisely because it was verified with a light
`--only` subset; the next full run caught it (`15 PASS / 3 FAIL`, all three
failures the same line). The pre-push hook always runs the full fast set, which
is the intended safety net.

## The pre-push hook

### Batching: five changes, one runner slot

Each push to master queues ~5 heavy jobs on the two runners, so a change waits
behind everything ahead of it. `scripts/batch-runner.sh` (or
`make batch-runner ARGS="--branches a,b,c,d,e"`) builds the same coverage for one
queue slot:

1. creates `batch/<utc-stamp>` off `origin/master` in a scratch worktree under
   `/tmp` (it never touches master);
2. merges up to five branches into it, aborting with the offending branch name
   on conflict;
3. runs the local gate set on that union (`--deep` adds `cargo test --workspace`);
4. pushes the batch branch and dispatches `production-gate.yml` on it, so the
   self-hosted runner executes the full release gate for every change at once.

`--dry-run` stops after the merges, `--no-dispatch` stops after the local gates,
`--ready` takes the branches of the open PRs, `--keep` leaves the worktree for
inspection. The batch branch is never merged automatically — review its runner
verdict and merge whichever branches it proves.

`.githooks/pre-push` delegates to `scripts/local-ci.sh --pre-push`:

1. the fast gate set above, always;
2. `--live` when the diff touches `X3-contracts/evm/**` or `X3-contracts/svm/**`;
3. `--variants` when the diff touches `runtime/**`;
4. `--release` and `--variants` when the push targets `master`/`main`.

Escape hatches, both deliberately loud:

```bash
X3_LOCAL_CI_SKIP_ALL=1 git push        # runs nothing; prints that the push is unverified
X3_LOCAL_CI_PREPUSH_MODE="--live --variants" git push   # any local-ci flags
```

Install the hooks in a fresh clone with `bash scripts/install_hooks.sh`.

## Known-red gates right now

Two gates in the fast set are red on `master` for reasons unrelated to whatever
change is being pushed. Both are tracked, and both are real:

| gate | why it is red | ticket |
| --- | --- | --- |
| `test node` | `x3-chain-node`'s own dev chain spec fails to boot: the genesis blob carries the full variant's pallet set while the runtime that rejects it knows the smaller set | #232 |
| `test cross-vm-coordinator` | `crates/cross-vm-coordinator` sits outside the workspace, so cargo has to hit the network to resolve its dependencies (reported as `BLOCKED`) | #233 |

Until those are fixed, the recorded way to push is an explicit, visible skip:

```bash
X3_LOCAL_CI_SKIP=test-node,test-cross-vm-coordinator git push
```

`--skip`/`X3_LOCAL_CI_SKIP` is deliberately not quiet: the gate is dropped from
the run and the summary prints `SKIPPED BY REQUEST: <slug>`. Do not reach for
`X3_LOCAL_CI_SKIP_ALL=1` instead - that runs nothing at all and says so.

## The box loses rustc / target-dir files mid-build (#330)

This machine periodically rewrites `~/.rustup` / `~/.cargo` — every symlink in
`~/.cargo/bin` (`cargo`, `rustc`, `clippy-driver`, `rustfmt`, ...) points at
one shared `rustup` proxy binary, and that binary (or a file already written
under a running build's target dir) can vanish out from under an in-flight
process, then exist again moments later. It is not this repo's code failing;
it is the toolchain or target dir disappearing mid-build, most often when
multiple builds (interactive sessions, self-hosted CI runners) are racing on
this box at once.

Every real occurrence so far has matched one of three shapes, none of which
is a compiler diagnostic about our code:

```
error: could not compile `sha2` (lib)
  Caused by:
    could not execute process `.../rustc ...` (never executed)
  Caused by:
    No such file or directory (os error 2)
```

```
error: could not parse/generate dep info at: .../deps/sp_runtime-....d
  Caused by: No such file or directory (os error 2)
```

```
error: failed to run custom build command for `ring v0.16.20`
  could not execute process `.../build-script-build` (never executed)
  Caused by: No such file or directory (os error 2)
```

`scripts/local-ci.sh` classifies a gate log matching this pairing (one of
`could not execute process` / `could not parse/generate dep info` / `failed
to run custom build command`, together with `No such file or directory (os
error 2)`) as `BLOCKED`, not `FAIL`, with a reason distinguishing it from the
older network-BLOCKED case in both the terminal summary and the machine-
readable `summary.json`'s `"reason"` field. `BLOCKED` still fails the overall
run — a gate that did not really execute has verified nothing — but a reader
(or an automated merge decision) can tell "the box lost rustc" from "your
code stopped compiling" without re-deriving it by hand. Just re-run the gate;
do not read a `BLOCKED (environment)` gate as evidence the change under test
is broken, and do not merge or revert based on it.

### The quieter failure mode: a shared `CARGO_TARGET_DIR` across revisions

Passing an external `CARGO_TARGET_DIR` (rather than each worktree's own
default) and reusing it across a checkout that has since moved to a
different revision — a real pattern, e.g. verifying several worktrees against
one scratch target dir for speed — can fool cargo's mtime-based freshness
check: it may skip recompiling a crate whose source actually changed, or
replay a stale build's cached warnings/panics. This is dangerous in *both*
directions (false green, or a red that describes code that no longer exists
in the tree). One confirmed case: a `x3-verification-router` test panic was
reported for a test that did not exist in the source at either revision
involved — cargo had replayed a stale binary.

When `local-ci.sh` is run with an explicit, non-default `CARGO_TARGET_DIR`,
it stamps that directory with the worktree path + revision it was built for.
On the next run, if that stamp does not match, it prints a loud warning and
touches every tracked file before the gates run, forcing cargo to re-examine
freshness instead of trusting a cache that may describe a different
revision. This costs a full rebuild the first time a mismatch is detected —
a real but bounded cost, worth paying over silently trusting stale results.
The default case (no `CARGO_TARGET_DIR` override; each worktree gets its own
`target/`) is unaffected and pays no extra cost.

## Workflow wiring reality

GitHub Actions is now deliberately a **manual orchestration layer**. Automatic
hosted `pull_request` / `push` CI was removed because those jobs terminate
before running a step on this account and therefore prove nothing.

The local verification path is:

1. `.githooks/pre-push`;
2. `scripts/local-ci.sh --pre-push`;
3. the self-hosted `x3` runners for explicitly dispatched heavy/release/live jobs.

Linux workflows that are useful on demand target
`[self-hosted, Linux, X64, x3]`. Cross-platform workflows that genuinely need
GitHub/macOS runners (for example desktop packaging / CodeQL matrices) remain
manual fallbacks and never auto-run.

True duplicates of the local gate suite and the temporary queue-drain workflow
were removed. Specialized security, release, deployment, benchmark, formal,
GPU, and live-chain workflows remain available by manual dispatch.

### Security rule for self-hosted runners

Do not add an automatic `pull_request` trigger to a workflow that executes on
a self-hosted runner. A public PR can modify scripts and project code; executing
untrusted PR code on infrastructure that holds caches, credentials, validator
state, or network access is not an acceptable trade for a green check.
