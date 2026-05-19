#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT_FILE="$ROOT_DIR/reports/swarm_health_report.md"

mkdir -p "$ROOT_DIR/reports"

echo "# X3 Swarm Health Report" > "$REPORT_FILE"
echo >> "$REPORT_FILE"
echo "## Files" >> "$REPORT_FILE"

for path in \
  crates/x3-swarm-core \
  services/x3-swarm-api \
  data/agent-memory \
  scripts/swarm \
  FEATURE_REGISTRY.toml
do
  if [ -e "$ROOT_DIR/$path" ]; then
    echo "- [OK] $path" >> "$REPORT_FILE"
  else
    echo "- [MISSING] $path missing" >> "$REPORT_FILE"
  fi
done

echo >> "$REPORT_FILE"
echo "## API" >> "$REPORT_FILE"

if curl -fsS http://127.0.0.1:8787/health > /tmp/x3-swarm-health.json 2>/dev/null; then
  echo "- [OK] x3-swarm-api healthy" >> "$REPORT_FILE"
  cat /tmp/x3-swarm-health.json >> "$REPORT_FILE"
else
  echo "- [WARN] x3-swarm-api not reachable" >> "$REPORT_FILE"
fi

cat "$REPORT_FILE"

