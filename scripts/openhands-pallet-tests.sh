#!/usr/bin/env bash
# Run one OpenHands test-writing job per pallet, each in its own git worktree off
# origin/master, through the openhands-x3 launcher (local x3-ai-router).
#
#   scripts/openhands-pallet-tests.sh --yes pallets/x3-slash pallets/evolution-core
#   scripts/openhands-pallet-tests.sh --yes --jobs 2 pallets/x3-slash pallets/x3-sentinel
#   scripts/openhands-pallet-tests.sh --dry-run pallets/x3-slash
#
# OpenHands --headless approves every action the agent takes, and the agent runs on
# this host, not in a sandbox. Hence --yes is required, and each job runs with:
#   * its own worktree and branch (oh/tests-<pallet>-<date>), nothing committed;
#   * pushes disabled (remote.origin.pushurl points nowhere) and an empty gh config;
#   * a time limit (--timeout, default 90 min).
# Review each branch's diff before keeping any of it.
set -euo pipefail

REPO="$(git rev-parse --show-toplevel)"
STAMP="$(date +%Y%m%d)"
JOBS=1
TIMEOUT_MIN=90
MIN_FREE_GB=10
YES=0
DRY_RUN=0
PALLETS=()

while [[ $# -gt 0 ]]; do
    case "$1" in
        --yes) YES=1 ;;
        --dry-run) DRY_RUN=1 ;;
        --jobs) JOBS="$2"; shift ;;
        --timeout) TIMEOUT_MIN="$2"; shift ;;
        -h|--help) sed -n '2,16p' "$0"; exit 0 ;;
        pallets/*) PALLETS+=("${1%/}") ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
    shift
done

[[ ${#PALLETS[@]} -gt 0 ]] || { echo "name at least one pallets/<dir>" >&2; exit 2; }
for p in "${PALLETS[@]}"; do
    [[ -d "$REPO/$p" ]] || { echo "no such directory: $p" >&2; exit 2; }
done
if [[ $YES -ne 1 && $DRY_RUN -ne 1 ]]; then
    echo "headless OpenHands approves every agent action on this host; rerun with --yes" >&2
    exit 2
fi
command -v openhands-x3 >/dev/null || { echo "openhands-x3 launcher not on PATH" >&2; exit 1; }

free_gb() { df -BG --output=avail "$HOME" | tail -1 | tr -dc '0-9'; }

task_for() {
    echo "Write real tests for the FRAME pallet in $1 (see docs/REPO_MAP.md for its \
package name). Follow the pallet-tests, build-on-low-disk and verify skills. Cover every \
extrinsic: the success path with its storage and balance effects, every error it can \
return (assert_noop!), origin checks, and repeated calls. If a test exposes a bug, fix \
the bug in the pallet in the smallest way and explain it, or leave the test failing and \
report it; never weaken or ignore a test. Only edit files under $1. Do not commit \
or push. Finish with the verify skill's report."
}

if [[ $DRY_RUN -eq 1 ]]; then
    echo "free: $(free_gb)G (need ${MIN_FREE_GB}G per job)"
    for p in "${PALLETS[@]}"; do
        n="$(basename "$p")"
        echo "[$n] worktree: $(dirname "$REPO")/x3-oh-$n-$STAMP  branch: oh/tests-$n-$STAMP"
        echo "[$n] task: $(task_for "$p")"
    done
    exit 0
fi

git -C "$REPO" fetch -q origin master
LOGDIR="$(dirname "$REPO")/openhands-runs-$STAMP"
mkdir -p "$LOGDIR"
NOPUSH_GH="$(mktemp -d)"

run_one() {
    local pallet="$1" name wt branch log status
    name="$(basename "$pallet")"
    wt="$(dirname "$REPO")/x3-oh-$name-$STAMP"
    branch="oh/tests-$name-$STAMP"
    log="$LOGDIR/$name.log"

    if (( $(free_gb) < MIN_FREE_GB )); then
        echo "[$name] skipped: under ${MIN_FREE_GB}G free" | tee "$log"
        return 0
    fi
    if [[ ! -d "$wt" ]]; then
        git -C "$REPO" worktree add -q -b "$branch" "$wt" origin/master
    fi

    echo "[$name] started: $wt (log: $log)"
    (
        cd "$wt"
        export GIT_CONFIG_COUNT=1
        export GIT_CONFIG_KEY_0=remote.origin.pushurl
        export GIT_CONFIG_VALUE_0=/dev/null/push-disabled-for-agents
        export GH_CONFIG_DIR="$NOPUSH_GH"
        unset GH_TOKEN GITHUB_TOKEN
        timeout "${TIMEOUT_MIN}m" openhands-x3 --headless --exit-without-confirmation \
            -t "$(task_for "$pallet")"
    ) >"$log" 2>&1 && status=ok || status="exit $?"

    {
        echo "[$name] finished: $status"
        git -C "$wt" status --short
        git -C "$wt" diff --stat
    } | tee -a "$log"
}

export -f run_one free_gb task_for
export REPO STAMP LOGDIR NOPUSH_GH TIMEOUT_MIN MIN_FREE_GB
printf '%s\n' "${PALLETS[@]}" | xargs -P "$JOBS" -I{} bash -c 'run_one "$@"' _ {}

echo
echo "Logs: $LOGDIR"
echo "Review each worktree's diff (git -C <worktree> diff) before committing anything."
