#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT_DIR="$ROOT_DIR/reports"
REPORT="$REPORT_DIR/runtime_upgrade_rehearsal.md"
mkdir -p "$REPORT_DIR"

CURRENT_SHA="$(git -C "$ROOT_DIR" rev-parse HEAD)"
OLD_SHA="$(git -C "$ROOT_DIR" rev-parse HEAD~1 2>/dev/null || true)"

build_current_ok=false
build_old_ok=false
storage_version_ok=false
on_runtime_upgrade_ok=false
dry_run_ok=false

cd "$ROOT_DIR"

if cargo build --release -p x3-chain-runtime >/dev/null 2>&1; then
	build_current_ok=true
fi

if [[ -n "$OLD_SHA" ]]; then
	WORKTREE_DIR="${TMPDIR:-/tmp}/x3-runtime-old-$$"
	cleanup() {
		git -C "$ROOT_DIR" worktree remove --force "$WORKTREE_DIR" >/dev/null 2>&1 || true
	}
	trap cleanup EXIT

	git -C "$ROOT_DIR" worktree add --detach "$WORKTREE_DIR" "$OLD_SHA" >/dev/null 2>&1
	if (cd "$WORKTREE_DIR" && cargo build --release -p x3-chain-runtime >/dev/null 2>&1); then
		build_old_ok=true
	fi
else
	build_old_ok=false
fi

if rg -n "STORAGE_VERSION|\[pallet::storage_version\(" runtime/src/lib.rs pallets/**/src/lib.rs >/dev/null 2>&1; then
	storage_version_ok=true
fi

if rg -n "on_runtime_upgrade" runtime/src/lib.rs pallets/**/src/lib.rs >/dev/null 2>&1; then
	on_runtime_upgrade_ok=true
fi

DRY_RUN_CMD="cargo run --release -p x3-chain-node -- build-spec --chain dev --disable-default-bootnode"
if $DRY_RUN_CMD >/dev/null 2>&1; then
	dry_run_ok=true
fi

result="PASS"
if [[ "$build_current_ok" != true || "$build_old_ok" != true || "$storage_version_ok" != true || "$on_runtime_upgrade_ok" != true || "$dry_run_ok" != true ]]; then
	result="FAIL"
fi

{
	echo "# Runtime Upgrade Rehearsal"
	echo
	echo "- Current commit: $CURRENT_SHA"
	echo "- Previous commit: ${OLD_SHA:-MISSING}"
	echo
	echo "## Checks"
	echo
	echo "- Build current runtime: $build_current_ok"
	echo "- Build previous runtime: $build_old_ok"
	echo "- Storage version metadata present: $storage_version_ok"
	echo "- on_runtime_upgrade hooks present: $on_runtime_upgrade_ok"
	echo "- Dry-run command: $DRY_RUN_CMD"
	echo "- Dry-run success: $dry_run_ok"
	echo
	echo "## Result"
	echo
	echo "Result: $result"
} > "$REPORT"

echo "runtime_upgrade_rehearsal: wrote $REPORT"

if [[ "$result" != "PASS" ]]; then
	exit 1
fi
