#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

SERVICE_FILE="node/src/service.rs"

require_pattern() {
  local file="$1"
  local pattern="$2"
  local message="$3"

  if ! rg -n --no-heading --fixed-strings "$pattern" "$file" >/dev/null; then
    echo "::error file=${file}::${message}"
    exit 1
  fi
}

echo "Checking node cross-VM safety unit tests..."

require_pattern "$SERVICE_FILE" "fn cross_vm_safety_preflight_rejects_when_bridge_paused()" "Missing preflight pause rejection unit test"
require_pattern "$SERVICE_FILE" "fn cross_vm_safety_postflight_rejects_empty_success_output()" "Missing postflight empty-output rejection unit test"
require_pattern "$SERVICE_FILE" "fn cross_vm_safety_postflight_accepts_non_empty_outputs()" "Missing postflight happy-path unit test"

require_pattern "$SERVICE_FILE" "assert!(blocked.is_err());" "Missing negative assertion for blocked cross-VM safety path"
require_pattern "$SERVICE_FILE" "assert!(gate.postflight(&results).is_ok());" "Missing positive assertion for cross-VM postflight acceptance"

echo "Node cross-VM safety test gate passed"
