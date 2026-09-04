#!/usr/bin/env bash
set -euo pipefail
# x3-detect-stubs.sh — search for fake-completion markers in source files

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

CRITICAL_PATHS="runtime/ pallets/ bridges/ adapters/ crates/x3-gateway/ crates/cross-vm-bridge/ crates/atomic-swap-orchestrator/ crates/flash-finality/ X3-contracts/ node/"
STUB_PATTERNS='TODO|FIXME|STUB|MOCK|NOOP|placeholder|fake|dummy|temporary|not implemented|unimplemented!|todo!|panic!\("not implemented"\)|panic!\("stub"\)|return Ok\(\(\)\)'

STUB_COUNT=0
CRITICAL_STUB_COUNT=0

echo "=== Stub Detection — $(date -u +%Y-%m-%dT%H:%M:%SZ) ==="

# Search source files only — skip .git, target, node_modules, .venv
FILES=$(find "$REPO_ROOT" \
    -type f \
    \( -name '*.rs' -o -name '*.sol' -o -name '*.ts' -o -name '*.tsx' -o -name '*.js' -o -name '*.py' -o -name '*.move' -o -name '*.cairo' \) \
    -not -path '*/.git/*' \
    -not -path '*/target/*' \
    -not -path '*/node_modules/*' \
    -not -path '*/.venv/*' \
    -not -path '*/forge-std/*' \
    -not -path '*/vendor/*' \
    2>/dev/null)

for file in $FILES; do
    if matches=$(grep -n -E "$STUB_PATTERNS" "$file" 2>/dev/null); then
        while IFS= read -r line; do
            STUB_COUNT=$((STUB_COUNT + 1))
            # Check if in critical path
            rel_path="${file#$REPO_ROOT/}"
            is_critical=0
            for cp in $CRITICAL_PATHS; do
                if [[ "$rel_path" == $cp* ]]; then
                    is_critical=1
                    break
                fi
            done
            
            if [ "$is_critical" -eq 1 ]; then
                CRITICAL_STUB_COUNT=$((CRITICAL_STUB_COUNT + 1))
                echo "[CRITICAL] $rel_path:$line"
            else
                echo "[WARN] $rel_path:$line"
            fi
        done <<< "$matches"
    fi
done

echo ""
echo "=== Stub Detection Summary ==="
echo "Total suspicious markers: $STUB_COUNT"
echo "Critical path markers: $CRITICAL_STUB_COUNT"

if [ "$CRITICAL_STUB_COUNT" -gt 0 ]; then
    echo "VERDICT: FAIL — $CRITICAL_STUB_COUNT critical-path stubs found."
    exit 1
else
    echo "VERDICT: PASS — No critical-path stubs found."
    exit 0
fi