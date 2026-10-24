#!/usr/bin/env bash
# Turns a PR comment into a review.sh run.
#
#   COMMENT_BODY='/ask is the rollback complete?' on-comment.sh <pr-number>
#
# Only the first line is read, and only these commands are accepted:
#   /review   /improve   /describe   /ask <question>
# Anything else exits 0 without a model call, so an ordinary comment that
# starts with "/" does not fail the job. Extra words after /review, /improve and
# /describe are dropped rather than forwarded: they would reach pr-agent as
# --config overrides, and a comment must not be able to change the model or
# the endpoint the diff is sent to.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

die() { echo "error: $*" >&2; exit 2; }

[ $# -eq 1 ] || die "usage: COMMENT_BODY=... on-comment.sh <pr-number>"
pr="$1"
case "$pr" in
  ''|*[!0-9]*) die "'$pr' is not a PR number" ;;
esac

first_line="${COMMENT_BODY:-}"
first_line="${first_line%%$'\n'*}"
first_line="${first_line%$'\r'}"
read -r word rest <<<"$first_line" || true

case "$word" in
  /review|/improve|/describe)
    exec bash "$ROOT_DIR/services/pr-agent/review.sh" "$pr" "${word#/}"
    ;;
  /ask)
    [ -n "${rest:-}" ] || die "/ask needs a question: /ask <question>"
    # pr-agent reads any argument starting with "--" as a settings override,
    # so a question shaped like one would reconfigure the run.
    case "$rest" in
      -*) die "an /ask question cannot start with '-'" ;;
    esac
    exec bash "$ROOT_DIR/services/pr-agent/review.sh" "$pr" ask "$rest"
    ;;
  *)
    echo "not a PR-Agent command: '${word:-}'; ignoring"
    ;;
esac
