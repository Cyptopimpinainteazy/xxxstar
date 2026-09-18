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
scripts/local-ci.sh --release       # + make mainnet-check
scripts/local-ci.sh --deep          # + cargo test --workspace (slow, broadest signal)
scripts/local-ci.sh --all           # everything (the release bar)
scripts/local-ci.sh --list          # show the gate list, run nothing
scripts/local-ci.sh --pre-push      # what the hook runs
```

Equivalent make targets: `make local-ci`, `local-ci-live`, `local-ci-cross`,
`local-ci-variants`, `local-ci-release`, `local-ci-all`, `local-ci-prepush`,
`local-ci-list`, `local-ci-dry-run`, `local-ci-deep`.

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

The `--live`, `--cross`, `--release` and `--variants` groups add the gates that
need a real chain, a full release build, or six runtime compilations.

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

## Workflow wiring reality

`scripts/check_ci_workflow_refs.py` is a gate, not a report: it fails when a
workflow invokes a script, in-repo action or make target that is not in the
tree, and when a self-hosted workflow is wired to a branch that never fires.
`--parity` prints the full picture, which as of this writing is:

* 42 workflows parsed;
* 11 run on self-hosted runners and can execute here;
* 30 are hosted-only and cannot produce a verdict on this account;
* every workflow reference resolves, and every workflow trigger can fire.

That asymmetry is the reason this file exists: a green hosted check means
nothing, and the only trustworthy signal is a gate that ran on this machine.

### Workflows that auto-run but cannot execute

`scripts/check_ci_workflow_refs.py --parity` also exposes the sharper version of
the problem: workflows that still fire on `push`/`pull_request` while requesting
only GitHub-hosted runners. Each of those produces a step-less failure
(`runner_name: ""`, `"steps": []`, ~2s) on every push, which is how `master` ends
up looking permanently red and why a real failure is easy to miss.

The five workflows on the merge/release path were fixed by removing the triggers
that cannot run (not by moving untrusted PR code onto the runner — see the
security note in `rust-clippy.yml`):

| workflow | now triggers on |
| --- | --- |
| `rust-clippy.yml` | `push`, `workflow_dispatch` |
| `production-gate.yml` | `push`, `workflow_dispatch` |
| `x3vm-live-lifecycle.yml` | `workflow_call`, `workflow_dispatch` |
| `x3vm-evm-live-lifecycle.yml` | `workflow_call`, `workflow_dispatch` |
| `x3vm-svm-live-lifecycle.yml` | `workflow_call`, `workflow_dispatch` |

26 other workflows still auto-run on hosted-only runners (the security scanners
`semgrep`, `trivy`, `osv-scan`, `codeql`; `formal-verification`,
`economic-attack-tests`, `proof-gates`, `release-hardening`,
`frame-benchmarking`, `zombienet-integration`, `x3-desktop-ci`, the
transparency/dashboard deploys, and others). They cannot be silently deleted -
they are security and release tooling - so they need one of three decisions:

1. route them to the self-hosted runner where the tooling exists (adds load and
   requires the tool to be installed on the runner);
2. make them `workflow_dispatch`-only with a comment, so they stay runnable the
   moment hosted minutes exist again without polluting every push;
3. delete the ones that duplicate a gate the local CI already runs.

Until one of those is chosen, treat any red check from those workflows as
"could not run", not "failed".
