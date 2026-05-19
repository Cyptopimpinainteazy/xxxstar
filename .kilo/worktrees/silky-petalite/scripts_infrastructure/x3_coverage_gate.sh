#!/usr/bin/env bash
# scripts/x3_coverage_gate.sh — Per-subsystem coverage enforcement
#
# Usage:
#   bash scripts/x3_coverage_gate.sh              # full check
#   bash scripts/x3_coverage_gate.sh --install    # install cargo-tarpaulin first
#   bash scripts/x3_coverage_gate.sh --report     # generate HTML report
#
# Thresholds match [workspace.metadata.coverage] in root Cargo.toml.
# Any subsystem below its threshold = hard failure.
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

INSTALL=0
REPORT=0
for arg in "$@"; do
  case "$arg" in
    --install) INSTALL=1 ;;
    --report)  REPORT=1 ;;
  esac
done

# ── Install tarpaulin if requested ────────────────────────────────────────────
if [ "$INSTALL" -eq 1 ]; then
  echo "Installing cargo-tarpaulin..."
  cargo install cargo-tarpaulin --locked
fi

if ! command -v cargo-tarpaulin &>/dev/null && ! cargo tarpaulin --version &>/dev/null 2>&1; then
  echo "❌ cargo-tarpaulin not found. Run: bash scripts/x3_coverage_gate.sh --install"
  exit 1
fi

# ── Thresholds ────────────────────────────────────────────────────────────────
# Format: "crate-name threshold"
declare -A THRESHOLDS=(
  ["x3-constitution"]=90
  ["x3-proof"]=90
  ["x3-launch-validator"]=85
  ["x3-agent"]=80
  ["x3-slash"]=85
  ["x3-verifier"]=85
  ["x3-economics"]=75
  ["x3-sdk"]=80
)

PASS=0
FAIL=0
BELOW=()
REPORT_DIR="$ROOT/target/coverage"
mkdir -p "$REPORT_DIR"

echo ""
echo "╔══════════════════════════════════════════════════════╗"
echo "║   X3 COVERAGE GATE — Per-Subsystem Enforcement      ║"
echo "╠══════════════════════════════════════════════════════╣"

for crate in "${!THRESHOLDS[@]}"; do
  threshold="${THRESHOLDS[$crate]}"
  crate_path=""

  # Find the crate directory
  for candidate in "$ROOT/crates/$crate" "$ROOT/pallets/$crate"; do
    [ -d "$candidate" ] && crate_path="$candidate" && break
  done

  if [ -z "$crate_path" ]; then
    printf "║  %-28s  SKIP  (not found)\n" "$crate"
    continue
  fi

  # Run tarpaulin
  report_file="$REPORT_DIR/${crate}.xml"
  tarp_args=(-p "$crate" --out Xml --output-dir "$REPORT_DIR")
  [ "$REPORT" -eq 1 ] && tarp_args+=(--out Html)

  if raw=$(cargo tarpaulin "${tarp_args[@]}" 2>&1); then
    # Extract line rate from Cobertura XML
    rate=$(grep -oP 'line-rate="\K[0-9.]+' "$report_file" 2>/dev/null | head -1 || echo "0")
    pct=$(echo "$rate * 100" | bc 2>/dev/null | cut -d. -f1 || echo 0)
  else
    pct=0
  fi

  if [ "$pct" -ge "$threshold" ]; then
    printf "║  %-28s  %3d%%  ≥ %d%%  ✅\n" "$crate" "$pct" "$threshold"
    ((PASS++))
  else
    printf "║  %-28s  %3d%%  < %d%%  ❌\n" "$crate" "$pct" "$threshold"
    BELOW+=("$crate (${pct}% < ${threshold}%)")
    ((FAIL++))
  fi
done

echo "╠══════════════════════════════════════════════════════╣"
echo "║  PASS: $PASS   FAIL: $FAIL                                   ║"
echo "╚══════════════════════════════════════════════════════╝"

if [ "$FAIL" -gt 0 ]; then
  echo ""
  echo "❌ COVERAGE FAILURES:"
  for b in "${BELOW[@]}"; do
    echo "   • $b"
  done
  echo ""
  echo "Raise coverage to meet thresholds, then re-run."
  exit 1
fi

echo ""
echo "✅ All coverage gates passed"
[ "$REPORT" -eq 1 ] && echo "   HTML reports in: $REPORT_DIR"
exit 0
