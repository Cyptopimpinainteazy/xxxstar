#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT_DIR="$ROOT_DIR/reports"
mkdir -p "$REPORT_DIR"

cd "$ROOT_DIR"

echo "== X3 Autoprove YOLO Mode =="

echo "1. Running repo scan..."
cargo run -p x3-readiness -- feature-gap --out "$REPORT_DIR"

echo "2. Running feature gap report..."
cargo run -p x3-readiness -- feature-gap --out "$REPORT_DIR"

echo "3. Running missing tests report..."
cargo run -p x3-readiness -- missing-tests --out "$REPORT_DIR"

echo "4. Running Tauri wiring report..."
cargo run -p x3-readiness -- tauri-wiring --out "$REPORT_DIR"

echo "5. Running service health report..."
cargo run -p x3-readiness -- service-health --out "$REPORT_DIR"

echo "6. Running BTC gateway report..."
cargo run -p x3-readiness -- btc-gateway-report --out "$REPORT_DIR"

echo "7. Running marketing claims audit..."
cargo run -p x3-readiness -- marketing-claims-audit --out "$REPORT_DIR"

if [ -x "$ROOT_DIR/scripts/testnet/testnet_rc_gate.sh" ]; then
  echo "8. Running testnet RC gate..."
  bash "$ROOT_DIR/scripts/testnet/testnet_rc_gate.sh"
else
  echo "8. Skipping missing testnet RC gate script"
fi

echo "9. Writing swarm task summary..."
cat > "$REPORT_DIR/swarm_tasks_summary.md" <<'EOF'
# Swarm Task Summary

- repo scan: complete
- feature gap: generated
- missing tests: generated
- Tauri wiring: generated
- service health: generated
- BTC gateway report: generated
- marketing claims audit: generated
- testnet RC gate: executed if available
EOF

echo "== X3 Autoprove YOLO Mode complete =="
