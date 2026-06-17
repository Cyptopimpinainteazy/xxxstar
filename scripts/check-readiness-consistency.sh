#!/usr/bin/env bash
# check-readiness-consistency.sh — Fail CI when status documents contradict FEATURE_REGISTRY.toml
#
# Reads FEATURE_REGISTRY.toml (canonical source), extracts readiness scores,
# then validates that other status documents don't claim higher completion
# than the registry permits.
#
# Exit 0 = consistent; Exit 1 = contradictions found.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

REGISTRY="$REPO_ROOT/FEATURE_REGISTRY.toml"
if [[ ! -f "$REGISTRY" ]]; then
  echo "FAIL: FEATURE_REGISTRY.toml not found at $REGISTRY"
  exit 1
fi

VIOLATIONS=0

# Extract readiness scores as key=score pairs
# Uses a simple sed+awk parser for TOML tables: [name] + readiness_score = N
extract_scores() {
  local current_key=""
  while IFS= read -r line; do
    if [[ "$line" =~ ^\[([a-z0-9_]+)\]$ ]]; then
      current_key="${BASH_REMATCH[1]}"
    elif [[ "$line" =~ ^readiness_score[[:space:]]*=[[:space:]]*([0-9]+) ]] && [[ -n "$current_key" ]]; then
      echo "${current_key}=${BASH_REMATCH[1]}"
    elif [[ "$line" =~ ^mode[[:space:]]*=[[:space:]]*\"([A-Z_]+)\" ]] && [[ -n "$current_key" ]]; then
      echo "${current_key}_mode=${BASH_REMATCH[1]}"
    fi
  done < "$REGISTRY"
}

SCORES=$(extract_scores)

get_score() {
  local feat="$1"
  echo "$SCORES" | grep "^${feat}=" | head -1 | cut -d= -f2 || echo "0"
}

get_mode() {
  local feat="$1"
  echo "$SCORES" | grep "^${feat}_mode=" | head -1 | cut -d= -f2 || echo "UNKNOWN"
}

echo "=== Readiness Consistency Check ==="
echo "Canonical source: FEATURE_REGISTRY.toml"
echo ""

# --- Check CURRENT_MAINNET_STATUS.md ---
CURRENT_STATUS="$REPO_ROOT/CURRENT_MAINNET_STATUS.md"
if [[ -f "$CURRENT_STATUS" ]]; then
  echo "--- Checking CURRENT_MAINNET_STATUS.md ---"

  # Check for "Production" claims against guarded/testnet features
  # Any line with "Production" that references a feature name
  while IFS= read -r line; do
    if echo "$line" | grep -q "Production"; then
      # Check if any known guarded/sim feature is on this line
      for feat in $(echo "$SCORES" | grep '_mode=' | cut -d_ -f1 | sort -u); do
        mode=$(get_mode "$feat")
        if [[ "$mode" == "GUARDED_TESTNET" || "$mode" == "SIM_TESTNET" ]]; then
          # Try a fuzzy match — if the line mentions something related to this feature
          if echo "$line" | grep -qi "$(echo "$feat" | sed 's/_/ /g')"; then
            echo "  VIOLATION: CURRENT_MAINNET_STATUS.md claims 'Production' for $feat (mode=$mode)"
            VIOLATIONS=$((VIOLATIONS + 1))
          fi
        fi
      done
    fi
  done < "$CURRENT_STATUS"

  # Check for "100%" claims against features scored <95
  while IFS= read -r line; do
    if echo "$line" | grep -q "100%"; then
      for feat in $(echo "$SCORES" | grep '=' | grep -v '_mode=' | cut -d= -f1 | sort -u); do
        score=$(get_score "$feat")
        if [[ -n "$score" && "$score" -lt 95 ]]; then
          # Fuzzy match: does this line reference the feature name?
          readable_name=$(echo "$feat" | sed 's/_/ /g')
          if echo "$line" | grep -qi "$readable_name"; then
            echo "  VIOLATION: CURRENT_MAINNET_STATUS.md claims 100% for $feat (registry score=$score%)"
            VIOLATIONS=$((VIOLATIONS + 1))
          fi
        fi
      done
    fi
  done < "$CURRENT_STATUS"

  # Check the System Completion Scoreboard section for any 100% bars
  # that correspond to features scored below 95
  if grep -q "██████████ 100%" "$CURRENT_STATUS"; then
    echo "  WARNING: CURRENT_MAINNET_STATUS.md contains '██████████ 100%' scoreboard bars. Verify these match registry scores."
  fi
fi

# --- Check x3-lang/README.md ---
X3_LANG_README="$REPO_ROOT/x3-lang/README.md"
if [[ -f "$X3_LANG_README" ]]; then
  echo "--- Checking x3-lang/README.md ---"
  if grep -q "production-ready\|production ready\|mainnet-ready" "$X3_LANG_README"; then
    # x3-lang VM has control-flow opcodes failing closed; it's not production-ready
    echo "  WARNING: x3-lang/README.md may claim production-readiness. Verify control-flow/atomic opcode status."
  fi
fi

# --- Summary ---
echo ""
if [[ "$VIOLATIONS" -gt 0 ]]; then
  echo "FAIL: $VIOLATIONS consistency violation(s) found."
  echo "Update the violating documents to match canonical readiness scores in FEATURE_REGISTRY.toml."
  echo "See docs/X3_CANONICAL_READINESS.md for the derived status rules."
  exit 1
else
  echo "PASS: All status documents are consistent with FEATURE_REGISTRY.toml."
  exit 0
fi