#!/bin/bash
# Install X3 Chain system services for auto-start on boot
# Run: sudo bash deployment/scripts/install-services.sh

set -e

PROJECT_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SERVICES_DIR="$PROJECT_ROOT/deployment/systemd"

echo "╔════════════════════════════════════════════════════════════╗"
echo "║  Installing X3 Chain System Services                  ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Check if running as root
if [[ $EUID -ne 0 ]]; then
   echo "❌ This script must be run as root (use: sudo $0)"
   exit 1
fi

# Create x3 system user if needed
if ! id "x3" &>/dev/null; then
    echo "👤 Creating 'x3' system user..."
    useradd --system --home /var/lib/x3 --shell /bin/false x3
    mkdir -p /var/lib/x3
    chown x3:x3 /var/lib/x3
else
    echo "✓ 'x3' user already exists"
fi

# Create log directory
mkdir -p /var/log/x3
chown x3:x3 /var/log/x3
chmod 755 /var/log/x3

# Verify binaries exist before installing
echo "📝 Verifying dependencies..."
if ! command -v redis-server &> /dev/null; then
    echo "⚠️  WARNING: redis-server not found. Install: sudo apt-get install redis-server"
fi

# Copy service files
echo "📝 Installing systemd service files..."
cp "$SERVICES_DIR/ccgv-validator.service" /etc/systemd/system/
cp "$SERVICES_DIR/x3-intelligence.service" /etc/systemd/system/
cp "$SERVICES_DIR/redis.service" /etc/systemd/system/

# Reload systemd daemon
echo "🔄 Reloading systemd daemon..."
systemctl daemon-reload || echo "⚠️  WARNING: systemctl daemon-reload failed. Your systemd may be unstable."

# Enable services (auto-start on boot)
echo "⚙️  Enabling services for auto-start..."
systemctl enable redis.service
systemctl enable ccgv-validator.service
systemctl enable x3-intelligence.service

echo ""
echo "╔════════════════════════════════════════════════════════════╗"
echo "║  Services Installed Successfully!                         ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""
echo "✓ Redis will auto-start on boot"
echo "✓ Cross-Chain GPU Validator will auto-start on boot"
echo "✓ X3 Intelligence Dashboard will auto-start on boot"
echo ""
echo "To start services now, run:"
echo "  sudo systemctl start redis.service"
echo "  sudo systemctl start ccgv-validator.service"
echo "  sudo systemctl start x3-intelligence.service"
echo ""
echo "To check service status:"
echo "  systemctl status redis.service"
echo "  systemctl status ccgv-validator.service"
echo "  systemctl status x3-intelligence.service"
echo ""
echo "To view service logs:"
echo "  journalctl -u ccgv-validator.service -f"
echo "  journalctl -u x3-intelligence.service -f"
echo "  journalctl -u redis.service -f"
echo ""
