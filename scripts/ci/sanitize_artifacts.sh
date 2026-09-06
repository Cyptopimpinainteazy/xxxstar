#!/usr/bin/env bash
set -euo pipefail

ARTIFACT_DIR=${1:-artifacts}
DRY_RUN=${2:---dry-run}

if [ ! -d "$ARTIFACT_DIR" ]; then
  echo "No artifact directory found at $ARTIFACT_DIR, nothing to sanitize."
  exit 0
fi

echo "Sanitizing artifacts under $ARTIFACT_DIR (dry_run=$DRY_RUN)"

# Counter for issues found
found_count=0

# PATTERNS: common secrets, PII, credentials that should never leak
patterns=(
  # Hex private keys (64 hex char strings, common in crypto)
  '\b[0-9a-fA-F]{64}\b'
  # Ethereum-like addresses (0x + 40 hex)
  '0x[0-9a-fA-F]{40}\b'
  # BTC addresses (26-35 alphanumeric)
  '\b[13][a-km-zA-HJ-NP-Z1-9]{25,34}\b'
  # Mnemonics (12+ English words in sequence)
  '\b(abandon|ability|able|about|above|absent|absorb|abstract)\s+(abandon|ability|...\s+){10,}\b'
  # AWS keys (AKIA + 16 alphanumeric)
  'AKIA[0-9A-Z]{16}'
  # Generic secret/private key patterns
  'secret.*=.*[a-zA-Z0-9+/]{20,}'
  'private[_-]?key.*=.*[^ \n]+'
  'password.*=.*[^ \n]+'
  'bearer\s+[a-zA-Z0-9._-]{20,}'
  # Long hex strings (likely keys)
  '0x[0-9a-fA-F]{20,}'
  # Database connection strings with passwords
  'postgresql?://[^:]+:[^@]+@'
  'mysql://[^:]+:[^@]+@'
  # SSH keys (-----BEGIN PRIVATE KEY-----)
  '-----BEGIN.*PRIVATE.*KEY-----'
  # API tokens in common formats
  'api[_-]?token.*=.*[^ \n]+'
  'X-API-Key.*:.*[^ \n]+'
)

# Find log/text/json/csv/md files
find "$ARTIFACT_DIR" -type f \( -iname '*.log' -o -iname '*.txt' -o -iname '*.json' -o -iname '*.csv' -o -iname '*.md' \) | while read -r F; do
  for pattern in "${patterns[@]}"; do
    if grep -qiE "$pattern" "$F" 2>/dev/null; then
      echo "⚠️  MATCH in $F: pattern=$pattern"
      found_count=$((found_count + 1))
      
      if [ "$DRY_RUN" != "--dry-run" ]; then
        # Apply sanitization: replace matches with redacted placeholder
        sed -E -i "s|$pattern|[REDACTED]|gi" "$F" || echo "Failed to sanitize $F"
      fi
    fi
  done
done

# Remove sensitive files entirely
sensitive_files=(
  '.env'
  '.env.local'
  '*.pem'
  '*.key'
  '*_key'
  'private*'
  'secret*'
)

for pattern in "${sensitive_files[@]}"; do
  find "$ARTIFACT_DIR" -type f -iname "$pattern" | while read -r F; do
    echo "🗑️  REMOVE $F (sensitive filename)"
    if [ "$DRY_RUN" != "--dry-run" ]; then
      rm -f "$F"
    fi
  done
done

if [ "$DRY_RUN" = "--dry-run" ]; then
  echo "✓ DRY-RUN complete: $found_count potential issues found (no files modified)"
  exit 0
else
  echo "✓ Sanitization complete: $found_count instances redacted"
  exit 0
fi
