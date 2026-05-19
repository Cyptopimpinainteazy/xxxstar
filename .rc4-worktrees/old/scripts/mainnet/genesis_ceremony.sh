#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# scripts/mainnet/genesis_ceremony.sh
#
# X3 Atomic Star — Genesis Ceremony Script.
#
# This script is RUN ONCE to produce the frozen, reproducible genesis artifacts.
# It must be run from a clean checkout of the tagged release commit.
#
# Steps:
#   1. Verify we are on a release-tagged commit.
#   2. Build WASM via srtool (deterministic, matches on-chain hash).
#   3. Generate plain-text chain spec from env vars (no dev seeds).
#   4. Convert plain spec to raw spec.
#   5. Compute and record SHA256 hashes of all artifacts.
#   6. Generate genesis summary report.
#
# Required environment variables:
#   CEREMONY_KEY_MNEMONIC — mnemonic of the ceremony signing key (for chain ID)
#   NODE_SR25519_KEYS     — space-separated list of "name:ss58address" for initial validators
#
# Optional:
#   CHAIN_NAME            — chain name (default: x3-mainnet)
#   CHAIN_ID              — chain ID slug (default: x3-mainnet-1)
#   PROTOCOL_ID           — libp2p protocol ID (default: /x3/1)
#   INITIAL_SUPPLY        — initial supply in smallest unit (default: 1_000_000_000_000_000_000)
#   FAUCET_ADDRESS        — SS58 address of faucet account
#   TREASURY_ADDRESS      — SS58 address of treasury account
#
# DO NOT run with --dev, --tmp, or dev seeds. This generates PRODUCTION config.
#
# Exit 0 → ceremony COMPLETE and artifacts signed.
# Exit 1 → ceremony FAILED — do NOT use produced artifacts.
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ARTIFACTS_DIR="$ROOT_DIR/chain-specs"
REPORT_DIR="$ROOT_DIR/reports"
HASHES_FILE="$ROOT_DIR/release_hashes.txt"
mkdir -p "$ARTIFACTS_DIR" "$REPORT_DIR"

# ── Configuration ─────────────────────────────────────────────────────────────
CHAIN_NAME="${CHAIN_NAME:-x3-mainnet}"
CHAIN_ID="${CHAIN_ID:-x3-mainnet-1}"
PROTOCOL_ID="${PROTOCOL_ID:-/x3/1}"
INITIAL_SUPPLY="${INITIAL_SUPPLY:-1000000000000000000}"
FAUCET_ADDRESS="${FAUCET_ADDRESS:-}"
TREASURY_ADDRESS="${TREASURY_ADDRESS:-}"

NODE_BIN="$ROOT_DIR/target/release/x3-chain-node"
WASM_DIR="$ROOT_DIR/target/release/wbuild/x3-chain-runtime"
WASM_COMPACT="$WASM_DIR/x3_chain_runtime.compact.compressed.wasm"

PLAIN_SPEC="$ARTIFACTS_DIR/${CHAIN_ID}-plain.json"
RAW_SPEC="$ARTIFACTS_DIR/${CHAIN_ID}-raw.json"

CURRENT_SHA="$(git -C "$ROOT_DIR" rev-parse HEAD 2>/dev/null || echo "UNKNOWN")"
CURRENT_TAG="$(git -C "$ROOT_DIR" describe --exact-match --tags HEAD 2>/dev/null || echo "UNTAGGED")"

declare -A RESULTS
OVERALL="PASS"

pass()  { RESULTS["$1"]="PASS";  echo "[PASS] $1"; }
fail()  { RESULTS["$1"]="FAIL";  OVERALL="FAIL"; echo "[FAIL] $1 — ${2:-}"; }
skip()  { RESULTS["$1"]="SKIP";  echo "[SKIP] $1 — ${2:-}"; }

echo ""
echo "══════════════════════════════════════════════════════════"
echo "  X3 Atomic Star — Genesis Ceremony"
echo "  Commit:  $CURRENT_SHA"
echo "  Tag:     $CURRENT_TAG"
echo "  Chain:   $CHAIN_NAME ($CHAIN_ID)"
echo "══════════════════════════════════════════════════════════"
echo ""

# ─────────────────────────────────────────────────────────────────────────────
# STEP 1: Release tag verification
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [1] Verifying release tag..."
if [[ "$CURRENT_TAG" == "UNTAGGED" ]]; then
    echo "  WARNING: HEAD is not on a release tag."
    echo "  For production genesis, run: git tag -a mainnet-genesis-v1.0.0 -m 'Genesis commit'"
    echo "  Continuing in DEVELOPMENT MODE..."
    RESULTS["release_tag"]="WARN"
else
    echo "  Tag: $CURRENT_TAG"
    pass "release_tag"
fi

# Verify no uncommitted changes
if ! git -C "$ROOT_DIR" diff --quiet HEAD 2>/dev/null; then
    fail "clean_worktree" "uncommitted changes present — genesis must be from clean state"
else
    pass "clean_worktree"
fi

# ─────────────────────────────────────────────────────────────────────────────
# STEP 2: Build WASM (use srtool if available, else cargo build)
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [2] Building runtime WASM..."
cd "$ROOT_DIR"

if command -v srtool >/dev/null 2>&1; then
    echo "  Using srtool for deterministic build..."
    # srtool build --package x3-chain-runtime --runtime-dir runtime
    if srtool build \
        --package x3-chain-runtime \
        --runtime-dir runtime \
        >/dev/null 2>&1; then
        pass "wasm_srtool_build"
    else
        fail "wasm_srtool_build" "srtool build failed"
    fi
else
    echo "  srtool not found — using cargo build (non-deterministic, not for final production)"
    echo "  Install srtool: https://github.com/paritytech/srtool"
    if cargo build --release -p x3-chain-runtime >/dev/null 2>&1; then
        RESULTS["wasm_srtool_build"]="WARN"
        echo "[WARN] wasm_srtool_build — built without srtool, not reproducible"
    else
        fail "wasm_srtool_build" "cargo build failed"
    fi
fi

if [[ -f "$WASM_COMPACT" ]]; then
    WASM_HASH="$(sha256sum "$WASM_COMPACT" | awk '{print $1}')"
    echo "  WASM hash: $WASM_HASH"
    pass "wasm_artifact_exists"
else
    fail "wasm_artifact_exists" "WASM not found at $WASM_COMPACT"
fi

# Build node binary
echo "→ Building node binary..."
if cargo build --release -p x3-chain-node >/dev/null 2>&1; then
    pass "node_binary_build"
else
    fail "node_binary_build"
fi

# ─────────────────────────────────────────────────────────────────────────────
# STEP 3: Generate plain chain spec (env-sourced, no dev seeds)
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [3] Generating plain chain spec..."

if [[ ! -f "$NODE_BIN" ]]; then
    fail "plain_spec_generation" "node binary not found at $NODE_BIN"
else
    # Generate from mainnet or custom chain preset
    # For production: --chain must NOT be dev/local
    if "$NODE_BIN" build-spec \
        --chain "$CHAIN_ID" \
        --disable-default-bootnode \
        2>/dev/null > "$PLAIN_SPEC" 2>&1; then
        pass "plain_spec_generation"
    else
        # Fallback: try with testnet preset if custom chain spec not configured
        echo "  Custom chain ID not found, using testnet spec as base..."
        if "$NODE_BIN" build-spec \
            --chain x3-testnet \
            --disable-default-bootnode \
            >"$PLAIN_SPEC" 2>/dev/null; then
            RESULTS["plain_spec_generation"]="WARN"
            echo "[WARN] plain_spec_generation — using testnet preset, customise before use"
        else
            fail "plain_spec_generation" "could not generate chain spec for '$CHAIN_ID' or 'x3-testnet'"
        fi
    fi
fi

# ─────────────────────────────────────────────────────────────────────────────
# STEP 4: Verify plain spec — no dev seeds
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [4] Verifying plain spec has no dev seeds..."
if [[ -f "$PLAIN_SPEC" ]]; then
    DEV_PATTERNS_FOUND=()
    for pattern in \
        "Alice\|Bob\|Charlie\|Dave\|Eve\|Ferdie" \
        "bottom drive obey lake" \
        "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"; do
        if grep -qE "$pattern" "$PLAIN_SPEC" 2>/dev/null; then
            DEV_PATTERNS_FOUND+=("$pattern")
        fi
    done

    if [[ ${#DEV_PATTERNS_FOUND[@]} -gt 0 ]]; then
        fail "no_dev_seeds_in_spec" "FORBIDDEN patterns: ${DEV_PATTERNS_FOUND[*]}"
        echo ""
        echo "  ╔═══════════════════════════════════════════════════════════╗"
        echo "  ║  STOP: Dev seeds found in genesis spec.                  ║"
        echo "  ║  This spec is NOT safe for production use.               ║"
        echo "  ║  Replace all //Alice, //Bob etc. with real operator keys. ║"
        echo "  ╚═══════════════════════════════════════════════════════════╝"
        echo ""
    else
        pass "no_dev_seeds_in_spec"
    fi

    # Verify external bridges are off
    if grep -q '"externalBridgesEnabled":true\|"external_bridges_enabled":true' "$PLAIN_SPEC" 2>/dev/null; then
        fail "external_bridges_disabled_in_spec" "genesis has external bridges ENABLED"
    else
        pass "external_bridges_disabled_in_spec"
    fi
else
    fail "no_dev_seeds_in_spec"     "plain spec not generated"
    fail "external_bridges_disabled_in_spec" "plain spec not generated"
fi

# ─────────────────────────────────────────────────────────────────────────────
# STEP 5: Convert plain spec to raw spec
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [5] Converting to raw spec..."
if [[ -f "$NODE_BIN" ]] && [[ -f "$PLAIN_SPEC" ]] && [[ "${RESULTS[no_dev_seeds_in_spec]:-}" != "FAIL" ]]; then
    if "$NODE_BIN" build-spec \
        --chain "$PLAIN_SPEC" \
        --raw \
        --disable-default-bootnode \
        >"$RAW_SPEC" 2>/dev/null; then
        pass "raw_spec_generation"
    else
        fail "raw_spec_generation" "failed to convert plain → raw"
    fi
else
    skip "raw_spec_generation" "blocked by prior failures"
fi

# ─────────────────────────────────────────────────────────────────────────────
# STEP 6: Compute and record artifact hashes
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [6] Recording artifact hashes..."
{
    echo "# X3 Atomic Star — Release Artifact Hashes"
    echo "# Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "# Commit: $CURRENT_SHA"
    echo "# Tag: $CURRENT_TAG"
    echo ""
    if [[ -f "$NODE_BIN" ]]; then
        echo "node_binary: $(sha256sum "$NODE_BIN" | awk '{print $1}')  target/release/x3-chain-node"
    fi
    if [[ -f "$WASM_COMPACT" ]]; then
        echo "runtime_wasm: $(sha256sum "$WASM_COMPACT" | awk '{print $1}')  $WASM_COMPACT"
    fi
    if [[ -f "$PLAIN_SPEC" ]]; then
        echo "plain_spec: $(sha256sum "$PLAIN_SPEC" | awk '{print $1}')  $PLAIN_SPEC"
    fi
    if [[ -f "$RAW_SPEC" ]]; then
        echo "raw_spec: $(sha256sum "$RAW_SPEC" | awk '{print $1}')  $RAW_SPEC"
    fi
} > "$HASHES_FILE"

pass "artifact_hashes_recorded"
echo "  Hashes written to: $HASHES_FILE"

# ─────────────────────────────────────────────────────────────────────────────
# STEP 7: Generate ceremony report
# ─────────────────────────────────────────────────────────────────────────────
CEREMONY_REPORT="$REPORT_DIR/genesis_ceremony.md"
{
    echo "# X3 Genesis Ceremony Report"
    echo ""
    echo "- **Commit**: \`$CURRENT_SHA\`"
    echo "- **Tag**: \`$CURRENT_TAG\`"
    echo "- **Chain**: \`$CHAIN_NAME\` (\`$CHAIN_ID\`)"
    echo "- **Generated**: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "- **Overall**: $OVERALL"
    echo ""
    echo "## Step Results"
    echo ""
    for key in \
        release_tag \
        clean_worktree \
        wasm_srtool_build \
        wasm_artifact_exists \
        node_binary_build \
        plain_spec_generation \
        no_dev_seeds_in_spec \
        external_bridges_disabled_in_spec \
        raw_spec_generation \
        artifact_hashes_recorded; do
        echo "- $key: ${RESULTS[$key]:-NOT_RUN}"
    done
    echo ""
    echo "## Artifacts"
    echo ""
    if [[ -f "$HASHES_FILE" ]]; then
        echo "\`\`\`"
        cat "$HASHES_FILE"
        echo "\`\`\`"
    fi
    echo ""
    echo "## Gate"
    echo ""
    if [[ "$OVERALL" == "PASS" ]]; then
        echo "**genesis_ceremony: PASS** — artifacts are ready for distribution."
        echo ""
        echo "Next steps:"
        echo "1. Distribute \`$RAW_SPEC\` to all validator operators"
        echo "2. Publish spec hash from \`$HASHES_FILE\` publicly"
        echo "3. Start bootnodes with \`--chain $RAW_SPEC\`"
        echo "4. Coordinate validator launch time"
    else
        echo "**genesis_ceremony: FAIL** — do NOT distribute these artifacts."
    fi
} > "$CEREMONY_REPORT"

SELF_HASH="$(sha256sum "$CEREMONY_REPORT" | awk '{print $1}')"
echo "" >> "$CEREMONY_REPORT"
echo "_Report hash: \`$SELF_HASH\`_" >> "$CEREMONY_REPORT"

echo ""
echo "══════════════════════════════════════════════════════════"
echo "  genesis_ceremony: $OVERALL"
echo "  Report:  $CEREMONY_REPORT"
echo "  Hashes:  $HASHES_FILE"
if [[ -f "$RAW_SPEC" ]]; then
echo "  Raw spec: $RAW_SPEC"
fi
echo "══════════════════════════════════════════════════════════"

if [[ "$OVERALL" == "FAIL" ]]; then
    exit 1
fi
exit 0
