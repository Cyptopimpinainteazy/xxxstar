#!/usr/bin/env bash
# Summarize embarrassment-scan evidence files into a compact markdown report.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
EVIDENCE_DIR="${1:-${REPO_ROOT}/launch-gates/evidence/ci}"
OUT_FILE="${2:-${EVIDENCE_DIR}/embarrassment-evidence-summary.md}"

SCAN_LOG="${EVIDENCE_DIR}/proof-embarrassment-scan.log"
RAW_FILE="${EVIDENCE_DIR}/embarrassment-raw-findings.txt"
SUPPRESSED_FILE="${EVIDENCE_DIR}/embarrassment-suppressed-findings.txt"
VALIDATION_FILE="${EVIDENCE_DIR}/embarrassment-suppressions-validation.log"

mkdir -p "$EVIDENCE_DIR"

if [ ! -f "$SCAN_LOG" ]; then
    echo "❌ Missing scan log: $SCAN_LOG"
    exit 1
fi

if [ ! -f "$RAW_FILE" ]; then
    echo "❌ Missing raw findings file: $RAW_FILE"
    exit 1
fi

if [ ! -f "$SUPPRESSED_FILE" ]; then
    echo "❌ Missing suppressed findings file: $SUPPRESSED_FILE"
    exit 1
fi

if [ ! -f "$VALIDATION_FILE" ]; then
    echo "❌ Missing validation file: $VALIDATION_FILE"
    exit 1
fi

extract_metric() {
    local key=$1
    grep -E "^${key}:" "$SCAN_LOG" | tail -1 | sed -E "s/^${key}: *//" || true
}

SCAN_RESULT=$(grep -E '^RESULT:' "$SCAN_LOG" | tail -1 || true)
P0=$(extract_metric 'P0 \(CRITICAL\)')
P1=$(extract_metric 'P1 \(HIGH\)')
P2=$(extract_metric 'P2 \(MEDIUM\)')
P2_BLOCKED=$(extract_metric 'P2 blocked by policy')
SUPPRESSED=$(extract_metric 'Suppressed')
TOTAL=$(extract_metric 'Total')

RAW_COUNT=$(grep -E '^[^[:space:]].*:[0-9]+:' "$RAW_FILE" | wc -l | tr -d '[:space:]')
SUPPRESSED_COUNT=0
if [ -f "$SUPPRESSED_FILE" ]; then
    SUPPRESSED_COUNT=$(tail -n +2 "$SUPPRESSED_FILE" | sed '/^[[:space:]]*$/d' | wc -l | tr -d '[:space:]')
fi

VALIDATION_RESULT=$(grep -E '^RESULT:' "$VALIDATION_FILE" | tail -1 || true)

suppressed_by_category() {
    if [ ! -s "$SUPPRESSED_FILE" ]; then
        return
    fi
    tail -n +2 "$SUPPRESSED_FILE" \
        | sed '/^[[:space:]]*$/d' \
        | cut -d'|' -f1 \
        | sort \
        | uniq -c \
        | awk '{printf "- %s: %s\n", $2, $1}'
}

{
    echo "# Embarrassment Scan Evidence Summary"
    echo
    echo "- Generated: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
    echo "- Evidence directory: ${EVIDENCE_DIR}"
    echo "- Scan result: ${SCAN_RESULT:-unknown}"
    echo "- Suppression validation: ${VALIDATION_RESULT:-unknown}"
    echo
    echo "## Metrics"
    echo
    echo "- P0 (critical): ${P0:-unknown}"
    echo "- P1 (high): ${P1:-unknown}"
    echo "- P2 (medium): ${P2:-unknown}"
    echo "- P2 blocked by policy: ${P2_BLOCKED:-unknown}"
    echo "- Scanner suppressed count: ${SUPPRESSED:-unknown}"
    echo "- Total findings score input: ${TOTAL:-unknown}"
    echo "- Raw findings lines: ${RAW_COUNT}"
    echo "- Suppressed findings entries: ${SUPPRESSED_COUNT}"
    echo
    echo "## Suppressed Findings By Category"
    echo
    CATEGORY_LINES="$(suppressed_by_category || true)"
    if [ -n "$CATEGORY_LINES" ]; then
        echo "$CATEGORY_LINES"
    else
        echo "- none"
    fi
    echo
    echo "## Evidence Files"
    echo
    echo "- ${SCAN_LOG}"
    echo "- ${RAW_FILE}"
    echo "- ${SUPPRESSED_FILE}"
    echo "- ${VALIDATION_FILE}"
} > "$OUT_FILE"

echo "✅ Wrote summary: $OUT_FILE"
