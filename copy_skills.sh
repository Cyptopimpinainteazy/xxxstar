#!/usr/bin/env bash
# Copy all built‑in Copilot skill templates into the local .roo/skills
# directory. This script is idempotent and will overwrite existing files.

SRC="/usr/share/code/resources/app/extensions/copilot/assets/prompts/skills"
DST=".roo/skills"

echo "Copying skills from $SRC to $DST"
mkdir -p "$DST"
cp -r "$SRC"/* "$DST/"
echo "Copy complete."
