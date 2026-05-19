#!/bin/bash

# Local SMTP Setup for X3 Desktop - No external service needed
# Supports: MailHog (recommended) or Python SMTP

set -e

echo "🚀 X3 Desktop - Local SMTP Setup"
echo "=================================="

# Check if already running
if command -v mailhog &> /dev/null; then
    echo "✅ MailHog already installed"
elif command -v docker &> /dev/null && docker --version &> /dev/null; then
    echo "📦 Installing MailHog via Docker..."
    docker run -d \
      --name mailhog \
      -p 1025:1025 \
      -p 8025:8025 \
      mailhog/mailhog
    echo "✅ MailHog container started"
    echo "📧 Web UI available at: http://localhost:8025"
else
    echo "⚠️  Docker not found. Installing MailHog locally..."
    
    # Try brew (macOS)
    if command -v brew &> /dev/null; then
        brew install mailhog
        echo "✅ MailHog installed via Homebrew"
    # Try apt (Linux)
    elif command -v apt &> /dev/null; then
        sudo apt-get update
        sudo apt-get install -y mailhog
        echo "✅ MailHog installed via apt"
    else
        echo "❌ Could not auto-install MailHog"
        echo "Manual installation: https://github.com/mailhog/MailHog/releases"
        exit 1
    fi
fi

# Start MailHog if not running
if ! pgrep -f "mailhog" > /dev/null; then
    echo "🔧 Starting MailHog..."
    mailhog > /tmp/mailhog.log 2>&1 &
    sleep 2
    echo "✅ MailHog started in background"
else
    echo "✅ MailHog already running"
fi

# Update or create .env with SMTP settings
echo ""
echo "📝 Configuring .env for local SMTP..."

# Read current .env if it exists
if [ -f .env ]; then
    # Remove existing SMTP settings
    sed -i.bak '/^SMTP_/d' .env
    rm -f .env.bak
fi

# Add SMTP settings to .env
cat >> .env << 'EOF'

# ==========================================
# LOCAL SMTP CONFIGURATION (MailHog)
# ==========================================
# No API keys needed - for local development only
SMTP_HOST=127.0.0.1
SMTP_PORT=1025
SMTP_USER=dev-user
SMTP_PASSWORD=dev-password
SMTP_FROM_EMAIL=noreply@x3-dev.local
SMTP_FROM_NAME=X3 Desktop (Dev)
SMTP_ENABLE_TLS=false
SMTP_TIMEOUT_SECS=30

# ==========================================
# IPFS CONFIGURATION
# ==========================================
IPFS_API_URL=http://127.0.0.1:5001
IPFS_GATEWAY=http://127.0.0.1:8080
IPFS_TIMEOUT_SECS=30
IPFS_MAX_FILE_SIZE=104857600

# ==========================================
# WEB SOCKET CONFIGURATION
# ==========================================
WEBSOCKET_PORT=9001
WEBSOCKET_ADDR=127.0.0.1
EOF

echo "✅ .env updated with SMTP settings"

# Test SMTP connection
echo ""
echo "🧪 Testing SMTP connection..."

python3 << 'PYTHON_EOF'
import smtplib
import sys

try:
    server = smtplib.SMTP('127.0.0.1', 1025, timeout=5)
    server.quit()
    print("✅ SMTP connection successful (MailHog running)")
except Exception as e:
    print(f"⚠️  Could not connect to SMTP: {e}")
    print("   Make sure MailHog is running: mailhog")
    sys.exit(1)
PYTHON_EOF

if [ $? -ne 0 ]; then
    echo "❌ SMTP connection failed. Make sure MailHog is running:"
    echo "   $ mailhog"
    exit 1
fi

# Done!
echo ""
echo "✅ Local SMTP Setup Complete!"
echo ""
echo "📧 MailHog Web UI:"
echo "   http://localhost:8025"
echo ""
echo "🔧 SMTP Configuration in .env:"
echo "   Host: 127.0.0.1"
echo "   Port: 1025"
echo "   User: dev-user"
echo "   Password: dev-password"
echo ""
echo "💡 How to use:"
echo "   1. All emails are captured by MailHog (not actually sent)"
echo "   2. View emails at http://localhost:8025"
echo "   3. No real email service needed"
echo "   4. Perfect for development & testing"
echo ""
echo "🚀 Ready to test CRM email features!"
