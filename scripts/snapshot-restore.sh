#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# snapshot-restore.sh — Validator snapshot backup & restore for X3 testnet
#
# Usage:
#   backup:  bash scripts/snapshot-restore.sh backup  <validator_base_path>
#   restore: bash scripts/snapshot-restore.sh restore <snapshot_tar> <target_base_path>
#   list:    bash scripts/snapshot-restore.sh list    [snapshot_dir]
#   verify:  bash scripts/snapshot-restore.sh verify-snapshot <manifest.json> <chunks_dir> \
#                [--chain-id <id> --block-hash <0x..> --state-root <0x..> --runtime-version <n>]
#   rebuild: bash scripts/snapshot-restore.sh rebuild <manifest.json> <chunks_dir> <out_spec.json> \
#                --chain-id <id> --block-hash <0x..> --state-root <0x..> --runtime-version <n> \
#                [--from-spec <template.json>] [--state-version 0|1] [--force]
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
#   X3_SNAPSHOT_VERIFIER — path to the x3-state-snapshot binary (else target/{release,debug})
#
# Set X3_SNAPSHOT_MANIFEST (plus X3_SNAPSHOT_CHUNKS and the anchor vars below) to
# make `restore` verify a content-addressed snapshot before it writes anything:
#   X3_SNAPSHOT_MANIFEST        manifest.json for the snapshot being restored
#   X3_SNAPSHOT_CHUNKS          directory holding the <index>.chunk files
#   X3_SNAPSHOT_CHAIN_ID        chain id from consensus
#   X3_SNAPSHOT_BLOCK_HASH      finalized block hash from consensus
#   X3_SNAPSHOT_STATE_ROOT      state root of that finalized block
#   X3_SNAPSHOT_RUNTIME_VERSION runtime spec_version of that block
#   X3_SNAPSHOT_STATE_VERSION   trie layout, 0 or 1 (default 1)
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
    echo "  verify:  bash scripts/snapshot-restore.sh verify-snapshot <manifest.json> <chunks_dir> \\"
    echo "               [--chain-id <id> --block-hash <0x..> --state-root <0x..> --runtime-version <n>]"
    echo "  rebuild: bash scripts/snapshot-restore.sh rebuild <manifest.json> <chunks_dir> <out_spec.json> \\"
    echo "               --chain-id <id> --block-hash <0x..> --state-root <0x..> --runtime-version <n>"
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

# ── content-addressed snapshot verification ─────────────────────────────────

# Locate the x3-state-snapshot verifier: explicit override first, then a built
# binary. Prints the path and returns 0, or returns 1 when there is none.
resolve_snapshot_verifier() {
    if [[ -n "${X3_SNAPSHOT_VERIFIER:-}" ]]; then
        printf '%s' "$X3_SNAPSHOT_VERIFIER"
        return 0
    fi

    local repo_root candidate
    repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    for candidate in target/release/x3-state-snapshot target/debug/x3-state-snapshot; do
        if [[ -x "$repo_root/$candidate" ]]; then
            printf '%s' "$repo_root/$candidate"
            return 0
        fi
    done

    return 1
}

# verify_snapshot_dir <manifest.json> <chunks_dir> [extra verifier args...]
verify_snapshot_dir() {
    local manifest="$1"
    shift
    local chunks_dir="$1"
    shift

    local verifier
    if ! verifier="$(resolve_snapshot_verifier)"; then
        echo -e "${RED}❌ x3-state-snapshot verifier not found${NC}"
        echo "   Build it:   cargo build --release -p x3-state-snapshot"
        echo "   Or set:     X3_SNAPSHOT_VERIFIER=/path/to/x3-state-snapshot"
        exit 4
    fi

    # An unset anchor is not silently treated as "verified": the verifier warns,
    # and then only checks chunk integrity plus the recomputed state root.
    "$verifier" verify --manifest "$manifest" --chunks "$chunks_dir" "$@"
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

    # A snapshot that ships a content-addressed manifest has to verify before
    # anything is written. Restoring first and asking questions later is how a
    # corrupted or substituted mirror ends up as a validator's database.
    if [[ -n "${X3_SNAPSHOT_MANIFEST:-}" ]]; then
        local chunks_dir="${X3_SNAPSHOT_CHUNKS:-}"
        local chain_id="${X3_SNAPSHOT_CHAIN_ID:-}"
        local block_hash="${X3_SNAPSHOT_BLOCK_HASH:-}"
        local state_root="${X3_SNAPSHOT_STATE_ROOT:-}"
        local runtime_version="${X3_SNAPSHOT_RUNTIME_VERSION:-}"

        if [[ -z "$chunks_dir" || -z "$chain_id" || -z "$block_hash" || -z "$state_root" || -z "$runtime_version" ]]; then
            echo -e "${RED}❌ X3_SNAPSHOT_MANIFEST is set but the check is incomplete${NC}"
            echo "   Also set X3_SNAPSHOT_CHUNKS, X3_SNAPSHOT_CHAIN_ID, X3_SNAPSHOT_BLOCK_HASH,"
            echo "   X3_SNAPSHOT_STATE_ROOT and X3_SNAPSHOT_RUNTIME_VERSION — the anchor must come"
            echo "   from consensus, not from the snapshot."
            exit 4
        fi

        echo -e "${BLUE}🔎 Verifying snapshot against the finalized anchor before restoring ...${NC}"
        local -a anchor_args=(
            --chain-id "$chain_id"
            --block-hash "$block_hash"
            --state-root "$state_root"
            --runtime-version "$runtime_version"
        )
        if [[ -n "${X3_SNAPSHOT_STATE_VERSION:-}" ]]; then
            anchor_args+=(--state-version "$X3_SNAPSHOT_STATE_VERSION")
        fi

        if ! verify_snapshot_dir "$X3_SNAPSHOT_MANIFEST" "$chunks_dir" "${anchor_args[@]}"; then
            echo -e "${RED}❌ Refusing to restore: the snapshot did not verify against the anchor${NC}"
            exit 4
        fi
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
    verify-snapshot)
        # Verify a content-addressed snapshot (manifest + chunk directory)
        # before anyone restores it. Anchor flags after the two paths are
        # passed straight through to the verifier:
        #   bash scripts/snapshot-restore.sh verify-snapshot <manifest.json> <chunks_dir> \
        #       --chain-id x3_testnet_v1 --block-hash 0x.. --state-root 0x.. --runtime-version N
        MANIFEST_PATH="${2:-}"
        CHUNKS_DIR="${3:-}"
        if [[ -z "$MANIFEST_PATH" || -z "$CHUNKS_DIR" ]]; then
            echo -e "${RED}❌ verify-snapshot needs <manifest.json> <chunks_dir>${NC}"
            exit 1
        fi
        verify_snapshot_dir "$MANIFEST_PATH" "$CHUNKS_DIR" "${@:4}"
        ;;
    rebuild)
        # The other half of `verify-snapshot`: write the verified state back out
        # as a raw chain spec a node can boot from. Unlike `restore` above (which
        # unpacks a tarball of a whole base path), this is the content-addressed
        # path: the state is re-derived from the chunks, and the node recomputes
        # the state root from the spec it is handed.
        #
        # The anchor is not optional here — the verifier refuses without it — so
        # there is no way to rebuild state without saying which block it is.
        MANIFEST_PATH="${2:-}"
        CHUNKS_DIR="${3:-}"
        OUT_SPEC="${4:-}"
        if [[ -z "$MANIFEST_PATH" || -z "$CHUNKS_DIR" || -z "$OUT_SPEC" ]]; then
            echo -e "${RED}❌ rebuild needs <manifest.json> <chunks_dir> <out_spec.json>${NC}"
            exit 1
        fi

        REBUILD_VERIFIER=""
        if ! REBUILD_VERIFIER="$(resolve_snapshot_verifier)"; then
            echo -e "${RED}❌ x3-state-snapshot verifier not found${NC}"
            echo "   Build it:   cargo build --release -p x3-state-snapshot"
            echo "   Or set:     X3_SNAPSHOT_VERIFIER=/path/to/x3-state-snapshot"
            exit 4
        fi

        if ! "$REBUILD_VERIFIER" restore --manifest "$MANIFEST_PATH" --chunks "$CHUNKS_DIR" \
            --out "$OUT_SPEC" "${@:5}"; then
            echo -e "${RED}❌ Refusing to rebuild: the snapshot did not verify against the anchor${NC}"
            exit 4
        fi
        echo ""
        echo -e "Booting a node from this spec gives a database built from that state:"
        echo "   x3-chain-node --dev --chain $OUT_SPEC --base-path <fresh-base-path>"
        echo "   (it is not joined to the source chain: the genesis header is number 0"
        echo "    while the state is block N's, so it will refuse to author)"
        ;;
    *)
        usage
        ;;
esac
