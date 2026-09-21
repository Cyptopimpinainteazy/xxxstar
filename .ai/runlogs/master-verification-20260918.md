# Master verification — 2026-09-18 (master 1035e6579)

Worktree used: `/tmp/x3-mine` (detached at `origin/master`), shared warm target dir
`/home/lojak/Desktop/xxxstar-main/target`.

## Landing sequence

| Step | Commit | Evidence |
| --- | --- | --- |
| base master before pass | `55176f030` | `git rev-parse origin/master` |
| PR #196 verified + merged | `4ac8d9ae9` → `78a1dce99` | `.ai/runlogs/pr196-x3lang-test.log` (403 pass), `.ai/runlogs/pr196-x3lang-clippy.log` (exit 0) |
| PR #185 rebased (`--onto origin/master 22391f5f4`) | `4aa67b8cc` | `.ai/runlogs/pr185-x3lang-test-rebased.log` (E0063, expected) then `.ai/runlogs/pr185-x3lang-test-final.log` (409 pass) |
| PR #185 merged | `2c6062687` | `gh pr view 185 --json state` → `MERGED` |
| PR #197 (salvage of orphaned `6cffd44d9`) | `8d98d1eff` → `d5f6b50ab` | `.ai/runlogs/livequotes-on-master-x3lang-test.log` (412 pass) |

## Gates on the merged master

| Gate | Command | Result |
| --- | --- | --- |
| x3-lang tests (final tip) | `cargo test --workspace --offline` in `x3-lang/` | 412 passed, 0 failed (`master-d5f6b50ab-x3lang-test.log`) |
| x3-lang tests (pre-#197 master) | same | 409 passed, 0 failed (`master-x3lang-test.log`) |
| coordinator tests | `cargo test --manifest-path crates/cross-vm-coordinator/Cargo.toml --offline` | 154 passed, 0 failed (`master-coordinator-test.log`) |
| root workspace check | `SKIP_WASM_BUILD=1 cargo check --workspace --offline` | exit 0 (`master-2c6062687-check-nowasm.log`) |
| x3-lang clippy | `cargo clippy --workspace --all-targets --offline -- -D warnings` | exit 0 |
| x3-lang fmt | `cargo fmt --all -- --check` | clean |
| local runner | `gh workflow run x3-local-runner-smoke.yml` | run 35295307485 → **success** |

## Orphaned-work check (do this on every pass)

```
for b in $(git branch -r --no-merged origin/master | grep -v HEAD | sed 's|origin/||'); do
  echo "$b: $(git log --oneline origin/master..origin/$b | wc -l) commits ahead"
done
```

Pushing to the head branch of a **merged** PR strands those commits: GitHub will not extend a
closed PR. That is how `6cffd44d9` (live price quotes + slippage enforcement) was lost until it
was salvaged as PR #197.

## Live-node lifecycle gate (PR #198)

| Step | Result |
| --- | --- |
| `node/tests/x3vm_live_lifecycle.rs` compile | **failed on master** (E0428 triple definition + E0560 stale `SecretReleaseEvidence` fields) |
| `cargo test -p x3-atomic-swap --features std` | did not compile before; **764 passed / 0 failed** after |
| `cargo test -p x3-chain-node --test x3vm_live_lifecycle -- --ignored --test-threads=1` | **3 passed / 0 failed** (159s) on a real `x3-chain-node --dev` node |
| `cargo test -p x3-chain-node` | 49 passed; the sole failure is the sandbox refusing TCP binds and passes unsandboxed |
| merged tree check | `git diff --stat origin/master 8dd7f36ab` empty ⇒ master `1035e6579` is exactly the verified tree |

Root cause of the last failing assertion, found by decoding the pallet error instead of reading a
bare `ExtrinsicFailed`: `Module(index: 31, error: [2])` = settlement-engine `InvalidIntentState`,
because `on_initialize` had already auto-refunded the intent once the canonical refund proof set
landed. Pallet index 31 was confirmed with a `PalletInfo` probe (hand-counting the declaration
order gives the wrong answer).

## Cross-VM live gates (PR #199)

| Gate | Command | Result |
| --- | --- | --- |
| EVM contract lifecycle | `X3-contracts/evm/test-live-lifecycle.sh` | **11 passed / 0 failed** (real anvil, real broadcaster, raw `cast` reads) |
| EVM cross-domain (X3VM ↔ Anvil) | `real_x3vm_evm_lock_claim_atomic_lifecycle` | **PASS** (57s) |
| EVM cross-domain timeout refund | `real_x3vm_evm_timeout_refund_atomic_lifecycle` | **PASS** (61s) |
| SVM contract lifecycle | `programs/svm/x3_atomic_swap/test-live-lifecycle.sh` | **15 passed / 0 failed** (real SBF build + `solana-test-validator`) |
| node package | `cargo test -p x3-chain-node` (unsandboxed) | **88 passed / 0 failed** |

False green removed: `x3vm-evm-live-lifecycle.yml` invoked
`real_x3vm_evm_refund_is_terminal_on_both_domains`, which exists nowhere in the repository;
`cargo test` exits 0 on an empty filter match, so the step passed while running nothing. A scan of all
workflow `--test <target> <name>` invocations found no other phantom reference.

Still unrun: the SVM **cross-domain** tests (`node/tests/x3vm_svm_live.rs`, 2 ignored tests).

## Round 2 (master 66d7c525b): cross-domain gates green in CI

| CI job (self-hosted x3star1) | Result |
| --- | --- |
| `x3vm-svm-live-lifecycle` / SVM HTLC live validator lifecycle (run `35303169753`) | **success**, 17/17 steps |
| `x3vm-evm-live-lifecycle` / `real X3VM to Anvil lock claim lifecycle` (run `35304527539`) | **success** — both `Run real X3VM-EVM …` lifecycles green |
| `x3vm-evm-live-lifecycle` / `EVM HTLC live anvil lifecycle` | **failure** → fixed by PR #203 (`forge install` vs. the runner's reused workspace) |

`forge install` failure (real CI defect surfaced by the routing):

```
Error: cannot safely install dependency at X3-contracts/evm/lib/forge-std
       because the target or .gitmodules has existing changes
```

The runner reuses one workspace across jobs, so the sibling job's install left `lib/` and
`.gitmodules` dirty. Both jobs now use an idempotent `ensure_pinned` clone of the `foundry.lock`
tags; verified locally against the same dirty condition (prints "already pinned to v1.16.1" /
"v4.9.6", exit 0).

## Round 3 (master 92338906f)

**All three live cross-VM gates are green in CI:**

| Workflow | CI run | Result |
| --- | --- | --- |
| `x3vm-svm-live-lifecycle` | `35303169753` | success, 17/17 steps |
| `x3vm-evm-live-lifecycle` (cross-domain job) | `35304527539` | success, both X3VM-EVM lifecycles |
| `x3vm-live-lifecycle` (X3-native) | `35304527633` | success, all three local-node lifecycles |

### atomic-kernel fund-trapping defect (PR #204)

Giving the never-covered EconomicHalt guard real tests exposed a fund-accounting bug:

```
do_rollback_atomic_bundle: slash() before unreserve(); slash spends free before reserved
=> 10,000,000 bond, SubmitterCancelled: 5,000,000 taken from free balance AND
   5,000,000 left reserved forever (RolledBack can never finalize/roll back again)
```

Fixed by unreserving the whole bond first, then slashing the penalty out of it. Penalty policy
unchanged. New tests: `economic_halt_blocks_bundle_submission`,
`economic_halt_does_not_trap_pending_bundle_funds` (the latter asserts reserved == 0, free loses exactly
the 50% penalty, and total issuance is unchanged because the slash moves to the treasury).

## Round 4 (master 9a5c1edd6): rollback invariants for every reason

PR #205 added `ExecutionFailed` (10%), `AccessSetViolation` (10%) and `DeadlineExceeded` (100%) balance
tests plus authorization/early-deadline negatives, all sharing `assert_bond_settled_once`.

**Pre-fix reproduction** (tests run against `HEAD~1`'s pallet source before acceptance):

| Test | Pre-fix result |
| --- | --- |
| `rollback_deadline_exceeded_slashes_the_whole_bond_once` | **FAILED** — `reserved_balance` left at **10000000** (entire bond) |
| `rollback_execution_failed_charges_only_the_ten_percent_penalty` | **FAILED** — left at **1000000** (the penalty) |
| `rollback_access_set_violation_charges_only_the_ten_percent_penalty` | **FAILED** — left at **1000000** |
| `rollback_rejects_callers_who_are_not_authorised_for_the_reason` | ok (no accounting dependency) |
| `rollback_deadline_exceeded_before_the_deadline_is_rejected` | ok |

Gates: `cargo test -p pallet-x3-atomic-kernel` → 87 passed / 0 failed; clippy → exit 0;
`scripts/check-readiness-consistency.sh` → PASS; `atomic_kernel` score 45 → 50 with seven real
`required_tests` entries.

## Round 5: every live cross-VM gate is green in CI

Latest run per workflow, all `completed/success` on the self-hosted runner:

| Workflow | Run | Jobs / steps |
| --- | --- | --- |
| `x3vm-live-lifecycle` | `35304527633` | 3 real local-node lifecycles (claim, timeout refund, early-refund fail-closed) |
| `x3vm-evm-live-lifecycle` | `35306574985` | both jobs: `real X3VM to Anvil lock claim lifecycle` **and** `EVM HTLC live anvil lifecycle` (contract gate, 11 checks) |
| `x3vm-svm-live-lifecycle` | `35303169753` | 17/17 steps incl. validator lifecycle, SBF build, both X3VM-SVM cross-domain lifecycles |

The EVM contract job's `Install forge dependencies (pinned via foundry.lock)` step is green, confirming
PR #203's `ensure_pinned` clone works on the runner's reused workspace — the exact step that failed
before.

## Round 6 (master bad90cc48): master gates were being cancelled

Push-triggered gates on the self-hosted runner were reaching the runner and then being cancelled by the
next commit:

```
2026-09-18T04:45:15Z push queued              9a5c1edd6  Rust Clippy
2026-09-18T04:37:18Z push completed/cancelled 92338906f  Rust Clippy
2026-09-18T04:20:18Z push completed/cancelled 66d7c525b  Rust Clippy
2026-09-18T04:37:18Z push completed/cancelled 92338906f  production-gate
```

Cause: 23 push-triggered workflows used
`group: ${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}` with
`cancel-in-progress: true`; on push the group collapses to `<workflow>-refs/heads/master`, so every new
commit killed the running gate. Fixed in PR #206 (per-commit group via `github.sha`, `cancel-in-progress`
PR-only).

Live verification after the merge: the previously running gate (`35308164935`, production-gate → nested
X3-native live lifecycle) stayed `in_progress`, while `production-gate`, `Rust Clippy`,
`x3vm-evm-live-lifecycle` and `x3vm-svm-live-lifecycle` for the new commit were all `queued` instead of
cancelling each other.

Diagnosis table worth keeping: `cancelled` ⇒ concurrency/queueing; `failure` with `"steps": []` ⇒ hosted
billing lock; started-but-red ⇒ actual code/CI defect.

## Round 7 (master 6219cdc6a): x3-atomic-swap lints clean under std

`cargo clippy -p x3-atomic-swap --all-targets --features std -- -D warnings` failed with 52 errors
(`ToOwned` 11, `vec` 13, `Box` 9, `format` 6, `BTreeSet` 5, `Vec` 1, `String`/`ToString` 5) — all
`alloc::*` imports that std's prelude makes redundant, in a crate the node builds **with** std.

PR #207 gates 49 such imports with `#[cfg(not(feature = "std"))]` (keeping them for the no_std
consumers) and splits one braced import because this toolchain rejects attributes on nested use-tree
items. The dead `LiveX3VmAdapter::transport()` accessor was removed too.

| Gate | Result |
| --- | --- |
| `cargo clippy -p x3-atomic-swap --all-targets --features std -- -D warnings` | exit 0 (was 52 errors) |
| `cargo check -p x3-atomic-swap --no-default-features` | exit 0 |
| `cargo test -p x3-atomic-swap --features std` | 764 passed / 0 failed |
| `SKIP_WASM_BUILD=1 cargo check --workspace` | exit 0 |

## Round 8 (master 9d5a5127b): persistent cargo target dir for live gates

Every live-gate run rebuilt the workspace because `actions/checkout` cleans the workspace at the start of
each job, destroying any in-tree build cache. The SVM workflow's `rust-cache` step also covered only the
`programs/svm/...` workspaces, never the root workspace that builds `x3-chain-node` and the runtime WASM.

PR #208 points `CARGO_TARGET_DIR` at `${{ github.workspace }}/../x3-cargo-target` (a sibling of the
checkout, so it survives the clean and is shared across jobs/commits) for the three live workflows —
job-level for X3-native and both EVM jobs, step-level for the SVM cross-domain tests only, so the SBF and
client builds keep their own target paths.

Verification status: placement verified by parsing the workflows; the speed-up can only be measured on a
run that uses the new definition (queued runs for older commits still use the old one), and the first such
run is cold by design. Durations to be reported next.

## Round 9 (master baa3fc387): the release gate's own job had never executed

```
JOB native-x3vm-live / real local-node X3VM lifecycles: completed/success
JOB gate: completed/skipped
```

`production-gate.yml`'s `gate` job (`make guard`, `make test-all-pallets`, srtool build,
`make mainnet-check`) declares `needs: [native-x3vm-live, evm-live, svm-live]`, but only the first call
was ever scheduled, so the job was skipped and the run concluded failure. The distinguishing factor was
the `concurrency:` block: `x3vm-live-lifecycle.yml` (no block) always ran; the EVM and SVM workflows (with
a block) never did, because a workflow-level concurrency group applies to the reusable call too and
collided with the standalone push run of the same workflow/commit.

PR #209 removes those two blocks. Verified: production-gate run `35309656263` schedules
`native-x3vm-live`, `evm-live / real X3VM to Anvil lock claim lifecycle`,
`evm-live / EVM HTLC live anvil lifecycle` and `svm-live / SVM HTLC live validator lifecycle` — three of
those four jobs appear for the first time in any recorded run.

## Round 10 (master c3300a517): lint gate green locally; release gate dispatchable

The repository's declared lint gate passes in every configuration the workflow uses (measured on
`baa3fc387`, `SKIP_WASM_BUILD=1`, escalated so cargo could unpack dev-deps into `~/.cargo`):

| Command (from `rust-clippy.yml`) | Result |
| --- | --- |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `-p x3-chain-runtime --all-targets --no-default-features --features std,mainnet-rc1` | PASS |
| `-p x3-chain-node --all-targets --features mainnet-rc1` | PASS |
| `-p x3-chain-node --all-targets --features mainnet-rc1,try-runtime` | PASS |
| `-p x3-chain-node --all-targets --features mainnet-rc1,runtime-benchmarks` | PASS |
| `-p x3-chain-node --all-targets --features mainnet-rc1,gpu-validator` | PASS |

This corrects an earlier assumption of a large lint backlog: the `--features std` failures fixed in
PR #207 were the real debt, and the rest of the failures I had seen were environmental (read-only
`~/.cargo` inside the sandbox).

PR #210 adds `workflow_dispatch` to `production-gate.yml`; `gh workflow run production-gate.yml --ref
master` now creates run `35310867732` instead of returning HTTP 422.

## Round 11 (master 4fa8b5cc5): local CI made first-class

| Change | Evidence |
| --- | --- |
| Second runner `x3star2` registered and running | both runners `online busy=true`; queue depth 10 → 7; it took the queued `clippy` job within seconds |
| `make guard` hang fixed | was hanging forever in `re.search` on binary ParityDB tables; now `[agent_guard] ok` in 27s (was 4m03s once binaries were skipped naively), `make guard` ~20s |
| `scripts/local-ci.sh` | runs 13 fast gates (+ `--live`, `--cross`), per-gate logs under `.ai/runlogs/`, summary table, non-zero exit on failure; verified format 10s, guards 35s, readiness 17s, workspace check 121s PASS |
| rustfmt | `cargo fmt --all -- --check` was failing with 123 diffs / 26 files; now exits 0 |

`.rc4-runtime-upgrade-work/` holds **475 tracked files / 238 MB** of generated ParityDB chain data — the
input that broke the guard. Flagged for an explicit removal decision rather than deleted silently.

## Round 12 (master 792bf07c8): local CI reachable from make; runner survives reboot

| Change | Evidence |
| --- | --- |
| `make local-ci` / `-live` / `-cross` / `-list` | wraps `scripts/local-ci.sh`; `make local-ci-list` prints the gate list, exit 0 (PR #213) |
| `x3star2` reboot persistence | crontab `@reboot` entry added (no sudo for a service unit here); reversible via `crontab -e` |
| `make mainnet-check` first local run | section 1 (documentation) ✓; section 2 is `cargo build --release -p x3-chain-node`, which exceeded 12 minutes — the earlier 900 s timeout killed it mid-build, so the full run continues detached in `.ai/runlogs/mainnet-check-detached.log` |

Both runners remain busy and the queue sits around 7 while the long gates run.

### Capacity observation

Two runners executing heavy Rust gates *plus* a local `make mainnet-check` release build pushed the machine
to a **load average of 54-67 on 32 cores**. The local CI's bottleneck is now CPU/re-build volume, not runner
count: further improvement should come from cache reuse (per-runner persistent `CARGO_TARGET_DIR`, #208),
scoped triggers, and smaller release builds — not from adding a third runner.

## Round 13 (master 3e0d4ba68): in-flight halt contract

`economic_halt_does_not_block_inflight_bundle_assignment` (PR #214) closes the untested half of the halt
contract: while halted, an already-submitted bundle can still be assigned an executor and rolled back with
its bond released, while new submissions stay refused. `atomic_kernel.required_tests` now holds eight real
test functions, verified by the readiness-consistency script.

Queue trend with two runners: 10 → 7 → 4. The release build inside `make mainnet-check` is progressing
(runtime then node rlib), so the release gate's own script is still mid-flight locally.

## Round 14 (master 1d8965916): the release gate passes locally

`make mainnet-check` → **`✅ mainnet_release_gate: PASS`** with readiness consistency PASS — 18 checks, 0
failures. This is the first complete execution of the release gate's own script anywhere.

```
1. Required documentation .............. ✓
2. Build (node + runtime WASM) ......... ✓  target/release/x3-chain-node
                                           target/release/wbuild/.../x3_chain_runtime.compact.compressed.wasm
3. Chain-spec / genesis artifacts ...... ✓  x3-local3-current-{plain,raw}.json, production_config()
4. Critical suites ..................... ✓  x3-chain-runtime, supply-ledger, packet-standard,
                                           bridge, fees, pallet-x3-slash
5. Reproducible-build prerequisites .... ✓  srtool installed, docker available, no SKIP_WASM_BUILD
6. Forbidden secrets scan .............. ✓
```

The sole blocker was environmental: `srtool` was not installed (section 5). Installing the pinned revision
CI uses (`cargo install --locked --git https://github.com/chevdor/srtool-cli --rev 0485b55... srtool-cli`)
closed it. Wired in as `make local-ci-release` / `scripts/local-ci.sh --release`.

## Round 15 (master 5e233e510): migration dry-run per runtime variant

`scripts/check-runtime-variants.sh` runs the new in-process rehearsal
(`runtime_upgrade_rehearsal`: `AllPalletsWithSystem` + `Migrations` hooks, weight must fit a block) once per
variant. First run:

| Variant | Result |
| --- | --- |
| full | **PASS** (42s) |
| frontier | **PASS** (88s) |
| mainnet-rc1 | **PASS** (66s) |
| testnet | **PASS** (39s) |
| dev | **FAIL** — test target does not compile |
| dev+frontier | **FAIL** — same |

The dev variants' tests have never compiled (modules gated `#[cfg(all(test, feature = "dev"))]`, mock drifted
from the SDK: no `MaxHolds` on `pallet_balances::Config`, `Test` out of scope). Also found:
`runtime/src/tests.rs` is an orphan file — never declared as a module — whose 267 lines include five tests
with empty bodies.

## Round 16 (master 504fe41d0): dev variants repaired, 6/6 dry-runs pass

Repaired `runtime/src/fraud_proofs/pallet.rs` (half-finished `Test`→`Runtime` rename, `pallet_balances`
member drift, `dev_accounts`, `ConstU32` BlockHashCount, missing assert macros, eleven self-referential
`disputed.scheduler_commitment` reads) so the dev-gated test modules compile:

| Gate | Result |
| --- | --- |
| `scripts/check-runtime-variants.sh` | **all six variants PASS** (full 49s, dev 13s, dev+frontier 11s, frontier 29s, mainnet-rc1 16s, testnet 11s) |
| dev suite | **80 passed / 0 failed** |
| dev+frontier suite | **84 passed / 0 failed** |

### Mistake recorded

`gh pr create` returned #219 while I merged **#218** — another agent's open PR
(`fix/bridge-evm-transfer-content-verification`, `5a6af9e3e`) — and its merge commit carries my subject
line. That PR is a real security fix: both EVM verifiers defaulted `require_erc20_transfer = false`, so
`verify_evm_transfer_proof` accepted any successfully-included Ethereum transaction as proof for an
arbitrary bridge request. Verified after the fact: `cargo test --workspace` in `x3-lang/` → 439 passed / 0
failed. The process failure stands: merged before verification instead of after.

## CI reality check (2026-09-18, master 123dc8f52)

- Hosted jobs still fail in 2-4s with **zero steps** (account billing lock): `production-gate`,
  `mainnet-readiness`, `Rust Clippy`, `Trivy`, `Semgrep`, `OSV-Scan`, `feature-matrix`.
- The `x3` self-hosted runner (`x3star1`, labels `self-hosted,Linux,X64,x3`) is the only executor, and it
  has **no usable sudo**. Its `Install system dependencies` step failed with
  `sudo: a terminal is required to read the password` even though `protoc`/`clang`/`cmake` were already
  installed — so the job died at step 3 and every live lifecycle behind it was skipped.
- **After PR #201**: `x3vm-svm-live-lifecycle` run `35303169753` → **success**, all 17 steps, including
  `Run real solana-test-validator HTLC lifecycle gate`, `Run SVM program + client unit tests`,
  `Build SBF program and broadcaster`, `Run real X3VM-SVM atomic lifecycle`,
  `Run real X3VM-SVM timeout/refund atomic lifecycle`.
- **After PR #202** (routing + idempotent deps): master `123dc8f52` triggered
  `x3vm-evm-live-lifecycle` (run `35304527539`; `real X3VM to Anvil lock claim lifecycle` in flight past
  Foundry install, AtlasHTLC build, Anvil start) and the previously untriggerable
  `x3vm-live-lifecycle` (queued behind it on the single runner).

## CI root cause

```
gh api repos/Cyptopimpinainteazy/xxxstar/check-runs/<id>/annotations
→ "The job was not started because your account is locked due to a billing issue."
```

Every GitHub-hosted job (40 of 43 workflows, including `ci.yml`, `production-gate.yml`,
`rust-clippy.yml`, `mainnet-readiness.yml`, Trivy/Semgrep/OSV/CodeQL) fails in 2–3 seconds with
zero steps on **every** branch, master included. Self-hosted jobs are unaffected.

## Open risk inventory observed this pass

- Dependabot open alerts on default branch: 30 total (12 high, 18 medium); high set is
  `libp2p-quic`, `libp2p-gossipsub` (x2), `yamux`, `rustls-webpki`, `hickory-proto`.
- Workflow labels with no registered runner: `[self-hosted, x3-benchmark]`,
  `[self-hosted, linux, gpu]`.
- `crates/cross-vm-coordinator` is excluded from the root workspace and gated by nothing in CI.

## Round 17 (master 5fb9f1af5): PR #216 verified then landed; #220 wires the dry-run in

`#216` (parallel agent, trading-core-v1 audit blindness) was verified on its own branch first — x3-lang
**441 passed / 0 failed**, clippy exit 0 — then merged; master's merge commit `c2693dd6d` lists the tested
SHA `0b379c4b4` as a parent. `#220` adds `--variants` / `make local-ci-variants` so the six-variant
migration dry-run is reachable from the local suite.

Merge procedure now: take the number from `gh pr create` output, confirm the branch with
`gh pr view <n> --json headRefName`, merge, then verify the merge commit's parents include the tested SHA.
