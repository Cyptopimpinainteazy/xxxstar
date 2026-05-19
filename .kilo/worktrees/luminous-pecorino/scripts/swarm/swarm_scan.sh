#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT_FILE="$ROOT_DIR/reports/swarm_scan_report.md"

mkdir -p "$ROOT_DIR/reports"

{
	echo "# X3 Swarm Scan Report"
	echo
	echo "Generated at: $(date -u +'%Y-%m-%dT%H:%M:%SZ')"
	echo
	echo "## Source Files"
	echo '```'
	cd "$ROOT_DIR"
	find . \
		-path './.git' -prune -o \
		-path './node_modules' -prune -o \
		-path './target' -prune -o \
		-type f \( -name '*.rs' -o -name '*.sh' -o -name '*.toml' -o -name '*.md' \) \
		-print | sed 's#^./##' | sort
	echo '```'
} > "$REPORT_FILE"

echo "X3 swarm scan report written to $REPORT_FILE"
