#!/usr/bin/env bash
# check-readiness-consistency.sh — Fail CI when status documents contradict FEATURE_REGISTRY.toml
#
# Reads FEATURE_REGISTRY.toml (canonical source), extracts readiness scores,
# then validates that other status documents don't claim higher completion
# than the registry permits.
#
# Also cross-checks TESTNET_FEATURE_FLAGS.toml mode assignments against the
# registry modes to detect contradictions.
#
# Exit 0 = consistent; Exit 1 = contradictions found.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

REGISTRY="$REPO_ROOT/FEATURE_REGISTRY.toml"
FLAGS="$REPO_ROOT/TESTNET_FEATURE_FLAGS.toml"

if [[ ! -f "$REGISTRY" ]]; then
  echo "FAIL: FEATURE_REGISTRY.toml not found at $REGISTRY"
  exit 1
fi

VIOLATIONS=0

# Extract readiness scores as key=score pairs
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

echo "========================================"
echo " Readiness Consistency Check"
echo "========================================"
echo "Canonical source: FEATURE_REGISTRY.toml"
echo ""

# --- Cross-check TESTNET_FEATURE_FLAGS.toml against registry modes ---
if [[ -f "$FLAGS" ]]; then
  echo "--- Checking TESTNET_FEATURE_FLAGS.toml vs registry modes ---"
  while IFS= read -r line; do
    if [[ "$line" =~ ^([a-z_]+)[[:space:]]*=[[:space:]]*\"([A-Z_]+)\" ]]; then
      feat="${BASH_REMATCH[1]}"
      flag_mode="${BASH_REMATCH[2]}"
      registry_mode=$(get_mode "$feat")
      if [[ -n "$registry_mode" && "$registry_mode" != "UNKNOWN" ]]; then
        # Map equivalent modes
        normalized_flag="$flag_mode"
        normalized_registry="$registry_mode"
        # LIVE_TESTNET in flags should match LIVE_TESTNET in registry
        # GUARDED_TESTNET should match
        # SIM_TESTNET should match
        if [[ "$normalized_flag" != "$normalized_registry" ]]; then
          echo "  VIOLATION: Feature '$feat' has mode '$flag_mode' in TESTNET_FEATURE_FLAGS.toml but '$registry_mode' in FEATURE_REGISTRY.toml"
          VIOLATIONS=$((VIOLATIONS + 1))
        fi
      fi
    fi
  done < "$FLAGS"
fi

# --- Check CURRENT_MAINNET_STATUS.md ---
CURRENT_STATUS="$REPO_ROOT/CURRENT_MAINNET_STATUS.md"
if [[ -f "$CURRENT_STATUS" ]]; then
  echo "--- Checking CURRENT_MAINNET_STATUS.md ---"

  # Check for "Production" claims against guarded/testnet features
  while IFS= read -r line; do
    if echo "$line" | grep -qi "production"; then
      for feat in $(echo "$SCORES" | grep '_mode=' | cut -d_ -f1 | sort -u); do
        mode=$(get_mode "$feat")
        if [[ "$mode" == "GUARDED_TESTNET" || "$mode" == "SIM_TESTNET" ]]; then
          if echo "$line" | grep -qi "$(echo "$feat" | sed 's/_/ /g')"; then
            echo "  VIOLATION: CURRENT_MAINNET_STATUS.md claims 'Production' for $feat (mode=$mode)"
            VIOLATIONS=$((VIOLATIONS + 1))
          fi
        fi
      done
    fi
  done < "$CURRENT_STATUS"

  # Check for claimed percentages > registry score
  while IFS= read -r line; do
    if echo "$line" | grep -qE '([0-9]+)%'; then
      claimed_pct=$(echo "$line" | grep -oE '[0-9]+%' | head -1 | tr -d '%')
      for feat in $(echo "$SCORES" | grep '=' | grep -v '_mode=' | cut -d= -f1 | sort -u); do
        score=$(get_score "$feat")
        readable_name=$(echo "$feat" | sed 's/_/ /g')
        if echo "$line" | grep -qi "$readable_name"; then
          if [[ -n "$score" && "$claimed_pct" -gt "$score" && "$claimed_pct" -gt 0 ]]; then
            echo "  VIOLATION: CURRENT_MAINNET_STATUS.md claims ${claimed_pct}% for $feat (registry score=$score%)"
            VIOLATIONS=$((VIOLATIONS + 1))
          fi
        fi
      done
    fi
  done < "$CURRENT_STATUS"
fi

# --- Check x3-lang/README.md ---
X3_LANG_README="$REPO_ROOT/x3-lang/README.md"
if [[ -f "$X3_LANG_README" ]]; then
  echo "--- Checking x3-lang/README.md ---"
  
  # Reject any "100% COMPLETE" or "PRODUCTION-READY" global claims
  if grep -qi "100%.*COMPLETE\|COMPLETE.*100%" "$X3_LANG_README"; then
    echo "  VIOLATION: x3-lang/README.md claims 100% complete (FEATURE_REGISTRY.toml scores are lower)"
    VIOLATIONS=$((VIOLATIONS + 1))
  fi
  
  if grep -qi "production-ready\|mainnet-ready" "$X3_LANG_README"; then
    echo "  VIOLATION: x3-lang/README.md claims production-readiness (check registry scores)"
    VIOLATIONS=$((VIOLATIONS + 1))
  fi
fi

# --- Check x3-lang/spec/INDEX.md ---
X3_LANG_SPEC="$REPO_ROOT/x3-lang/spec/INDEX.md"
if [[ -f "$X3_LANG_SPEC" ]]; then
  echo "--- Checking x3-lang/spec/INDEX.md ---"
  
  if grep -qi "100%.*COMPLETE\|COMPLETE.*100%" "$X3_LANG_SPEC"; then
    echo "  VIOLATION: x3-lang/spec/INDEX.md claims 100% complete"
    VIOLATIONS=$((VIOLATIONS + 1))
  fi
  
  if grep -qi "production-ready\|mainnet-ready" "$X3_LANG_SPEC"; then
    echo "  VIOLATION: x3-lang/spec/INDEX.md claims production-readiness"
    VIOLATIONS=$((VIOLATIONS + 1))
  fi
fi

# --- Check for contradictions in feature inventory ---
echo "--- Checking feature inventory consistency ---"
# Extract all feature keys from the registry
REGISTRY_FEATURES=$(echo "$SCORES" | grep '=' | grep -v '_mode=' | cut -d= -f1 | sort -u)

# Count features in registry
REGISTRY_COUNT=$(echo "$REGISTRY_FEATURES" | wc -l)

# Check feature count in CURRENT_MAINNET_STATUS.md scoreboard
if [[ -f "$CURRENT_STATUS" ]]; then
  SCOREBOARD_COUNT=$(grep -cE '^\|.*\|.*[0-9]+%.*\|' "$CURRENT_STATUS" || true)
  # Scoreboard items should roughly match registry features
  # Allow for items that are aggregates (like "All swarm agents (6 agents)")
fi

echo "  Registry features: $REGISTRY_COUNT"
echo ""

# --- Summary ---
echo "========================================"
if [[ "$VIOLATIONS" -gt 0 ]]; then
  echo "FAIL: $VIOLATIONS consistency violation(s) found."
  echo "Update the violating documents to match canonical readiness scores in FEATURE_REGISTRY.toml."
  exit 1
else
  echo "PASS: All status documents are consistent with FEATURE_REGISTRY.toml."
  exit 0
fi
