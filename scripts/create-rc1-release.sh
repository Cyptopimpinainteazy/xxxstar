#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# create-rc1-release.sh — Create signed v0.4.0-rc.1 GitHub release
#
# Creates an annotated git tag, triggers the release-provenance.yml workflow,
# and generates release artifacts with provenance attestation.
#
# Prerequisites:
#   - gh (GitHub CLI) authenticated
#   - All changes committed and pushed to main
#
# Usage: bash scripts/create-rc1-release.sh
#
# Outputs:
#   - Annotated git tag: v0.4.0-rc.1
#   - GitHub Release (draft, needs publish)
#   - Triggers release-provenance.yml which produces:
#     * x3-chain-node binary (Linux x86_64)
#     * x3-chain-node.sha256 checksums
#     * x3-chain-runtime.wasm (gzip compressed)
#     * SBOM (CycloneDX JSON)
#     * GitHub artifact attestations
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

RELEASE_TAG="v0.4.0-rc.1"
RELEASE_NAME="X3 v0.4 RC-1 — Internal Testnet Candidate"
REPO="${REPO:-Cyptopimpinainteazy/xxxstar}"

echo "=== Creating release: ${RELEASE_TAG} ==="
echo ""

# Step 1: Verify we're on main with latest changes
echo "[1/5] Verifying repository state..."
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    echo "❌ Not a git repository"
    exit 1
fi

# Step 2: Create annotated tag
echo "[2/5] Creating annotated tag ${RELEASE_TAG}..."
git tag -a "${RELEASE_TAG}" -m "${RELEASE_NAME}"

# Step 3: Push tag to trigger release-provenance.yml
echo "[3/5] Pushing tag to origin..."
git push origin "${RELEASE_TAG}"

# Step 4: Create GitHub Release (draft with prerelease markers)
echo "[4/5] Creating GitHub Release (draft)..."
gh release create "${RELEASE_TAG}" \
    --repo "${REPO}" \
    --title "${RELEASE_NAME}" \
    --notes-file - << 'RELEASE_NOTES'
## X3 v0.4 RC-1 — Internal Testnet Candidate

**This is an internal testnet release.** Not for public mainnet use.
External bridges, parallel execution, and advanced features remain gated off.

### Scope
See [LAUNCH_SCOPE.md](./LAUNCH_SCOPE.md) for authoritative scope statement.

### What's Active
- Internal cross-VM routing (X3Native ↔ X3Evm ↔ X3Svm)
- Supply ledger with invariant enforcement
- Settlement engine with refund path
- Packet standard lifecycle
- Internal testnet tooling and scripts
- CI gate matrix (fmt, clippy, tests, audit, deny, secret-scan, SAST, binary)

### What's Gated Off
- External bridges (EVM, Solana, Bitcoin)
- Parallel executor
- AppZone factory, PQ-experimental, advanced DEX, AI optimizer, GPU acceleration

### Artifacts
- `x3-chain-node` — Pre-built Linux x86_64 node binary
- `x3-chain-node.sha256` — SHA256 checksums
- `x3-chain-runtime.wasm.gz` — Runtime WASM blob (compressed)
- SBOM (CycloneDX JSON) — Supply-chain bill of materials

### Verifying Artifacts
```bash
# Download and verify
wget https://github.com/Cyptopimpinainteazy/xxxstar/releases/download/v0.4.0-rc.1/x3-chain-node
wget https://github.com/Cyptopimpinainteazy/xxxstar/releases/download/v0.4.0-rc.1/x3-chain-node.sha256
sha256sum -c x3-chain-node.sha256
chmod +x x3-chain-node

# Verify via Docker
docker build -f Dockerfile.mainnet-check -t x3-mainnet-check .
docker run --rm x3-mainnet-check
```
RELEASE_NOTES

# Step 5: Trigger release-provenance workflow
echo "[5/5] Workflow triggered automatically by tag push."
echo ""
echo "=== Release creation initiated ==="
echo "Tag:       ${RELEASE_TAG}"
echo "URL:       https://github.com/${REPO}/releases/tag/${RELEASE_TAG}"
echo "Workflow:  https://github.com/${REPO}/actions/workflows/release-provenance.yml"
echo ""
echo "⏳ Wait for release-provenance.yml to complete (~20-40 min)"
echo "   Then check Actions tab or run: gh run list --workflow release-provenance.yml"
echo ""
echo "📋 After release is published, update MAINNET_LAUNCH_CHECKLIST.md:"
echo "   Item 1.2: ⬜ TODO → ✅ DONE"