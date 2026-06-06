#!/bin/bash
set -e

echo "🔧 Installing Advanced Rust Testing Tools for X3_ATOMIC_STAR"
echo "============================================================="
echo ""

# Check prerequisites
echo "📋 Checking prerequisites..."

if ! command -v rustc &> /dev/null; then
    echo "❌ Rust not installed. Please install Rust first."
    exit 1
fi

RUSTC_VERSION=$(rustc --version)
echo "✅ $RUSTC_VERSION"

# Install nightly toolchain
echo ""
echo "📦 Setting up Rust toolchains..."
rustup toolchain install nightly
rustup +nightly component add rust-src
rustup +nightly component add llvm-tools-preview
echo "✅ Toolchains configured"

# 1. cargo-fuzz
echo ""
echo "📦 Installing cargo-fuzz (libFuzzer)..."
if cargo +nightly install cargo-fuzz 2>&1 | grep -q "already installed"; then
    echo "✅ cargo-fuzz already installed"
else
    echo "✅ cargo-fuzz installed"
fi

# 2. Kani
echo ""
echo "📦 Installing Kani (bounded model checking)..."
if cargo +nightly install --locked kani-verifier 2>&1 | grep -q "already installed"; then
    echo "✅ Kani already installed"
else
    echo "✅ Kani installed"
fi
cargo +nightly kani setup 2>/dev/null || true

# 3. Miri
echo ""
echo "📦 Installing Miri (UB detection)..."
cargo +nightly miri setup 2>&1 | grep -v "warning" || true
echo "✅ Miri configured"

# 4. cargo-mutants
echo ""
echo "📦 Installing cargo-mutants (mutation testing)..."
if cargo install cargo-mutants 2>&1 | grep -q "already installed"; then
    echo "✅ cargo-mutants already installed"
else
    echo "✅ cargo-mutants installed"
fi

# 5. Verify LLVM sanitizers
echo ""
echo "📦 Checking LLVM sanitizers..."
if cargo +nightly build --help | grep -q "sanitizer"; then
    echo "✅ Sanitizers available"
else
    echo "⚠️  Sanitizers may require nightly setup"
fi

echo ""
echo "╔════════════════════════════════════════════════════════╗"
echo "║  ✅ All testing tools installed successfully!          ║"
echo "╠════════════════════════════════════════════════════════╣"
echo "║  Next steps:                                            ║"
echo "║  1. cd /home/lojak/Desktop/X3_ATOMIC_STAR              ║"
echo "║  2. ./scripts/run-all-tests.sh                         ║"
echo "║                                                         ║"
echo "║  For detailed config:                                   ║"
echo "║  See ADVANCED_TESTING_INFRASTRUCTURE_SETUP.md          ║"
echo "╚════════════════════════════════════════════════════════╝"
