#!/usr/bin/env bash
set -euo pipefail

count=0
while IFS= read -r -d '' file; do
  if grep -qE '```[a-zA-Z0-9_+-]+[[:space:]]+--run(\s|$)' "$file"; then
    echo "Validating runmd blocks in $file"
    npx --yes runmd "$file" --output /tmp/runmd-check-output.md >/dev/null
    count=$((count + 1))
  fi
done < <(find . -type f -name '*.md' \
  -not -path './node_modules/*' \
  -not -path './target/*' \
  -not -path './patches/*' \
  -not -path './forge-std/*' \
  -not -path './tests/security/lib/*' \
  -not -path './botchain-tri-vm-genesis/*' \
  -print0)

if [[ $count -eq 0 ]]; then
  echo "No runmd --run code blocks found in markdown files."
else
  echo "Validated runmd examples in $count markdown file(s)."
fi
