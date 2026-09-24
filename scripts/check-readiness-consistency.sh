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

# --- Path-exists sanity: every registry entry must point at real code ---
# Catches the "FICTIONAL registry row" failure mode (registry claims a crate
# that does not exist on disk). Checks crate_or_service against the repo root
# and tolerates either a directory, a Cargo.toml, or a shell script.
echo "--- Checking crate_or_service paths exist on disk ---"
PATH_MISSING=0
current_key=""
while IFS= read -r line; do
  if [[ "$line" =~ ^\[([a-z0-9_]+)\]$ ]]; then
    current_key="${BASH_REMATCH[1]}"
  elif [[ "$line" =~ ^crate_or_service[[:space:]]*=[[:space:]]*\"([^\"]+)\" ]] && [[ -n "$current_key" ]]; then
    p="${BASH_REMATCH[1]}"
    # Resolve relative to repo root
    abs_path="$REPO_ROOT/$p"
    found=0
    for cand in "$abs_path" "$abs_path/Cargo.toml" "$abs_path/src/lib.rs"; do
      if [[ -e "$cand" ]]; then
        found=1
        break
      fi
    done
    if [[ "$found" -eq 0 ]]; then
      echo "  VIOLATION: feature '$current_key' points at '$p' which does not exist on disk"
      PATH_MISSING=$((PATH_MISSING + 1))
    fi
  fi
done < "$REGISTRY"
VIOLATIONS=$((VIOLATIONS + PATH_MISSING))
if [[ "$PATH_MISSING" -gt 0 ]]; then
  echo "  → $PATH_MISSING fictional registry path(s). Delete or rewrite the affected entries."
fi
echo ""

# --- required_tests existence check ---
# CRITICAL-TOK-1 (2026-09-06 independent audit): FEATURE_REGISTRY.toml's
# `required_tests` arrays are the evidentiary basis for each feature's
# readiness_score, but nothing previously verified that a cited test name
# corresponds to a real `#[test] fn` anywhere in the feature's own
# crate_or_service path. That let [atomic_kernel] (score 85, the highest
# in the registry) cite five test names — halt_blocks_new_mint,
# halt_blocks_new_transfer, halt_blocks_new_swap, halt_allows_refund,
# halt_allows_recovery — that did not exist anywhere in the repository
# under any name. This section closes that gap for every registry entry,
# not just that one.
echo "--- Checking required_tests exist as real test functions ---"
TESTS_MISSING=0

# Check every name cited by one `required_tests` array body.
flush_required_tests() { # <feature-key> <crate_or_service> <array-body>
  local key="$1" path="$2" body="$3"
  [ -z "$key" ] || [ -z "$body" ] || [ -z "$path" ] && return 0
  local abs_path="$REPO_ROOT/$path"
  [ -d "$abs_path" ] || return 0
  local rs_file_count
  rs_file_count=$(find "$abs_path" -name "*.rs" 2>/dev/null | wc -l)
  [ "$rs_file_count" -gt 0 ] || return 0
  local citation test_name
  # Whole quoted strings, not just the `[a-zA-Z0-9_]+` runs inside them: a
  # citation carrying a separator ("script.sh::case", "path/to/file.rs::name")
  # used to match nothing at all, so it was never checked — an entry could cite a
  # test nobody wrote and pass. The name that has to exist is the part after the
  # last `::`; `required_tests` is for `fn` names, so a citation whose tail is not
  # a function is a violation worth printing.
  while IFS= read -r citation; do
    [ -z "$citation" ] && continue
    test_name="${citation##*::}"
    if ! grep -rqE "fn[[:space:]]+${test_name}[[:space:]]*\(" "$abs_path" --include="*.rs" 2>/dev/null; then
      echo "  VIOLATION: feature '$key' required_tests cites '$citation' but no 'fn $test_name' exists under $path"
      TESTS_MISSING=$((TESTS_MISSING + 1))
    fi
  done < <(printf '%s' "$body" | grep -oE '"[^"]+"' | tr -d '"')
}

current_key=""
current_path=""
array_body=""
in_tests_array=0
while IFS= read -r line; do
  if [[ "$line" =~ ^\[([a-z0-9_]+)\]$ ]]; then
    if [[ "$in_tests_array" -eq 1 ]]; then
      flush_required_tests "$current_key" "$current_path" "$array_body"
    fi
    current_key="${BASH_REMATCH[1]}"
    current_path=""
    array_body=""
    in_tests_array=0
  elif [[ "$line" =~ ^crate_or_service[[:space:]]*=[[:space:]]*\"([^\"]+)\" ]] && [[ -n "$current_key" ]]; then
    current_path="${BASH_REMATCH[1]}"
  elif [[ "$in_tests_array" -eq 0 && "$line" =~ ^required_tests[[:space:]]*=[[:space:]]*\[(.*)$ ]]; then
    # Single-line arrays ("required_tests = [\"a\", \"b\"]") are the common form
    # in this registry. The previous version set the in-array flag and then
    # cleared it on the very same line, so every name in those arrays was skipped
    # — 12 of the 15 entries were never verified at all.
    array_body="${BASH_REMATCH[1]}"
    if [[ "$array_body" == *"]"* ]]; then
      array_body="${array_body%%]*}"
      flush_required_tests "$current_key" "$current_path" "$array_body"
      array_body=""
    else
      in_tests_array=1
    fi
  elif [[ "$in_tests_array" -eq 1 ]]; then
    array_body+=" $line"
    if [[ "$line" == *"]"* ]]; then
      array_body="${array_body%%]*}"
      flush_required_tests "$current_key" "$current_path" "$array_body"
      array_body=""
      in_tests_array=0
    fi
  fi
done < "$REGISTRY"
if [[ "$in_tests_array" -eq 1 ]]; then
  flush_required_tests "$current_key" "$current_path" "$array_body"
fi
VIOLATIONS=$((VIOLATIONS + TESTS_MISSING))
if [[ "$TESTS_MISSING" -gt 0 ]]; then
  echo "  → $TESTS_MISSING fictional required_tests citation(s). Write the missing test(s) or correct the citation."
fi
echo ""

# --- proof_report existence check ---
# `proof_report` is a row's pointer to its written evidence. Eight of the ten
# distinct paths cited when this check was added did not exist on disk — a
# citation to a missing file is not evidence, and nothing was resolving them, so
# they had drifted unread. An empty value stays allowed: it says "this row has no
# written report". A path that resolves to nothing does not.
echo "--- Checking proof_report paths resolve ---"
REPORTS_MISSING=0
while IFS= read -r line; do
  if [[ "$line" =~ ^proof_report[[:space:]]*=[[:space:]]*\"([^\"]*)\" ]]; then
    p="${BASH_REMATCH[1]}"
    [ -z "$p" ] && continue
    if [[ ! -e "$REPO_ROOT/$p" ]]; then
      echo "  VIOLATION: proof_report '$p' does not exist on disk"
      REPORTS_MISSING=$((REPORTS_MISSING + 1))
    fi
  fi
done < "$REGISTRY"
VIOLATIONS=$((VIOLATIONS + REPORTS_MISSING))
if [[ "$REPORTS_MISSING" -gt 0 ]]; then
  echo "  → $REPORTS_MISSING proof_report path(s) do not resolve. Write the report, or clear the field."
fi
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
