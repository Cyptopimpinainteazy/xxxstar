#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# build-release-artifacts.sh — produce the files a release is made of
#
# The release pipeline has never run a single step. `v0.4.0-rc.1` is 1,027
# commits behind master, and the workflow file *at that tag* asked for
# `ubuntu-latest`, which is billing-locked for this account — every job finished
# in ~4 seconds with zero steps, so the draft release has no assets and
# `install-validator.sh --from-release` has nothing to download. The workflow on
# master targets the self-hosted runners (both online), but its dispatch path
# builds whatever ref it is dispatched on, which is not the tag you want to
# release.
#
# This script produces the same set of files locally, from the tree you point it
# at, and prints the exact upload command:
#
#   x3-chain-node                    the node binary
#   x3-chain-node.sha256             its digest (the installer requires this file)
#   x3_chain_runtime.compact.wasm(.gz)   the runtime blob the node embeds
#   x3-chain-node.cdx.json           SBOM, when `cargo cyclonedx` is available
#   MANIFEST.txt                     commit, toolchain, sizes, digests
#   x3-chain-node-<tag>-linux-x86_64.tar.gz  the above, for humans
#
# Usage:
#   scripts/mainnet/build-release-artifacts.sh <tag> [--out DIR] [--chain SPEC]
#                                                [--binary PATH] [--skip-sbom]
#
# `--binary` skips the build and packages a binary you already have (what the
# release-artifact gate uses, so it does not rebuild the node a second time).
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TAG="${1:?usage: build-release-artifacts.sh <tag> [--out DIR] [--chain SPEC] [--binary PATH] [--skip-sbom]}"
shift

OUT_DIR=""
CHAIN_SPEC=""
NODE_BIN=""
SKIP_SBOM=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --out) OUT_DIR="${2:?--out needs a directory}"; shift 2 ;;
    --chain) CHAIN_SPEC="${2:?--chain needs a file}"; shift 2 ;;
    --binary) NODE_BIN="${2:?--binary needs a path}"; shift 2 ;;
    --skip-sbom) SKIP_SBOM=1; shift ;;
    *) echo "unknown option: $1" >&2; exit 2 ;;
  esac
done

OUT_DIR="${OUT_DIR:-$ROOT/dist/${TAG}}"
BINARY_NAME="x3-chain-node"
RUNTIME_WASM="x3_chain_runtime.compact.compressed.wasm"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"

info() { printf '[release-artifacts] %s\n' "$*"; }
die() { printf '[release-artifacts] FAIL: %s\n' "$*" >&2; exit 1; }

[ -n "$TAG" ] || die "a tag is required"

COMMIT="$(git -C "$ROOT" rev-parse HEAD)"
if [ -n "$(git -C "$ROOT" status --porcelain --untracked-files=no)" ]; then
  info "WARNING: the tree has uncommitted changes; the manifest names $COMMIT but the"
  info "         artifacts were built from the working tree. Commit first for a release."
fi

# ── the binary ───────────────────────────────────────────────────────────────
if [ -z "$NODE_BIN" ]; then
  info "building $BINARY_NAME (release)"
  ( cd "$ROOT" && cargo build --release -p x3-chain-node )
  NODE_BIN="$TARGET_DIR/release/$BINARY_NAME"
fi
[ -x "$NODE_BIN" ] || die "node binary not found or not executable: $NODE_BIN"

mkdir -p "$OUT_DIR"
install -m 0755 "$NODE_BIN" "$OUT_DIR/$BINARY_NAME"

# The runtime blob the binary embeds. Take srtool's if it is there (that is the
# build stage 6b attests), else the workspace's own wbuild output.
WASM_PATH=""
for candidate in \
  "$ROOT/runtime/target/srtool/..." \
  "$TARGET_DIR/release/wbuild/x3-chain-runtime/$RUNTIME_WASM" \
  "$TARGET_DIR/release/wbuild/x3-chain-runtime/x3_chain_runtime.compact.wasm"; do
  case "$candidate" in *"..."*) continue ;; esac
  if [ -f "$candidate" ]; then WASM_PATH="$candidate"; break; fi
done
if [ -n "$WASM_PATH" ]; then
  base="$(basename "$WASM_PATH")"
  install -m 0644 "$WASM_PATH" "$OUT_DIR/$base"
  gzip -c "$OUT_DIR/$base" >"$OUT_DIR/$base.gz"
  info "runtime blob: $base ($(stat -c%s "$OUT_DIR/$base") bytes)"
else
  info "WARNING: no runtime wasm found under $TARGET_DIR; the bundle will not carry one"
fi

# ── SBOM ─────────────────────────────────────────────────────────────────────
if [ "$SKIP_SBOM" = 0 ]; then
  if command -v cargo-cyclonedx >/dev/null 2>&1 || cargo cyclonedx --version >/dev/null 2>&1; then
    info "generating SBOM (cyclonedx)"
    ( cd "$ROOT" && cargo cyclonedx -p x3-chain-node --output "$OUT_DIR" --format json ) \
      || info "WARNING: cyclonedx failed; continuing without an SBOM"
  else
    info "WARNING: cargo-cyclonedx is not installed; continuing without an SBOM"
    info "         install it with: cargo install cargo-cyclonedx"
  fi
fi

if [ -n "$CHAIN_SPEC" ]; then
  [ -f "$CHAIN_SPEC" ] || die "chain spec not found: $CHAIN_SPEC"
  install -m 0644 "$CHAIN_SPEC" "$OUT_DIR/genesis.json"
  info "genesis: $(basename "$CHAIN_SPEC")"
fi

# ── checksums and manifest ───────────────────────────────────────────────────
(
  cd "$OUT_DIR"
  # The installer fetches `<binary>.sha256` and runs `sha256sum -c` against it, so
  # it must name exactly the binary asset.
  sha256sum "$BINARY_NAME" >"$BINARY_NAME.sha256"
  for extra in *.wasm *.wasm.gz; do
    [ -f "$extra" ] || continue
    sha256sum "$extra" >>"$BINARY_NAME.sha256"
  done
)

{
  echo "tag:            $TAG"
  echo "commit:         $COMMIT"
  echo "built_at:       $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "rustc:          $(rustc --version 2>/dev/null || echo unknown)"
  echo "cargo:          $(cargo --version 2>/dev/null || echo unknown)"
  echo "target:         $(rustc -vV 2>/dev/null | sed -n 's/^host: //p' || echo unknown)"
  echo "binary_size:    $(stat -c%s "$OUT_DIR/$BINARY_NAME")"
  echo
  echo "files:"
  ( cd "$OUT_DIR" && ls -1 )
  echo
  echo "digests:"
  cat "$OUT_DIR/$BINARY_NAME.sha256"
} >"$OUT_DIR/MANIFEST.txt"

TARBALL="$ROOT/dist/${TAG}-linux-x86_64.tar.gz"
mkdir -p "$(dirname "$TARBALL")"
(
  cd "$OUT_DIR"
  tar -czf "$TARBALL" .
)
info "bundle: $TARBALL ($(stat -c%s "$TARBALL") bytes)"

echo
info "artifacts in $OUT_DIR"
( cd "$OUT_DIR" && sha256sum -c "$BINARY_NAME.sha256" ) || die "checksums do not verify"
info "checksums verify"
echo
info "to attach them to the release (a draft is fine; publish when you are ready):"
info "  gh release upload $TAG $OUT_DIR/$BINARY_NAME $OUT_DIR/$BINARY_NAME.sha256 \\"
info "      $OUT_DIR/*.wasm $OUT_DIR/*.wasm.gz $OUT_DIR/*.cdx.json --clobber"
info
info "the public download path only serves published releases, so"
info "  install-validator.sh --from-release $TAG"
info "works after the release is published, not while it is a draft."
