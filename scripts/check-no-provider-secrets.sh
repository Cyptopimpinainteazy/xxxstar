#!/usr/bin/env bash
# Refuse to commit a paid RPC provider's key, or a signing key, into tracked source.
#
# On 2026-09-23 this repository had four live secrets in its own source: an Alchemy key, a DRPC key
# and an Ankr key in `crates/external-chains` (twice — the config defaults and the endpoint table),
# a wallet **private key** beside them, and an Infura key in `infra/mcp-config.json` repeated six
# times. They were removed, but removal does not un-expose them: git history still has them, so the
# operator has to rotate. What this script prevents is the next one.
#
# It scans `git ls-files` — what would actually be committed — and prints the file and line and the
# *kind* of match. It never prints the matched value: a secret leaked into a CI log is still leaked,
# and this script runs in CI.
#
# Third-party trees are skipped: `forge-std` vendors Infura's public demo key upstream, which is not
# a secret of ours and not ours to edit.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

ALLOW_RE="$ROOT/scripts/allowed-provider-secrets.txt"

# keyed provider URLs (a bare host is fine; a key in the path is not) and key-shaped local secrets
PATTERNS=(
  'lb\.drpc\.org/[A-Za-z0-9_-]{20,}'
  'rpc\.ankr\.com/[a-z]+/[0-9a-f]{32,}'
  '[a-z0-9-]+\.g\.alchemy\.com/v2/[A-Za-z0-9_-]{20,}'
  'infura\.io/v3/[0-9a-f]{16,}'
  'helius-rpc\.com/\?api-key=[A-Za-z0-9-]{16,}'
  '[a-z0-9-]+\.quiknode\.pro/[A-Za-z0-9]{20,}'
  '(private_key|privateKey|secret_key|secretKey)["'"'"']?\s*[:=]\s*["'"'"']0x?[0-9a-fA-F]{64}["'"'"']'
)

failures=0
for pattern in "${PATTERNS[@]}"; do
  while IFS= read -r hit; do
    [ -z "$hit" ] && continue
    file="${hit%%:*}"
    # third-party trees, and an explicit allow-list for anything audited as intentional
    case "$file" in
      */forge-std/*|forge-std/*|*node_modules/*|*vendor/*|*tauri-vendor/*) continue ;;
    esac
    if [ -f "$ALLOW_RE" ] && grep -qF "$file" "$ALLOW_RE"; then continue; fi
    line="${hit#*:}"; line="${line%%:*}"
    printf 'provider-secret: %s:%s matches %s\n' "$file" "$line" "$pattern"
    failures=$((failures + 1))
  done < <(git ls-files -z -- "$ROOT" 2>/dev/null | xargs -0 -r grep -nIE "$pattern" 2>/dev/null | sed "s#^$ROOT/##" || true)
done

if [ "$failures" -ne 0 ]; then
  echo
  echo "FAIL: $failures keyed value(s) in tracked source. Read the key from the environment instead"
  echo "      (ALCHEMY_API_KEY / DRPC_API_KEY / ANKR_API_KEY / INFURA_API_KEY, written up in"
  echo "      docs/reports/PUBLIC_TESTNET_LAUNCH.md) and rotate anything that has already been pushed."
  exit 1
fi

echo "[no-provider-secrets] ok — no keyed provider URLs or signing keys in tracked files"
