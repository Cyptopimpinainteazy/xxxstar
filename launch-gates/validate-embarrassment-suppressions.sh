#!/usr/bin/env bash
# Validates launch-gates/embarrassment-suppressions.conf policy entries.
# Fails on malformed lines or dangerously broad suppressions.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SUPPRESSIONS_FILE="${1:-${REPO_ROOT}/launch-gates/embarrassment-suppressions.conf}"

if [ ! -f "$SUPPRESSIONS_FILE" ]; then
    echo "❌ Suppressions file not found: $SUPPRESSIONS_FILE"
    exit 1
fi

# Keep in sync with embarrassment-scan.sh categories.
VALID_CATEGORIES=(PANIC UNWRAP TODO HARDCODED KEYS DEV MEMORY LOCALHOST ALICE SUDOS)

is_valid_category() {
    local candidate=$1
    local category
    for category in "${VALID_CATEGORIES[@]}"; do
        if [ "$candidate" = "$category" ]; then
            return 0
        fi
    done
    return 1
}

LINE_NO=0
ENTRY_COUNT=0
ERROR_COUNT=0

while IFS= read -r raw_line; do
    LINE_NO=$((LINE_NO + 1))

    # Skip comments and empty lines.
    if [[ -z "${raw_line// }" ]] || [[ "$raw_line" =~ ^[[:space:]]*# ]]; then
        continue
    fi

    # Exactly 3 fields are required: CATEGORY|REGEX|REASON.
    IFS='|' read -r category regex reason extra <<< "$raw_line"

    if [ -n "${extra:-}" ]; then
        echo "❌ ${SUPPRESSIONS_FILE}:${LINE_NO}: too many fields; expected CATEGORY|REGEX|REASON"
        ERROR_COUNT=$((ERROR_COUNT + 1))
        continue
    fi

    if [[ -z "${category:-}" || -z "${regex:-}" || -z "${reason:-}" ]]; then
        echo "❌ ${SUPPRESSIONS_FILE}:${LINE_NO}: missing field; expected CATEGORY|REGEX|REASON"
        ERROR_COUNT=$((ERROR_COUNT + 1))
        continue
    fi

    if ! is_valid_category "$category"; then
        echo "❌ ${SUPPRESSIONS_FILE}:${LINE_NO}: invalid category '$category'"
        ERROR_COUNT=$((ERROR_COUNT + 1))
        continue
    fi

    # Reject obviously broad suppressions.
    if [[ "$regex" == ".*" || "$regex" == "^.*$" ]]; then
        echo "❌ ${SUPPRESSIONS_FILE}:${LINE_NO}: overbroad regex '$regex'"
        ERROR_COUNT=$((ERROR_COUNT + 1))
        continue
    fi

    # Require basic path anchoring to keep suppressions auditable.
    if [[ ! "$regex" =~ ^\^ ]]; then
        echo "❌ ${SUPPRESSIONS_FILE}:${LINE_NO}: regex must be anchored with '^'"
        ERROR_COUNT=$((ERROR_COUNT + 1))
        continue
    fi

    # Reason should be descriptive, not a token.
    reason_len=${#reason}
    if [ "$reason_len" -lt 12 ]; then
        echo "❌ ${SUPPRESSIONS_FILE}:${LINE_NO}: reason too short; provide evidence-backed context"
        ERROR_COUNT=$((ERROR_COUNT + 1))
        continue
    fi

    ENTRY_COUNT=$((ENTRY_COUNT + 1))
done < "$SUPPRESSIONS_FILE"

if [ "$ERROR_COUNT" -gt 0 ]; then
    echo ""
    echo "RESULT: ❌ FAIL"
    echo "Found $ERROR_COUNT suppression policy errors in $SUPPRESSIONS_FILE"
    exit 1
fi

echo "RESULT: ✅ PASS"
echo "Validated $ENTRY_COUNT suppression entries in $SUPPRESSIONS_FILE"
