#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

if ! command -v gh >/dev/null 2>&1; then
  echo "::error::GitHub CLI (gh) is required"
  exit 1
fi

POLICY_FILE=".github/branch-protection/release-mainnet.required-checks.json"
if [[ ! -f "$POLICY_FILE" ]]; then
  echo "::error::Policy file not found: $POLICY_FILE"
  exit 1
fi

REPO="${GITHUB_REPOSITORY:-}"
if [[ -z "$REPO" ]]; then
  remote_url="$(git config --get remote.origin.url || true)"
  if [[ "$remote_url" =~ github.com[:/]([^/]+/[^/.]+)(\.git)?$ ]]; then
    REPO="${BASH_REMATCH[1]}"
  fi
fi

if [[ -z "$REPO" ]]; then
  echo "::error::Unable to determine repository. Set GITHUB_REPOSITORY=owner/repo"
  exit 1
fi

BRANCH="${BRANCH_NAME:-release/mainnet}"
API_PATH="/repos/${REPO}/branches/${BRANCH}/protection"

echo "Repository: ${REPO}"
echo "Branch: ${BRANCH}"
echo "Policy: ${POLICY_FILE}"

if [[ "${1:-}" != "--apply" ]]; then
  echo "Dry-run mode (no changes applied)."
  echo "Previewing policy payload:"
  cat "$POLICY_FILE"
  echo
  echo "To apply, run:"
  echo "  BRANCH_NAME=${BRANCH} GITHUB_REPOSITORY=${REPO} bash scripts/ci/configure_release_mainnet_branch_protection.sh --apply"
  exit 0
fi

echo "Applying branch protection via GitHub API..."
gh api \
  --method PUT \
  -H "Accept: application/vnd.github+json" \
  "$API_PATH" \
  --input "$POLICY_FILE" >/dev/null

echo "Branch protection applied successfully for ${REPO}:${BRANCH}"
