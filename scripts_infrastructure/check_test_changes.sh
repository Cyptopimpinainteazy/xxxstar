#!/usr/bin/env bash
set -euo pipefail

# Usage: check_test_changes.sh [BASE_SHA] [HEAD_SHA]
# If BASE_SHA is empty, we fall back to origin/main

BASE_SHA=${1:-}
HEAD_SHA=${2:-$(git rev-parse --verify HEAD)}

# Print banner
scripts/print_execution_banner.sh || true

# Ensure we have a full history
git fetch --no-tags --prune --depth=0 origin || true

if [ -z "$BASE_SHA" ] || [ "$BASE_SHA" = "null" ]; then
  echo "No base sha provided; using origin/main as baseline"
  git fetch origin main:refs/remotes/origin/main || true
  BASE_SHA=$(git rev-parse origin/main)
fi

echo "Comparing $BASE_SHA -> $HEAD_SHA"

changed_files=$(git diff --name-only "$BASE_SHA" "$HEAD_SHA" || true)
if [ -z "$changed_files" ]; then
  echo "No changed files detected." && exit 0
fi

# Define test file patterns (directories and test file naming)
is_test_file() {
  local f="$1"
  if [[ "$f" =~ ^tests/ || "$f" =~ (^|/)test/ || "$f" =~ (^|/)[._]?spec(/|$) || "$f" =~ _test\.|/__tests__/ || "$f" =~ \.spec\.(js|ts|py|rs)$ || "$f" =~ \.test\.(js|ts|py|rs)$ || "$f" =~ (^|/)e2e/ ]]; then
    return 0
  fi
  return 1
}

only_tests_changed=true
while IFS= read -r f; do
  if ! is_test_file "$f"; then
    only_tests_changed=false
    break
  fi
done <<< "$changed_files"

if [ "$only_tests_changed" = true ]; then
  echo "" >&2
  echo "🚨 POLICY VIOLATION: PR/commit modifies test files but *no* non-test files were changed." >&2
  echo "This may indicate test-mangling (changing tests to make them pass)." >&2
  echo "If this change is legitimate, add a justification to the PR body or include a non-test fix in the same PR." >&2
  echo "" >&2
  exit 1
fi

# Otherwise OK
echo "Test-file change policy passed (tests changed alongside code or no test-only change)."
exit 0
