#!/usr/bin/env bash
# Local CI — run the gates that actually execute, on this machine.
#
# GitHub-hosted CI for this repository is blocked (account billing lock), so the
# self-hosted runner and this script are the only places these gates run. Every
# gate below is a real command with a real exit code; nothing is swallowed.
#
# Usage:
#   scripts/local-ci.sh                 # fast: format, guards, check, lint, unit tests
#   scripts/local-ci.sh --live          # + the two self-contained contract gates (anvil/SVM)
#   scripts/local-ci.sh --cross         # + the X3-native and X3VM<->EVM/SVM cross-domain lifecycles
#   scripts/local-ci.sh --release       # + the release gate (make mainnet-check)
#   scripts/local-ci.sh --variants      # + the runtime migration dry-run for all six variants
#   scripts/local-ci.sh --loom          # + the loom model checks (needs the pinned nightly)
#   scripts/local-ci.sh --failure       # + the validator failure drill (boots and kills validators)
#   scripts/local-ci.sh --testnet       # + the testnet ceremony drill (records and verifies a launch)
#   scripts/local-ci.sh --soak          # + a 10-minute consensus soak (MINUTES= to change it)
#   scripts/local-ci.sh --rotation      # + the validator key rotation drill
#   scripts/local-ci.sh --deep          # + the whole workspace test suite (slow, ~5-15 min)
#   scripts/local-ci.sh --all           # everything
#   scripts/local-ci.sh --list          # show the gate list without running it
#
# Scheduling and scoping (added for the pre-push hook and for triage):
#   --jobs N          run N gates at once (default 3, or X3_LOCAL_CI_JOBS)
#   --cargo-jobs N    CARGO_BUILD_JOBS for every gate (default 10, or the env var)
#   --only a,b        run exactly these gate slugs (see --list)
#   --skip a,b        drop these gate slugs; the summary records the skip loudly
#   --dry-run         print the gate list that would run, then exit
#   --fail-fast       stop scheduling new gates once one has failed
#   --changed-from R  add gates implied by the files changed in R...HEAD
#   --pre-push        the push-time set: fast gates + diff-scoped gates, plus the
#                     release/variant gates when pushing the default branch
#
# Results are written to .ai/runlogs/local-ci-<timestamp>.log (aggregate), one
# local-ci-<timestamp>-<slug>.log per gate, and a machine-readable
# local-ci-<timestamp>-summary.json. Exit status is non-zero if any gate failed,
# 2 on a usage error, 3 when the whole run was skipped by X3_LOCAL_CI_SKIP_ALL=1.
#
# A gate that could not even start because the environment lacks something
# (crates.io / github.com unreachable while cargo resolves dependencies, or
# this box rewriting ~/.rustup / ~/.cargo / a target-dir file out from under a
# running build — see #330) is reported as BLOCKED, not FAIL — and BLOCKED
# still fails the run, because a gate that did not execute has verified
# nothing. It is also not a code diagnostic: do not read a BLOCKED gate as
# "the change under test broke something."
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Resolve the Rust toolchain defensively. This box has repeatedly lost
# `~/.cargo/bin` (the rustup shims) *during* a session — once it took `srtool`
# and the registry cache with it, and at 2026-09-18T07:36Z it briefly removed
# `cargo` itself, which made twenty gates "fail" in 0s with
# `cargo: command not found`. Prefer a full toolchain (one that ships rustfmt)
# over the shim directory, and refuse to run rather than emit misleading reds.
if ! command -v cargo >/dev/null 2>&1; then
  for candidate in "$HOME"/.rustup/toolchains/*/bin "$HOME/.cargo/bin"; do
    if [ -x "$candidate/cargo" ] && [ -x "$candidate/rustfmt" ]; then
      PATH="$candidate:$PATH"
      export PATH
      echo "local-ci: note: cargo was not on PATH; using $candidate" >&2
      break
    fi
  done
fi
# Put the *real* toolchain directory ahead of the rustup shims. The shims are
# symlinks to `~/.cargo/bin/rustup`, and this box deletes that binary
# mid-session: the symlinks then dangle, `cargo`/`rustc` resolve to a path that
# does not exist, and every cargo gate dies with
#   `could not execute process rustc ... (never executed) No such file or directory`
# even though a perfectly good toolchain sits under ~/.rustup. Prefer the
# pinned toolchain (rust-toolchain.toml), then any complete one, then the shims.
PINNED_CHANNEL="$(sed -n 's/^[[:space:]]*channel[[:space:]]*=[[:space:]]*"\(.*\)".*/\1/p' "$ROOT/rust-toolchain.toml" 2>/dev/null | head -1)"
TOOLCHAIN_BIN=""
for d in "$HOME"/.rustup/toolchains/"$PINNED_CHANNEL"*/bin "$HOME"/.rustup/toolchains/*/bin; do
  if [ -x "$d/cargo" ] && [ -x "$d/rustc" ] && [ -x "$d/rustfmt" ]; then
    TOOLCHAIN_BIN="$d"
    break
  fi
done
if [ -n "$TOOLCHAIN_BIN" ]; then
  export PATH="$TOOLCHAIN_BIN:$PATH"
  # Pin the compiler by absolute path as well: cargo looks `rustc` up on PATH
  # for every crate it compiles, and a PATH lookup that lands on a shim the box
  # is in the middle of replacing reports
  #   `could not execute process rustc --crate-name ... (never executed)`
  # with no path at all. RUSTC removes that lookup.
  export RUSTC="$TOOLCHAIN_BIN/rustc"
  export CARGO="$TOOLCHAIN_BIN/cargo"
elif ! command -v cargo >/dev/null 2>&1; then
  echo "local-ci: cargo is not on PATH and no toolchain was found under" >&2
  echo "local-ci: ~/.rustup/toolchains/*/bin or ~/.cargo/bin — nothing can be" >&2
  echo "local-ci: verified. Restore the toolchain and re-run." >&2
  exit 2
else
  export PATH="${HOME}/.cargo/bin:${PATH}"
fi

# `openssl-sys` resolves whatever OpenSSL the package manager advertises first.
# On this box that is Homebrew's, whose libcrypto references glibc 2.38 symbols
# the host cannot link against, so any gate whose crates link OpenSSL dies with
#   rust-lld: error: undefined reference: __isoc23_strtol@GLIBC_2.38
# and the failure reads like a defect in the change under test. Use the system
# OpenSSL when the environment has not chosen one and one is installed; a
# machine that wants its own still sets OPENSSL_DIR and this does nothing.
if [ -z "${OPENSSL_DIR:-}" ] && [ -f /usr/include/openssl/ssl.h ] && [ -d /usr/lib/x86_64-linux-gnu ]; then
  export OPENSSL_DIR=/usr
  export OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu
  export OPENSSL_INCLUDE_DIR=/usr/include
fi

RUN_LIVE=0
RUN_CROSS=0
RUN_RELEASE=0
RUN_VARIANTS=0
RUN_DEEP=0
RUN_LOOM=0
RUN_FAILURE=0
RUN_TESTNET=0
RUN_SOAK=0
RUN_ROTATION=0
RUN_PREPUSH=0
LIST_ONLY=0
DRY_RUN=0
FAIL_FAST=0
JOBS="${X3_LOCAL_CI_JOBS:-3}"
CARGO_JOBS="${CARGO_BUILD_JOBS:-10}"
ONLY=""
SKIP="${X3_LOCAL_CI_SKIP:-}"
CHANGED_FROM=""

usage() { sed -n '2,32p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; }

while [ "$#" -gt 0 ]; do
  case "$1" in
    --live) RUN_LIVE=1 ;;
    --cross) RUN_LIVE=1; RUN_CROSS=1 ;;
    --release) RUN_RELEASE=1 ;;
    --variants) RUN_VARIANTS=1 ;;
    --failure) RUN_FAILURE=1 ;;
    --testnet) RUN_TESTNET=1 ;;
    --soak) RUN_SOAK=1 ;;
    --rotation) RUN_ROTATION=1 ;;
    --deep) RUN_DEEP=1 ;;
    --all) RUN_LIVE=1; RUN_CROSS=1; RUN_RELEASE=1; RUN_VARIANTS=1; RUN_DEEP=1; RUN_LOOM=1; RUN_FAILURE=1; RUN_TESTNET=1; RUN_SOAK=1; RUN_ROTATION=1 ;;
    --loom) RUN_LOOM=1 ;;
    --pre-push) RUN_PREPUSH=1 ;;
    --list) LIST_ONLY=1 ;;
    --dry-run) DRY_RUN=1 ;;
    --fail-fast) FAIL_FAST=1 ;;
    --jobs) JOBS="${2:-}"; shift ;;
    --cargo-jobs) CARGO_JOBS="${2:-}"; shift ;;
    --only) ONLY="${2:-}"; shift ;;
    --skip) SKIP="${SKIP:+$SKIP,}${2:-}"; shift ;;
    --changed-from) CHANGED_FROM="${2:-}"; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

case "$JOBS" in ''|*[!0-9]*) echo "local-ci: --jobs must be a positive integer" >&2; exit 2 ;; esac
case "$CARGO_JOBS" in ''|*[!0-9]*) echo "local-ci: --cargo-jobs must be a positive integer" >&2; exit 2 ;; esac

if [ "${X3_LOCAL_CI_SKIP_ALL:-0}" = "1" ]; then
  echo "local-ci: SKIPPED — X3_LOCAL_CI_SKIP_ALL=1 was set for this invocation."
  echo "local-ci: nothing was verified. Re-run without it before trusting the result."
  exit 3
fi

STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
LOG_DIR="$ROOT/.ai/runlogs"
mkdir -p "$LOG_DIR"
LOG="$LOG_DIR/local-ci-$STAMP.log"
PROGRESS="$LOG_DIR/local-ci-$STAMP-progress.log"
: >"$PROGRESS"

declare -a GATE_NAMES=() GATE_STATUS=() GATE_SECS=() GATE_LOGS=() GATE_REASONS=()
FAILED=0

declare -A SEEN_SLUG=()

# ── the repository's own gates ─────────────────────────────────────────────
GATES_FAST=(
  "format check:cargo fmt --all -- --check"
  "agent guards:make guard"
  "make gate exit codes:make check-make-gates"
  "script syntax:bash scripts/check-script-syntax.sh"
  "workflow wiring:python3 scripts/check_ci_workflow_refs.py --parity"
  "workspace membership:python3 scripts/check-workspace-membership.py"
  # PHASE 43 is a prohibition, not a feature: a value that becomes a balance, a reward, a slash or a
  # settlement amount must be computed exactly. `x3-lang` has had this check for its own two crates for
  # a while (`compiler/tests/test_computation_discipline.rs`); the *root* workspace's consensus surface
  # was never scanned, which is how "no native float in a consensus-sensitive path" came to be a claim
  # about one directory (TICKET-094). Measured when the gate was added: `pallets/*/src` and
  # `runtime/src` contain zero `f64`/`f32`, so this is a line held, not a line drawn — the scan refuses
  # the next one, and a legitimate float states `// float-exemption: <reason>` on its own line.
  "no float in consensus:python3 scripts/check-no-float-in-consensus.py"
  # A `Cargo.lock` resolved by hand during a merge has to be re-checked by cargo. It
  # was not: the x3-swap-router merge kept the branch's package entry verbatim, where
  # its sole `sp-std` is the 14.0.0 one while the merged graph has two, so the bare
  # name was ambiguous and cargo rewrote it — `cargo metadata --locked` exited 101
  # ("the lock file needs to be updated but --locked was passed") on master, which is
  # what every `--locked` gate in CI would have reported (TICKET-062). Measured both
  # ways: exit 0 on a consistent lock, and exit 101 after adding a dependency to one
  # member's manifest without touching the lock.
  "cargo lockfile locked:cargo metadata --locked --format-version 1"
  # Every id in `tests/invariants/registry.toml` must be referenced by a test.
  # This check lived in `tests_core/invariant_registry_check.rs`, which no crate,
  # script or Makefile ever compiled — so an invariant could be registered with
  # nothing checking it and the tree stayed green.
  "invariant registry:python3 scripts/check-invariant-registry.py"
  "test integrity diff:python3 scripts/test_cheat_guard.py --base ${X3_LOCAL_CI_BASE:-origin/master}"
  "readiness consistency:bash scripts/check-readiness-consistency.sh"
  "economic model:bash scripts/mainnet/x3_economic_model_gate.sh --quiet"
  # A `migrations.rs` whose crate never declares the module is uncompiled,
  # untested dead weight that looks exactly like the real thing. Five such files
  # were sitting under pallets when this gate was added.
  "migration modules wired:bash scripts/ci/check_migration_modules_are_wired.sh"
  # The snapshot format's whole job is to refuse bad input, so the refusals are
  # demonstrated on disk against a real export rather than only in unit tests:
  # corrupt/truncated/missing chunks, substituted state under a forged but
  # self-consistent manifest, wrong block, wrong chain, wrong runtime, stale.
  "snapshot murder test:bash launch-gates/snapshot-murder-test.sh"
  # `scripts/feature_matrix.py check` validates the granular readiness manifest:
  # every declared path and evidence path has to exist, scores have to respect the
  # per-priority caps, and the enums have to hold. It takes a tenth of a second and
  # was in no gate list, so it had drifted into four errors: two matrix rows cited
  # hosted workflows (`.github/workflows/pr-supervisor.yml`,
  # `.github/workflows/rust-clippy.yml`) that this repository does not contain. A
  # readiness manifest nothing validates is a claim, not a record.
  # A paid RPC key or a signing key committed to source is a live credential for everyone who can
  # read the repository, and removing it does not un-expose it. This scans `git ls-files` and prints
  # file, line and pattern only — never the matched value, which would leak it into the CI log.
  "provider secret guard:bash scripts/check-no-provider-secrets.sh"
  "feature matrix check:python3 scripts/feature_matrix.py check"
  # `make mainnet-check` stage 6b rebuilds the runtime and fails when it no
  # longer matches `docs/reports/runtime-wasm-hashes.json`. That is ten minutes
  # into a release, and runtime changes have landed without the record three
  # times in a day, so this is the cheap early warning: if the outgoing diff
  # touches any package in the runtime's dependency graph, the record has to
  # move in the same change (`./scripts/update-runtime-hashes.sh`).
  "runtime hash freshness:python3 scripts/check-runtime-hash-freshness.py --base ${X3_LOCAL_CI_BASE:-origin/master}"
  "workspace check:env SKIP_WASM_BUILD=1 cargo check --workspace"
  "clippy workspace:cargo clippy --workspace --all-targets -- -D warnings"
  "clippy runtime rc1:cargo clippy -p x3-chain-runtime --all-targets --no-default-features --features std,mainnet-rc1 -- -D warnings"
  "clippy node rc1:cargo clippy -p x3-chain-node --all-targets --features mainnet-rc1 -- -D warnings"
  "test x3-lang:cargo test --manifest-path x3-lang/Cargo.toml"
  # The Python half of `make test` (parser, typechecker, mocked e2e). Nothing else
  # in this gate set runs pytest, so without this line the x3-lang Python suites
  # were invisible to the CI of record.
  "test x3-lang python:pytest -q x3-lang/tests/test_parser.py x3-lang/tests/test_typechecker.py x3-lang/tests/test_e2e_mocked.py"
  "test atomic-kernel:cargo test -p pallet-x3-atomic-kernel"
  "test atomic-swap std:cargo test -p x3-atomic-swap --features std"
  "test settlement-engine:cargo test -p pallet-x3-settlement-engine"
  # The snapshot format is the trust boundary for state sync: a mirror must not
  # be able to alter metadata or a chunk without the verifier noticing. Its
  # murder-test matrix (wrong chain, stale, wrong state root, corrupt /
  # reordered / missing chunk) is cheap, so it belongs in the gate set of record.
  "test state snapshot:cargo test -p x3-state-snapshot"
  # No SKIP_WASM_BUILD here on purpose: the service tests boot a real node whose
  # chain spec is decoded by the *embedded* runtime, so the runtime WASM must be
  # built for this feature set. `SKIP_WASM_BUILD=1` used to embed whatever blob
  # sat in target/<profile>/wbuild/ (once a mainnet-rc1 blob against a full-variant
  # genesis), which the node then rejected at boot. runtime/build.rs now refuses a
  # cache built for another feature set, so this gate has to build it.
  "test node:env -u SKIP_WASM_BUILD cargo test -p x3-chain-node"
  # This crate is deliberately outside the workspace (its own lockfile, its own
  # dependency graph), so without `--offline --locked` cargo re-resolves against
  # crates.io/github every run and the gate reports BLOCKED whenever the network
  # is unavailable. `crates/cross-vm-coordinator/Cargo.lock` is committed for
  # exactly this reason: `cargo fetch --locked --manifest-path ...` once, then
  # this gate is deterministic and offline. If the cache is ever cold the gate
  # still fails loudly rather than passing quietly.
  # `--locked` keeps the versions pinned; the fetch is only there because this
  # box keeps losing ~/.cargo (shims, srtool and the registry cache have all
  # disappeared mid-session). Offline, the fetch fails and the test still runs
  # against whatever cache exists - cold cache surfaces as BLOCKED, never green.
  "test cross-vm-coordinator:cargo fetch --locked --manifest-path crates/cross-vm-coordinator/Cargo.toml || echo 'local-ci: coordinator dependency fetch failed (offline?); running against the existing cache'; cargo test --offline --locked --manifest-path crates/cross-vm-coordinator/Cargo.toml"
  # The crates below are `exclude`d from the root workspace: each declares its
  # own `[workspace]` (or path-depends on one that does), and cargo refuses to
  # have them as members ("multiple workspace roots found in the same
  # workspace"). `cargo check --workspace` therefore never sees them, and
  # nothing else did either: that is how `x3-solvency-sidecar` stopped
  # compiling against tungstenite 0.29, and how `services/x3-swarm-api` rotted
  # against `x3-swarm-core`'s API (a removed re-export and a `String` field that
  # had become an enum) without a single red gate. `--locked` keeps their
  # committed lockfiles honest.
  # `crates/dylint-determinism` is excluded too but needs a nightly toolchain
  # (`#![feature(rustc_private)]`), so it is not part of this gate yet.
  # Each crate gets its own target dir: they resolve different versions of the
  # same dependencies (x3-sidecar still pins reqwest 0.11, the root workspace is
  # on 0.12), and sharing the harness target dir with the main build produced
  # `could not parse/generate dep info ... No such file or directory`.
  #
  # `crates/x3-sidecar` is checked without `--all-targets`: its test targets do
  # not compile (78 errors — tests reference `wiremock`, `uuid`, a `x3_rpc`
  # crate and `axum::Server`, none of which the manifest declares or which
  # axum 0.7 still has). The library and binary are covered; its tests are a
  # recorded follow-up rather than something this gate pretends to check.
  # `crates/x3-sidecar` runs its tests too. Until this change its manifest
  # declared no `[lib]` target at all, so the eleven-module library (and the e2e
  # test that links it) was never compiled; now that it is, the gate checks and
  # runs every target. `SKIP_WASM_BUILD=1` because the library pulls `x3-rpc` for
  # its types, which pulls the runtime — this check needs the types, not the
  # embedded blob, and the blob is built by the root workspace gates anyway.
  "nested workspaces:for d in crates/x3-swarm-core services/x3-swarm-api services/x3-swarm-worker services/x3-solvency-sidecar; do echo \"== \$d\"; CARGO_TARGET_DIR=\"/tmp/x3-nested-\$(basename \"\$d\")\" cargo check --locked --all-targets --manifest-path \"\$d/Cargo.toml\" || exit 1; done; echo '== crates/x3-sidecar (all targets; runtime wasm skipped)'; SKIP_WASM_BUILD=1 CARGO_TARGET_DIR=/tmp/x3-nested-x3-sidecar cargo test --locked --all-targets --manifest-path crates/x3-sidecar/Cargo.toml || exit 1"
  # `AGENTS.md` asks for `pnpm test`, `pnpm build` and `npm test` before any
  # task is called complete. Nothing in this harness ran any of them, and
  # nothing else did either: **412 tests across twelve JS projects** had never
  # executed. There is no workspace lockfile — the root package.json drives 21
  # independent `npm --prefix` projects — so each one installs from its own
  # committed lockfile when `node_modules` is missing, then runs its suite.
  # Offline, the install fails and the gate reports BLOCKED rather than green.
  # Tests only, deliberately: `npm run build` passes, but it writes into
  # **tracked** output (12 deleted + 4 modified files under the committed
  # `dist/` directories of apps/x3-desktop, packages/polkawallet-plugin and
  # packages/atomic-swap-sdk), so builds need their own tier.
  "js sdk tests:for d in packages/ts-sdk packages/atomic-swap-sdk packages/blockchain-connector packages/x3-foundry-sdk packages/polkawallet-bridge-adapter packages/polkawallet-plugin apps/shared apps/wallet apps/inferstructor-dashboard apps/x3-desktop tests/wallet-integration; do echo \"== \$d\"; ( cd \"\$d\" && { [ -d node_modules ] || npm ci --no-audit --no-fund --prefer-offline; } && npm test ) || exit 1; done; echo '== apps/x3-studio (pnpm)'; ( cd apps/x3-studio && { [ -d node_modules ] || corepack pnpm install --prefer-offline; } && corepack pnpm test ) || exit 1"
  # Two configurations of the proof-verification router, because the `--deep`
  # workspace build unifies `test-verifier` through the gateway pallet's
  # dev-dependency and would therefore never exercise the fail-closed posture.
  # Default features = production posture: every strategy without a real
  # verifier must refuse the proof.
  "test verification router:cargo test -p x3-verification-router"
  # The opt-in permissive path that the plumbing tests use.
  "test verification router test-verifier:cargo test -p x3-verification-router --features test-verifier"
)

# Gates that boot real chains (anvil / solana-test-validator / x3 dev node).
GATES_LIVE=(
  # Boots a dev chain and checks that it authors, finalizes, and answers its own
  # operator CLI over RPC. Self-contained: no anvil, no solana, no network.
  "local node smoke:bash scripts/local-node-smoke.sh"
  # Three validators on the built-in `local3` chain: they have to find each
  # other, finalize, and agree on the canonical hash at a finalized height.
  "local network smoke:bash scripts/local-network-smoke.sh"
  "EVM contract lifecycle:X3-contracts/evm/test-live-lifecycle.sh"
  # `cargo build-sbf` runs `cargo +1.89.0-sbpf-solana-v1.54 …` internally, and
  # `+toolchain` only works through the rustup shim — which this script puts
  # behind the pinned toolchain directory on purpose. Put the shim back in front
  # for this gate, or the build dies with "no such command: `+…`".
  "SVM contract lifecycle:env PATH=\"$HOME/.cargo/bin:$PATH\" programs/svm/x3_atomic_swap/test-live-lifecycle.sh"
)

GATES_VARIANTS=(
  "runtime variant dry-runs:bash scripts/check-runtime-variants.sh"
  # `cargo check --workspace` builds every crate *with* its default features, so a crate
  # whose own declaration says it can be `no_std` and cannot be stays invisible. TICKET-082
  # measured one of those at 244 errors, and behind it TICKET-092: a
  # `#[cfg(any(test, feature = "std"))]` around an Ed25519 check with no `else`, so the
  # runtime's wasm configuration counted a validator's stake without verifying anything.
  # The gate derives its list from the `no_std` posture itself and checks each crate
  # **alone**, because cargo unifies features across a graph — a single invocation with
  # every crate passed to it passed with TICKET-082 deliberately reintroduced.
  "no-default-features crates:bash scripts/check-no-default-features.sh"
)

GATES_RELEASE=(
  "release gate (mainnet-check):make mainnet-check"
)

# The loom model checks live outside the workspace (`tests/loom-concurrency` is
# excluded: it is `#![cfg(loom)]` and loom needs a nightly). Opt-in, because it
# needs a toolchain the default run does not: without it the gate reports
# BLOCKED, which is the honest answer — nothing was verified.
GATES_LOOM=(
  "loom concurrency tests:bash scripts/run-loom-tests.sh"
)

# What the consensus network does when validators die. Opt-in and separate from
# `--live` because it boots four to seven validators, kills a minority and then a
# supermajority-breaking number, and restarts them: finality must continue in the
# first case, must stop in the second (that is the safety property, and a chain
# that keeps finalizing below 2/3 has none), and must resume afterwards. Ten to
# fifteen minutes and seven processes, so it does not belong in a default run.
GATES_FAILURE=(
  "validator failure drill:bash scripts/testnet/validator-failure-drill.sh"
)

# What a published testnet needs beyond "it starts": a record of *what* was launched
# (spec hash, node hash, genesis hash, authorities, runtime version, peer ids) that a
# running network can be checked against — and a verifier that has been seen to fail.
# Opt-in: it builds a spec, boots four validators, records the manifest, verifies it,
# then tampers with a copy and requires the verifier to reject it.
GATES_TESTNET=(
  "testnet ceremony drill:bash scripts/testnet/testnet-ceremony-drill.sh"
  # The other half of "a testnet can be brought up": a chain that starts with its Bitcoin
  # trust root already pinned. Builds a spec carrying a real regtest header (captured from
  # a Bitcoin node by `scripts/btc/capture-regtest-spv.py`), boots a node from it, reads
  # `BtcCheckpoints` / `BtcHeaderMetaStore` / `BtcBestHeight` back over RPC, requires the
  # chain to keep authoring, and requires a spec whose checkpoint is not a mined header to
  # be refused. Needs `--features dev`: only that runtime's `powLimit` is regtest's.
  "btc checkpoint genesis:bash scripts/testnet/btc-checkpoint-drill.sh"
  # The other half of the Bitcoin path: real headers from a real Bitcoin node pushed onto an
  # anchored chain, in order, with a gapped push refused. Needs a Bitcoin Core install
  # (X3_BITCOIND_DIR) and the dev runtime; without Bitcoin Core it skips and says so.
  "btc header push:bash scripts/testnet/btc-header-push-drill.sh"
)

# Consensus has to last, not just start. This boots four validators and samples them on
# an interval for `MINUTES` (default 10): it fails on a stall longer than a minute, on a
# validator that dies, on disagreement at sampled heights, and on a node whose RSS grows
# past a gigabyte; it writes a per-validator report either way. Long soaks are an
# operator run — set MINUTES=60 for a real duration check.
GATES_SOAK=(
  "consensus soak:bash scripts/testnet/consensus-soak.sh"
)

# Validator key rotation is operator-driven, and the on-chain custody registry is
# the single source of truth. This drills the `validator rotate` command against a
# live node: an unregistered account must be refused, and a registered account must
# produce a signed `session.set_keys` extrinsic and a next-due block. It never
# submits unless the drill itself is passed `--submit`.
GATES_ROTATION=(
  "validator rotation drill:bash scripts/testnet/validator-rotation-drill.sh"
)

# The broadest automated signal the repository has: every test target in every
# workspace member. SLOW (thousands of tests), so it is opt-in rather than part
# of the default set. No SKIP_WASM_BUILD: x3-chain-node's service tests boot a
# real node whose chain spec is decoded by the embedded runtime.
GATES_DEEP=(
  "test workspace:env -u SKIP_WASM_BUILD cargo test --workspace"
)

# The cross-domain gates were listed in `describe_all` for a long time without
# being runnable: the three lifecycles below boot a chain each, and the two
# X3VM<->EVM / X3VM<->SVM tests are `#[ignore]`d in the source because they need
# one. Their pass evidence lived only in `.ai/runlogs` (which is gitignored) and
# in CI history, so "the cross-domain leg is proven" was a claim about a past
# run, not something a reader could reproduce with one command. The two scripts
# now supply anvil + AtlasHTLC and solana-test-validator + the SBF program, so
# they are gates like any other.
GATES_CROSS=(
  "X3-native lifecycles:env -u SKIP_WASM_BUILD cargo test -p x3-chain-node --test x3vm_live_lifecycle -- --ignored --nocapture --test-threads=1"
  "cross-domain EVM:bash scripts/cross-domain-evm-gate.sh"
  # Same reason as the `SVM contract lifecycle` gate above: this one also runs
  # `cargo build-sbf`, which shells out to `cargo +1.89.0-sbpf-solana-v1.54` and
  # therefore needs the rustup shim ahead of the pinned toolchain directory.
  "cross-domain SVM:env PATH=\"$HOME/.cargo/bin:$PATH\" bash scripts/cross-domain-svm-gate.sh"
  # The three gates above boot the dev chain, whose genesis allows unattested
  # cross-domain proof sets. That is a dev-only posture; this one runs the same
  # X3VM<->EVM lifecycles against a dev spec with that policy flipped, and the
  # test refuses to start unless the chain reports the strict value.
  "cross-domain EVM (strict posture):env X3_STRICT_CROSS_DOMAIN_PROOFS=1 bash scripts/cross-domain-evm-gate.sh"
  # The SVM twin of the gate above, and it was missing: `cross-domain-svm-gate.sh`
  # has honoured `X3_STRICT_CROSS_DOMAIN_PROOFS` since it was written, but nothing
  # in this list ever set it, so the SVM leg was only ever proven in the dev
  # posture that allows unattested proof sets. Same posture, same refusal-to-start
  # check as the EVM gate.
  "cross-domain SVM (strict posture):env X3_STRICT_CROSS_DOMAIN_PROOFS=1 PATH=\"$HOME/.cargo/bin:$PATH\" bash scripts/cross-domain-svm-gate.sh"
)

# Slugs are the `--only`/`--skip` keys, so keep them lowercase: gate names carry
# acronyms ("EVM contract lifecycle") that nobody types in caps.
slugify() { printf '%s' "$1" | tr '[:upper:] ' '[:lower:]-'; }

describe_all() {
  echo "fast gates (default):"
  printf '  - %s\n' "${GATES_FAST[@]%%:*}"
  echo "live gates (--live):"
  printf '  - %s\n' "${GATES_LIVE[@]%%:*}"
  echo "cross-domain gates (--cross):"
  printf '  - %s\n' "${GATES_CROSS[@]%%:*}"
  echo "release gate (--release):"
  printf '  - %s\n' "${GATES_RELEASE[@]%%:*}"
  echo "runtime variants (--variants):"
  printf '  - %s\n' "${GATES_VARIANTS[@]%%:*}"
  echo "loom gates (--loom, also implied by --all):"
  printf '  - %s\n' "${GATES_LOOM[@]%%:*}"
  echo
  echo "failure drills (--failure, also implied by --all):"
  printf '  - %s\n' "${GATES_FAILURE[@]%%:*}"
  echo
  echo "testnet bring-up (--testnet, also implied by --all):"
  printf '  - %s\n' "${GATES_TESTNET[@]%%:*}"
  echo
  echo "soak (--soak, also implied by --all; MINUTES= to change the window):"
  printf '  - %s\n' "${GATES_SOAK[@]%%:*}"
  echo
  echo "validator key rotation (--rotation, also implied by --all):"
  printf '  - %s\n' "${GATES_ROTATION[@]%%:*}"
  echo "deep gates (--deep, also implied by --all):"
  printf '  - %s\n' "${GATES_DEEP[@]%%:*}"
  echo "scheduling: --jobs N --cargo-jobs N --only a,b --skip a,b --changed-from R --pre-push --dry-run --fail-fast"
}

if [ "$LIST_ONLY" = 1 ]; then
  describe_all
  exit 0
fi

# ── what the outgoing diff implies ─────────────────────────────────────────
BASE_REF="${X3_LOCAL_CI_BASE:-origin/master}"
git rev-parse --verify --quiet "${BASE_REF}^{commit}" >/dev/null || BASE_REF="HEAD"

NOTES=()
TOUCHED=()

collect_touched() {
  local base="$1"
  {
    git diff --name-only "$base...HEAD" 2>/dev/null
    git diff --name-only HEAD 2>/dev/null
  } | sort -u
}

if [ "$RUN_PREPUSH" = 1 ] || [ -n "$CHANGED_FROM" ]; then
  DIFF_BASE="${CHANGED_FROM:-$BASE_REF}"
  if git rev-parse --verify --quiet "${DIFF_BASE}^{commit}" >/dev/null; then
    mapfile -t TOUCHED < <(collect_touched "$DIFF_BASE")
    for path in "${TOUCHED[@]}"; do
      case "$path" in
        X3-contracts/evm/*|X3-contracts/svm/*)
          RUN_LIVE=1
          NOTES+=("$path changed -> live contract gates added (--live)")
          ;;
        runtime/*)
          RUN_VARIANTS=1
          NOTES+=("$path changed -> runtime variant dry-runs added (--variants)")
          ;;
      esac
    done
    NOTES+=("diff vs $DIFF_BASE touched ${#TOUCHED[@]} file(s)")
  else
    NOTES+=("changed-from ref $DIFF_BASE does not resolve; diff scoping skipped")
  fi
fi

PUSH_REF="${X3_LOCAL_CI_PUSH_REF:-}"
if [ "$RUN_PREPUSH" = 1 ] && { [ "$PUSH_REF" = "refs/heads/master" ] || [ "$PUSH_REF" = "refs/heads/main" ]; }; then
  RUN_RELEASE=1
  RUN_VARIANTS=1
  NOTES+=("push to $PUSH_REF -> release + variant gates added")
fi

# ── select gates ───────────────────────────────────────────────────────────
SELECTED=()
for spec in "${GATES_FAST[@]}"; do SELECTED+=("$spec"); done
[ "$RUN_LIVE" = 1 ] && for spec in "${GATES_LIVE[@]}"; do SELECTED+=("$spec"); done
[ "$RUN_CROSS" = 1 ] && for spec in "${GATES_CROSS[@]}"; do SELECTED+=("$spec"); done
[ "$RUN_VARIANTS" = 1 ] && for spec in "${GATES_VARIANTS[@]}"; do SELECTED+=("$spec"); done
[ "$RUN_RELEASE" = 1 ] && for spec in "${GATES_RELEASE[@]}"; do SELECTED+=("$spec"); done
[ "$RUN_FAILURE" = 1 ] && for spec in "${GATES_FAILURE[@]}"; do SELECTED+=("$spec"); done
[ "$RUN_TESTNET" = 1 ] && for spec in "${GATES_TESTNET[@]}"; do SELECTED+=("$spec"); done
[ "$RUN_SOAK" = 1 ] && for spec in "${GATES_SOAK[@]}"; do SELECTED+=("$spec"); done
[ "$RUN_ROTATION" = 1 ] && for spec in "${GATES_ROTATION[@]}"; do SELECTED+=("$spec"); done
[ "$RUN_LOOM" = 1 ] && for spec in "${GATES_LOOM[@]}"; do SELECTED+=("$spec"); done
[ "$RUN_DEEP" = 1 ] && for spec in "${GATES_DEEP[@]}"; do SELECTED+=("$spec"); done

if [ -n "$ONLY" ]; then
  IFS=',' read -r -a ONLY_LIST <<<"$ONLY"
  FILTERED=()
  UNMATCHED=()
  for spec in "${SELECTED[@]}"; do
    name="${spec%%:*}"; slug="$(slugify "$name")"
    for want in "${ONLY_LIST[@]}"; do
      [ "$slug" = "$want" ] && FILTERED+=("$spec") && break
    done
  done
  # A name that matches nothing is a typo, not a no-op: saying nothing would let
  # an operator believe a gate ran when it never did.
  for want in "${ONLY_LIST[@]}"; do
    matched=0
    for spec in "${SELECTED[@]}"; do
      [ "$(slugify "${spec%%:*}")" = "$want" ] && matched=1
    done
    [ "$matched" = 0 ] && UNMATCHED+=("$want")
  done
  if [ "${#UNMATCHED[@]}" -gt 0 ]; then
    echo "local-ci: --only names gate(s) that are not in the selected set: ${UNMATCHED[*]}" >&2
    echo "local-ci: see --list for the slugs" >&2
    exit 2
  fi
  if [ "${#FILTERED[@]}" -eq 0 ]; then
    echo "local-ci: --only matched no gate; see --list" >&2
    exit 2
  fi
  SELECTED=("${FILTERED[@]}")
fi

if [ -n "$SKIP" ]; then
  IFS=',' read -r -a SKIP_LIST <<<"$SKIP"
  KEPT=()
  for spec in "${SELECTED[@]}"; do
    name="${spec%%:*}"; slug="$(slugify "$name")"
    drop=0
    for skipper in "${SKIP_LIST[@]}"; do
      [ "$slug" = "$skipper" ] && drop=1
    done
    if [ "$drop" = 1 ]; then
      NOTES+=("SKIPPED BY REQUEST: $slug")
    else
      KEPT+=("$spec")
    fi
  done
  SELECTED=("${KEPT[@]}")
fi

if [ "${#SELECTED[@]}" -eq 0 ]; then
  echo "local-ci: no gates selected" >&2
  exit 2
fi

BRANCH="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)"
HEAD_SHA="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
DIRTY="clean"
[ -n "$(git status --porcelain 2>/dev/null | head -n 1)" ] && DIRTY="dirty"

# An externally-shared CARGO_TARGET_DIR (the default is this worktree's own
# $ROOT/target, which is always safe) is a known trap: cargo's mtime-based
# freshness check can be fooled when the same target dir is reused across
# worktrees at different revisions (#330) — it can skip recompiling a crate
# whose source actually changed, or replay a stale build's cached
# warnings/panics, in either direction (false green or false red). Stamp the
# target dir with the worktree + revision it was last built for, and force a
# freshness re-check on mismatch instead of trusting the cache.
if [ -n "${CARGO_TARGET_DIR:-}" ] && [ "$CARGO_TARGET_DIR" != "$ROOT/target" ]; then
  mkdir -p "$CARGO_TARGET_DIR"
  TARGET_DIR_MARKER="$CARGO_TARGET_DIR/.local-ci-last-revision"
  TARGET_DIR_STAMP="$ROOT@$HEAD_SHA"
  if [ -f "$TARGET_DIR_MARKER" ] && [ "$(cat "$TARGET_DIR_MARKER" 2>/dev/null)" != "$TARGET_DIR_STAMP" ]; then
    echo "local-ci: WARNING: CARGO_TARGET_DIR=$CARGO_TARGET_DIR was last built for"
    echo "local-ci:          $(cat "$TARGET_DIR_MARKER"), this run is $TARGET_DIR_STAMP."
    echo "local-ci:          Reusing a target dir across worktrees/revisions can make"
    echo "local-ci:          cargo trust a stale dep-info cache instead of rebuilding"
    echo "local-ci:          (#330) — touching every tracked file so cargo re-checks"
    echo "local-ci:          freshness instead of trusting it."
    # Some tracked paths (submodule placeholders, toolchain download stubs)
    # are not materialized in every checkout — a failed touch on those is
    # harmless (nothing to mark fresh), so stderr is discarded rather than
    # letting a wall of "No such file or directory" bury the real warning.
    git ls-files -z 2>/dev/null | xargs -0 -r touch 2>/dev/null
  fi
  printf '%s' "$TARGET_DIR_STAMP" >"$TARGET_DIR_MARKER"
fi

echo "local-ci $STAMP — root=$ROOT"
echo "local-ci: ${#SELECTED[@]} gate(s), jobs=$JOBS, cargo-jobs=$CARGO_JOBS, $BRANCH@$HEAD_SHA ($DIRTY)"
for note in "${NOTES[@]:-}"; do [ -n "$note" ] && echo "local-ci: note: $note"; done
echo "local-ci: prereqs: $(for tool in cargo python3 node docker srtool; do if command -v "$tool" >/dev/null 2>&1; then printf '%s=ok ' "$tool"; else printf '%s=MISSING ' "$tool"; fi; done)"
command -v srtool >/dev/null 2>&1 || cat <<'EOF'
local-ci: note: srtool missing -> `--release` / `make mainnet-check` fails on its
local-ci:       reproducibility section. This box has lost the binary more than
local-ci:       once (something rewrites ~/.cargo/bin). Re-install it — the same
local-ci:       pinned revision the self-hosted gate job uses — with:
local-ci:         make srtool-install
EOF
echo "log: $LOG"

if [ "$DRY_RUN" = 1 ]; then
  printf 'would run %-34s %s\n' GATE COMMAND
  for spec in "${SELECTED[@]}"; do
    printf 'would run %-34s %s\n' "${spec%%:*}" "${spec#*:}"
  done
  exit 0
fi

export CARGO_BUILD_JOBS="$CARGO_JOBS"
export CARGO_TERM_COLOR=never

{
  echo "local-ci $STAMP"
  echo "root=$ROOT"
  echo "branch=$BRANCH head=$HEAD_SHA tree=$DIRTY"
  echo "live=$RUN_LIVE cross=$RUN_CROSS release=$RUN_RELEASE variants=$RUN_VARIANTS loom=$RUN_LOOM failure=$RUN_FAILURE testnet=$RUN_TESTNET soak=$RUN_SOAK rotation=$RUN_ROTATION jobs=$JOBS cargo_jobs=$CARGO_JOBS"
} >"$LOG"

# run_gate <name> <slug> <command> — one process per gate so the parent keeps the
# real exit code, the timing, and a per-gate log even when gates run in parallel.
run_gate() {
  local name="$1" slug="$2" cmd="$3"
  local gate_log="$LOG_DIR/local-ci-$STAMP-$slug.log"
  local start end rc status reason=""
  start=$(date +%s)
  # WASM_BUILD_WORKSPACE_HINT: substrate-wasm-builder looks for the workspace
  # Cargo.lock by walking up from the *target* directory. When this run redirects
  # CARGO_TARGET_DIR outside the workspace (a dedicated dir, e.g. to dodge
  # artifacts another toolchain left in a shared target/), that walk finds
  # nothing, so the nested wasm build silently re-resolves its whole dependency
  # graph and then dies on `crypto-common`:
  #   cargo:warning=Could not find `Cargo.lock` for .../runtime/Cargo.toml
  #   error[E0463]: can't find crate for `std` ... wasm32v1-none
  # Pointing the hint at the workspace makes the redirected run behave exactly
  # like the default one. Both `clippy runtime rc1` and `clippy workspace` go
  # green on a redirected target dir with this set (see docs/local-ci.md).
  env CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}" \
      WASM_BUILD_WORKSPACE_HINT="$ROOT" \
      bash -c "$cmd" >"$gate_log" 2>&1
  rc=$?
  end=$(date +%s)
  status=PASS
  if [ "$rc" -ne 0 ]; then
    # Missing network, or a `--offline` gate whose cargo cache went cold (this box
    # has lost parts of ~/.cargo more than once — see docs/local-ci.md). A gate
    # whose toolchain is not installed is the same kind of thing: nothing ran, so
    # the run cannot call it green.
    if grep -qE "Could not resolve host|failed to resolve address|network failure seems to have happened|spurious network error|failed to get .* as a dependency|you're using offline mode|toolchain '[^']*' is not installed" "$gate_log"; then
      status=BLOCKED
      reason=network
    # This box periodically rewrites ~/.rustup / ~/.cargo and loses files under
    # the shared target dir mid-build (see #330 and the header of this script).
    # The named path exists again moments later — this is not a diagnostic
    # about our code, it is the toolchain or target dir vanishing out from
    # under a running process. Every real occurrence pairs one of cargo's
    # "could not execute process / could not parse dep info / failed to run
    # custom build command" framings with the raw OS error for a missing file.
    elif grep -qE "could not execute process|could not parse/generate dep info|failed to run custom build command" "$gate_log" \
      && grep -qE "No such file or directory \(os error 2\)" "$gate_log"; then
      status=BLOCKED
      reason=environment
    # The shared `$ROOT/target` is written by whatever toolchain ran last. This
    # workspace pins 1.90.0 (rust-toolchain.toml) but `stable` is also installed
    # here, so a single command run without the pin leaves ~50G of artifacts that
    # the pinned compiler refuses to link against: cargo then reports
    # "found crate `x` compiled by an incompatible version of rustc" (plus a
    # cascade of inference errors in *crates nobody changed*). Nothing about the
    # code failed; the target dir is the problem, and the remedy is a dedicated
    # one. This cost an hour of misreading two clippy gates as code failures —
    # classify it instead.
    elif grep -qE "compiled by an incompatible version of rustc|please recompile that crate using this compiler" "$gate_log"; then
      status=BLOCKED
      reason=toolchain-mix
    else
      status=FAIL
    fi
  fi
  printf '%s' "$status" >"$LOG_DIR/local-ci-$STAMP-$slug.status"
  printf '%s' "$((end - start))" >"$LOG_DIR/local-ci-$STAMP-$slug.secs"
  printf '%s' "$reason" >"$LOG_DIR/local-ci-$STAMP-$slug.reason"
  # One short line per gate: atomic appends, so parallel gates cannot interleave.
  if [ "$status" = "PASS" ]; then
    printf 'PASS %-34s %ss\n' "$name" "$((end - start))" | tee -a "$LOG"
  elif [ "$status" = "BLOCKED" ] && [ "$reason" = "toolchain-mix" ]; then
    printf 'BLOCKED %-31s %ss — the shared target dir holds artifacts from another rustc; rerun with CARGO_TARGET_DIR=<dedicated dir> (%s)\n' \
      "$name" "$((end - start))" "${gate_log#"$ROOT"/}" | tee -a "$LOG"
  elif [ "$status" = "BLOCKED" ] && [ "$reason" = "environment" ]; then
    printf 'BLOCKED %-31s %ss — this box lost rustc/cargo or a target-dir file mid-build; not a code diagnostic (%s)\n' \
      "$name" "$((end - start))" "${gate_log#"$ROOT"/}" | tee -a "$LOG"
  elif [ "$status" = "BLOCKED" ]; then
    printf 'BLOCKED %-31s %ss — environment could not fetch dependencies; nothing verified (%s)\n' \
      "$name" "$((end - start))" "${gate_log#"$ROOT"/}" | tee -a "$LOG"
  else
    printf 'FAIL %-34s %ss — %s\n' "$name" "$((end - start))" "${gate_log#"$ROOT"/}" | tee -a "$LOG"
  fi
}

any_failed() {
  local file
  for file in "$LOG_DIR"/local-ci-"$STAMP"-*.status; do
    [ -e "$file" ] || continue
    [ "$(cat "$file")" = "PASS" ] || return 0
  done
  return 1
}

kill_running() {
  local pid
  for pid in $(jobs -p); do
    pkill -P "$pid" 2>/dev/null
    kill "$pid" 2>/dev/null
  done
  wait 2>/dev/null
}

for spec in "${SELECTED[@]}"; do
  name="${spec%%:*}"; cmd="${spec#*:}"; slug="$(slugify "$name")"
  if [ -n "${SEEN_SLUG[$slug]:-}" ]; then
    echo "local-ci: duplicate gate slug '$slug' — refusing to overwrite its log" >&2
    exit 2
  fi
  SEEN_SLUG[$slug]=1
  while [ "$(jobs -rp | wc -l)" -ge "$JOBS" ]; do
    wait -n
    if [ "$FAIL_FAST" = 1 ] && any_failed; then
      echo "local-ci: --fail-fast triggered — stopping remaining gates"
      kill_running
      break 2
    fi
  done
  run_gate "$name" "$slug" "$cmd" &
done
wait

# ── summary ────────────────────────────────────────────────────────────────
for spec in "${SELECTED[@]}"; do
  name="${spec%%:*}"; slug="$(slugify "$name")"
  status="$(cat "$LOG_DIR/local-ci-$STAMP-$slug.status" 2>/dev/null || echo FAIL)"
  secs="$(cat "$LOG_DIR/local-ci-$STAMP-$slug.secs" 2>/dev/null || echo 0)"
  reason="$(cat "$LOG_DIR/local-ci-$STAMP-$slug.reason" 2>/dev/null || echo "")"
  GATE_NAMES+=("$name"); GATE_STATUS+=("$status"); GATE_SECS+=("$secs")
  GATE_REASONS+=("$reason")
  GATE_LOGS+=("$LOG_DIR/local-ci-$STAMP-$slug.log")
  [ "$status" = "PASS" ] || FAILED=1
done

BLOCKED_COUNT=0
BLOCKED_ENV_COUNT=0
for i in "${!GATE_STATUS[@]}"; do
  if [ "${GATE_STATUS[$i]}" = "BLOCKED" ]; then
    BLOCKED_COUNT=$((BLOCKED_COUNT + 1))
    [ "${GATE_REASONS[$i]}" = "environment" ] && BLOCKED_ENV_COUNT=$((BLOCKED_ENV_COUNT + 1))
  fi
done

echo ""
echo "──────── local-ci summary ($STAMP) ────────"
printf '%-34s %-6s %s\n' "GATE" "RESULT" "SECONDS"
for i in "${!GATE_NAMES[@]}"; do
  printf '%-34s %-6s %s\n' "${GATE_NAMES[$i]}" "${GATE_STATUS[$i]}" "${GATE_SECS[$i]}"
done
if [ "$((BLOCKED_COUNT - BLOCKED_ENV_COUNT))" -gt 0 ]; then
  echo ""
  echo "$((BLOCKED_COUNT - BLOCKED_ENV_COUNT)) gate(s) reported BLOCKED (network): the environment is missing"
  echo "what the gate needs (network access to fetch dependencies, or a toolchain"
  echo "it pins), so those gates did not execute and verified nothing. Re-run with"
  echo "network access (or pre-fetch the dependency) before treating this as"
  echo "coverage. A gate that runs with --offline warms up with:"
  echo "  cargo fetch --locked --manifest-path crates/cross-vm-coordinator/Cargo.toml"
fi
if [ "$BLOCKED_ENV_COUNT" -gt 0 ]; then
  echo ""
  echo "$BLOCKED_ENV_COUNT gate(s) reported BLOCKED (environment): this box rewrote"
  echo "~/.rustup, ~/.cargo, or a shared target dir out from under a running build"
  echo "(see issue #330 and docs/local-ci.md). The failing log has a 'could not"
  echo "execute process' / 'could not parse dep info' / 'failed to run custom build"
  echo "command' framing paired with a raw 'No such file or directory (os error 2)'"
  echo "— that pairing is never a diagnostic about this repo's code. Just re-run the"
  echo "gate; do not read it as a real failure and do not merge/revert based on it."
fi

{
  echo ""
  echo "──────── summary ────────"
  printf '%-34s %-6s %s\n' "GATE" "RESULT" "SECONDS"
  for i in "${!GATE_NAMES[@]}"; do
    printf '%-34s %-6s %s\n' "${GATE_NAMES[$i]}" "${GATE_STATUS[$i]}" "${GATE_SECS[$i]}"
  done
  if [ "${#NOTES[@]}" -gt 0 ]; then
    echo ""
    for note in "${NOTES[@]}"; do echo "note: $note"; done
  fi
} >> "$LOG"

{
  printf '{\n'
  printf '  "stamp": "%s",\n' "$STAMP"
  printf '  "branch": "%s",\n' "$BRANCH"
  printf '  "head": "%s",\n' "$HEAD_SHA"
  printf '  "working_tree": "%s",\n' "$DIRTY"
  printf '  "jobs": %s,\n' "$JOBS"
  printf '  "cargo_jobs": %s,\n' "$CARGO_JOBS"
  printf '  "result": "%s",\n' "$([ "$FAILED" -eq 1 ] && echo FAIL || echo PASS)"
  printf '  "touched_files": %s,\n' "${#TOUCHED[@]}"
  printf '  "gates": [\n'
  for i in "${!GATE_NAMES[@]}"; do
    comma=","
    [ "$i" -eq "$((${#GATE_NAMES[@]} - 1))" ] && comma=""
    printf '    {"name": "%s", "result": "%s", "seconds": %s, "reason": "%s", "log": "%s"}%s\n' \
      "${GATE_NAMES[$i]}" "${GATE_STATUS[$i]}" "${GATE_SECS[$i]}" "${GATE_REASONS[$i]}" \
      "$(basename "${GATE_LOGS[$i]}")" "$comma"
  done
  printf '  ]\n}\n'
} >"$LOG_DIR/local-ci-$STAMP-summary.json"

echo "local-ci: summary json -> .ai/runlogs/local-ci-$STAMP-summary.json"

if [ "$FAILED" = 1 ]; then
  echo ""
  echo "local-ci: FAILED — see the per-gate logs listed above"
  exit 1
fi
echo ""
echo "local-ci: all gates passed"
