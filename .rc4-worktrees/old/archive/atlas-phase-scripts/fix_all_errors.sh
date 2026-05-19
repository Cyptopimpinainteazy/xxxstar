#!/bin/bash
# COMPLETE ERROR FIX SCRIPT
# Run with: sudo bash fix_all_errors.sh

set -e

echo "============================================"
echo "   FIXING ALL SYSTEM ERRORS"
echo "============================================"
echo ""

# ============================================
# FIX 1: NVIDIA DRIVER
# ============================================
echo "=== FIX 1: NVIDIA Driver ==="
apt update -qq
apt install -y nvidia-driver-535
echo "✓ NVIDIA driver installed"
echo ""

# ============================================
# FIX 2: X3 SERVICE BINARY PATH
# ============================================
echo "=== FIX 2: X3 Service Binary Path ==="
sed -i 's|target/debug/x3-chain-node|target/release/x3-chain-node|g' /etc/systemd/system/x3-chain-node.service
systemctl daemon-reload
echo "✓ Fixed x3-chain-node.service path"
echo ""

# ============================================
# FIX 3: X3 EVERYTHING SERVICE
# ============================================
echo "=== FIX 3: X3 Everything Service ==="
if [ -f /etc/systemd/system/x3-chain-everything.service ]; then
    sed -i 's|target/debug/x3-chain-node|target/release/x3-chain-node|g' /etc/systemd/system/x3-chain-everything.service
    systemctl daemon-reload
    echo "✓ Fixed x3-chain-everything.service"
fi
echo ""

# ============================================
# FIX 4: NETWORK WAIT ONLINE
# ============================================
echo "=== FIX 4: Network Service ==="
systemctl disable systemd-networkd-wait-online.service 2>/dev/null || true
echo "✓ Disabled network wait-online (not needed for most setups)"
echo ""

# ============================================
# FIX 5: APACHE2 (if not needed)
# ============================================
echo "=== FIX 5: Apache2 ==="
systemctl stop apache2 2>/dev/null || true
systemctl disable apache2 2>/dev/null || true
echo "✓ Stopped Apache2 (can enable if needed)"
echo ""

echo "============================================"
echo "   ALL FIXES COMPLETE"
echo "============================================"
echo ""
echo "NOW RUN:"
echo "  sudo reboot"
echo ""
echo "AFTER REBOOT, VERIFY NVIDIA:"
echo "  nvidia-smi"
