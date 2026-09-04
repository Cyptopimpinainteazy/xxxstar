#!/usr/bin/env bash
set -euo pipefail

# X3 Test Tool Stack Installation Script
# Installs all tools recommended for X3 edge-case hell-testing

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
INSTALL_LOG="$PROJECT_DIR/tools/install-x3-tools.log"

echo "=== X3 Test Tool Stack Installation ===" | tee "$INSTALL_LOG"
echo "Started: $(date)" | tee -a "$INSTALL_LOG"
echo "Project: $PROJECT_DIR" | tee -a "$INSTALL_LOG"
echo "" | tee -a "$INSTALL_LOG"

# ---- Helper ----
check_installed() {
    local tool="$1"
    local label="${2:-$tool}"
    if command -v "$tool" &>/dev/null; then
        echo "  ✓ $label ($(command -v $tool))" | tee -a "$INSTALL_LOG"
        return 0
    else
        echo "  ✗ $label NOT installed" | tee -a "$INSTALL_LOG"
        return 1
    fi
}

install_cargo_tool() {
    local pkg="$1"
    local binary="${2:-$pkg}"
    if ! command -v "$binary" &>/dev/null; then
        echo "  Installing $pkg..." | tee -a "$INSTALL_LOG"
        cargo install "$pkg" --locked 2>&1 | tail -3 | tee -a "$INSTALL_LOG"
    else
        echo "  $pkg already installed" | tee -a "$INSTALL_LOG"
    fi
}

echo ""
echo "=== 1. Checking current tool status ===" | tee -a "$INSTALL_LOG"
echo "" | tee -a "$INSTALL_LOG"

# Already installed from earlier checks: cargo-audit, cargo-deny, cargo-mutants, cargo-llvm-cov

check_installed "cargo" "cargo (Rust)"
check_installed "rustc" "rustc"
check_installed "python3" "Python 3"
check_installed "node" "Node.js"
check_installed "npm" "npm"
check_installed "pip3" "pip"

echo "" | tee -a "$INSTALL_LOG"

echo "=== 2. Rust toolchain targets ===" | tee -a "$INSTALL_LOG"
rustup target list --installed 2>&1 | tee -a "$INSTALL_LOG"

echo "" | tee -a "$INSTALL_LOG"

echo "=== 3. Installing Rust cargo tools ===" | tee -a "$INSTALL_LOG"
echo "" | tee -a "$INSTALL_LOG"

install_cargo_tool "cargo-fuzz" "cargo-fuzz"
install_cargo_tool "cargo-nextest" "cargo-nextest"
install_cargo_tool "cargo-geiger" "cargo-geiger"

# loom and shuttle are libraries, not cargo subcommands - added as dev-deps later
# kani is installed via rustup

echo "" | tee -a "$INSTALL_LOG"

echo "=== 4. Installing Kani Rust Verifier ===" | tee -a "$INSTALL_LOG"
if ! command -v kani-driver &>/dev/null; then
    echo "  Installing kani (rustup toolchain)..." | tee -a "$INSTALL_LOG"
    cargo install kani-verifier --locked 2>&1 | tail -5 | tee -a "$INSTALL_LOG"
else
    echo "  kani already installed" | tee -a "$INSTALL_LOG"
fi

echo "" | tee -a "$INSTALL_LOG"

echo "=== 5. Installing Python tooling ===" | tee -a "$INSTALL_LOG"
echo "" | tee -a "$INSTALL_LOG"

# Slither
if ! command -v slither &>/dev/null; then
    echo "  Installing slither-analyzer..." | tee -a "$INSTALL_LOG"
    pip3 install slither-analyzer 2>&1 | tail -5 | tee -a "$INSTALL_LOG"
else
    echo "  slither already installed" | tee -a "$INSTALL_LOG"
fi

# Echidna - binary install from GitHub
if ! command -v echidna &>/dev/null; then
    echo "  Installing echidna (Solana/EVM fuzzer)..." | tee -a "$INSTALL_LOG"
    ECHIDNA_VERSION="2.2.4"
    ARCH="x86_64-linux"
    if ! command -v echidna &>/dev/null; then
        curl -L "https://github.com/crytic/echidna/releases/download/v${ECHIDNA_VERSION}/echidna-${ECHIDNA_VERSION}-Ubuntu-22.04.tar.gz" -o /tmp/echidna.tar.gz 2>&1
        tar -xzf /tmp/echidna.tar.gz -C /tmp/ 2>&1
        cp /tmp/echidna ~/.cargo/bin/ 2>/dev/null || true
        rm -f /tmp/echidna.tar.gz /tmp/echidna
    fi
else
    echo "  echidna already installed" | tee -a "$INSTALL_LOG"
fi

# Aderyn - Solidity static analyzer
if ! command -v aderyn &>/dev/null; then
    echo "  Installing aderyn..." | tee -a "$INSTALL_LOG"
    cargo install aderyn --locked 2>&1 | tail -5 | tee -a "$INSTALL_LOG"
else
    echo "  aderyn already installed" | tee -a "$INSTALL_LOG"
fi

echo "" | tee -a "$INSTALL_LOG"

echo "=== 6. Installing Foundry (forge, cast, anvil) ===" | tee -a "$INSTALL_LOG"
if ! command -v forge &>/dev/null; then
    echo "  Installing Foundry..." | tee -a "$INSTALL_LOG"
    curl -L https://foundry.paradigm.xyz | bash 2>&1 | tail -5 | tee -a "$INSTALL_LOG"
    export PATH="$HOME/.foundry/bin:$PATH"
    foundryup 2>&1 | tail -10 | tee -a "$INSTALL_LOG"
else
    echo "  Foundry already installed (forge $(forge --version 2>/dev/null))" | tee -a "$INSTALL_LOG"
fi

echo "" | tee -a "$INSTALL_LOG"

echo "=== 7. Installing k6 (load testing) ===" | tee -a "$INSTALL_LOG"
if ! command -v k6 &>/dev/null; then
    echo "  Installing k6..." | tee -a "$INSTALL_LOG"
    curl -L "https://github.com/grafana/k6/releases/download/v0.54.0/k6-v0.54.0-linux-amd64.tar.gz" -o /tmp/k6.tar.gz 2>&1
    tar -xzf /tmp/k6.tar.gz -C /tmp/ 2>&1
    cp /tmp/k6-v0.54.0-linux-amd64/k6 ~/.cargo/bin/ 2>/dev/null || true
    rm -rf /tmp/k6.tar.gz /tmp/k6-v0.54.0-linux-amd64
else
    echo "  k6 already installed" | tee -a "$INSTALL_LOG"
fi

echo "" | tee -a "$INSTALL_LOG"

echo "=== 8. Installing Toxiproxy ===" | tee -a "$INSTALL_LOG"
if ! command -v toxiproxy-cli &>/dev/null; then
    echo "  Installing Toxiproxy..." | tee -a "$INSTALL_LOG"
    curl -L "https://github.com/shopify/toxiproxy/releases/download/v2.9.0/toxiproxy-server-linux-amd64" -o ~/.cargo/bin/toxiproxy-server 2>&1
    curl -L "https://github.com/shopify/toxiproxy/releases/download/v2.9.0/toxiproxy-cli-linux-amd64" -o ~/.cargo/bin/toxiproxy-cli 2>&1
    chmod +x ~/.cargo/bin/toxiproxy-server ~/.cargo/bin/toxiproxy-cli
else
    echo "  Toxiproxy already installed" | tee -a "$INSTALL_LOG"
fi

echo "" | tee -a "$INSTALL_LOG"

echo "=== 9. Checking Substrate tools ===" | tee -a "$INSTALL_LOG"
echo "" | tee -a "$INSTALL_LOG"

# try-runtime is already in workspace deps as frame-try-runtime
echo "  try-runtime: in workspace deps (frame-try-runtime)" | tee -a "$INSTALL_LOG"

# subwasm
install_cargo_tool "subwasm" "subwasm"

# srtool - Docker-based, check if docker available
if command -v docker &>/dev/null; then
    echo "  Docker available for srtool builds" | tee -a "$INSTALL_LOG"
else
    echo "  Docker not available - srtool will use CI/CD" | tee -a "$INSTALL_LOG"
fi

echo "" | tee -a "$INSTALL_LOG"

echo "=== 10. Final verification ===" | tee -a "$INSTALL_LOG"
echo "" | tee -a "$INSTALL_LOG"

echo "--- Rust Tools ---" | tee -a "$INSTALL_LOG"
for tool in cargo-audit cargo-deny cargo-mutants cargo-fuzz cargo-nextest cargo-geiger cargo-llvm-cov; do
    check_installed "$tool" || true
done

echo "" | tee -a "$INSTALL_LOG"
echo "--- Python Tools ---" | tee -a "$INSTALL_LOG"
for tool in slither echidna; do
    check_installed "$tool" || true
done

echo "" | tee -a "$INSTALL_LOG"
echo "--- Foundry Tools ---" | tee -a "$INSTALL_LOG"
for tool in forge cast anvil; do
    check_installed "$tool" || true
done

echo "" | tee -a "$INSTALL_LOG"
echo "--- Infrastructure Tools ---" | tee -a "$INSTALL_LOG"
for tool in k6 toxiproxy-cli subwasm; do
    check_installed "$tool" || true
done

echo "" | tee -a "$INSTALL_LOG"
echo "Installation complete: $(date)" | tee -a "$INSTALL_LOG"
echo "Log: $INSTALL_LOG" | tee -a "$INSTALL_LOG"