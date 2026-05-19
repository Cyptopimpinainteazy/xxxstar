#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "=== X3 Apps Main-Chain Integration Audit ==="

APPS=(dex wallet x3-desktop x3-intelligence validators inferstructor-dashboard explorer)
CHECK_FILES=(
  "apps/dex/next.config.js"
  "apps/wallet/next.config.js"
  "apps/x3-desktop/src/lib/substrate/client.ts"
  "apps/x3-intelligence/src/pages/FloorDashboard.tsx"
  "apps/x3-intelligence/src/services/authService.ts"
  "apps/validators/src/App.tsx"
  "apps/inferstructor-dashboard/src/api.ts"
)

MAIN_WS="wss://ws.x3star.net/ws"
MAIN_HTTP="https://rpc.x3star.net/rpc"
MAIN_API="https://api.x3star.net"

fails=0

for app in "${APPS[@]}"; do
  if [ -d "apps/$app" ]; then
    echo "[OK] app found: $app"
  else
    echo "[FAIL] missing app dir: $app"
    fails=$((fails+1))
  fi
done

echo ""
echo "-- Checking chain endpoint defaults --"

assert_contains() {
  local file="$1"
  local needle="$2"
  local label="$3"
  if grep -q "$needle" "$file"; then
    echo "[OK] $label"
  else
    echo "[FAIL] $label"
    fails=$((fails+1))
  fi
}

assert_contains "apps/dex/next.config.js" "$MAIN_WS" "dex uses main-chain WS fallback"
assert_contains "apps/dex/next.config.js" "$MAIN_HTTP" "dex uses main-chain HTTP fallback"
assert_contains "apps/wallet/next.config.js" "$MAIN_WS" "wallet uses main-chain WS fallback"
assert_contains "apps/wallet/next.config.js" "$MAIN_HTTP" "wallet uses main-chain HTTP fallback"
assert_contains "apps/x3-desktop/src/lib/substrate/client.ts" "$MAIN_WS" "x3-desktop has main-chain WS fallback"
assert_contains "apps/x3-intelligence/src/pages/FloorDashboard.tsx" "$MAIN_WS" "x3-intelligence uses main-chain WS fallback"
assert_contains "apps/x3-intelligence/src/services/authService.ts" "$MAIN_API" "x3-intelligence uses main-chain API fallback"
assert_contains "apps/inferstructor-dashboard/src/api.ts" "$MAIN_API" "inferstructor uses main-chain API fallback"
assert_contains "apps/validators/src/App.tsx" "$MAIN_HTTP" "validators live RPC wired to main-chain endpoint"

echo ""
echo "-- Flagging localhost fallbacks in integration files --"
for file in "${CHECK_FILES[@]}"; do
  if grep -E "localhost|127\\.0\\.0\\.1" "$file" >/dev/null 2>&1; then
    echo "[WARN] $file still contains localhost fallback/reference"
  else
    echo "[OK] $file has no localhost fallback"
  fi
done

echo ""
echo "-- Build sanity for directly changed apps --"
if npm --prefix apps/validators run type-check >/dev/null 2>&1 && npm --prefix apps/validators run build >/dev/null 2>&1; then
  echo "[OK] validators type-check/build"
else
  echo "[FAIL] validators type-check/build"
  fails=$((fails+1))
fi

if npm --prefix apps/inferstructor-dashboard run build >/dev/null 2>&1; then
  echo "[OK] inferstructor-dashboard build"
else
  echo "[FAIL] inferstructor-dashboard build"
  fails=$((fails+1))
fi

echo ""
if [ "$fails" -eq 0 ]; then
  echo "Integration audit PASSED"
else
  echo "Integration audit found $fails hard failures"
  exit 1
fi
