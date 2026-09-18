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
export PATH="${HOME}/.cargo/bin:${PATH}"

RUN_LIVE=0
RUN_CROSS=0
RUN_RELEASE=0
RUN_VARIANTS=0
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
    --all) RUN_LIVE=1; RUN_CROSS=1; RUN_RELEASE=1; RUN_VARIANTS=1 ;;
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
  "test integrity diff:python3 scripts/test_cheat_guard.py --base ${X3_LOCAL_CI_BASE:-origin/master}"
  "readiness consistency:bash scripts/check-readiness-consistency.sh"
  "workspace check:env SKIP_WASM_BUILD=1 cargo check --workspace"
  "clippy workspace:cargo clippy --workspace --all-targets -- -D warnings"
  "clippy runtime rc1:cargo clippy -p x3-chain-runtime --all-targets --no-default-features --features std,mainnet-rc1 -- -D warnings"
  "clippy node rc1:cargo clippy -p x3-chain-node --all-targets --features mainnet-rc1 -- -D warnings"
  "test x3-lang:cargo test --manifest-path x3-lang/Cargo.toml"
  "test atomic-kernel:cargo test -p pallet-x3-atomic-kernel"
  "test atomic-swap std:cargo test -p x3-atomic-swap --features std"
  "test settlement-engine:cargo test -p pallet-x3-settlement-engine"
  "test node:env SKIP_WASM_BUILD=1 cargo test -p x3-chain-node"
  "test cross-vm-coordinator:cargo test --manifest-path crates/cross-vm-coordinator/Cargo.toml"
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

GATES_CROSS=(
  "X3-native lifecycles:env -u SKIP_WASM_BUILD cargo test -p x3-chain-node --test x3vm_live_lifecycle -- --ignored --nocapture --test-threads=1"
)

slugify() { printf '%s' "$1" | tr ' ' '-'; }

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

if [ -n "$ONLY" ]; then
  IFS=',' read -r -a ONLY_LIST <<<"$ONLY"
  FILTERED=()
  for spec in "${SELECTED[@]}"; do
    name="${spec%%:*}"; slug="$(slugify "$name")"
    for want in "${ONLY_LIST[@]}"; do
      [ "$slug" = "$want" ] && FILTERED+=("$spec")
    done
  done
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
command -v srtool >/dev/null 2>&1 || echo "local-ci: note: srtool missing -> the release gate fails on its reproducibility section"
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
    if grep -qE 'Could not resolve host|failed to resolve address|network failure seems to have happened|spurious network error|failed to get .* as a dependency' "$gate_log"; then
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
  echo "coverage."
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
