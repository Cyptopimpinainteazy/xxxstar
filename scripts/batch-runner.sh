#!/usr/bin/env bash
# batch-runner.sh — land a batch of branches into one integration branch and
# hit the self-hosted runner once for the whole batch.
#
# Why: each push to master costs ~5 heavy jobs on the two runners, and a change
# waits behind everything already queued. Building up to N local branches, merging
# them into one integration branch, verifying that union locally, and dispatching
# one runner run over it gives the same coverage for one queue slot.
#
# Usage:
#   scripts/batch-runner.sh --branches a,b,c,d,e      # explicit list
#   scripts/batch-runner.sh --ready                   # every local branch with an
#                                                     # open PR (gh must be authed)
#   scripts/batch-runner.sh --branches a,b --deep     # also run cargo test --workspace
#   scripts/batch-runner.sh --branches a,b --no-dispatch   # stop before the runner
#
# What it does:
#   1. creates `batch/<utc-stamp>` off origin/master in a scratch worktree;
#   2. merges each branch into it (aborts on conflict, listing the branch);
#   3. runs the local CI fast set on the union (plus --deep when asked);
#   4. pushes the branch and dispatches `production-gate.yml` on it, so the
#      runner executes the full release gate for all changes at once.
#
# It never touches master and never merges the batch branch itself.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BRANCHES=""
READY=0
DEEP=0
DISPATCH=1
KEEP=0
DRY_RUN=0
MAX=5
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"

usage() { sed -n '2,26p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; }

while [ "$#" -gt 0 ]; do
  case "$1" in
    --branches) BRANCHES="${2:-}"; shift ;;
    --ready) READY=1 ;;
    --deep) DEEP=1 ;;
    --no-dispatch) DISPATCH=0 ;;
    --dry-run) DRY_RUN=1 ;;
    --keep) KEEP=1 ;;
    --max) MAX="${2:-}"; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "batch-runner: unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

if [ "$READY" = "1" ]; then
  # Open PRs authored from this clone, newest first.
  mapfile -t BRANCHES_LIST < <(
    gh pr list --state open --limit 50 --json headRefName --jq '.[].headRefName' 2>/dev/null
  )
  if [ "${#BRANCHES_LIST[@]}" -eq 0 ]; then
    echo "batch-runner: no open PRs found (is gh authed?)" >&2
    exit 2
  fi
else
  IFS=',' read -r -a BRANCHES_LIST <<<"$BRANCHES"
fi

if [ "${#BRANCHES_LIST[@]}" -eq 0 ] || [ -z "${BRANCHES_LIST[0]}" ]; then
  echo "batch-runner: nothing to batch; pass --branches a,b,c or --ready" >&2
  exit 2
fi
if [ "${#BRANCHES_LIST[@]}" -gt "$MAX" ]; then
  echo "batch-runner: ${#BRANCHES_LIST[@]} branches given, --max is $MAX" >&2
  exit 2
fi

BATCH="batch/$STAMP"
WORKTREE="/tmp/x3-batch-$(printf '%s' "$STAMP" | tr -d ':')"

echo "batch-runner: batch branch   $BATCH"
echo "batch-runner: worktree       $WORKTREE"
echo "batch-runner: branches       ${BRANCHES_LIST[*]}"

git fetch origin master --quiet || { echo "batch-runner: git fetch failed" >&2; exit 2; }

if ! git worktree add -b "$BATCH" "$WORKTREE" origin/master >/dev/null 2>&1; then
  echo "batch-runner: could not create the batch worktree" >&2
  exit 2
fi

cleanup() {
  if [ "$KEEP" = "1" ]; then
    echo "batch-runner: keeping $WORKTREE (--keep)"
    return
  fi
  git worktree remove --force "$WORKTREE" >/dev/null 2>&1 || true
}
trap cleanup EXIT

cd "$WORKTREE"
for branch in "${BRANCHES_LIST[@]}"; do
  if git merge --no-edit "$branch" >/dev/null 2>&1; then
    printf '  merged   %s\n' "$branch"
  else
    printf '  CONFLICT %s\n' "$branch" >&2
    echo "batch-runner: '$branch' does not merge cleanly onto origin/master + the earlier branches." >&2
    echo "batch-runner: resolve it by rebasing that branch (or trim the batch) and re-run." >&2
    git merge --abort >/dev/null 2>&1 || true
    exit 1
  fi
done

echo "batch-runner: ${#BRANCHES_LIST[@]} branch(es) merged into $BATCH"

if [ "$DRY_RUN" = "1" ]; then
  git log --oneline -1
  echo "batch-runner: --dry-run: stopping before the gate set"
  exit 0
fi

MODE_ARGS=()
[ "$DEEP" = "1" ] && MODE_ARGS+=(--deep)

echo "batch-runner: running the local gate set on the union"
if ! bash scripts/local-ci.sh "${MODE_ARGS[@]}"; then
  echo "batch-runner: local gates failed on the union; not touching the runner" >&2
  exit 1
fi

if [ "$DISPATCH" != "1" ]; then
  echo "batch-runner: local gates passed; --no-dispatch so the runner was not touched"
  exit 0
fi

git push -u origin "$BATCH" >/dev/null 2>&1 || { echo "batch-runner: push failed" >&2; exit 2; }
gh workflow run production-gate.yml --ref "$BATCH" >/dev/null 2>&1 || {
  echo "batch-runner: dispatched push done but 'gh workflow run' failed" >&2
  echo "batch-runner: the push itself still triggers the gate on $BATCH" >&2
  exit 0
}

echo "batch-runner: dispatched production-gate on $BATCH"
sleep 5
gh run list --workflow=production-gate.yml --limit 3 \
  --json databaseId,status,headBranch,headSha \
  --jq '.[] | "  \(.databaseId) \(.status) \(.headBranch) \(.headSha[0:9])"' 2>/dev/null || true
