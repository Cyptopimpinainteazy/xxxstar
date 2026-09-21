#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# release-artifacts-gate.sh — prove the release bundle is installable and that its
# digest is load-bearing
#
# `build-release-artifacts.sh` produces the files a release is made of. This gate
# runs it and then does what an operator does with them:
#
#   1. the checksum file verifies against the binary it names
#   2. the tarball extracts, and the extracted binary runs (`--version`)
#   3. the manifest names the commit under test
#   4. `install-validator.sh --check` accepts the *bundled* binary with the digest
#      from the bundle (binary source and installer, checked against each other)
#   5. a tampered copy of the binary is refused by sha256sum and by the installer
#      — the digest has to be load-bearing, not decorative
#
# Usage: bash scripts/mainnet/release-artifacts-gate.sh
#        X3_NODE_BIN=… bash scripts/mainnet/release-artifacts-gate.sh
# ─────────────────────────────────────────────────────────────────────────────
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
WORK="$(mktemp -d)"
TAG="v0.0.0-gate-test"

cleanup() { rm -rf "$WORK"; }
trap cleanup EXIT

info() { printf '[artifact-gate] %s\n' "$*"; }
fail() {
  printf '[artifact-gate] FAIL: %s\n' "$*" >&2
  exit 1
}

BUILDER="$ROOT/scripts/mainnet/build-release-artifacts.sh"
INSTALLER="$ROOT/scripts/install-validator.sh"
[ -f "$BUILDER" ] || fail "scripts/mainnet/build-release-artifacts.sh is missing"
[ -f "$INSTALLER" ] || fail "scripts/install-validator.sh is missing"

# The release binary stage 2 built, so this gate does not compile the node again.
NODE_BIN="${X3_NODE_BIN:-}"
if [ -z "$NODE_BIN" ]; then
  for candidate in "$TARGET_DIR/release/x3-chain-node" "$ROOT/target/release/x3-chain-node"; do
    if [ -x "$candidate" ]; then NODE_BIN="$candidate"; break; fi
  done
fi
[ -n "$NODE_BIN" ] && [ -x "$NODE_BIN" ] \
  || fail "no release binary; build it with cargo build --release -p x3-chain-node"
info "packaging $NODE_BIN"

# ── 1. build the bundle ──────────────────────────────────────────────────────
OUT="$WORK/bundle"
bash "$BUILDER" "$TAG" --out "$OUT" --binary "$NODE_BIN" --skip-sbom >"$WORK/build.log" 2>&1
rc=$?
if [ "$rc" -ne 0 ]; then
  tail -20 "$WORK/build.log" >&2
  fail "build-release-artifacts.sh failed"
fi
for required in x3-chain-node x3-chain-node.sha256 MANIFEST.txt; do
  [ -f "$OUT/$required" ] || fail "the bundle is missing $required"
done
info "bundle built: $(ls -1 "$OUT" | tr '\n' ' ')"

# ── 2. the checksum file verifies ────────────────────────────────────────────
( cd "$OUT" && sha256sum -c x3-chain-node.sha256 >/dev/null ) \
  || fail "the bundle's own checksum file does not verify"
info "checksums verify"

# ── 3. the manifest names the commit under test ──────────────────────────────
HEAD_SHA="$(git -C "$ROOT" rev-parse HEAD)"
MANIFEST_COMMIT="$(sed -n 's/^commit: *//p' "$OUT/MANIFEST.txt" | head -1)"
[ "$MANIFEST_COMMIT" = "$HEAD_SHA" ] \
  || fail "manifest names $MANIFEST_COMMIT, HEAD is $HEAD_SHA"
info "manifest names HEAD"

# ── 4. the tarball carries a runnable binary ─────────────────────────────────
TARBALL="$ROOT/dist/${TAG}-linux-x86_64.tar.gz"
[ -f "$TARBALL" ] || fail "expected tarball not written: $TARBALL"
mkdir -p "$WORK/extracted"
tar -xzf "$TARBALL" -C "$WORK/extracted" || fail "the tarball does not extract"
[ -x "$WORK/extracted/x3-chain-node" ] || fail "the extracted bundle has no executable binary"
version="$("$WORK/extracted/x3-chain-node" --version 2>/dev/null | head -1)"
[ -n "$version" ] || fail "the extracted binary does not answer --version"
info "extracted binary runs: $version"
rm -f "$TARBALL"   # a gate must not leave a tarball in dist/

# ── 5. the installer accepts the bundled artifact ────────────────────────────
# A Live genesis, generated from fixture keys, so the install check exercises the
# same path an operator takes.
X3_NODE_BIN="$NODE_BIN" bash "$ROOT/scripts/mainnet/make-fixture-live-spec.sh" \
  "$WORK/spec" >"$WORK/fixture.log" 2>&1 \
  || { tail -15 "$WORK/fixture.log" >&2; fail "could not build a fixture Live genesis"; }
LIVE_SPEC="$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['spec'])" "$WORK/spec/fixture.json")"
DIGEST="$(awk '{print $1}' "$OUT/x3-chain-node.sha256" | head -1)"

env X3_INSTALL_DIR="$WORK/prefix/bin" X3_DATA_DIR="$WORK/prefix/data" \
    X3_CONFIG_DIR="$WORK/prefix/etc" X3_LOG_DIR="$WORK/prefix/log" \
  bash "$INSTALLER" --check --binary "$OUT/x3-chain-node" --sha256 "$DIGEST" --chain "$LIVE_SPEC" \
  >"$WORK/install.log" 2>&1
rc=$?
if [ "$rc" -ne 0 ]; then
  tail -15 "$WORK/install.log" >&2
  fail "the installer refused the artifact the release ships"
fi
grep -q "sha256 verified" "$WORK/install.log" \
  || { tail -10 "$WORK/install.log" >&2; fail "the installer did not verify the bundled digest"; }
info "installer accepts the bundled binary with the bundled digest"

# ── 6. a tampered artifact is refused, by both checks ────────────────────────
cp "$OUT/x3-chain-node" "$WORK/tampered"
printf 'x' >>"$WORK/tampered"   # one byte is enough
# Verify the tampered copy against the *release* checksum file. (Checking a file
# against a digest computed from itself proves nothing — it always matches, which
# is how this assertion was wrong the first time.)
mkdir -p "$WORK/tamper-check"
cp "$WORK/tampered" "$WORK/tamper-check/x3-chain-node"
cp "$OUT/x3-chain-node.sha256" "$WORK/tamper-check/"
cp "$OUT"/x3_chain_runtime*.wasm "$OUT"/x3_chain_runtime*.wasm.gz "$WORK/tamper-check/" 2>/dev/null || true
if ( cd "$WORK/tamper-check" && sha256sum -c x3-chain-node.sha256 >/dev/null 2>&1 ); then
  fail "a tampered binary passed the release checksum file"
fi
info "release checksum file rejects the tampered binary"
env X3_INSTALL_DIR="$WORK/prefix2/bin" X3_DATA_DIR="$WORK/prefix2/data" \
    X3_CONFIG_DIR="$WORK/prefix2/etc" X3_LOG_DIR="$WORK/prefix2/log" \
  bash "$INSTALLER" --check --binary "$WORK/tampered" --sha256 "$DIGEST" --chain "$LIVE_SPEC" \
  >"$WORK/tampered-install.log" 2>&1
rc=$?
[ "$rc" -ne 0 ] || fail "the installer accepted a tampered binary"
grep -q "sha256 mismatch" "$WORK/tampered-install.log" \
  || { tail -10 "$WORK/tampered-install.log" >&2; fail "the refusal is not reported as a digest mismatch"; }
info "tampered artifact refused (sha256 mismatch)"

echo
echo "[artifact-gate] PASS — the release bundle verifies, extracts, runs, and is accepted"
echo "                by the installer with its own digest; a tampered copy is refused"
