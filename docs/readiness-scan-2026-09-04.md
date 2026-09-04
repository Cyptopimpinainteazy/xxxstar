# X3 Monorepo — AGENTS.md Required-Proof Readiness Scan

- **Date:** 2026-09-04 (America/Denver)
- **Scope:** Read-only project-wide scan of `/home/lojak/Desktop/xxxstar-main` against the AGENTS.md required-proof gate.
- **Host:** x3star1 · Linux 6.8.0-139-generic x86_64 · 32 cores · 109 GiB RAM · cargo 1.90.0 (rust-toolchain pin) · node v22.23.2, npm present, **pnpm NOT installed**, system `python3` 3.10.12 (no pytest), repo `.venv` has pytest 9.1.1.
- **Guardrails honored:** No source files modified. Did not touch the write-in-progress testnet artifacts. Only repo write is this report. Target/ side effects only. Nothing committed.

## Verdict summary

The workspace is **not clean** against the AGENTS required-proof gate today. `cargo check --workspace`
PASSES cleanly (0 errors). The clippy gate FAILS (style lints escalated by `-D warnings`, dominated by
`#[cfg(test)]` code in three crates plus four library-source lints). `cargo test --workspace` FAILS with rc=101
on one e2e test (spawned release node SIGKILL'd — env/resource-dependent), halting the run before later packages.
The JS `npm test` gate is red in 6 of 10 targets (root causes vary: 1 reproducible off-by-one, 2 jest TS-transform gaps, 1 empty suite, 1 racy boundary + stale-dist pollution).
`pnpm test` / `pnpm build` are **not runnable
as written** because pnpm is not installed and there is no pnpm-lock.yaml. `python -m pytest` is not
runnable as written (`python` absent on PATH; system python3 has no pytest). Full details and per-target
evidence follow.

## Per-target result table

| # | AGENTS command | Result | Evidence |
|---|---|---|---|
| 1 | `cargo check --workspace` | **PASS** | Finished in 3m21s, 0 errors. Only warning: future-incompat on dependency `uint v0.4.1` (upstream). See log lines below. |
| 2 | `cargo test --workspace` | **FAIL** (rc=101) | Build succeeded and earlier binaries ran, but the e2e package failed: `tests/e2e/cross_vm_real_chain_test` → `test_cross_vm_rpc_methods_present` FAILED (5 passed / 1 failed). Its spawned `target/release/x3-chain-node` was SIGKILL'd before RPC ready (env/resource). cargo aborts remaining packages at the first failing binary, so later crate tests were not reached. See note below row. |
| 3 | `cargo clippy --workspace --all-targets -- -D warnings` | **FAIL** | Non-zero. 170+ lint escalations; 3 crates reported "(lib test) due to N previous errors". Zero hard E-code type errors — all are clippy lints promoted by `-D warnings`. See breakdown. |
| 4 | `pnpm test` | **EXCEPTION** | `pnpm` binary not installed; no `pnpm-lock.yaml`. Root lock = package-lock.json; workflows that use `pnpm --dir` cannot resolve. |
| 5 | `pnpm build` | **EXCEPTION** | Same as above (pnpm missing). |
| 6 | `npm test` | **FAIL** | 10 fragments of the root `npm test` chain; 4 passed, 6 failed (details below). |
| 7 | `python -m pytest` | **EXCEPTION** | `python` not on PATH; `/usr/bin/python3 -m pytest` → "No module named pytest". Repo `.venv/bin/python -m pytest` (pytest 9.1.1) exists but top-level `tests/` collection already errors on imports (`swarm.db` not importable from repo root); see sample. |
| 8 | fake-code scan (`grep -RIn ...`) | **PASS (no genuine placeholders), noisy** | Literal command = 55,358 hits, but ~41.5k are inside untracked/ignored trees (`vendor/` 31.9k, `.venv` 9.6k). Git-tracked source = 12,979 hits, dominated by docs/tooling/"mock" token. Zero genuine `todo!`/`unimplemented!`/`panic!("not implemented"` in production crates/pallets/runtime/node (`tools/launchops` matches are a scanner regex, not placeholders). |
| 9 (bonus) | deployment-keys placeholder check (mainnet-readiness gate) | **PASS** | `bootnode-keys.json` + `bootnode-node-key` contain `REPLACED_RUN_KEY_ROTATION_SCRIPT=FILL_IN_…` markers only → match the CI "placeholder-only" grep. `cargo-audit` / `trufflehog` binaries are NOT installed on host (offline), so those CI jobs could not be reproduced locally. |

## 1) cargo check --workspace — PASS

Command run exactly (via nohup, `--color=never` appended only):

```
$ cargo check --workspace
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 21s
warning: the following packages contain code that will be rejected by a future version of Rust: uint v0.4.1
note: to see what the problems were, use the option `--future-incompat-report`, or run `cargo report future-incompatibilities --id 1`
```

- `grep -c '^error' cargo_check_ws.log` → **0**.
- The single warning is an upstream dependency (`uint v0.4.1`) future-incompat notice; `rustc`/toolchain warning, not this repo's code.
- Note: `cargo check` does **not** build `#[cfg(test)]` targets, so it does not exercise test code (see clippy/test rows).

## 3) cargo clippy --workspace --all-targets -- -D warnings — FAIL

Command run exactly:

```
$ cargo clippy --workspace --all-targets --color=never -- -D warnings
error: could not compile `pallet-x3-proof-carrying-agent` (lib test) due to 8 previous errors
error: could not compile `x3-foundry-core` (lib test) due to 1 previous error
error: could not compile `pallet-x3-kernel` (lib test) due to 160 previous errors
```

Zero hard rustc E-code errors (`grep -cE '^error\[E[0-9]{4}\]'` → 0). All 170+ entries are **lint
escalations** promoted to error by `-D warnings`. Breakdown of distinct messages:

```
  101 useless use of `vec!`
   48 the borrowed expression implements the required traits
    6 unused `std::result::Result` that must be used
    3 using `clone` on type `H256` which implements the `Copy` trait
    1 you should consider adding a `Default` implementation for `InMemoryRegistry`
    1 you should consider adding a `Default` implementation for `InMemoryPendingStore`
    1 you should consider adding a `Default` implementation for `InMemoryMappingStore`
    1 used `unwrap()` on `Ok` value
    1 unused variable: `svm_bytes`  |  1 unused variable: `evm_bytes`
    1 type alias `Test` is never used |  1 struct `MockDispatcher` is never constructed
    1 this `if` has identical blocks |  …(3 more single-class items)
```

Location breakdown (via `-->` refs in the clippy log):

```
  123 pallets/x3-kernel/src/tests.rs
   29 pallets/x3-kernel/src/tests/property_tests.rs
    8 pallets/pallet-x3-proof-carrying-agent/src/tests.rs
    3 pallets/x3-kernel/src/packet_integration_tests.rs
    2 pallets/x3-kernel/src/mock.rs
    1 pallets/x3-kernel/src/registry.rs        <- library source
    1 pallets/x3-kernel/src/pending_transfer.rs  <- library source
    1 pallets/x3-kernel/src/mapping.rs           <- library source
    1 crates/x3-foundry-core/src/error.rs        <- library source
```

Takeaways:
- ~165/170 findings are in **`#[cfg(test)]` code** (x3-kernel tests + property tests + proof-carrying-agent
  tests + kernel test-mock), driven overwhelmingly by `useless use of vec!` (101) and needless_borrow (48).
- **Four are in library source (real, not test):** `Default` impls missing on `InMemoryRegistry`,
  `InMemoryPendingStore`, `InMemoryMappingStore` (x3-kernel) and an `unwrap() on Ok` in
  `x3-foundry-core/src/error.rs`.
- These are clippy lint findings, not compile/type errors, so `cargo test` (plain rustc) may still compile the
  same code — see cargo-test row.
- CI intersection: `full-ci.yml` and `rust-clippy.yml` run the same `clippy --workspace --all-targets`
  (`rust-clippy.yml` also `--all-features --tests`), so CI would also go red on master unless these are cleared.

## 2) cargo test --workspace

Run exactly (background, `--color=never`): `cargo test --workspace`. Substrate graph compiled then ran:

```
test node_binary_prefers_release_build ... ok
... (other tests ok)
test test_cross_vm_rpc_methods_present ... FAILED
test result: FAILED. 5 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 14.73s
error: test failed, to rerun pass `-p e2e_tests --test cross_vm_real_chain_test`
cargo test rc=101
```

Root cause of the single failure: `tests/e2e/cross_vm_real_chain_test.rs:111` spawns
`target/release/x3-chain-node --dev --tmp --validator --rpc-port 9944`, which started (DB opened, AUTHORITY
role) but then exited with **`signal: 9 (SIGKILL)`** before the RPC port opened — consistent with resource
pressure / OOM-killer / supervisor-kill while the machine also ran the clippy/test web builds, not with a code
assertion failure (the same binary's other 5 e2e tests passed: connect, extrinsic, block-production/finality,
ws). This is an environment-dependent gate failure and must be re-run on an idle host. Because cargo aborts at the
first failing package, pallet/crate tests ordered after e2e were not executed this run. A blocking `--no-fail-fast`
re-run (or excluding the `tests/e2e` member) is needed for a full per-crate tally. Verdict: the workspace test
gate is RED on this host (rc=101) due to this test.

## 6) npm test — FAIL (4/10 pass)

The root `package.json` `test` script chains 11 fragments with `npm --prefix` (and `pnpm --dir` for
apps/x3-studio). Each was run independently to record per-target exit codes:

| npm-test fragment | rc | Verdict | Evidence |
|---|---|---|---|
| apps/inferstructor-dashboard (`-- --run`) | 0 | **PASS** | 8 files, **61 tests passed** |
| packages/blockchain-connector | 0 | **PASS** | 8 test files passed |
| packages/polkawallet-bridge-adapter (`-- --runInBand`) | 0 | **PASS** | 1 suite / **4 passed** |
| packages/ts-sdk (`-- --runInBand`) | 0 | **PASS** | 8 passed + 1 skipped suites / **185 passed, 2 skipped** |
| apps/shared (`-- --runInBand`) | 1 | **FAIL** | jest cannot parse TS → `config/__tests__/chain.test.ts` jsx/TS annotation token; no jest config and no ts-jest declared → babel transform. See actionable #A4. |
| apps/wallet (`-- --runInBand`) | 1 | **FAIL** | jest cannot parse TS → `src/__tests__/x3-types.test.ts:62` `(status: AgentStatus)` annotation. ts-jest IS installed (`node_modules/.bin/ts-jest`) but **no jest.config*** wired to it → default babel-jest. See actionable #A3. |
| apps/x3-desktop | 1 | **FAIL (deterministic, 2/2)** | `tests/arena.test.ts` "should cap at 30 blocks": got 31, expected ≤30; 1 failed / 26 passed. Real off-by-one in BlockStore (see actionable #A1). |
| packages/atomic-swap-sdk | 1 | **FAIL** | 2 failed / 23 passed + 3 "failed" files. Two boundary tests racy at second granularity (`calculateTimeLocks` vs test `Date.now()`), plus stale compiled `dist/**/__tests__/*.test.js` are matched by jest default and error (no jest config, no `testPathIgnorePatterns`/`testMatch`). See actionable #A5. |
| packages/polkawallet-plugin (`-- --runInBand`) | 1 | **FAIL** | TS compile error `src/config/env.ts:72` `getEnv('X3_RPC_ENDPOINT', undefined)` — `undefined` not assignable to `string`. See actionable #A6. |
| packages/x3-foundry-sdk (`-- --runInBand`) | 1 | **FAIL** | jest "No tests found… 9 files checked, 0 matches" — package ships zero test files, so the documented test command fails (`--passWithNoTests` needed). Also fails its build (see § build). |
| apps/x3-studio test | n/a | **EXCEPTION** | Root invokes `pnpm --dir apps/x3-studio test` → pnpm missing; not run. |

### Build gate (root `npm run build` / `pnpm build`)

`pnpm` is required by the root build script for `apps/x3-studio`; pnpm is absent, so the documented root build
can't run end to end here. Representative pure-`tsc` package builds (the low-risk slice that *can* run):

| build target | rc | evidence |
|---|---|---|
| packages/blockchain-adapter | 0 | PASS |
| packages/polkawallet-bridge-adapter | 0 | PASS |
| packages/blockchain-connector | 0 | PASS |
| packages/atomic-swap-sdk | 0 | PASS |
| packages/ts-sdk | 0 | PASS |
| packages/x3-foundry-sdk | 2 | **FAIL** — 7 TS errors in `src/`: `client.ts:164,205` `string|undefined`→`string`; `deploy.ts/index.ts/revenue.ts` `BigNumberish` declared in `./types` but **not exported**; unused locals/imports in `index.ts/revenue.ts/templates.ts`. See actionable #A7. |

`apps/*` (Next.js `next build`, Vite content builds, tauri desktop) were not executed for this scan: they need
full dependency trees and network/platform tooling inconsistent with a read-only readiness pass; treat as
"not executed — enormous/env-bound" rather than claimed green.

## 7) python -m pytest

- `python` → not a command (`/usr/bin/python3` only). `python3 -m pytest` → `No module named pytest`.
- Repo-local `.venv/bin/python -m pytest --version` → pytest 9.1.1 is present in `.venv`.
- But even via `.venv`, top-level collection fails immediately: `tests/test_json_import.py` does
  `from swarm.db import SessionLocal` → `ModuleNotFoundError: No module named 'swarm.db'` (import path /
  env not set at repo root). 109 pytest Python test files exist (mostly `tests/`, `swarm_infrastructure/`,
  `scripts/`), several GPU/substrate-dependent.
- Verdict: the literal AGENTS `python -m pytest` is **EXCEPTION (not runnable as written)**; automated python
  testing on this host requires activating `.venv` and fixing collection imports; out of scope for read-only scan.

## 8) Fake-code scan

Literal command ran (55,358 matches, grep rc=2 partly from unreadable/huge files). Composition:

- Top contributors: `vendor/` **31,915** (0 files git-tracked, ignored), `.venv/` **9,584** (0 tracked),
  then tracked `apps/` 4,090 / `crates/` 914 / `pallets/` 572 / `packages/` 261, docs/tooling (`.x3`,
  `.launchops`, `.toolchain`, `launch-gates`, `reports`, `docs`) contributing thousands more, plus
  `forge-std/` 667 etc.
- Git-tracked source subset (`git ls-files`, code extensions): **12,979 matches**, token mix:
  `mock` 6,528 · `TODO` 2,377 · `stub` 1,471 · `dummy` 1,306 · `placeholder` 1,302 · `fake` 1,291 ·
  `FIXME` 578 · `unimplemented!` 399 · `todo!` 88 · `not implemented` 24.
  The word "mock" legitimately dominates (mocks in tests are permitted by AGENTS).
- **Genuine hard placeholders in production Rust:** 0. A targeted grep for `todo!|unimplemented!()|
  panic!("not implemented|not implemented yet|unreachable!("not implemented` over
  `crates pallets runtime node primitives integration-tests tools` (excluding test specs) returned only
  `tools/launchops/{test_weaken.rs,verify.rs}` — those are a **test-weakening scanner** whose regexes
  *look for* `unimplemented!`/`todo!`, not placeholder code. No real `unimplemented!`/`todo!` remains in
  production paths.
- Verdict: the raw grep-anywhere command is dominated by untracked/vendor noise; on real git-tracked source
  there are **no genuine no-op/placeholder code paths** to report as defects (TODO/FIXME are mainly benign
  slogans/comments; reported separately as counts only, not violations).

## CI workflow inventory (release / mainnet-readiness gates) — not run

Workflows found under `.github/workflows/` (44 files). Gate-relevant ones:

| Workflow | Enforces (jobs/steps) |
|---|---|
| `full-ci.yml` | secret-scan (trufflehog), cargo-audit, cargo-deny, **fmt `--check`**, **`clippy --workspace --all-targets -- -D warnings`**, **`cargo test --workspace --all-targets`**, proof-gates, swarm CI (`cargo test -p x3-swarm-core`). Mirrors AGENTS gate. |
| `mainnet-readiness.yml` | trufflehog filesystem secret scan; **deployment/keys placeholder check** (`REPLACED_RUN_KEY_ROTATION_SCRIPT`/`REPLACE_ME` grep — **passes locally**); cargo-audit; cargo-deny; `make mainnet-check`; `make fresh-machine-check` (release node build + all-pallet tests); produces release artifact + `release_hashes.txt`. |
| `production-gate.yml` | `make guard`, `make test-all-pallets`, `make audit`, `make test-node-build`, `make mainnet-check`. |
| `build.yml` | fmt `--check`, `clippy --all --all-targets --all-features -- -D warnings`, release build `-p x3-chain-node`, coverage, security-audit, fuzz, advanced-test gate. |
| `ci.yml` (x3 critical-path) | fmt, `cargo check -p x3-chain-runtime --features std`, `cargo check -p x3-chain-node` (default + testnet), cross-vm-router tests, supply-ledger tests, per-pallet gates. |
| `rust-clippy.yml` | `clippy --workspace --all-targets --all-features -- -D warnings` and `--tests` variant (stricter than local default-feature run). |
| `testnet-deploy.yml` | release `x3-chain-node --features testnet` build+test, spec generation, readiness report, staging deploy, EVM verifier + contract deploy. (Testnet infra lane — separately tracked by the other session.) |
| `v04-ship-gate.yml` | v0.4 crates: fmt/check/clippy(-D warnings)+lib test, property tests (`x3-packet-standard`, `x3-ixl`), cross-vm-router full+scope-freeze suites, sidecar, mainnet_rc1 E2E launch-blocker. |
| `release-hardening.yml` | `cargo build --release -p x3-chain-node`, SBOM (cyclonedx), `cargo build --release -p x3-chain-runtime --features std`. |
| `release-provenance.yml` | release build + version/provenance metadata. |
| `proof-gates.yml` / `repo-scanner.yml` / scanners (codeql, osv, semgrep, trivy, snyk, security-dashboard) | automated proof verification + scanning layers. |
| others | benchmark-regression, frame-benchmarking, markdown-autodocs, docs-consistency, formal-verification, economic-attack, try-runtime-upgrade, zombienet, x3-lang-readiness, x3-desktop-ci, x3fronend-gpu, swarm-tps-gpu-soak, deploy-dashboard, test-integrity. |

**Mainnet-readiness release-gate jobs that would veto a release today:** (the clippy `-D warnings` failure in
`full-ci.yml`/`rust-clippy.yml` blocks; `build.yml`/`full-ci` cargo-test blocks; production-gate/`make
mainnet-check`, fresh-machine-check and the v0.4 ship gate are separate and were not executed locally.
Dependency audit jobs (cargo-audit/cargo-deny) and trufflehog need installed binaries/network — not run here.)

## 9/10) Actionable violations (real, reproducible code defects)

- **A1 (P1, code) — x3-desktop block store off-by-one keeps 31 blocks.** `apps/x3-desktop/src/blockchain/BlockStore.ts:64`
  `recentBlocks: [...state.recentBlocks.slice(-30), block] // keep last 30` slices the prior 30 **then appends**,
  so the cap is really 31. `apps/x3-desktop/tests/arena.test.ts` "should cap at 30 blocks" fails deterministically
  (2/2 runs, got 31, expected ≤ 30).
- **A2 (P1, JS toolchain) — jest cannot transform TypeScript in apps/shared and apps/wallet.**
  `apps/shared` has no jest config and **no TS transform declared** (jest + babel only), yet ships `.ts`/`.tsx`
  tests → `chain.test.ts` fails to parse; `apps/wallet` installs `ts-jest` but defines **no jest config**, so
  jest falls back to babel and rejects type annotations in `x3-types.test.ts`. Contrast: the 3 passing packages
  (ts-sdk, polkawallet-plugin, polkawallet-bridge-adapter) each carry an explicit `jest.config.js`.
  Fix: add `jest.config` (preset ts-jest / swc) or **transformIgnore** to those two apps.
- **A3 (P1, package) — packages/x3-foundry-sdk is broken both ways.** `npm test` → jest "No tests found"
  (zero test files; command exits 1); `npm run build` → 7 TS errors: `client.ts:164,205` `string|undefined`
  not assignable to `string`; `./types` declares `BigNumberish` **without exporting** it (used by deploy/index/
  revenue); unused imports/locals (`Template`, `PaginatedResponse`, `PaginationParams`, `DAppType`) flagged as errors.
- **A4 (P2, palette) — packages/atomic-swap-sdk suite is not green.** (a) Two boundary assertions are
  second-granularity racy: `calculateTimeLocks(3600)` computes `counterpartyTimeLock = nowFn+3600`, but the test
  recomputes `now = Date.now()` afterward and asserts `> now+3600`, failing when both land in the same second
  (`expected greater than 1788535318 … got 1788535318`). (b) Stale compiled tests under `dist/**/__tests__/`
  are executed by jest default because the package has no jest config / `testPathIgnorePatterns` for `dist`.
- **A5 (P2, clippy-src) — four library-source clippy findings that -D warnings flags.**
  `pallet-x3-kernel/{registry,pending_transfer,mapping}.rs`: `InMemoryRegistry/PendingStore/MappingStore` lack
  `Default` impls (new_without_default); `crates/x3-foundry-core/src/error.rs`: `unwrap()` on a known-`Ok` value.
- **A6 (P2, test hygiene) — clippy -D warnings fails on test code.** ~165 findings live in
  `#[cfg(test)]`: `pallet-x3-kernel/src/tests.rs`(123) + `tests/property_tests.rs`(29) +
  `packet_integration_tests.rs`(3) + `src/mock.rs`(2) + `pallet-x3-proof-carrying-agent/src/tests.rs`(8).
  Top drivers: `useless use of vec!` ×101 and needless borrow ×48. No hard type errors — pure style lints
  escalated by `-D warnings`. Blocking canonical clippy gate on master.
- **A7 (P1? P2, root/orchestration) — documented JS gate commands are not runnable as spec'd on a stock host.**
  `pnpm` is required by root `test`/`build` scripts and `apps/x3-studio` (`pnpm --dir …`) and by
  `packageManager: pnpm@10.15.1`, but pnpm is not installed and there is **no pnpm-lock.yaml** committed
  (only package-lock.json) → `pnpm test`/`pnpm build` cannot run + the lockfile is missing for the declared PM.
- **A8 (env) — `python -m pytest` not runnable.** `python` absent; system `python3` lacks pytest; repo relies
  on `.venv` (pytest 9.1.1) but top-level collection errors on `swarm.db` import. Documented gate assumes an
  activated venv + import path that isn't in place at the repo root.

(Common-but-benign "TODO/FIXME in comments & doc slogans" plus tooling dirs `.x3/.launchops/launch-gates/
.reports` are counted in scan but **not** listed as violations.)

## Environment / blockers to report

- No `pnpm`; no system `python`; system python3 w/o pytest; no `cargo-audit`; no `trufflehog` (~offline).
- `cargo test --workspace` gate is RED on this host (rc=101, single e2e SIGKILL failure); on a quiet host re-run
  expected to clear per clippy's 0-hard-error finding, but that must be evidenced, not assumed.
- `apps/*` heavy web/desktop builds (next/vite/tauri) not executed (env-bound) — reported not-green only where a
  runnable equivalent (`package build`) failed (x3-foundry-sdk).

---

## Completion Report (AGENTS.md-required)

**Files changed (this task):**
- `docs/readiness-scan-2026-09-04.md` (this report — the only repo write; new file).
- No source, crate, pallet, docs-maintained, or other files changed. Nothing committed. Testnet-in-progress
  files untouched.

**Commands run:** (see per-target table; representative set reproduced here)
- `cargo check --workspace` → PASS (0 errors, 3m21s).
- `cargo clippy --workspace --all-targets -- -D warnings` → FAIL (lint escalations; see row 3).
- `cargo test --workspace` → FAIL rc=101 (build OK; single e2e failure `cross_vm_real_chain_test::test_cross_vm_rpc_methods_present`, node SIGKILL).
- `pnpm test` / `pnpm build` → EXCEPTION (pnpm not installed; no pnpm-lock.yaml).
- `npm test` (11-fragment root chain run per-fragment) → 4 PASS / 6 FAIL / 1 EXCEPTION(pnpm).
- Representative `tsc` package builds → 5 PASS / 1 FAIL (x3-foundry-sdk).
- `python -m pytest` → EXCEPTION (`python` absent, pytest absent, collection import error).
- fake-code scan (literal) + git-tracked subset + targeted production-directory scan.
- Deployment/keys placeholder check (bonus) → PASS. cargo-audit/trufflehog: not installed.

**Proof result:** cargo check clean (the single green Rust gate). Rust clippy gate red on test-code style lints
(+4 lib-src clippy findings). `cargo test --workspace` FAILED rc=101 (1 e2e test; node SIGKILL env-dependent).
JS npm-test gate red (4/10 npm-test fragments pass). x3-foundry-sdk red on
both test and build. pnpm & python gates unrunnable as written on this host. No genuine production placeholders/
todo!/unimplemented! in Rust source. Mainnet secret/placeholder key check green.

**Remaining blockers:**
1. cargo test workspace gate RED on this host: 1 e2e failure (SIGKILL) → re-run clean + capture full per-crate tally.
2. pnpm + python tooling absent (gate commands unrunnable as spec'd regardless of repo state).
3. ci clippy -D warnings failure (test-code lints + 4 lib-src) — needs clear or allowance.
4. JS gates red in 6 of 10 npm-test fragments + x3-foundry-sdk build.

**Next 10 tasks:**
1. Re-run `cargo test --workspace --no-fail-fast` on an idle host → complete per-crate pass/fail tally + confirm e2e SIGKILL was env-only.
2. Fix `BlockStore` cap off-by-one (slice then append → cap at 30) so `arena.test.ts` passes deterministically.
3. Add `jest.config` (ts-jest/swc preset) to `apps/shared` and `apps/wallet`; remove `ts-jest` install-only state.
4. Fix `packages/x3-foundry-sdk` TS errors (export `BigNumberish`/`string|undefined`/dead imports) or align tsconfig.
5. Add/fix jest config in `packages/atomic-swap-sdk` to ignore stale `dist/**`; make boundary assertions non-racy.
6. Fix `InMemoryRegistry/PendingStore/MappingStore` `Default` impls + `x3-foundry-core` unnecessary unwrap.
7. Triage 165 test-code clippy lints (x3-kernel tests/property_tests, proof-carrying-agent) — cleanup not allow.
8. Resolve pnpm-vs-npm split: install pnpm via corepack + commit pnpm-lock, or convert JS gates to npm-only.
9. Reproducible `python` test story: activate `.venv`, set `PYTHONPATH`/project root, fix `swarm.db` import in `tests/`.
10. Re-run the full AGENTS gate chain to a single green proof; record final readiness doc.

**Completion percent (scan):** scan/inventory complete (100%). Aggregate gate verdict RED/partial pending the above violations and a clean-host cargo-test revalidation.
