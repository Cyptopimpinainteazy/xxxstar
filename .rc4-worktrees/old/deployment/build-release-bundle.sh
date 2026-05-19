#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

RELEASE_NAME="x3-chain-v1.1"
RELEASE_TARBALL="${RELEASE_NAME}-release.tar.gz"
CHECKSUMS_FILE="CHECKSUMS.sha256"
SIGNATURE_FILE="CHECKSUMS.sha256.asc"
BUNDLE_CHECKSUMS_FILE="CHECKSUMS.bundle.sha256"
WASM_PATH="target/release/wbuild/x3-chain-runtime/x3_chain_runtime.compact.compressed.wasm"
BINARY_PATH="target/release/x3-chain-node"

for required in \
  "$BINARY_PATH" \
  "$WASM_PATH" \
  "scripts/x3_node_healthcheck.sh" \
  "run-dev-node.sh" \
  "run-production-node.sh" \
  "X3_OPERATOR_SOP.md" \
  "DEVELOPMENT.md" \
  "NODE_REQUIREMENTS.md" \
  "deployment/chain-specs/x3-dev-new.json" \
  "deployment/chain-specs/x3-testnet-raw.json" \
  ".env.example"
  do
  if [[ ! -f "$required" ]]; then
    echo "Missing required release input: $required" >&2
    exit 1
  fi
done

STAGE_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "$STAGE_DIR"
}
trap cleanup EXIT

mkdir -p "$STAGE_DIR"/{runtime,scripts,docs,config}
cp "$BINARY_PATH" "$STAGE_DIR/x3-chain-node"
cp "$WASM_PATH" "$STAGE_DIR/runtime/x3_chain_runtime.compact.compressed.wasm"
cp scripts/x3_node_healthcheck.sh "$STAGE_DIR/scripts/x3_node_healthcheck.sh"
cp run-dev-node.sh "$STAGE_DIR/scripts/run-dev-node.sh"
cp run-production-node.sh "$STAGE_DIR/scripts/run-production-node.sh"
cp X3_OPERATOR_SOP.md DEVELOPMENT.md NODE_REQUIREMENTS.md "$STAGE_DIR/docs/"
cp deployment/chain-specs/x3-dev-new.json "$STAGE_DIR/config/chain-spec-local.json"
cp deployment/chain-specs/x3-testnet-raw.json "$STAGE_DIR/config/chain-spec-testnet.json"
cp .env.example "$STAGE_DIR/config/.env.example"

cat > "$STAGE_DIR/$BUNDLE_CHECKSUMS_FILE" <<EOF
$(sha256sum "$STAGE_DIR/x3-chain-node" | awk '{print $1}')  x3-chain-node
$(sha256sum "$STAGE_DIR/runtime/x3_chain_runtime.compact.compressed.wasm" | awk '{print $1}')  runtime/x3_chain_runtime.compact.compressed.wasm
EOF

if [[ -f RELEASE_NOTES.md ]]; then
  cp RELEASE_NOTES.md "$STAGE_DIR/RELEASE_NOTES.md"
elif [[ -f "$RELEASE_TARBALL" ]]; then
  tar -xzf "$RELEASE_TARBALL" --to-stdout ./RELEASE_NOTES.md > "$STAGE_DIR/RELEASE_NOTES.md"
else
  cat > "$STAGE_DIR/RELEASE_NOTES.md" <<'EOF'
# X3 Chain v1.1 Release Notes

X3 Chain v1.1 completes Phase 8 release-readiness work with signed artifacts,
operator runbooks, health checks, and validated dual-VM execution.
EOF
fi

rm -f "$RELEASE_TARBALL"
tar -czf "$RELEASE_TARBALL" -C "$STAGE_DIR" .

sha256sum "$RELEASE_TARBALL" > "$CHECKSUMS_FILE"

if gpg --list-secret-keys > /dev/null 2>&1; then
  rm -f "$SIGNATURE_FILE"
  gpg --detach-sign --armor "$CHECKSUMS_FILE"
  gpg --verify "$SIGNATURE_FILE" "$CHECKSUMS_FILE" > /dev/null 2>&1
fi

echo "Built $RELEASE_TARBALL"
ls -lh "$RELEASE_TARBALL"
echo
echo "Contents:"
tar -tzf "$RELEASE_TARBALL" | sort
echo
echo "Checksums:"
cat "$CHECKSUMS_FILE"
