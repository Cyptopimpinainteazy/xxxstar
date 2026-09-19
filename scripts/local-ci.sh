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
# (crates.io / github.com unreachable while cargo resolves dependencies) is
# reported as BLOCKED, not FAIL — and BLOCKED still fails the run, because a
# gate that did not execute has verified nothing.
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

RUN_LIVE=0
RUN_CROSS=0
RUN_RELEASE=0
RUN_VARIANTS=0
RUN_DEEP=0
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
    --deep) RUN_DEEP=1 ;;
    --all) RUN_LIVE=1; RUN_CROSS=1; RUN_RELEASE=1; RUN_VARIANTS=1; RUN_DEEP=1 ;;
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

declare -a GATE_NAMES=() GATE_STATUS=() GATE_SECS=() GATE_LOGS=()
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
  "test integrity diff:python3 scripts/test_cheat_guard.py --base ${X3_LOCAL_CI_BASE:-origin/master}"
  "readiness consistency:bash scripts/check-readiness-consistency.sh"
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
  "nested workspaces:for d in crates/x3-swarm-core services/x3-swarm-api services/x3-swarm-worker services/x3-solvency-sidecar; do echo \"== \$d\"; CARGO_TARGET_DIR=\"/tmp/x3-nested-\$(basename \"\$d\")\" cargo check --locked --all-targets --manifest-path \"\$d/Cargo.toml\" || exit 1; done; echo '== crates/x3-sidecar (lib+bin only; its tests are a follow-up)'; CARGO_TARGET_DIR=/tmp/x3-nested-x3-sidecar cargo check --locked --manifest-path crates/x3-sidecar/Cargo.toml || exit 1"
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
  "EVM contract lifecycle:X3-contracts/evm/test-live-lifecycle.sh"
  "SVM contract lifecycle:programs/svm/x3_atomic_swap/test-live-lifecycle.sh"
)

GATES_VARIANTS=(
  "runtime variant dry-runs:bash scripts/check-runtime-variants.sh"
)

GATES_RELEASE=(
  "release gate (mainnet-check):make mainnet-check"
)

# The broadest automated signal the repository has: every test target in every
# workspace member. SLOW (thousands of tests), so it is opt-in rather than part
# of the default set. No SKIP_WASM_BUILD: x3-chain-node's service tests boot a
# real node whose chain spec is decoded by the embedded runtime.
GATES_DEEP=(
  "test workspace:env -u SKIP_WASM_BUILD cargo test --workspace"
)

GATES_CROSS=(
  "X3-native lifecycles:env -u SKIP_WASM_BUILD cargo test -p x3-chain-node --test x3vm_live_lifecycle -- --ignored --nocapture --test-threads=1"
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
  cat <<'EOF'
  - X3VM<->EVM cross-domain lifecycles (needs anvil + deployed AtlasHTLC; see .ai/runlogs for the runner recipe)
  - X3VM<->SVM cross-domain lifecycles (needs solana-test-validator + SBF program)
EOF
  echo "release gate (--release):"
  printf '  - %s\n' "${GATES_RELEASE[@]%%:*}"
  echo "runtime variants (--variants):"
  printf '  - %s\n' "${GATES_VARIANTS[@]%%:*}"
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
  echo "live=$RUN_LIVE cross=$RUN_CROSS release=$RUN_RELEASE variants=$RUN_VARIANTS jobs=$JOBS cargo_jobs=$CARGO_JOBS"
} >"$LOG"

# run_gate <name> <slug> <command> — one process per gate so the parent keeps the
# real exit code, the timing, and a per-gate log even when gates run in parallel.
run_gate() {
  local name="$1" slug="$2" cmd="$3"
  local gate_log="$LOG_DIR/local-ci-$STAMP-$slug.log"
  local start end rc status
  start=$(date +%s)
  env CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}" bash -c "$cmd" >"$gate_log" 2>&1
  rc=$?
  end=$(date +%s)
  status=PASS
  if [ "$rc" -ne 0 ]; then
    # Missing network, or a `--offline` gate whose cargo cache went cold (this box
    # has lost parts of ~/.cargo more than once — see docs/local-ci.md).
    if grep -qE "Could not resolve host|failed to resolve address|network failure seems to have happened|spurious network error|failed to get .* as a dependency|you're using offline mode" "$gate_log"; then
      status=BLOCKED
    else
      status=FAIL
    fi
  fi
  printf '%s' "$status" >"$LOG_DIR/local-ci-$STAMP-$slug.status"
  printf '%s' "$((end - start))" >"$LOG_DIR/local-ci-$STAMP-$slug.secs"
  # One short line per gate: atomic appends, so parallel gates cannot interleave.
  if [ "$status" = "PASS" ]; then
    printf 'PASS %-34s %ss\n' "$name" "$((end - start))" | tee -a "$LOG"
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
  GATE_NAMES+=("$name"); GATE_STATUS+=("$status"); GATE_SECS+=("$secs")
  GATE_LOGS+=("$LOG_DIR/local-ci-$STAMP-$slug.log")
  [ "$status" = "PASS" ] || FAILED=1
done

BLOCKED_COUNT=0
for status in "${GATE_STATUS[@]}"; do
  [ "$status" = "BLOCKED" ] && BLOCKED_COUNT=$((BLOCKED_COUNT + 1))
done

echo ""
echo "──────── local-ci summary ($STAMP) ────────"
printf '%-34s %-6s %s\n' "GATE" "RESULT" "SECONDS"
for i in "${!GATE_NAMES[@]}"; do
  printf '%-34s %-6s %s\n' "${GATE_NAMES[$i]}" "${GATE_STATUS[$i]}" "${GATE_SECS[$i]}"
done
if [ "$BLOCKED_COUNT" -gt 0 ]; then
  echo ""
  echo "$BLOCKED_COUNT gate(s) reported BLOCKED: the environment could not fetch"
  echo "dependencies, so those gates did not execute and verified nothing. Re-run"
  echo "with network access (or pre-fetch the dependency) before treating this as"
  echo "coverage. A gate that runs with --offline warms up with:"
  echo "  cargo fetch --locked --manifest-path crates/cross-vm-coordinator/Cargo.toml"
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
    printf '    {"name": "%s", "result": "%s", "seconds": %s, "log": "%s"}%s\n' \
      "${GATE_NAMES[$i]}" "${GATE_STATUS[$i]}" "${GATE_SECS[$i]}" \
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
