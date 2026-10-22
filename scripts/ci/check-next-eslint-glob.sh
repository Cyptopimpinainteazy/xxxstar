#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
for project in apps/dex apps/explorer apps/wallet; do
  npm --prefix "$project" ci --no-audit --no-fund
  node scripts/ci/test-next-eslint-glob.cjs "$project"
done
