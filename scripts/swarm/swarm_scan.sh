#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT_FILE="$ROOT_DIR/reports/swarm_scan_report.md"

mkdir -p "$ROOT_DIR/reports"

{
	echo "# X3 Swarm Repo Scan"
	echo
	echo "Generated at: $(date -u +'%Y-%m-%dT%H:%M:%SZ')"
	echo
	echo "## TODO / FIXME / STUB markers"
	cd "$ROOT_DIR"
	rg -n "TODO|FIXME|STUB|PLACEHOLDER|unimplemented!|todo!|panic!|unwrap\(" \
	  --glob '!target/**' \
	  --glob '!node_modules/**' \
	  --glob '!dist/**' \
	  . || true
	echo
	echo "## Feature files found"
	for feature in \
	  atomic kernel router gateway btc axe forge sentinel reactor signal keeper oracle swarm readiness tauri launch
	do
	  echo "### $feature"
	  find . \
	    -path './target' -prune -o \
	    -path './node_modules' -prune -o \
	    -iname "*$feature*" -print \
	    | head -50 || true
	done
} > "$REPORT_FILE"

echo "Generated reports/swarm_scan_report.md"
