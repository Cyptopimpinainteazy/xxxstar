#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PLAIN="$ROOT_DIR/chain-specs/x3-mainnet-plain.json"
RAW="$ROOT_DIR/chain-specs/x3-mainnet-raw.json"
REPORT_DIR="$ROOT_DIR/reports"
REPORT="$REPORT_DIR/genesis_lint.md"
mkdir -p "$REPORT_DIR"

failures=()

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    failures+=("missing file: $path")
  fi
}

check_absent() {
  local pattern="$1"
  local desc="$2"
  if rg -n --pcre2 "$pattern" "$PLAIN" "$RAW" >/dev/null 2>&1; then
    failures+=("forbidden content detected: $desc")
  fi
}

check_present() {
  local pattern="$1"
  local desc="$2"
  if ! rg -n --pcre2 "$pattern" "$PLAIN" "$RAW" >/dev/null 2>&1; then
    failures+=("required content missing: $desc")
  fi
}

require_file "$PLAIN"
require_file "$RAW"

if [[ ${#failures[@]} -eq 0 ]]; then
  check_absent 'Alice|Bob|Charlie|//Alice|//Bob' 'dev authority names (Alice/Bob/Charlie/seed aliases)'
  check_absent '"ExternalBridgesEnabled"\s*:\s*true|"externalBridgesEnabled"\s*:\s*true|"external_bridges"\s*:\s*true' 'ExternalBridgesEnabled true'
  check_absent '"authorities"\s*:\s*\[\s*\]' 'empty authorities'
  check_absent '"boot[Nn]odes"\s*:\s*\[\s*\]' 'empty bootnodes'
  check_absent '"sudo"\s*:\s*"//(Alice|Bob|Charlie|Ferdie)"|"sudo"\s*:\s*"[^"]*Alice[^"]*"' 'dev sudo key'

  check_present 'treasury|Treasury' 'treasury configuration'
  check_present 'council|Council' 'council configuration'
  check_present 'security[_ -]?council|Security[_ -]?Council|securityCouncil' 'security council configuration'
fi

{
  echo "# Genesis Lint"
  echo
  echo "- plain spec: $PLAIN"
  echo "- raw spec: $RAW"
  echo
  if [[ ${#failures[@]} -eq 0 ]]; then
    echo "## Result"
    echo
    echo "Result: PASS"
  else
    echo "## Result"
    echo
    echo "Result: FAIL"
    echo
    echo "## Findings"
    echo
    for f in "${failures[@]}"; do
      echo "- $f"
    done
  fi
} > "$REPORT"

echo "genesis_lint: wrote $REPORT"

if [[ ${#failures[@]} -ne 0 ]]; then
  exit 1
fi
