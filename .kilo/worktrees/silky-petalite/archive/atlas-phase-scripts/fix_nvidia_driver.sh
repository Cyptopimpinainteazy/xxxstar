#!/bin/bash
# Complete Fix Script for Ubuntu 22.04
# Run with: sudo bash fix_nvidia_driver.sh

set -e

echo "============================================"
echo "   X3 CHAIN - SYSTEM FIX SCRIPT"
echo "============================================"
echo ""

# ============================================
# PART 1: NVIDIA DRIVER FIX
# ============================================
echo "=== PART 1: NVIDIA Driver ==="
echo "Detected: Ubuntu 22.04 with 3x GTX 1070 GPUs"
echo ""

# Update package list
echo "[1/4] Updating package list..."
apt update

# Install recommended NVIDIA driver
echo "[2/4] Installing nvidia-driver-535..."
apt install -y nvidia-driver-535

# Install utility tools
echo "[3/4] Installing CUDA utilities..."
apt install -y nvidia-utils-535

echo "[4/4] NVIDIA driver installed!"
echo ""

# ============================================
# PART 2: FIX X3 SERVICE
# ============================================
echo "=== PART 2: X3 Chain Node Service ==="
echo "Fixing binary path in systemd service..."

# Find the actual binary location
BINARY_PATH="/home/lojak/Desktop/x3-chain-master/target/release/x3-chain-node"

if [ -f "$BINARY_PATH" ]; then
    echo "✓ Found binary at: $BINARY_PATH"
    
    # Check if service file exists and fix it
    SERVICE_FILE="/etc/systemd/system/x3-chain-node.service"
    if [ -f "$SERVICE_FILE" ]; then
        echo "✓ Fixing service file to use correct binary path..."
        # The service needs to point to target/release not target/debug
        sed -i 's|target/debug/x3-chain-node|target/release/x3-chain-node|g' "$SERVICE_FILE"
        systemctl daemon-reload
        echo "✓ Service file updated!"
    else
        echo "⚠ Service file not found at $SERVICE_FILE"
    fi
else
    echo "✗ Binary not found at $BINARY_PATH"
    echo "  You may need to build the project first:"
    echo "  cd /home/lojak/Desktop/x3-chain-master && cargo build --release"
fi

echo ""
echo "============================================"
echo "   FIX COMPLETE!"
echo "============================================"
echo ""
echo "NEXT STEPS:"
echo "1. Reboot your system:     sudo reboot"
echo "2. After reboot, verify NVIDIA:  nvidia-smi"
echo "3. Start the service:     sudo systemctl start x3-chain-node"
echo ""
