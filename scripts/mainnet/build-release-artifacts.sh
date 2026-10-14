#!/usr/bin/env bash
# Package one committed candidate. Existing binaries are deliberately not accepted.
# Usage: build-release-artifacts.sh LABEL --chain PLAIN_SPEC --features FEATURES
#        [--out DIR] [--skip-sbom]
# FEATURES is the explicit Cargo feature list, including cli (e.g. cli,testnet).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TAG="${1:?usage: build-release-artifacts.sh LABEL --chain SPEC --features FEATURES [--out DIR] [--skip-sbom]}"
shift
OUT_DIR=""
CHAIN_SPEC=""
FEATURES=""
SKIP_SBOM=0
info() { printf '[release-artifacts] %s\n' "$*"; }
die() { printf '[release-artifacts] FAIL: %s\n' "$*" >&2; exit 1; }
while (( $# )); do
  case "$1" in
    --out) OUT_DIR="${2:?--out needs a directory}"; shift 2 ;;
    --chain) CHAIN_SPEC="${2:?--chain needs a file}"; shift 2 ;;
    --features) FEATURES="${2:?--features needs a feature list}"; shift 2 ;;
    --skip-sbom) SKIP_SBOM=1; shift ;;
    --binary) die "prebuilt binaries have no verified source provenance; omit --binary" ;;
    *) die "unknown option: $1" ;;
  esac
done
[[ "$TAG" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]] || die "invalid release label"
[[ "$FEATURES" =~ ^[A-Za-z0-9_-]+(,[A-Za-z0-9_-]+)*$ ]] || die "an explicit Cargo feature list is required"
[[ -f "$CHAIN_SPEC" ]] || die "a plain chain spec is required"
CHAIN_SPEC="$(realpath "$CHAIN_SPEC")"
COMMIT="$(git -C "$ROOT" rev-parse HEAD)"
[[ -z "$(git -C "$ROOT" status --porcelain --untracked-files=all)" ]] \
  || die "candidate checkout must be clean, including untracked files"
OUT_DIR="$(realpath -m "${OUT_DIR:-$ROOT/dist/$TAG}")"
[[ ! -e "$OUT_DIR" ]] || die "output directory already exists; choose a new release directory"
TARBALL="${OUT_DIR}.tar.gz"
[[ ! -e "$TARBALL" ]] || die "archive already exists"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
mkdir "$WORK/source" "$WORK/bundle"
# Ignored build products and local configuration cannot enter the source snapshot.
git -C "$ROOT" archive "$COMMIT" | tar -x -C "$WORK/source"
cp "$CHAIN_SPEC" "$WORK/bundle/genesis.json"
# Resolve rustup's pinned toolchain in the archived source, not the caller's cwd.
RUSTC_VERSION="$(cd "$WORK/source" && rustc --version)"
CARGO_VERSION="$(cd "$WORK/source" && cargo --version)"
BUILD_HOST="$(cd "$WORK/source" && rustc -vV | sed -n 's/^host: //p')"
info "building committed source $COMMIT with features $FEATURES"
(
  cd "$WORK/source"
  env -u SKIP_WASM_BUILD CARGO_TARGET_DIR="$WORK/target" \
    cargo build --locked --release -p x3-chain-node --no-default-features --features "$FEATURES"
)
NODE_BIN="$WORK/target/release/x3-chain-node"
[[ -x "$NODE_BIN" ]] || die "build did not produce an executable node"
install -m 0755 "$NODE_BIN" "$WORK/bundle/x3-chain-node"
# Loading the supplied plain spec uses the node's own genesis validation. Its raw
# twin is generated here, never taken from an unrelated previous build.
"$NODE_BIN" build-spec --chain "$WORK/bundle/genesis.json" --raw > "$WORK/bundle/genesis-raw.json"
"$NODE_BIN" build-spec --chain local3 > "$WORK/embedded.json"
python3 - "$WORK/bundle" "$WORK/embedded.json" <<'PY'
import json
from pathlib import Path
import sys
bundle = Path(sys.argv[1])
plain = json.loads((bundle / 'genesis.json').read_text())
raw = json.loads((bundle / 'genesis-raw.json').read_text())
embedded = json.loads(Path(sys.argv[2]).read_text())
def decode(value):
    if not isinstance(value, str) or not value.startswith('0x'):
        raise ValueError('runtime code must be hex prefixed with 0x')
    code = bytes.fromhex(value[2:])
    if not code:
        raise ValueError('runtime code is empty')
    return code
code = decode(plain['genesis']['runtimeGenesis']['code'])
if code != decode(embedded['genesis']['runtimeGenesis']['code']):
    raise ValueError('plain spec runtime differs from the built binary embedded runtime')
if code != decode(raw['genesis']['raw']['top']['0x3a636f6465']):
    raise ValueError('raw spec runtime differs from plain spec')
for field in ('id', 'name', 'chainType', 'bootNodes', 'properties', 'protocolId'):
    if plain.get(field) != raw.get(field):
        raise ValueError(f'plain/raw identity mismatch: {field}')
# Preserve the exact :code bytes, including compression if present.
(bundle / 'x3-runtime.wasm').write_bytes(code)
PY
gzip -n -c "$WORK/bundle/x3-runtime.wasm" > "$WORK/bundle/x3-runtime.wasm.gz"
SBOM_STATUS=omitted_by_request
if (( SKIP_SBOM == 0 )); then
  ( cd "$WORK/source" && cargo cyclonedx --manifest-path node/Cargo.toml \
      --format json --no-default-features --features "$FEATURES" )
  [[ -s "$WORK/source/node/bom.json" ]] || die "SBOM command produced no node/bom.json"
  cp "$WORK/source/node/bom.json" "$WORK/bundle/x3-chain-node.cdx.json"
  SBOM_STATUS=generated
fi
(
  cd "$WORK/bundle"
  # Keep the installer-compatible binary digest separate from the complete bundle.
  sha256sum x3-chain-node > x3-chain-node.sha256
  printf '%s\n' "$COMMIT" > git-revision.txt
  {
    echo "label:          $TAG"
    echo "commit:         $COMMIT"
    echo "features:       $FEATURES"
    echo "default_features: false"
    echo "build:          cargo build --locked --release -p x3-chain-node --no-default-features --features $FEATURES"
    echo "rustc:          $RUSTC_VERSION"
    echo "cargo:          $CARGO_VERSION"
    echo "build_host:     $BUILD_HOST"
    echo "sbom:           $SBOM_STATUS"
    echo "runtime:        exact genesis :code bytes; compression preserved"
    echo "reproducibility: not established by this single build"
  } > MANIFEST.txt
  find . -maxdepth 1 -type f ! -name SHA256SUMS -printf '%f\0' \
    | sort -z | xargs -0 sha256sum > SHA256SUMS
  sha256sum -c SHA256SUMS
)
mkdir -p "$(dirname "$OUT_DIR")"
# No files reach the release destination until the build and identity checks pass.
mv -T "$WORK/bundle" "$OUT_DIR"
tar -czf "$TARBALL" -C "$OUT_DIR" .
info "bundle: $OUT_DIR"
info "archive: $TARBALL"
