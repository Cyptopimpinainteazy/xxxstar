#!/usr/bin/env bash
# Auto-merge open PRs whose local-ci gate passes, in a scratch worktree.
#
# GitHub-hosted CI is intentionally disabled for this repo (see
# .github/workflows/README.md) — the real gate is scripts/local-ci.sh run on
# this trusted self-hosted machine. This mirrors the existing manual policy
# (merge any agent's PR once its build is clean) instead of relying on
# GitHub's native auto-merge, which has nothing to wait on here.
set -uo pipefail

cd /home/lojak/Desktop/xxxstar-main
LOG="/home/lojak/Desktop/xxxstar-main/.ai/reports/auto-merge-$(date +%Y%m%d).log"
mkdir -p "$(dirname "$LOG")"

# Prevent overlapping runs: if a previous invocation is still mid-build
# (local-ci can run long), skip this tick instead of racing it on the same
# scratch worktrees.
LOCK="/tmp/x3-automerge.lock"
exec 200>"$LOCK"
if ! flock -n 200; then
  echo "$(date) previous run still in progress, skipping this tick" >>"$LOG"
  exit 0
fi

git fetch origin --prune >>"$LOG" 2>&1
git worktree prune >>"$LOG" 2>&1

# Shared build cache across worktree runs, kept separate from the primary
# interactive checkout's target/ so this doesn't contend with your own
# manual `cargo build` / lock on the main working tree.
export CARGO_TARGET_DIR="/home/lojak/.cache/x3-automerge-target"
mkdir -p "$CARGO_TARGET_DIR"

# This box (x3star1) is itself a self-hosted CI runner sharing ~/.rustup and
# ~/.cargo with every interactive session; concurrent `rustup toolchain
# install` from a CI job racing a build here intermittently corrupts the
# shared rustup proxy binary, producing "cargo: command not found" or
# "could not execute process rustc" mid-build — not a real code failure.
# Reinstalling rustup is the known-safe recovery (used repeatedly on this
# box already); do it and retry once before treating this as a real
# local-ci failure.
is_rustup_race() {
  grep -qE "cargo: command not found|could not execute process .*(rustc|cargo|clippy-driver|cargo-clippy)|rustup:.*No such file or directory" <<<"$1"
}

reinstall_rustup() {
  echo "$(date) rustup-race signature detected, reinstalling rustup" >>"$LOG"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path >>"$LOG" 2>&1
}

gh pr list --state open --json number,headRefName,isDraft,mergeable,author \
  --jq '.[] | select(.isDraft==false and .mergeable=="MERGEABLE") | [.number,.headRefName] | @tsv' \
  | while IFS=$'\t' read -r num branch; do
      echo "=== PR #$num ($branch) $(date) ===" >>"$LOG"

      # Re-fetch master fresh before each PR: if an earlier PR in this same
      # run just merged, later PRs must be validated against that new
      # master, not the master that existed when this run started.
      git fetch origin master >>"$LOG" 2>&1

      wt="/tmp/x3-automerge-pr-$num"
      rm -rf "$wt"
      if ! git worktree add -f "$wt" "origin/$branch" >>"$LOG" 2>&1; then
        echo "PR #$num: worktree checkout failed, skipping" >>"$LOG"
        continue
      fi

      # Validate the PR combined with current master, not just the PR's
      # diff in isolation — two PRs can each pass local-ci alone and still
      # conflict semantically once both are on master.
      if ! (cd "$wt" && git merge --no-edit origin/master) >>"$LOG" 2>&1; then
        echo "PR #$num: conflicts with current master (not a local-ci failure), skipping" >>"$LOG"
        git worktree remove -f "$wt" >>"$LOG" 2>&1
        continue
      fi

      ci_output=$(cd "$wt" && scripts/local-ci.sh --pre-push 2>&1)
      ci_rc=$?
      echo "$ci_output" >>"$LOG"

      if [ $ci_rc -ne 0 ] && is_rustup_race "$ci_output"; then
        reinstall_rustup
        ci_output=$(cd "$wt" && scripts/local-ci.sh --pre-push 2>&1)
        ci_rc=$?
        echo "$ci_output" >>"$LOG"
        if [ $ci_rc -ne 0 ]; then
          echo "PR #$num: still failing after rustup reinstall+retry (real failure, or race persisted)" >>"$LOG"
        else
          echo "PR #$num: passed on retry after rustup reinstall" >>"$LOG"
        fi
      fi

      if [ $ci_rc -eq 0 ]; then
        echo "PR #$num: local-ci passed against current master, merging" >>"$LOG"
        if gh pr merge "$num" --merge --delete-branch >>"$LOG" 2>&1; then
          echo "PR #$num: merge succeeded" >>"$LOG"
        else
          echo "PR #$num: gh pr merge FAILED (see log above), left open" >>"$LOG"
        fi
      else
        echo "PR #$num: local-ci failed, leaving open" >>"$LOG"
      fi

      git worktree remove -f "$wt" >>"$LOG" 2>&1
    done
