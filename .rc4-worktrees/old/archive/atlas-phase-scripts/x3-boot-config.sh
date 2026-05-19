#!/bin/bash
# Source this file in your shell rc file to enable boot commands
# Add to ~/.bashrc or ~/.zshrc:
#   source /path/to/x3-boot-config.sh

PROJECT_ROOT="/home/lojak/Desktop/x3-chain-master"

# Quick commands for managing The Beast
alias beast-start="bash $PROJECT_ROOT/scripts/start-beast.sh"
alias beast-stop="bash $PROJECT_ROOT/scripts/stop-beast.sh"
alias beast-status="echo 'X3 Intelligence:' && curl -s http://localhost:5173 > /dev/null && echo '✓ Running' || echo '✗ Not running'; echo 'GPU Validator:' && curl -s http://localhost:8000/metrics.json > /dev/null && echo '✓ Running' || echo '✗ Not running'"
alias beast-logs-x3="tail -f /tmp/x3-intelligence.log"
alias beast-logs-gpu="tail -f /tmp/ccgv-validator.log"
alias beast-login="echo 'Username: admin'; echo 'Password: x3-chain-2026'; echo 'Dashboard: http://localhost:5173'"

# Function to setup systemd auto-start
function beast-setup-autostart() {
    echo "Setting up systemd auto-start services..."
    sudo bash "$PROJECT_ROOT/scripts/setup-autostart.sh"
}

# Export functions
export -f beast-setup-autostart

echo "✓ X3 Chain boot commands loaded!"
echo ""
echo "Available commands:"
echo "  beast-start          - Start all services manually"
echo "  beast-stop           - Stop all services"
echo "  beast-status         - Check service status"
echo "  beast-logs-x3        - View X3 Intelligence logs"
echo "  beast-logs-gpu       - View GPU Validator logs"
echo "  beast-login          - Show login credentials"
echo "  beast-setup-autostart - Enable auto-start on boot (requires sudo)"
echo ""
