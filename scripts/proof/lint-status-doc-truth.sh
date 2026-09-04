#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

BASE_REF="${STATUS_DOC_BASE_REF:-}"
if [[ -z "$BASE_REF" ]]; then
  if [[ -n "${GITHUB_BASE_REF:-}" ]]; then
    BASE_REF="origin/${GITHUB_BASE_REF}"
  else
    BASE_REF="HEAD~1"
  fi
fi

if ! git rev-parse --verify "$BASE_REF" >/dev/null 2>&1; then
  echo "WARN: base ref '$BASE_REF' not found; using initial commit as fallback"
  BASE_REF="$(git rev-list --max-parents=0 HEAD | head -1)"
fi

mapfile -t changed_md < <(git diff --name-only "$BASE_REF"...HEAD -- '*.md')

if [[ ${#changed_md[@]} -eq 0 ]]; then
  echo "status-doc truth lint: no changed markdown files"
  exit 0
fi

mapfile -t status_docs < <(printf '%s\n' "${changed_md[@]}" | rg -i '(status|readiness|summary|complete|delivery|report).*\.md$' || true)

if [[ ${#status_docs[@]} -eq 0 ]]; then
  echo "status-doc truth lint: no changed status-style markdown docs"
  exit 0
fi

echo "status-doc truth lint: checking ${#status_docs[@]} file(s)"

failed=0

# Overclaim language that should not appear in status docs without nuance.
OVERCLAIM_RE='\b(100% complete|fully complete|fully implemented|production ready|mainnet ready|battle-tested|seamless)\b'

for file in "${status_docs[@]}"; do
  if [[ ! -f "$file" ]]; then
    continue
  fi

  echo "- linting $file"

  if rg -n -i "$OVERCLAIM_RE" "$file" >/dev/null; then
    echo "  ERROR: overclaim wording detected (use partial/verified/blocked wording instead)"
    rg -n -i "$OVERCLAIM_RE" "$file" || true
    failed=1
  fi

  if ! rg -n -i 'current reality|verified|gaps\s*/\s*risks|release impact|next required work' "$file" >/dev/null; then
    echo "  ERROR: missing truth-structured status sections (expected at least one of: Current Reality, Verified, Gaps / Risks, Release Impact, Next Required Work)"
    failed=1
  fi

  if ! rg -n -i 'implemented|partial|planned|blocked|untested|unverified|in progress' "$file" >/dev/null; then
    echo "  ERROR: missing explicit status qualifiers (implemented/partial/planned/blocked/etc.)"
    failed=1
  fi
done

if [[ $failed -ne 0 ]]; then
  echo "status-doc truth lint: FAILED"
  exit 1
fi

echo "status-doc truth lint: PASSED"