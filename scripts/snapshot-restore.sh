#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# snapshot-restore.sh — Validator snapshot backup & restore for X3 testnet
#
# Usage:
#   backup:  bash scripts/snapshot-restore.sh backup  <validator_base_path>
#   restore: bash scripts/snapshot-restore.sh restore <snapshot_tar> <target_base_path>
#   list:    bash scripts/snapshot-restore.sh list    [snapshot_dir]
#
# Exit codes:
#   0 — success
#   1 — usage / missing args
#   2 — validator not stopped (refusing backup on live node)
#   3 — restore target not empty / exists
#   4 — snapshot file missing or corrupt
#
# Environment variables:
#   X3_SNAPSHOT_DIR   — snapshot staging directory (default: /tmp/x3-snapshots)
#   X3_RPC_ENDPOINT   — for optional pre-backup health check
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

SNAPSHOT_DIR="${X3_SNAPSHOT_DIR:-/tmp/x3-snapshots}"
ACTION="${1:-}"
ARG="${2:-}"

usage() {
    echo "Usage:"
    echo "  backup:  bash scripts/snapshot-restore.sh backup  <validator_base_path>"
    echo "  restore: bash scripts/snapshot-restore.sh restore <snapshot_tar> <target_base_path>"
    echo "  list:    bash scripts/snapshot-restore.sh list    [snapshot_dir]"
    exit 1
}

# ── Pre-checks ──────────────────────────────────────────────────────────────
check_validator_stopped() {
    local base="$1"
    # Check if x3-chain-node is running on this base path
    if pgrep -f "x3-chain-node.*$base" > /dev/null 2>&1; then
        echo -e "${RED}❌ Validator appears to be running on base path: $base${NC}"
        echo "   Stop the validator first: systemctl stop x3-validator (or Ctrl+C / pkill)"
        exit 2
    fi
}

check_not_empty() {
    local path="$1"
    if [[ -d "$path" ]] && [[ -n "$(ls -A "$path" 2>/dev/null)" ]]; then
        echo -e "${RED}❌ Target directory exists and is not empty: $path${NC}"
        echo "   Remove it first: rm -rf $path"
        exit 3
    fi
}

# ── backup ──────────────────────────────────────────────────────────────────
do_backup() {
    local BASE="$1"

    if [[ ! -d "$BASE" ]]; then
        echo -e "${RED}❌ Validator base path not found: $BASE${NC}"
        exit 1
    fi

    check_validator_stopped "$BASE"

    mkdir -p "$SNAPSHOT_DIR"

    local STAMP
    STAMP=$(date -u +%Y%m%dT%H%M%SZ)
    local TAR_NAME="x3-validator-snapshot-${STAMP}.tar.gz"
    local TAR_PATH="$SNAPSHOT_DIR/$TAR_NAME"
    local MANIFEST_PATH="$SNAPSHOT_DIR/$TAR_NAME.manifest"

    echo -e "${BLUE}📦 Creating snapshot from $BASE ...${NC}"

    # Capture metadata before archive
    {
        echo "snapshot: $TAR_NAME"
        echo "timestamp: $(date -u -Iseconds)"
        echo "base_path: $BASE"
        echo "hostname: $(hostname)"
        echo "uname: $(uname -a)"
        echo ""
        echo "--- pre-snapshot db stats ---"
        du -sh "$BASE"/chains 2>/dev/null || echo "(no chains dir)"
        du -sh "$BASE" 2>/dev/null
    } > "$MANIFEST_PATH"

    # Create tarball of the base path, excluding in-memory /tmp files
    tar -czf "$TAR_PATH" \
        --exclude='*.lock' \
        --exclude='node-key' \
        -C "$(dirname "$BASE")" \
        "$(basename "$BASE")"

    local SIZE
    SIZE=$(du -h "$TAR_PATH" | cut -f1)

    # Append post-archive info to manifest
    {
        echo ""
        echo "--- post-snapshot ---"
        echo "size: $SIZE"
        echo "sha256: $(sha256sum "$TAR_PATH" | awk '{print $1}')"
    } >> "$MANIFEST_PATH"

    echo -e "${GREEN}✅ Snapshot created:${NC}"
    echo "   Archive:  $TAR_PATH"
    echo "   Size:     $SIZE"
    echo "   Manifest: $MANIFEST_PATH"
    echo ""
    echo "To restore:"
    echo "   bash scripts/snapshot-restore.sh restore $TAR_PATH <target_base_path>"
}

# ── restore ─────────────────────────────────────────────────────────────────
do_restore() {
    local TAR="$1"
    local TARGET="$2"

    if [[ ! -f "$TAR" ]]; then
        echo -e "${RED}❌ Snapshot archive not found: $TAR${NC}"
        exit 4
    fi

    # Verify tarball integrity
    if ! tar -tzf "$TAR" > /dev/null 2>&1; then
        echo -e "${RED}❌ Snapshot archive is corrupt or unreadable: $TAR${NC}"
        exit 4
    fi

    check_validator_stopped "$TARGET"
    check_not_empty "$TARGET"

    echo -e "${BLUE}🔄 Restoring snapshot to $TARGET ...${NC}"

    mkdir -p "$(dirname "$TARGET")"
    tar -xzf "$TAR" -C "$(dirname "$TARGET")"

    echo -e "${GREEN}✅ Snapshot restored to $TARGET${NC}"
    echo ""
    echo "Next steps:"
    echo "   1. Verify chain spec matches target network"
    echo "   2. Start validator: systemctl start x3-validator (or ./scripts/testnet-full-launch.sh)"
    echo "   3. Monitor finality: tail -f /tmp/x3-testnet-logs/validator1.log"
}

# ── list ────────────────────────────────────────────────────────────────────
do_list() {
    local dir="${1:-$SNAPSHOT_DIR}"
    if [[ ! -d "$dir" ]]; then
        echo -e "${YELLOW}No snapshot directory: $dir${NC}"
        exit 1
    fi
    echo -e "${BLUE}📋 Snapshots in $dir:${NC}"
    echo ""
    printf "%-55s %10s  %s\n" "NAME" "SIZE" "DATE"
    printf "%.0s─" {1..80}; echo ""
    for t in "$dir"/*.tar.gz; do
        [[ -f "$t" ]] || continue
        local name size mtime
        name=$(basename "$t")
        size=$(du -h "$t" | cut -f1)
        mtime=$(stat -c %y "$t" 2>/dev/null | cut -d. -f1 || date -r "$t" '+%Y-%m-%d %H:%M:%S' 2>/dev/null || echo "unknown")
        printf "%-55s %10s  %s\n" "$name" "$size" "$mtime"
    done
}

# ── Dispatch ────────────────────────────────────────────────────────────────
case "$ACTION" in
    backup)
        [[ -z "$ARG" ]] && usage
        do_backup "$ARG"
        ;;
    restore)
        TARGET="${3:-}"
        [[ -z "$ARG" || -z "$TARGET" ]] && usage
        do_restore "$ARG" "$TARGET"
        ;;
    test-restore)
        # End-to-end restore-path test: creates a dummy snapshot dir,
        # backs it up, restores to a new target, and verifies content.
        TMP_SRC=$(mktemp -d)
        TMP_TAR=$(mktemp -d)
        TMP_TARGET=$(mktemp -d)
        echo "test-data-$(date +%s)" > "$TMP_SRC/test-file"
        tar -czf "$TMP_TAR/test-restore.tar.gz" -C "$(dirname "$TMP_SRC")" "$(basename "$TMP_SRC")"
        do_restore "$TMP_TAR/test-restore.tar.gz" "$TMP_TARGET"
        if [[ -f "$TMP_TARGET/$(basename "$TMP_SRC")/test-file" ]]; then
            echo -e "${GREEN}✅ restore-path smoke test passed${NC}"
        else
            echo -e "${RED}❌ restore-path smoke test failed${NC}"
        fi
        rm -rf "$TMP_SRC" "$TMP_TAR" "$TMP_TARGET"
        ;;
    list)
        do_list "${2:-$SNAPSHOT_DIR}"
        ;;
    *)
        usage
        ;;
esac