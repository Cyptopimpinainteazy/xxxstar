#!/usr/bin/env bash
set -euo pipefail

HOOK_DIR=".git/hooks"
TEMPLATE_DIR=".githooks"

if [ ! -d ".git" ]; then
  echo "This repository does not appear to have a .git directory. Run this from the repo root."
  exit 1
fi

mkdir -p "$HOOK_DIR"
for f in "$TEMPLATE_DIR"/*; do
  name=$(basename "$f")
  cp -f "$f" "$HOOK_DIR/$name"
  chmod +x "$HOOK_DIR/$name"
  echo "Installed $HOOK_DIR/$name"
done

echo "
Done. Git hooks installed. You can also enable pre-commit framework by running:
  pip3 install --user pre-commit || pip install --user pre-commit
  pre-commit install
"