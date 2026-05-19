#!/usr/bin/env bash
# scripts/x3_release_sign.sh — build, bundle, checksum, sign, and verify X3 releases
#
# Build mode:
#   bash scripts/x3_release_sign.sh [--sign-key <gpg-key-id>] [--version <vX.Y.Z>] [--release-dir <dir>] [--skip-build]
#
# Verify mode:
#   bash scripts/x3_release_sign.sh --verify <release-dir>
#
# The release directory layout produced in build mode is:
#   <release-dir>/
#     bin/x3-chain-node
#     runtime/x3_chain_runtime.compact.compressed.wasm   (if present)
#     config/.env.example
#     config/x3-testnet-raw.json                         (if present)
#     docs/DEVELOPMENT.md
#     docs/X3_OPERATOR_SOP.md
#     docs/X3_RELEASE_READINESS_CHECKLIST.md
#     scripts/run-dev-node.sh
#     scripts/run-production-node.sh
#     scripts/x3_node_healthcheck.sh
#     MANIFEST.txt
#     RELEASE_MANIFEST.json
#   <release-dir>.tar.gz
#   CHECKSUMS.sha256            (outer tarball checksum, optionally signed)
#   CHECKSUMS.sha256.asc        (optional detached signature for CHECKSUMS.sha256)
#
# The release directory also contains:
#   CHECKSUMS.bundle.sha256     (checksums for files inside the extracted bundle)

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

SIGN_KEY=""
VERIFY_DIR=""
SKIP_BUILD=0
VERSION="${X3_RELEASE_VERSION:-v1.1.0}"
RELEASE_DIR="${X3_RELEASE_DIR:-$ROOT/.artifacts/release-v1.1}"
BUNDLE_PREFIX="${X3_RELEASE_BUNDLE_PREFIX:-x3-chain}"
BINARY_NAME="x3-chain-node"
BINARY_PATH="$ROOT/target/release/$BINARY_NAME"
WASM_PATH="$ROOT/target/release/wbuild/x3-chain-runtime/x3_chain_runtime.compact.compressed.wasm"
CHECKSUM_FILE="$ROOT/CHECKSUMS.sha256"
CHECKSUM_SIG_FILE="$ROOT/CHECKSUMS.sha256.asc"
BUNDLE_CHECKSUM_FILE_NAME="CHECKSUMS.bundle.sha256"

usage() {
  cat <<'EOF'
Usage:
  bash scripts/x3_release_sign.sh [--sign-key <gpg-key-id>] [--version <vX.Y.Z>] [--release-dir <dir>] [--skip-build]
  bash scripts/x3_release_sign.sh --verify <release-dir>

Options:
  --sign-key <id>     GPG key id/email for detached signing of CHECKSUMS.sha256
  --version <vX.Y.Z>  Release version used in manifests and tarball naming
  --release-dir <dir> Directory to stage the release bundle
  --skip-build        Reuse existing build artifacts instead of running cargo build
  --verify <dir>      Verify an existing release directory and matching checksums/signature
  --help              Show this help text
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --sign-key)
      [ "$#" -ge 2 ] || { echo "❌ Missing value for --sign-key"; exit 1; }
      SIGN_KEY="$2"
      shift 2
      ;;
    --version)
      [ "$#" -ge 2 ] || { echo "❌ Missing value for --version"; exit 1; }
      VERSION="$2"
      shift 2
      ;;
    --release-dir)
      [ "$#" -ge 2 ] || { echo "❌ Missing value for --release-dir"; exit 1; }
      RELEASE_DIR="$2"
      shift 2
      ;;
    --skip-build)
      SKIP_BUILD=1
      shift
      ;;
    --verify)
      [ "$#" -ge 2 ] || { echo "❌ Missing value for --verify"; exit 1; }
      VERIFY_DIR="$2"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "❌ Unknown argument: $1"
      usage
      exit 1
      ;;
  esac
done

RELEASE_DIR="$(python3 - <<'PY' "$RELEASE_DIR"
import os, sys
print(os.path.abspath(sys.argv[1]))
PY
)"
RELEASE_PARENT="$(dirname "$RELEASE_DIR")"
RELEASE_BASENAME="$(basename "$RELEASE_DIR")"
TARBALL_PATH="$RELEASE_PARENT/${BUNDLE_PREFIX}-${VERSION}.tar.gz"

require_file() {
  local path="$1"
  [ -f "$path" ] || { echo "❌ Required file missing: $path"; exit 1; }
}

relative_to_root() {
  python3 - <<'PY' "$ROOT" "$1"
import os, sys
print(os.path.relpath(sys.argv[2], sys.argv[1]))
PY
}

copy_if_exists() {
  local src="$1"
  local dst="$2"
  if [ -f "$src" ]; then
    mkdir -p "$(dirname "$dst")"
    cp "$src" "$dst"
    return 0
  fi
  return 1
}

write_manifest_txt() {
  local manifest_path="$1"
  local git_commit="$2"
  local manifest_date="$3"
  cat > "$manifest_path" <<EOF
[package]
version = $VERSION
date = $manifest_date
commit = $git_commit

[contents]
bin/$BINARY_NAME
EOF

  if [ -f "$RELEASE_DIR/runtime/$(basename "$WASM_PATH")" ]; then
    cat >> "$manifest_path" <<EOF
runtime/$(basename "$WASM_PATH")
EOF
  fi

  local optional_paths=(
    "${BUNDLE_CHECKSUM_FILE_NAME}"
    "config/.env.example"
    "config/x3-testnet-raw.json"
    "docs/DEVELOPMENT.md"
    "docs/X3_OPERATOR_SOP.md"
    "docs/X3_RELEASE_READINESS_CHECKLIST.md"
    "scripts/run-dev-node.sh"
    "scripts/run-production-node.sh"
    "scripts/x3_node_healthcheck.sh"
    "RELEASE_MANIFEST.json"
    "MANIFEST.txt"
  )

  local rel
  for rel in "${optional_paths[@]}"; do
    if [ -f "$RELEASE_DIR/$rel" ]; then
      echo "$rel" >> "$manifest_path"
    fi
  done
}

write_bundle_checksums() {
  local checksum_path="$1"

  cat > "$checksum_path" <<EOF
$(sha256sum "$RELEASE_DIR/bin/$BINARY_NAME" | awk '{print $1}')  bin/$BINARY_NAME
EOF

  if [ -f "$RELEASE_DIR/runtime/$(basename "$WASM_PATH")" ]; then
    cat >> "$checksum_path" <<EOF
$(sha256sum "$RELEASE_DIR/runtime/$(basename "$WASM_PATH")" | awk '{print $1}')  runtime/$(basename "$WASM_PATH")
EOF
  fi
}

write_manifest_json() {
  local manifest_path="$1"
  local git_commit="$2"
  local git_tag="$3"
  local rust_version="$4"
  local build_date="$5"
  local source_date_epoch="$6"
  local signed="$7"
  local tarball_name="$8"
  local binary_sha="$9"
  local wasm_sha="${10}"
  local tarball_sha="${11}"

  cat > "$manifest_path" <<EOF
{
  "version": "$VERSION",
  "bundle": "$RELEASE_BASENAME",
  "tarball": "$(basename "$tarball_name")",
  "git_commit": "$git_commit",
  "git_tag": "$git_tag",
  "rust_version": "$rust_version",
  "build_date": "$build_date",
  "source_date_epoch": $source_date_epoch,
  "signed": $signed,
  "artifacts": {
    "binary": {
      "path": "bin/$BINARY_NAME",
      "sha256": "$binary_sha"
    },
    "wasm": $(if [ -n "$wasm_sha" ]; then printf '{\n      "path": "runtime/%s",\n      "sha256": "%s"\n    }' "$(basename "$WASM_PATH")" "$wasm_sha"; else printf 'null'; fi),
    "tarball": {
      "path": "$(basename "$tarball_name")",
      "sha256": "$tarball_sha"
    }
  }
}
EOF
}

verify_release() {
  local dir="$1"
  dir="$(python3 - <<'PY' "$dir"
import os, sys
print(os.path.abspath(sys.argv[1]))
PY
)"
  local base="$(basename "$dir")"
  local parent="$(dirname "$dir")"
  local manifest="$dir/RELEASE_MANIFEST.json"
  local manifest_txt="$dir/MANIFEST.txt"
  local checksums="$ROOT/CHECKSUMS.sha256"
  local signature="$ROOT/CHECKSUMS.sha256.asc"
  local bundle_checksums="$dir/$BUNDLE_CHECKSUM_FILE_NAME"
  local tarball_extract_root

  echo "=== X3 Release Verification ==="
  require_file "$manifest"
  require_file "$manifest_txt"
  require_file "$dir/bin/$BINARY_NAME"
  require_file "$checksums"
  require_file "$bundle_checksums"

  echo "Verifying tracked distributable checksums from $(basename "$checksums")..."
  (
    cd "$ROOT"
    sha256sum --check "$checksums"
  )

  if [ -f "$signature" ]; then
    echo "Verifying detached GPG signature..."
    gpg --verify "$signature" "$checksums"
  else
    echo "ℹ️  No CHECKSUMS.sha256.asc found at repo root; skipping signature verification"
  fi

  local tarball=""
  tarball="$(python3 - <<'PY' "$manifest"
import json, sys
with open(sys.argv[1], 'r', encoding='utf-8') as fh:
    data = json.load(fh)
print(data.get('tarball', ''))
PY
)"

  if [ -n "$tarball" ] && [ -f "$parent/$tarball" ]; then
    echo "Validating tarball structure..."
    tar -tzf "$parent/$tarball" >/dev/null

    tarball_extract_root="$(mktemp -d)"
    trap 'rm -rf "$tarball_extract_root"' RETURN
    tar -xzf "$parent/$tarball" -C "$tarball_extract_root"

    local extracted_bundle_dir=""
    extracted_bundle_dir="$(find "$tarball_extract_root" -type f -name "$BUNDLE_CHECKSUM_FILE_NAME" -printf '%h\n' | head -n 1)"
    if [ -n "$extracted_bundle_dir" ]; then
      echo "Verifying extracted bundle contents..."
      (
        cd "$extracted_bundle_dir"
        sha256sum --check "$BUNDLE_CHECKSUM_FILE_NAME"
      )
    else
      echo "❌ Extracted tarball is missing $BUNDLE_CHECKSUM_FILE_NAME"
      exit 1
    fi
  else
    echo "ℹ️  Tarball referenced in manifest not found beside release directory; skipping tarball listing check"
  fi

  echo "Verifying staged bundle contents..."
  (
    cd "$dir"
    sha256sum --check "$BUNDLE_CHECKSUM_FILE_NAME"
  )

  echo "Checking manifest-listed files..."
  python3 - <<'PY' "$dir"
import pathlib, sys
release_dir = pathlib.Path(sys.argv[1])
manifest = release_dir / 'MANIFEST.txt'
missing = []
section = None
for raw in manifest.read_text(encoding='utf-8').splitlines():
    line = raw.strip()
    if not line:
        continue
    if line.startswith('[') and line.endswith(']'):
        section = line
        continue
    if section == '[contents]':
        path = release_dir / line
        if not path.exists():
            missing.append(line)
if missing:
    print('❌ Missing files declared in MANIFEST.txt:')
    for item in missing:
        print(item)
    raise SystemExit(1)
print('✅ Manifest contents present')
PY

  echo ""
  echo "=== Release Manifest ==="
  cat "$manifest"
  echo ""
  echo "✅ Verification complete"
}

if [ -n "$VERIFY_DIR" ]; then
  verify_release "$VERIFY_DIR"
  exit 0
fi

echo "=== X3 Release Bundle Build ==="

export SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-$(git log -1 --format=%ct 2>/dev/null || echo 1700000000)}"
export CARGO_INCREMENTAL=0
export RUSTFLAGS="${RUSTFLAGS:--C debuginfo=0 -C codegen-units=1 -C opt-level=3}"

if [ "$SKIP_BUILD" -eq 0 ]; then
  echo "Building $BINARY_NAME (release, locked)..."
  cargo build --release --locked -p x3-chain-node
else
  echo "Skipping cargo build; using existing artifacts"
fi

require_file "$BINARY_PATH"
mkdir -p "$RELEASE_DIR/bin" "$RELEASE_DIR/config" "$RELEASE_DIR/docs" "$RELEASE_DIR/scripts"
rm -f "$RELEASE_DIR/RELEASE_MANIFEST.json" "$RELEASE_DIR/MANIFEST.txt"
rm -f "$RELEASE_DIR/$BUNDLE_CHECKSUM_FILE_NAME"
rm -rf "$RELEASE_DIR/runtime"

cp "$BINARY_PATH" "$RELEASE_DIR/bin/$BINARY_NAME"
copy_if_exists "$WASM_PATH" "$RELEASE_DIR/runtime/$(basename "$WASM_PATH")" || true
copy_if_exists "$ROOT/.env.example" "$RELEASE_DIR/config/.env.example" || true
copy_if_exists "$ROOT/testnet/x3-testnet-raw.json" "$RELEASE_DIR/config/x3-testnet-raw.json" || true
copy_if_exists "$ROOT/DEVELOPMENT.md" "$RELEASE_DIR/docs/DEVELOPMENT.md" || true
copy_if_exists "$ROOT/X3_OPERATOR_SOP.md" "$RELEASE_DIR/docs/X3_OPERATOR_SOP.md" || true
copy_if_exists "$ROOT/X3_RELEASE_READINESS_CHECKLIST.md" "$RELEASE_DIR/docs/X3_RELEASE_READINESS_CHECKLIST.md" || true
copy_if_exists "$ROOT/run-dev-node.sh" "$RELEASE_DIR/scripts/run-dev-node.sh" || true
copy_if_exists "$ROOT/run-production-node.sh" "$RELEASE_DIR/scripts/run-production-node.sh" || true
copy_if_exists "$ROOT/scripts/x3_node_healthcheck.sh" "$RELEASE_DIR/scripts/x3_node_healthcheck.sh" || true

write_bundle_checksums "$RELEASE_DIR/$BUNDLE_CHECKSUM_FILE_NAME"

GIT_COMMIT="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
GIT_TAG="$(git describe --tags --exact-match 2>/dev/null || echo untagged)"
RUST_VERSION="$(rustc --version 2>/dev/null || echo unknown)"
BUILD_DATE="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
MANIFEST_DATE="$(date -u +"%Y-%m-%d")"

write_manifest_txt "$RELEASE_DIR/MANIFEST.txt" "$GIT_COMMIT" "$MANIFEST_DATE"

echo "Creating tarball $(basename "$TARBALL_PATH")..."
mkdir -p "$RELEASE_PARENT"
tar -czf "$TARBALL_PATH" -C "$RELEASE_PARENT" "$RELEASE_BASENAME"

BINARY_SHA="$(sha256sum "$RELEASE_DIR/bin/$BINARY_NAME" | awk '{print $1}')"
WASM_SHA=""
if [ -f "$RELEASE_DIR/runtime/$(basename "$WASM_PATH")" ]; then
  WASM_SHA="$(sha256sum "$RELEASE_DIR/runtime/$(basename "$WASM_PATH")" | awk '{print $1}')"
fi
TARBALL_SHA="$(sha256sum "$TARBALL_PATH" | awk '{print $1}')"

RELEASE_DIR_REL="$(relative_to_root "$RELEASE_DIR")"
TARBALL_REL="$(relative_to_root "$TARBALL_PATH")"

cat > "$CHECKSUM_FILE" <<EOF
$TARBALL_SHA  $TARBALL_REL
EOF

SIGNED=false
if [ -n "$SIGN_KEY" ]; then
  echo "Signing checksums with GPG key: $SIGN_KEY"
  gpg --armor --detach-sign \
      --default-key "$SIGN_KEY" \
      --output "$CHECKSUM_SIG_FILE" \
      "$CHECKSUM_FILE"
  SIGNED=true
else
  echo "ℹ️  No --sign-key provided; checksum file generated without detached signature"
fi

write_manifest_json \
  "$RELEASE_DIR/RELEASE_MANIFEST.json" \
  "$GIT_COMMIT" \
  "$GIT_TAG" \
  "$RUST_VERSION" \
  "$BUILD_DATE" \
  "$SOURCE_DATE_EPOCH" \
  "$SIGNED" \
  "$TARBALL_PATH" \
  "$BINARY_SHA" \
  "$WASM_SHA" \
  "$TARBALL_SHA"

# Refresh manifest to include RELEASE_MANIFEST.json after it exists.
write_manifest_txt "$RELEASE_DIR/MANIFEST.txt" "$GIT_COMMIT" "$MANIFEST_DATE"

echo ""
echo "=== Release Outputs ==="
echo "Release dir:   $RELEASE_DIR"
echo "Tarball:       $TARBALL_PATH"
echo "Checksums:     $CHECKSUM_FILE"
if [ "$SIGNED" = true ]; then
  echo "Signature:     $CHECKSUM_SIG_FILE"
fi
echo ""
echo "Next steps:"
echo "  1. Review bundle contents in $RELEASE_DIR"
echo "  2. Verify locally: bash scripts/x3_release_sign.sh --verify $RELEASE_DIR"
if [ "$SIGNED" = false ]; then
  echo "  3. Re-run with --sign-key <gpg-key-id> to generate CHECKSUMS.sha256.asc"
fi
