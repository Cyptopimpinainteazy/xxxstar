#!/usr/bin/env bash
# Validate Actions expressions as well as YAML; use a verified portable linter
# when this host does not already provide actionlint. Never install globally.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

lint_bin="$(command -v actionlint || true)"
lint_tmp=""
cleanup() {
  if [ -n "$lint_tmp" ]; then rm -rf -- "$lint_tmp"; fi
}
trap cleanup EXIT
if [ -z "$lint_bin" ]; then
  if [ "$(uname -s)" != Linux ] || [ "$(uname -m)" != x86_64 ]; then
    echo 'workflow schema: install actionlint for this platform' >&2
    exit 2
  fi
  lint_tmp="$(mktemp -d)"
  archive="$lint_tmp/actionlint.tar.gz"
  curl --fail --silent --show-error --location \
    https://github.com/rhysd/actionlint/releases/download/v1.7.12/actionlint_1.7.12_linux_amd64.tar.gz \
    --output "$archive"
  printf '%s  %s\n' \
    8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8 \
    "$archive" | sha256sum --check --status
  tar -xzf "$archive" -C "$lint_tmp" actionlint
  lint_bin="$lint_tmp/actionlint"
fi
# Shell and Python syntax have their own gates. This gate checks Actions schema,
# contexts, dependencies and registered runner labels, without optional linters.
"$lint_bin" -shellcheck= -pyflakes= .github/workflows/local-ci.yml
