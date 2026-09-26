#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# scripts/mainnet/build_runtime_upgrade_artifact.sh
#
# Build the "next" runtime that the governance upgrade rehearsal swaps in.
#
# The rehearsal has to replace the running code with *different* code, or there is
# nothing to observe: `system.set_code` would write identical bytes into `:code`
# and the spec version would not move. The runtime has no "version +1" knob, and the
# artifact the node binary embeds is by definition the runtime the chain is already
# running — so the next version is built the way a release builds one: from this
# tree, with the runtime version incremented by exactly one and nothing else
# touched.
#
# The build is isolated on purpose. It uses a detached git worktree at the
# requested revision and its own CARGO_TARGET_DIR, so `runtime/src/lib.rs` in the
# working tree is never edited — the other gates on this box read that file, and an
# edit in place would make them report a runtime hash for code nobody is running.
#
# Usage:
#   scripts/mainnet/build_runtime_upgrade_artifact.sh [--rev <gitrev>] [--out <wasm>]
#
# Output:
#   <out>                     the compact+compressed runtime blob `set_code` wants
#   <out>.provenance.json     revision, both spec versions, sha256, byte size
#
# Exit 0 → artifact built and its provenance written.
# Exit 1 → nothing usable was produced; read the message, do not paper over it.
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REV="HEAD"
OUT="$ROOT_DIR/target/upgrade-artifact/x3_chain_runtime.next.compact.compressed.wasm"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --rev) REV="$2"; shift 2 ;;
        --out) OUT="$2"; shift 2 ;;
        -h|--help) sed -n '2,32p' "${BASH_SOURCE[0]}"; exit 0 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

for tool in cargo git jq sha256sum; do
    command -v "$tool" >/dev/null 2>&1 || { echo "missing required tool: $tool" >&2; exit 1; }
done

RESOLVED_REV="$(git -C "$ROOT_DIR" rev-parse --verify "$REV^{commit}")"
OUT_DIR="$(dirname "$OUT")"
mkdir -p "$OUT_DIR"

TARGET_DIR="${X3_UPGRADE_ARTIFACT_TARGET_DIR:-$ROOT_DIR/target/upgrade-artifact-target}"

# A source worktree may be supplied to reuse an in-flight build (this box has
# several agents compiling at once; throwing away a warm target dir because a
# second copy of the same tree was wanted is the expensive mistake). A reused
# worktree is left alone; one this script creates is removed again.
SRC_DIR="${X3_UPGRADE_ARTIFACT_SRC_DIR:-}"
CREATED_SRC=0
if [[ -z "$SRC_DIR" ]]; then
    SRC_DIR="$(mktemp -d "${TMPDIR:-/tmp}/x3-upgrade-artifact-src.XXXXXX")"
    rmdir "$SRC_DIR"   # `git worktree add` wants to create the directory itself
    CREATED_SRC=1
fi

cleanup() {
    if [[ "$CREATED_SRC" == "1" ]]; then
        git -C "$ROOT_DIR" worktree remove --force "$SRC_DIR" >/dev/null 2>&1 || true
        git -C "$ROOT_DIR" worktree prune >/dev/null 2>&1 || true
    fi
}
trap cleanup EXIT

if [[ "$CREATED_SRC" == "1" ]]; then
    echo "→ worktree for $RESOLVED_REV at $SRC_DIR"
    git -C "$ROOT_DIR" worktree add --detach "$SRC_DIR" "$RESOLVED_REV" >/dev/null
else
    [[ -d "$SRC_DIR" ]] || { echo "reused worktree $SRC_DIR does not exist" >&2; exit 1; }
    echo "→ reusing worktree at $SRC_DIR"
fi

# A reused worktree must be the revision under test, not simply a directory that
# happens to exist: a version bump applied on top of a different commit would
# produce an artifact nobody can reproduce.
SRC_HEAD="$(git -C "$SRC_DIR" rev-parse HEAD)"
if [[ "$SRC_HEAD" != "$RESOLVED_REV" ]] && [[ "$(git -C "$SRC_DIR" stash list | wc -l)" -eq 0 ]]; then
    echo "reused worktree is at $SRC_HEAD, expected $RESOLVED_REV" >&2
    exit 1
fi

RUNTIME_LIB="$SRC_DIR/runtime/src/lib.rs"
[[ -f "$RUNTIME_LIB" ]] || { echo "no runtime/src/lib.rs in $SRC_DIR" >&2; exit 1; }

# The worktree is expected to carry exactly the one-line bump already, or to be a
# clean checkout that this script bumps. Anything else (two spec_version lines, a
# bump on top of other edits) is refused rather than guessed at.
SPEC_LINES="$(grep -c -E '^    spec_version: [0-9]+,$' "$RUNTIME_LIB")"
if [[ "$SPEC_LINES" -ne 1 ]]; then
    echo "expected exactly one 'spec_version: <n>,' line in runtime/src/lib.rs, found $SPEC_LINES" >&2
    exit 1
fi
SPEC_AFTER="$(grep -m1 -E '^    spec_version: [0-9]+,$' "$RUNTIME_LIB" | grep -oE '[0-9]+')"
# `sed -n` reads the whole stream, unlike `grep -m1`, which exits after its first
# match and hands the producer a SIGPIPE — fatal under `pipefail`.
SPEC_BEFORE="$(git -C "$SRC_DIR" show "$RESOLVED_REV:runtime/src/lib.rs" \
    | sed -n 's/^    spec_version: \([0-9][0-9]*\),$/\1/p')"
SPEC_BEFORE="${SPEC_BEFORE%%$'\n'*}"
if [[ ! "$SPEC_BEFORE" =~ ^[0-9]+$ ]]; then
    echo "could not read VERSION.spec_version from $RESOLVED_REV:runtime/src/lib.rs" >&2
    exit 1
fi
if [[ "$SPEC_AFTER" == "$SPEC_BEFORE" ]]; then
    SPEC_AFTER="$(( SPEC_BEFORE + 1 ))"
    sed -i -E "s/^    spec_version: ${SPEC_BEFORE},$/    spec_version: ${SPEC_AFTER},/" "$RUNTIME_LIB"
fi
if [[ "$SPEC_AFTER" -ne "$(( SPEC_BEFORE + 1 ))" ]]; then
    echo "artifact worktree is at spec_version ${SPEC_AFTER}; expected ${SPEC_BEFORE} + 1" >&2
    exit 1
fi
# Nothing else may differ from the revision under test.
if [[ -n "$(git -C "$SRC_DIR" diff --stat -- . ':!runtime/src/lib.rs')" ]]; then
    echo "artifact worktree has changes outside runtime/src/lib.rs; refusing to build it" >&2
    git -C "$SRC_DIR" diff --stat >&2
    exit 1
fi
if [[ "$(git -C "$SRC_DIR" diff -- runtime/src/lib.rs | grep -c '^[+-][^+-]')" -ne 2 ]]; then
    echo "artifact worktree's runtime/src/lib.rs differs from the revision by more than the version bump" >&2
    git -C "$SRC_DIR" diff -- runtime/src/lib.rs >&2
    exit 1
fi
echo "→ spec_version ${SPEC_BEFORE} -> ${SPEC_AFTER} in the artifact worktree"

echo "→ building the runtime (SKIP_WASM_BUILD unset, target dir $TARGET_DIR)"
(
    cd "$SRC_DIR"
    # substrate-wasm-builder copies the *nearest ancestor Cargo.lock* into the
    # throwaway project it generates for the wasm target. Inside a normal checkout
    # that search finds the workspace lock; when CARGO_TARGET_DIR lives outside the
    # tree it finds nothing, the generated project re-resolves its own dependency
    # graph, and it can settle on crates this workspace deliberately patches
    # (`crypto-common 0.1.6` does not compile for wasm32v1-none here). This variable
    # is that search's documented override, and it is what keeps the artifact built
    # from the same graph the node binary was.
    env -u SKIP_WASM_BUILD CARGO_TARGET_DIR="$TARGET_DIR" WASM_BUILD_WORKSPACE_HINT="$SRC_DIR" \
        cargo build --release --locked -p x3-chain-runtime
)

BUILT="$TARGET_DIR/release/wbuild/x3-chain-runtime/x3_chain_runtime.compact.compressed.wasm"
[[ -f "$BUILT" ]] || { echo "build finished without $BUILT" >&2; exit 1; }

# The blob `set_code` accepts carries the Substrate compression magic. Checking it
# here keeps a raw `.wasm` from being handed to the rehearsal by accident.
if [[ "$(head -c 8 "$BUILT" | xxd -p)" != "52bc537646db8e05" ]]; then
    echo "$BUILT does not start with the compact+compressed magic" >&2
    exit 1
fi

cp "$BUILT" "$OUT"
SHA256="$(sha256sum "$OUT" | awk '{print $1}')"
SIZE="$(stat -c %s "$OUT")"

jq -n \
    --arg revision "$RESOLVED_REV" \
    --arg artifact "$OUT" \
    --arg sha256 "$SHA256" \
    --argjson size "$SIZE" \
    --argjson spec_before "$SPEC_BEFORE" \
    --argjson spec_after "$SPEC_AFTER" \
    --arg built_from "$SRC_DIR" \
    --arg built_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    '{
        what: "next runtime for the governance upgrade rehearsal: this tree with VERSION.spec_version incremented by one",
        revision: $revision,
        spec_version_before: $spec_before,
        spec_version_after: $spec_after,
        artifact: $artifact,
        sha256: $sha256,
        size_bytes: $size,
        built_from_worktree: $built_from,
        built_at: $built_at
    }' > "$OUT.provenance.json"

echo "→ artifact: $OUT"
echo "  sha256:   $SHA256"
echo "  size:     $SIZE bytes"
echo "  spec:     $SPEC_BEFORE -> $SPEC_AFTER"
