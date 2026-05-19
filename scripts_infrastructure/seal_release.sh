#!/usr/bin/env bash
# ☢️ YOLO FINISHER v5.0 — RELEASE SEAL SCRIPT
# Generates a cryptographic seal for a repository that has achieved 100/100 score.

set -euo pipefail

REPO_ROOT="$(pwd)"
SCORE_FILE="${REPO_ROOT}/SCORE_REPORT.json"
SEAL_FILE="${REPO_ROOT}/RELEASE_SEAL.json"

echo "☢️ YOLO FINISHER — Release Sealing Process"
echo "────────────────────────────────────────"

if [ ! -f "$SCORE_FILE" ]; then
    echo "❌ ERROR: SCORE_REPORT.json not found."
    echo "   Run completion-judge first."
    exit 1
fi

SCORE=$(grep -o '"total_score": [0-9]\+' "$SCORE_FILE" | awk '{print $2}')

if [ "$SCORE" -lt 100 ]; then
    echo "❌ ERROR: Readiness Score ($SCORE/100) is too low for sealing."
    echo "   Failing gates must be resolved before release."
    exit 1
fi

echo "✅ Score Verified: 100 / 100"
echo "🔐 Generating Release Seal..."

TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
COMMIT_HASH=$(git rev-parse HEAD || echo "untracked")
FILES_HASH=$(find . -type f -not -path '*/.*' | sort | xargs sha256sum | sha256sum | awk '{print $1}')

cat > "$SEAL_FILE" <<EOF
{
  "system": "YOLO FINISHER v5.0",
  "status": "SEALED",
  "readiness_score": $SCORE,
  "timestamp": "$TIMESTAMP",
  "commit": "$COMMIT_HASH",
  "merkle_root": "$FILES_HASH",
  "attestation": "This repository has been audited by the nuclear finisher stack and meets all hard gates: zero s, zero vulnerabilities, full integration, verified symmetry."
}
EOF

echo "────────────────────────────────────────"
echo "☢️  RELEASE SEAL GENERATED: $SEAL_FILE"
echo "   Repo is officially PRODUCTION-READY."
chmod 444 "$SEAL_FILE" # Make read-only
