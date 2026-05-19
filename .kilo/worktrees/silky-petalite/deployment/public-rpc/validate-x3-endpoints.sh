#!/usr/bin/env bash
set -euo pipefail

# Validates that public-facing nginx server_name entries are `.x3`.
#
# Default scope: deployment/public-rpc/nginx/*.conf*.example
#
# Usage:
#   deployment/public-rpc/validate-x3-endpoints.sh
#   deployment/public-rpc/validate-x3-endpoints.sh path/to/nginx.conf

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

shopt -s nullglob

files=("$@")
if [[ ${#files[@]} -eq 0 ]]; then
  files=("$ROOT_DIR/deployment/public-rpc/nginx/"*.conf*)
fi

fail=0

for f in "${files[@]}"; do
  if [[ ! -f "$f" ]]; then
    echo "[x3-validate] Skipping missing file: $f" >&2
    continue
  fi

  while IFS= read -r line; do
    # Strip comments
    line="${line%%#*}"

    # Match: server_name a b c;
    if [[ "$line" =~ ^[[:space:]]*server_name[[:space:]]+([^;]+)\; ]]; then
      names="${BASH_REMATCH[1]}"
      for name in $names; do
        # nginx wildcard/placeholder cases
        if [[ "$name" == "_" ]]; then
          continue
        fi
        # Ignore localhost and bare IPs
        if [[ "$name" == "localhost" ]] || [[ "$name" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
          continue
        fi

        if [[ "$name" != *.x3 ]]; then
          echo "[x3-validate] FAIL: $f has non-.x3 server_name: $name" >&2
          fail=1
        fi
      done
    fi
  done < "$f"
done

if [[ $fail -ne 0 ]]; then
  echo "[x3-validate] Validation failed" >&2
  exit 1
fi

echo "[x3-validate] OK: all checked server_name entries are .x3"