#!/bin/bash
# Setup auto-start services for X3 Chain

set -e

PROJECT_ROOT="/home/lojak/Desktop/x3-chain-master"
SERVICES_DIR="$PROJECT_ROOT/services"

echo "╔════════════════════════════════════════════════════════════╗"
echo "║  X3 Chain - Auto-Start Service Setup                   ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Check if running as root
if [[ $EUID -ne 0 ]]; then
   echo "⚠️  This script must be run as root (sudo)"
   echo "Run: sudo bash $0"
   exit 1
fi

echo "[*] Installing systemd services..."
echo ""

# Install x3-intelligence service
echo "[*] Installing X3 Intelligence Dashboard service..."
cp "$SERVICES_DIR/x3-intelligence.service" /etc/systemd/system/
systemctl daemon-reload
systemctl enable x3-intelligence.service
echo "✓ Service installed: x3-intelligence"
echo ""

# Install CCGV validator service
echo "[*] Installing Cross-Chain GPU Validator service..."
cp "$SERVICES_DIR/ccgv-validator.service" /etc/systemd/system/
systemctl daemon-reload
systemctl enable ccgv-validator.service
echo "✓ Service installed: ccgv-validator"
echo ""

echo "╔════════════════════════════════════════════════════════════╗"
echo "║  Setup Complete!                                           ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""
echo "Services installed and will start on boot."
echo ""
echo "To start services immediately:"
echo "  sudo systemctl start x3-intelligence"
echo "  sudo systemctl start ccgv-validator"
echo ""
echo "To check status:"
echo "  sudo systemctl status x3-intelligence"
echo "  sudo systemctl status ccgv-validator"
echo ""
echo "To view logs:"
echo "  sudo journalctl -u x3-intelligence -f"
echo "  sudo journalctl -u ccgv-validator -f"
echo ""
echo "To disable auto-start:"
echo "  sudo systemctl disable x3-intelligence"
echo "  sudo systemctl disable ccgv-validator"
echo ""
