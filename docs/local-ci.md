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
scripts/local-ci.sh --all           # everything (the release bar)
scripts/local-ci.sh --list          # show the gate list, run nothing
scripts/local-ci.sh --pre-push      # what the hook runs
```

Equivalent make targets: `make local-ci`, `local-ci-live`, `local-ci-cross`,
`local-ci-variants`, `local-ci-release`, `local-ci-all`, `local-ci-prepush`,
`local-ci-list`, `local-ci-dry-run`.

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

## The pre-push hook

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
