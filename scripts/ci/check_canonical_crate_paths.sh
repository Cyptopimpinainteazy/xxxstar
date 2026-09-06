#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

workspace_file="Cargo.toml"

require_member() {
  local member="$1"
  if ! grep -q "\"${member}\"" "$workspace_file"; then
    echo "::error::Missing required workspace member: ${member}"
    exit 1
  fi
}

forbidden_member() {
  local member="$1"
  if grep -q "\"${member}\"" "$workspace_file"; then
    echo "::error::Forbidden workspace member still present: ${member}"
    exit 1
  fi
}

if [[ ! -d "crates/x3-relayer" ]]; then
  echo "::error::Canonical relayer crate path missing: crates/x3-relayer"
  exit 1
fi

require_member "crates/x3-relayer"
require_member "crates/x3-bridge-security-council"
require_member "crates/x3-genesis-builder"
forbidden_member "crates/relayer"

echo "Canonical crate path checks passed"
