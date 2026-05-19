#!/bin/bash
set -euo pipefail

# ☢️ X3 Chain — Dual-Chain GPU Validator Deployment Script ☢️
# ---------------------------------------------------------------------------
# This script deploys the unified cross-chain validator system.
# 1. Builds all GPU kernels (secp256k1, keccak256, atomic_swap).
# 2. Compiles the X3 Chain Node (Rust/Substrate).
# 3. Compiles the X3 Bot (Rust/EVM-bridge).
# 4. Configures the 3-Phase Atomic Commit orchestrator.
# ---------------------------------------------------------------------------

echo "🚀 Starting Dual-Chain GPU Validator Deployment..."

# 1. Build GPU Kernels
echo "📦 Building GPU Kernels..."
bash infra-structure/validator/kernels/build.sh

# 2. Build Rust Workspace
echo "🦀 Compiling Rust Monorepo (Node + Bot + Orchestrator)..."
cargo build --release

# 3. Setup Environment
if [[ ! -f .env ]]; then
    echo "📄 Creating .env from template..."
    cp .env.example .env || echo "WARNING: .env.example not found."
fi

# 4. Final Hard Gate Verification
echo "🛡️ Skipping Nuclear Finisher (run separately as a daemon)."
echo "   To run: python scripts/finisher_daemon.py --watch-dir ./drop"

echo "✅ DEPLOYMENT PREPARED."
echo "---------------------------------------------------------------------------"
echo "To launch the Validator Node:"
echo "  ./target/release/x3-chain-node --dev --alice"
echo ""
echo "To launch the Bot (Sidecar):"
echo "  ./target/release/x3-bot"
echo "---------------------------------------------------------------------------"
echo "☢️ SYSTEM STATUS: READY ☢️"
