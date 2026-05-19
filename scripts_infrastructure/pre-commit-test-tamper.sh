#!/usr/bin/env bash
set -euo pipefail

echo "🔍 Pre-commit: Test Integrity Enforcement"

# Files staged for commit
STAGED=$(git diff --cached --name-only --diff-filter=ACM)

TEST_FILES=$(echo "$STAGED" | grep -E "(test_|_test\.|/tests/|\.spec\.|\.test\.)" || true)
SRC_FILES=$(echo "$STAGED" | grep -E "(src/|lib/|app/|crates/|pallets/|packages/)" || true)

if [[ -n "${TEST_FILES}" && -z "${SRC_FILES}" ]]; then
  echo "❌ BLOCKED: Tests modified without corresponding source changes."
  echo ""
  echo "Rules:"
  echo "- Fix the code, not the test."
  echo "- If the test is wrong, justify it in the commit message."
  echo ""
  echo "Modified test files:"
  echo "${TEST_FILES}"
  exit 1
fi

# Detect suspicious assertion weakening in staged diffs
STAGED_DIFF=$(git diff --cached --unified=0 --no-color || true)
if echo "$STAGED_DIFF" | grep -E "assert\s*\(.*(>=|<=|!=|==).*\)" >/dev/null; then
  # If someone replaces a strict comparison with an assert of True/False, flag it
  if echo "$STAGED_DIFF" | grep -E "(assert\s*\(.*(True|False).*\))|\bassert_true\b|\bassert_false\b" >/dev/null; then
    echo "❌ BLOCKED: Possible assertion weakening detected in staged changes."
    echo "Review the assertion changes and ensure you are not weakening test checks."
    exit 1
  fi
fi

# No suspicious changes found
echo "✅ Test integrity check passed."