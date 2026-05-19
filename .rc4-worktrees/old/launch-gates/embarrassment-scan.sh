#!/usr/bin/env bash
# X3 Embarrassment Scanner
# Finds all the things that make mainnet go boom:
# - panic! unwrap() expect() in runtime code
# - TODO FIXME in critical paths
# - hardcoded values, private keys, dev-only code
# - stub implementations, mocks in production
# - unimplemented! todo!() remaining
#
# Philosophy: If it can crash or lie, mainnet will find it.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
STRICT="${STRICT:-1}"
SCAN_PATHS="${SCAN_PATHS:-crates pallets runtime node}"
read -r -a SEARCH_PATHS <<< "${SCAN_PATHS}"
STRICT_P2="${STRICT_P2:-0}"
BLOCK_P2_CATEGORIES="${BLOCK_P2_CATEGORIES:-}"
read -r -a P2_BLOCK_CATEGORIES <<< "${BLOCK_P2_CATEGORIES}"
SUPPRESSIONS_FILE="${SUPPRESSIONS_FILE:-${REPO_ROOT}/launch-gates/embarrassment-suppressions.conf}"

PROOF_LOG="${1:-${REPO_ROOT}/launch-gates/evidence/proof-embarrassment-scan.log}"
TEMP_SCAN="${2:-${REPO_ROOT}/launch-gates/evidence/embarrassment-raw-findings.txt}"
SUPPRESSED_LOG="${SUPPRESSED_LOG:-${REPO_ROOT}/launch-gates/evidence/embarrassment-suppressed-findings.txt}"

RED='\033[0;31m'
YELLOW='\033[1;33m'
ORANGE='\033[38;5;208m'
NC='\033[0m'

mkdir -p "$(dirname "$PROOF_LOG")"
mkdir -p "$(dirname "$SUPPRESSED_LOG")"

{
    echo "category|finding|reason"
} > "$SUPPRESSED_LOG"

{
    echo "=== X3 Embarrassment Scanner ==="
    echo "Start time: $(date)"
    echo "Scanning: crates/ pallets/ runtime/ node/"
    echo ""
} | tee "$PROOF_LOG"

CATEGORIES=(
    "PANIC:panic!\(|unimplemented!\(|unreachable!\("
    "UNWRAP:\.unwrap\(|\.expect\("
    "TODO:TODO|FIXME|XXX|HACK"
    "HARDCODED:hardcoded|123456|0xdeadbeef|magic number"
    "KEYS:private_key|secret_key|seed_phrase|mnemonic"
    "DEV:dev_only|test_only|mock|stub|fake"
    "MEMORY:MemoryStore|in-memory|volatile|ephemeral"
    "LOCALHOST:127\.0\.0\.1|localhost|0\.0\.0\.0"
    "ALICE:\\b(alice|bob|charlie|dave|eve)\\b"
    "SUDOS:ensure_root|ensure_signed_by_admin"
)

rg_scan() {
    local pattern=$1
    rg -n "$pattern" "${SEARCH_PATHS[@]}" --type rust \
        -g '!**/tests/**' \
        -g '!**/*test*.rs' \
        -g '!**/mock.rs' \
        -g '!**/benchmarking.rs' \
        -g '!**/benches/**' \
        2>/dev/null
}

count_lines() {
    local content=$1
    if [ -z "$content" ]; then
        echo 0
    else
        printf "%s\n" "$content" | wc -l | tr -d '[:space:]'
    fi
}

echo "=== Scan Results ===" >> "$TEMP_SCAN"

# Track severity
P0_COUNT=0
P1_COUNT=0
P2_COUNT=0
P2_BLOCK_COUNT=0
SUPPRESSED_COUNT=0

p2_category_blocks() {
    local category=$1
    local blocked
    for blocked in "${P2_BLOCK_CATEGORIES[@]}"; do
        if [ "$blocked" = "$category" ]; then
            return 0
        fi
    done
    return 1
}

is_suppressed() {
    local category=$1
    local finding=$2
    SUPPRESSION_REASON=""

    if [ ! -f "$SUPPRESSIONS_FILE" ]; then
        return 1
    fi

    local suppress_category suppress_regex suppress_reason
    while IFS='|' read -r suppress_category suppress_regex suppress_reason; do
        if [[ -z "${suppress_category// }" ]]; then
            continue
        fi
        if [[ "$suppress_category" =~ ^[[:space:]]*# ]]; then
            continue
        fi
        if [ "$suppress_category" = "$category" ] && [[ "$finding" =~ $suppress_regex ]]; then
            SUPPRESSION_REASON="$suppress_reason"
            return 0
        fi
    done < "$SUPPRESSIONS_FILE"

    return 1
}

for category_def in "${CATEGORIES[@]}"; do
    IFS=':' read -r CATEGORY PATTERN <<< "$category_def"
    
    echo "" | tee -a "$PROOF_LOG" "$TEMP_SCAN"
    echo "--- $CATEGORY ---" | tee -a "$PROOF_LOG" "$TEMP_SCAN"
    
    FINDINGS="$(rg_scan "$PATTERN" || true)"
    FILTERED_FINDINGS=""
    CATEGORY_SUPPRESSED=0

    if [ -n "$FINDINGS" ]; then
        while IFS= read -r finding; do
            if [ -z "$finding" ]; then
                continue
            fi
            if is_suppressed "$CATEGORY" "$finding"; then
                CATEGORY_SUPPRESSED=$((CATEGORY_SUPPRESSED + 1))
                printf '%s|%s|%s\n' "$CATEGORY" "$finding" "$SUPPRESSION_REASON" >> "$SUPPRESSED_LOG"
            else
                if [ -z "$FILTERED_FINDINGS" ]; then
                    FILTERED_FINDINGS="$finding"
                else
                    FILTERED_FINDINGS+=$'\n'"$finding"
                fi
            fi
        done <<< "$FINDINGS"
    fi

    MATCHES="$(count_lines "$FILTERED_FINDINGS")"
    
    if [ "$MATCHES" -gt 0 ]; then
        echo "Found $MATCHES occurrences:" | tee -a "$PROOF_LOG" "$TEMP_SCAN"
        printf "%s\n" "$FILTERED_FINDINGS" | head -20 | tee -a "$PROOF_LOG" "$TEMP_SCAN" || true
        
        if [ $MATCHES -gt 20 ]; then
            echo "  ... and $((MATCHES - 20)) more" | tee -a "$PROOF_LOG" "$TEMP_SCAN"
        fi
        
        # Classify severity
        case "$CATEGORY" in
            PANIC|UNWRAP)
                P0_COUNT=$((P0_COUNT + MATCHES))
                echo -e "${RED}SEVERITY: P0 (CRITICAL - Can crash mainnet)${NC}" | tee -a "$PROOF_LOG" "$TEMP_SCAN"
                ;;
            KEYS|SUDOS|ALICE)
                if [ "$CATEGORY" = "SUDOS" ]; then
                    P2_COUNT=$((P2_COUNT + MATCHES))
                    echo -e "${YELLOW}SEVERITY: P2 (MEDIUM - Review governance origin usage)${NC}" | tee -a "$PROOF_LOG" "$TEMP_SCAN"
                else
                    P0_COUNT=$((P0_COUNT + MATCHES))
                    echo -e "${RED}SEVERITY: P0 (CRITICAL - Security/config issue)${NC}" | tee -a "$PROOF_LOG" "$TEMP_SCAN"
                fi
                ;;
            TODO|FIXME)
                P1_COUNT=$((P1_COUNT + MATCHES))
                echo -e "${ORANGE}SEVERITY: P1 (HIGH - Incomplete code)${NC}" | tee -a "$PROOF_LOG" "$TEMP_SCAN"
                ;;
            *)
                P2_COUNT=$((P2_COUNT + MATCHES))
                if p2_category_blocks "$CATEGORY"; then
                    P2_BLOCK_COUNT=$((P2_BLOCK_COUNT + MATCHES))
                    echo -e "${RED}SEVERITY: P2-BLOCK (policy-blocked category)${NC}" | tee -a "$PROOF_LOG" "$TEMP_SCAN"
                else
                echo -e "${YELLOW}SEVERITY: P2 (MEDIUM - Code smell)${NC}" | tee -a "$PROOF_LOG" "$TEMP_SCAN"
                fi
                ;;
        esac
        if [ $CATEGORY_SUPPRESSED -gt 0 ]; then
            echo "Suppressed $CATEGORY_SUPPRESSED occurrences via $SUPPRESSIONS_FILE" | tee -a "$PROOF_LOG" "$TEMP_SCAN"
            SUPPRESSED_COUNT=$((SUPPRESSED_COUNT + CATEGORY_SUPPRESSED))
        fi
    else
        echo "✅ No matches" | tee -a "$PROOF_LOG" "$TEMP_SCAN"
        if [ $CATEGORY_SUPPRESSED -gt 0 ]; then
            echo "Suppressed $CATEGORY_SUPPRESSED occurrences via $SUPPRESSIONS_FILE" | tee -a "$PROOF_LOG" "$TEMP_SCAN"
            SUPPRESSED_COUNT=$((SUPPRESSED_COUNT + CATEGORY_SUPPRESSED))
        fi
    fi
done

# Additional checks
echo "" | tee -a "$PROOF_LOG" "$TEMP_SCAN"
echo "--- Additional Checks ---" | tee -a "$PROOF_LOG" "$TEMP_SCAN"

# Check for unbounded loops
UNBOUNDED_FINDINGS=$(rg -n "loop\s*\{" "${SEARCH_PATHS[@]}" --type rust \
    -g '!**/tests/**' -g '!**/*test*.rs' -g '!**/mock.rs' -g '!**/benchmarking.rs' -g '!**/benches/**' \
    2>/dev/null || true)
UNBOUNDED="$(count_lines "$UNBOUNDED_FINDINGS")"
if [ "$UNBOUNDED" -gt 0 ]; then
    echo -e "${ORANGE}Found $UNBOUNDED infinite loops (check if bounded)${NC}" | tee -a "$PROOF_LOG" "$TEMP_SCAN"
fi

# Check for default implementations of critical traits
DANGEROUS_DEFAULTS_FINDINGS=$(rg -n "impl.*Default.*for.*(Config|Runtime)" "${SEARCH_PATHS[@]}" --type rust \
    -g '!**/tests/**' -g '!**/*test*.rs' -g '!**/mock.rs' -g '!**/benchmarking.rs' -g '!**/benches/**' \
    2>/dev/null || true)
DANGEROUS_DEFAULTS="$(count_lines "$DANGEROUS_DEFAULTS_FINDINGS")"
if [ "$DANGEROUS_DEFAULTS" -gt 0 ]; then
    echo -e "${RED}Found $DANGEROUS_DEFAULTS Default impls on critical types (P0 risk)${NC}" | tee -a "$PROOF_LOG" "$TEMP_SCAN"
    P0_COUNT=$((P0_COUNT + DANGEROUS_DEFAULTS))
fi

# Summary
echo "" | tee -a "$PROOF_LOG" "$TEMP_SCAN"
{
    echo "=== Embarrassment Scanner Summary ==="
    echo "P0 (CRITICAL): $P0_COUNT"
    echo "P1 (HIGH): $P1_COUNT"
    echo "P2 (MEDIUM): $P2_COUNT"
    echo "P2 blocked by policy: $P2_BLOCK_COUNT"
    echo "Suppressed: $SUPPRESSED_COUNT"
    if [ "$SUPPRESSED_COUNT" -gt 0 ]; then
        echo "Suppressed findings log: $SUPPRESSED_LOG"
    fi
    echo "Total: $((P0_COUNT + P1_COUNT + P2_COUNT))"
    echo ""
} | tee -a "$PROOF_LOG" "$TEMP_SCAN"

if [ "$STRICT" = "1" ] && [ $P0_COUNT -gt 0 ]; then
    {
        echo "RESULT: ❌ FAIL"
        echo "Found $P0_COUNT critical hazards - strict mode blocks release."
        echo "Score: $(( (50 - P0_COUNT) % 50 ))%"
        echo ""
        echo "Required actions:"
        echo "1. Remove all panic!/unwrap() from consensus/validator code"
        echo "2. Remove all hardcoded values, private keys, test data"
        echo "3. Remove all TODO/FIXME from critical paths"
        echo "4. Full code audit before mainnet"
    } | tee -a "$PROOF_LOG"
    exit 1
elif [ "$STRICT" = "1" ] && [ "$STRICT_P2" = "1" ] && [ $P2_BLOCK_COUNT -gt 0 ]; then
    {
        echo "RESULT: ❌ FAIL"
        echo "Found $P2_BLOCK_COUNT policy-blocked P2 hazards."
        echo "Blocked categories: ${BLOCK_P2_CATEGORIES:-none}"
        echo ""
        echo "Required actions:"
        echo "1. Resolve or suppress each finding with documented justification"
        echo "2. Keep STRICT_P2 enabled only for release-critical categories"
    } | tee -a "$PROOF_LOG"
    exit 1
elif [ $P0_COUNT -eq 0 ]; then
    {
        echo "RESULT: ✅ PASS"
        echo "No critical hazards found in production paths."
        echo "Score: 95%"
    } | tee -a "$PROOF_LOG"
    exit 0
elif [ $P0_COUNT -lt 5 ]; then
    {
        echo "RESULT: ⚠️  CONDITIONAL PASS"
        echo "Found $P0_COUNT critical hazards (minor) in advisory mode (STRICT=0)."
        echo "Score: 70%"
    } | tee -a "$PROOF_LOG"
    exit 0
else
    {
        echo "RESULT: ❌ FAIL"
        echo "Found $P0_COUNT critical hazards - MAINNET BLOCKER."
        echo "Score: $(( (50 - P0_COUNT) % 50 ))%"
        echo ""
        echo "Required actions:"
        echo "1. Remove all panic!/unwrap() from consensus/validator code"
        echo "2. Remove all hardcoded values, private keys, test data"
        echo "3. Remove all TODO/FIXME from critical paths"
        echo "4. Full code audit before mainnet"
    } | tee -a "$PROOF_LOG"
    exit 1
fi
