#!/usr/bin/env bash
set -euo pipefail

mkdir -p .repomix .cache .reports

OLD_PROJECT_ROOT="${OLD_PROJECT_ROOT:-../old-x3-project}"
REPOMIX_BIN="${REPOMIX_BIN:-}"

if [ -n "$REPOMIX_BIN" ]; then
  repomix_cmd=("$REPOMIX_BIN")
elif command -v repomix >/dev/null 2>&1; then
  repomix_cmd=(repomix)
else
  repomix_cmd=(npx -y repomix@latest)
fi

common_ignore="node_modules,target,target_strict,.git,dist,build,coverage,.cache,.reports,.repomix,launch-gates/repomix,*.wasm,*.so,*.o"

echo "== Packing current X3 Atomic Star repo =="
"${repomix_cmd[@]}" . \
  --style markdown \
  --output .repomix/current_x3_atomic_star.md \
  --ignore "$common_ignore"

if [ -d "$OLD_PROJECT_ROOT" ]; then
  echo "== Packing old X3 project: $OLD_PROJECT_ROOT =="
  "${repomix_cmd[@]}" "$OLD_PROJECT_ROOT" \
    --style markdown \
    --output .repomix/old_x3_project.md \
    --ignore "$common_ignore"
else
  printf '# Old X3 Project Missing\n\nOLD_PROJECT_ROOT not found: `%s`\n' "$OLD_PROJECT_ROOT" > .repomix/old_x3_project.md
  printf 'old project root not found: %s\n' "$OLD_PROJECT_ROOT" > .reports/old_project_missing.txt
fi

echo "== Packing focused runtime / bridge / DEX context =="
"${repomix_cmd[@]}" . \
  --style markdown \
  --output .repomix/x3_runtime_bridge_dex.md \
  --include "runtime/**,crates/**,pallets/**,contracts/**,programs/**,X3-contracts/**" \
  --ignore "$common_ignore"

wc -c .repomix/*.md

{
  echo "# X3 Repomix Manifest"
  echo
  echo "- Generated: $(date -Iseconds)"
  echo "- Current repo: $(pwd)"
  echo "- Old project root: ${OLD_PROJECT_ROOT}"
  echo
  echo "## Packs"
  for pack in .repomix/*.md; do
    [ "$(basename "$pack")" = "MANIFEST.md" ] && continue
    bytes="$(wc -c < "$pack" | tr -d ' ')"
    echo "- \`${pack}\` - ${bytes} bytes"
  done
  echo
  if [ -f .reports/old_project_missing.txt ]; then
    echo "## Blockers"
    echo
    echo "- OLD_PROJECT_ROOT missing: ${OLD_PROJECT_ROOT}"
    echo "- Old/current comparison is incomplete until OLD_PROJECT_ROOT points at the old X3 repo."
  fi
} > .repomix/MANIFEST.md

echo "== Wrote .repomix/MANIFEST.md =="
