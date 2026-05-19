#!/usr/bin/env bash
set -euo pipefail

ARTIFACT_DIR=${1:-artifacts}
DRY_RUN=${2:-0}  # Set to 1 to check what would be redacted without modifying

if [ ! -d "$ARTIFACT_DIR" ]; then
  echo "[sanitizer] No artifact directory found at $ARTIFACT_DIR, nothing to sanitize."
  exit 0
fi

echo "[sanitizer] Sanitizing artifacts under $ARTIFACT_DIR (dry_run=$DRY_RUN)"

# Find log/text/json files and apply strict redaction patterns
find "$ARTIFACT_DIR" -type f \( -iname '*.log' -o -iname '*.txt' -o -iname '*.json' -o -iname '*.csv' -o -iname '*.stdout' -o -iname '*.stderr' \) | while read -r F; do
  echo "[sanitizer]   Processing: $F"
  
  if [ "$DRY_RUN" = "1" ]; then
    # Dry run: only report matches
    if grep -qE '0x[0-9a-fA-F]{20,}|[0-9a-fA-F]{64}|(Bearer|bearer) [A-Za-z0-9._-]{10,}|mnemonic|PRIVATE|SECRET|PASSWORD|API_KEY' "$F" 2>/dev/null || true; then
      echo "[sanitizer]     ⚠️  Matches found in: $F"
    fi
  else
    # Actually sanitize
    sed -E -i 's/0x[0-9a-fA-F]{20,}/0x[REDACTED]/g' "$F" || true
    sed -E -i 's/\b[0-9a-fA-F]{64}\b/[REDACTED_KEY]/g' "$F" || true
    sed -E -i 's/(Bearer|bearer) [A-Za-z0-9._-]{10,}/[REDACTED_TOKEN]/gi' "$F" || true
    sed -E -i 's/\b(mnemonic|seed)\b[^=]*=[^"]+"[^"]*"/[REDACTED_MNEMONIC]/gi' "$F" || true
    sed -E -i 's/^.*\b(PRIVATE|SECRET|PASSWORD|API_KEY|ACCESS_TOKEN|PRIV_KEY|PRIVATE_KEY)\b.*$/[REDACTED_LINE]/i' "$F" || true
  fi
done

# Remove sensitive file types entirely
echo "[sanitizer] Removing sensitive files (*.pem, .env, *.key, .secret, *credentials*)"
find "$ARTIFACT_DIR" -type f \( -iname '*.pem' -o -iname '.env' -o -iname '*.key' -o -iname '.secret' -o -iname '*credentials*' \) -delete -print 2>/dev/null || true

echo "[sanitizer] Sanitization complete (dry_run=$DRY_RUN)"
