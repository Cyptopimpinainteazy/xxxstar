#!/usr/bin/env bash
# ProofForge - Dashboard Publisher
# Generates and publishes dashboard metrics
# Usage: ./scripts/publish-dashboard.sh [output_dir]

set -euo pipefail

REPO_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
PROOF_BINARY="${REPO_ROOT}/target/release/x3-proof"
OUTPUT_DIR="${1:-${REPO_ROOT}/dashboard}"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

log_header() {
    echo -e "${CYAN}════════════════════════════════════════${NC}"
    echo -e "${CYAN}  $1${NC}"
    echo -e "${CYAN}════════════════════════════════════════${NC}"
}

log_step() {
    echo -e "\n${BLUE}▶ $1${NC}"
}

log_pass() {
    echo -e "${GREEN}✓ $1${NC}"
}

log_fail() {
    echo -e "\033[0;31m✗ $1${NC}" >&2
}

main() {
    log_header "ProofForge Dashboard Publisher"

    # A stale repository-root proof-score.json used to be copied straight into the
    # published output. It is a hand-made artifact with no recorded evidence, so
    # publication stops here rather than serving it.
    if [ -f "${REPO_ROOT}/proof-score.json" ]; then
        log_fail "stale repository-root proof-score.json present; refusing to publish an artifact"
        log_fail "with no recorded evidence (delete it, or publish the generator's output instead)"
        exit 1
    fi

    # Create output directory
    mkdir -p "$OUTPUT_DIR"
    log_pass "Output directory: $OUTPUT_DIR"
    
    # Always build. This used to build only when the binary was absent, so a
    # stale `target/release/x3-proof` published the numbers of whatever code was
    # current when it was last compiled — which is how a build taken before the
    # dashboard generator stopped inventing a score kept publishing "A- / 0.92".
    # Cargo makes this a no-op when nothing changed.
    log_step "Building ProofForge binary..."
    cd "$REPO_ROOT"
    # The pipeline used to swallow the build's exit status (`| tail -3`), so a
    # failed build still produced a dashboard.
    if ! cargo build -p proof-forge --release 2>&1 | tail -3; then
        log_fail "could not build proof-forge; nothing published"
        exit 1
    fi
    [ -x "$PROOF_BINARY" ] || { log_fail "build produced no $PROOF_BINARY"; exit 1; }
    
    log_step "Generating dashboard data..."
    
    # Generate main dashboard JSON
    local dashboard_file="${OUTPUT_DIR}/proof-score.json"
    if "$PROOF_BINARY" dashboard --output "$dashboard_file" -v > /dev/null 2>&1; then
        log_pass "Dashboard generated: $dashboard_file"
    else
        # This branch used to `log_pass "Dashboard export completed"` — a failure
        # reported as a success.
        log_fail "the dashboard generator failed; nothing published"
        exit 1
    fi
    

    # ── a dashboard is a claim about evidence ────────────────────────────────
    #
    # Everything below is read from the generator's JSON. This script used to
    # invent a second set of numbers (0.94 / "A-" / 20 modules verified / a CSV of
    # per-module scores and VERIFIED statuses) on top of whatever the generator
    # said, and it logged a *pass* when the generator failed. Nothing here writes
    # a number that did not come from `proof-score.json`.
    if [ ! -s "$dashboard_file" ]; then
        log_fail "the dashboard generator wrote no output; nothing to publish"
        exit 1
    fi

    python3 - "$dashboard_file" "$OUTPUT_DIR" <<'PYEOF'
import json, os, sys

dashboard_path, out_dir = sys.argv[1], sys.argv[2]
with open(dashboard_path, encoding="utf-8") as handle:
    dashboard = json.load(handle)

status = dashboard.get("overall_status", "Unverified")
score = dashboard.get("overall_score")
grade = dashboard.get("grade", "Not assessed")
reason = dashboard.get("reason", "")
areas = dashboard.get("areas_proven") or []
coverage = dashboard.get("test_coverage") or {}

# metadata.json: the generator's values, plus provenance. No constants.
metadata = {
    "generated_at": dashboard.get("timestamp"),
    "source": os.path.basename(dashboard_path),
    "overall_status": status,
    "overall_score": score,
    "grade": grade,
    "reason": reason,
    "areas_proven": len(areas),
}
with open(os.path.join(out_dir, "metadata.json"), "w", encoding="utf-8") as handle:
    json.dump(metadata, handle, indent=2)
    handle.write("\n")

# module-scores.csv: only areas the dashboard actually carries. Empty means the
# header alone — a claim with no evidence is not a row.
with open(os.path.join(out_dir, "module-scores.csv"), "w", encoding="utf-8") as handle:
    handle.write("Area,Score,Grade,Status,Proven,Total\n")
    for area in areas:
        handle.write(
            "{},{},{},{},{},{}\n".format(
                area.get("area", ""), area.get("score", ""), area.get("grade", ""),
                area.get("status", ""), area.get("proven_claims", 0), area.get("total_claims", 0),
            )
        )

def row(label, value):
    return f"<tr><td>{label}</td><td>{value}</td></tr>"

score_text = "not scored" if score in (None, 0.0) else f"{score}"
coverage_rows = "".join(
    row(key.replace("_", " "), value) for key, value in coverage.items()
)
area_rows = "".join(
    row(area.get("area", ""), f"{area.get('score', '')} {area.get('status', '')}") for area in areas
) or '<tr><td colspan="2">no area has recorded evidence</td></tr>'

html = f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>X3 ProofForge Dashboard</title>
<style>
 body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
        background: #0f1419; color: #e1e8ed; padding: 24px; }}
 .container {{ max-width: 900px; margin: 0 auto; }}
 h1 {{ color: #1da1f2; margin-bottom: 4px; }}
 .subtitle {{ color: #aab8c2; margin-bottom: 24px; }}
 table {{ width: 100%; border-collapse: collapse; margin: 16px 0; }}
 td, th {{ border: 1px solid #38444d; padding: 10px 12px; text-align: left; }}
 th {{ background: rgba(29,161,242,0.1); }}
 .status {{ font-size: 1.6em; font-weight: bold; color: #f5a623; }}
 .reason {{ background: rgba(245,166,35,0.1); border-left: 4px solid #f5a623; padding: 12px; margin: 16px 0; }}
 code {{ color: #8ab4f8; }}
</style>
</head>
<body>
<div class="container">
  <h1>X3 ProofForge Dashboard</h1>
  <p class="subtitle">Generated {dashboard.get('timestamp', '')} from <code>{os.path.basename(dashboard_path)}</code></p>

  <p>Overall status: <span class="status">{status}</span></p>
  <table>
    {row("Overall score", score_text)}
    {row("Grade", grade)}
  </table>
  {"<div class='reason'><strong>Why:</strong> " + reason + "</div>" if reason else ""}

  <h2>Test coverage as recorded</h2>
  <table><tbody>{coverage_rows}</tbody></table>

  <h2>Areas with recorded evidence</h2>
  <table><tbody>{area_rows}</tbody></table>

  <p class="subtitle">This page publishes what <code>proof-score.json</code> says. It carries no
  number that was not produced by <code>x3-proof dashboard</code>, and a run with no recorded
  evidence is reported as unverified rather than graded.</p>
</div>
</body>
</html>
"""
with open(os.path.join(out_dir, "index.html"), "w", encoding="utf-8") as handle:
    handle.write(html)

print(f"published: status={status} score={score_text} grade={grade} areas={len(areas)}")
PYEOF

    log_pass "Dashboard published from the generator's own output"

    log_step "Dashboard Publication Complete"
    echo ""
    echo "📊 Dashboard Files Generated:"
    ls -lh "$OUTPUT_DIR"/ | tail -n +2 | awk '{print "   " $9 " (" $5 ")"}'
    echo ""
    echo "📖 View Dashboard:"
    echo "   Open: file://${OUTPUT_DIR}/index.html"
}

main "$@"
