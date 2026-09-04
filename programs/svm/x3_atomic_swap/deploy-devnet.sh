#!/usr/bin/env bash
# Deploy x3-atomic-swap Solana program to devnet
set -euo pipefail

echo "=== X3 Atomic Swap Solana Program Deploy (Devnet) ==="

# Check prerequisites
command -v solana >/dev/null 2>&1 || { echo "Error: solana CLI not installed"; exit 1; }
command -v cargo-build-sbf >/dev/null 2>&1 || command -v cargo-build-bpf >/dev/null 2>&1 || { echo "Error: cargo-build-sbf not installed (try: cargo install solana-cli)"; exit 1; }

# Configure for devnet
echo "Configuring solana CLI for devnet..."
solana config set --url https://api.devnet.solana.com

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Build the BPF program
echo ""
echo "Building BPF program..."
cargo build-sbf --manifest-path "$SCRIPT_DIR/Cargo.toml" 2>&1 || cargo build-bpf --manifest-path "$SCRIPT_DIR/Cargo.toml" 2>&1

# Get the program keypair (create if not exists)
PROGRAM_DIR="$SCRIPT_DIR/target/deploy"
mkdir -p "$PROGRAM_DIR"
PROGRAM_KEYPAIR="$PROGRAM_DIR/x3_atomic_swap-keypair.json"

if [ ! -f "$PROGRAM_KEYPAIR" ]; then
    echo ""
    echo "Creating program keypair..."
    solana-keygen new --no-bip39-passphrase -f -o "$PROGRAM_KEYPAIR"
fi

PROGRAM_ID=$(solana-keygen pubkey "$PROGRAM_KEYPAIR")
echo ""
echo "Program ID: $PROGRAM_ID"

# Deploy
echo ""
echo "Deploying to Solana devnet..."
echo "This may take a minute and cost SOL for rent + deployment fee."
echo ""
solana program deploy \
    --program-id "$PROGRAM_KEYPAIR" \
    "$PROGRAM_DIR/x3_atomic_swap.so"

echo ""
echo "=== Deployment complete ==="
echo "Program ID: $PROGRAM_ID"
echo "Explorer: https://explorer.solana.com/address/$PROGRAM_ID?cluster=devnet"
echo ""
echo "Update devnet-config.json and relayer config with this Program ID."
