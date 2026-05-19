#!/usr/bin/env bash
set -euo pipefail

mkdir -p .cache .reports

OLD_PROJECT_ROOT="${OLD_PROJECT_ROOT:-../old-x3-project}"
roots=(.)

if [ -d "$OLD_PROJECT_ROOT" ]; then
  roots+=("$OLD_PROJECT_ROOT")
else
  printf 'old project root not found: %s\n' "$OLD_PROJECT_ROOT" > .reports/old_project_missing.txt
fi

find "${roots[@]}" \
  -type f \
  -not -path "*/node_modules/*" \
  -not -path "*/target/*" \
  -not -path "*/.git/*" \
  -not -path "*/dist/*" \
  -not -path "*/build/*" \
  | sort > .cache/full_file_list.txt

cp .cache/full_file_list.txt .cache/file_list.txt

wc -l .cache/full_file_list.txt
