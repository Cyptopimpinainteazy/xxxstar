#!/usr/bin/env bash
# SSH-only gateway to a provisioned worker. No credentials or new listener needed.
set -euo pipefail
if [[ "${1:-}" == "--help" ]]; then
  echo 'Usage: run-remote-checks.sh USER@TRUSTED_HOST FULL_COMMIT_SHA GATE_SLUGS'
  echo 'Worker: ~/.local/share/x3-ci/bin/run-checks.sh; uses scoped local CI, never publishes full-CI status.'
  exit 0
fi
host="${1:-}"; sha="${2:-}"; gates="${3:-}"
[[ $# == 3 ]] || { echo 'Expected host, commit and gate list' >&2; exit 2; }
[[ "$host" =~ ^[a-zA-Z0-9_][a-zA-Z0-9_.-]*@[a-zA-Z0-9][a-zA-Z0-9.-]*$ ]] || { echo 'Invalid worker host' >&2; exit 2; }
[[ "$sha" =~ ^[0-9a-f]{40}$ ]] || { echo 'Invalid full commit SHA' >&2; exit 2; }
[[ "$gates" =~ ^[a-z0-9][a-z0-9-]*(,[a-z0-9][a-z0-9-]*)*$ ]] || { echo 'Invalid gate list' >&2; exit 2; }
# Inputs contain no shell metacharacters; the fixed command never accepts raw options.
ssh -o BatchMode=yes -o StrictHostKeyChecking=yes -o ConnectTimeout=10 -- "$host" \
  "bash ~/.local/share/x3-ci/bin/run-checks.sh '$sha' '$gates'"
