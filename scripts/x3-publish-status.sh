#!/usr/bin/env bash
# Publish a local-ci verdict as a GitHub commit status.
#
# GitHub-hosted workflows are disabled in this repository (every workflow is
# workflow_dispatch on a local runner), so nothing runs automatically against a PR.
# The gate of record is scripts/local-ci.sh, executed on this box, and branch
# protection can only require a verdict that exists on the PR head. This helper
# posts one: context `x3/local-ci`, bound to the exact SHA that was tested.
#
# Usage:
#   scripts/x3-publish-status.sh --state success|failure|error|pending \
#       [--sha <commit>] [--description <text>] [--context x3/local-ci]
#
# Exit codes:
#   0  status posted
#   2  usage error
#   3  environment cannot publish (no gh, no auth, no origin repository)
set -u

STATE=""
SHA="HEAD"
CONTEXT="x3/local-ci"
DESCRIPTION=""

while [ $# -gt 0 ]; do
  case "$1" in
    --state) STATE="${2:-}"; shift ;;
    --sha) SHA="${2:-}"; shift ;;
    --description) DESCRIPTION="${2:-}"; shift ;;
    --context) CONTEXT="${2:-}"; shift ;;
    *) echo "x3-publish-status: unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done

case "$STATE" in
  success|failure|error|pending) ;;
  *) echo "x3-publish-status: --state must be success|failure|error|pending" >&2; exit 2 ;;
esac

command -v gh >/dev/null 2>&1 || { echo "x3-publish-status: gh is not installed"; exit 3; }
command -v git >/dev/null 2>&1 || { echo "x3-publish-status: git is not installed"; exit 3; }

REQUESTED_SHA="$SHA"
SHA="$(git rev-parse --verify --quiet "$REQUESTED_SHA^{commit}")" || {
  echo "x3-publish-status: $REQUESTED_SHA is not a commit in this repository"
  exit 3
}

REPO="$(gh repo view --json nameWithOwner -q .nameWithOwner 2>/dev/null)" || {
  echo "x3-publish-status: cannot resolve the origin repository (is gh authenticated?)"
  exit 3
}

[ -n "$DESCRIPTION" ] || DESCRIPTION="local-ci $STATE"
# GitHub caps the description at 140 characters.
DESCRIPTION="${DESCRIPTION:0:140}"

gh api -X POST "repos/$REPO/statuses/$SHA" \
  -f state="$STATE" -f context="$CONTEXT" -f description="$DESCRIPTION" \
  --jq '"status " + .context + " " + .state + " " + (.sha[0:12])'
