#!/usr/bin/env bash
# A migration module that no crate declares is not a migration.
#
# The storage audit found `migrations.rs` files sitting under pallets whose
# `lib.rs` never declares them: uncompiled, untested, and at a glance
# indistinguishable from the four the runtime actually runs. Editing one of
# those would have changed nothing, and a real storage-layout change routed
# through one would have shipped silently. Five such files existed when this
# gate was added; they were deleted rather than wired, because none of those
# pallets had ever bumped its storage version, so there was nothing to migrate.
#
# The rule this enforces: if a `migrations.rs` exists, its crate must declare the
# module, which means it is compiled, linted, and available to test.
#
# Usage: check_migration_modules_are_wired.sh
# Exit 0 = every migration module is reachable; 1 = at least one orphan exists.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

orphans=0

for file in "$REPO_ROOT"/pallets/*/src/migrations.rs; do
  [ -e "$file" ] || continue

  pallet_dir="$(dirname "$(dirname "$file")")"
  lib="$pallet_dir/src/lib.rs"

  if [[ ! -f "$lib" ]]; then
    echo "FAIL  $file exists but $lib does not"
    orphans=$((orphans + 1))
    continue
  fi

  # Accept `mod migrations;`, `pub mod migrations;` and `pub(crate) mod migrations;`.
  if ! grep -qE '^[[:space:]]*(pub(\([a-z]+\))?[[:space:]]+)?mod[[:space:]]+migrations[[:space:]]*;' "$lib"; then
    echo "FAIL  ${file#"$REPO_ROOT"/} is never declared by ${lib#"$REPO_ROOT"/}"
    orphans=$((orphans + 1))
  fi
done

if [[ "$orphans" -gt 0 ]]; then
  echo ""
  echo "$orphans orphan migration module(s). Either wire them — declare the module and add the"
  echo "pallet's Migration to the runtime Migrations tuple — or delete them. An unreachable"
  echo "migration module is a claim that storage upgrades are handled when they are not."
  exit 1
fi

echo "OK - every migrations.rs under pallets/ is declared by its crate"
