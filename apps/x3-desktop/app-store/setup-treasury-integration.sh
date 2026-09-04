#!/bin/bash
# X3 Desktop App Store - Setup Script
# Configures all apps with X3 Treasury integration

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_STORE_DIR="$SCRIPT_DIR"

# Optional: pass a single app directory name to only setup that app
TARGET_APP="$1"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
ORANGE='\033[0;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   X3 Desktop App Store - Treasury Setup    ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════╝${NC}"
echo ""

# Check if X3 treasury address is set
if [ -z "$X3_TREASURY_ADDRESS" ]; then
    echo -e "${ORANGE}⚠️  X3_TREASURY_ADDRESS environment variable not set${NC}"
    echo "Please set your treasury addresses:"
    echo ""
    echo "  export X3_TREASURY_ADDRESS='your_main_treasury_address'"
    echo "  export X3_TREASURY_ETH='0x...'"
    echo "  export X3_TREASURY_BSC='0x...'"
    echo "  export X3_TREASURY_SOL='...'"
    echo ""
    read -p "Continue with default placeholders? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

echo -e "${GREEN}✅ Treasury Configuration${NC}"
echo "   Main Address: ${X3_TREASURY_ADDRESS:-'X3Treasury_DefaultAddress'}"
echo "   ETH Address: ${X3_TREASURY_ETH:-'0xX3_TREASURY_ETH'}"
echo "   BSC Address: ${X3_TREASURY_BSC:-'0xX3_TREASURY_BSC'}"
echo "   SOL Address: ${X3_TREASURY_SOL:-'X3Treasury_SOL_ADDRESS'}"
echo ""

# Function to setup an app
setup_app() {
    local app_name=$1
    local app_dir="$APP_STORE_DIR/$app_name"
    
    echo -e "${BLUE}→ Setting up ${app_name}...${NC}"
    
    if [ ! -d "$app_dir" ]; then
        echo -e "${RED}  ❌ Directory not found: $app_dir${NC}"
        return 1
    fi
    
    cd "$app_dir"
    
    # Install dependencies if needed
    if [ -f "package.json" ]; then
        echo "  📦 Installing Node.js dependencies..."
        npm install --silent > /dev/null 2>&1 || true
    fi
    
    if [ -f "requirements.txt" ]; then
        echo "  🐍 Creating virtualenv and installing Python dependencies..."
        # create a venv per-app to avoid polluting the global environment
        python3 -m venv .venv > /dev/null 2>&1 || true
        source .venv/bin/activate > /dev/null 2>&1 || true
        pip install -r requirements.txt --quiet > /dev/null 2>&1 || true
        deactivate > /dev/null 2>&1 || true
    fi
    
    if [ -f "Cargo.toml" ]; then
        echo "  🦀 Building Rust project..."
        cargo build --release --quiet > /dev/null 2>&1 || true
    fi
    
    echo -e "${GREEN}  ✅ ${app_name} setup complete${NC}"
    echo ""

    # Copy app into Tauri appDataDir so the AppLauncher can find it during dev
    TAURI_DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/com.atlassphere.atlasdesktop"
    if [ -d "$TAURI_DATA_DIR" ]; then
        mkdir -p "$TAURI_DATA_DIR/app-store"
        rm -rf "$TAURI_DATA_DIR/app-store/$app_name" || true
        cp -a "$app_dir" "$TAURI_DATA_DIR/app-store/"
        echo "  🔁 Copied $app_name to $TAURI_DATA_DIR/app-store/$app_name"
    fi
}

# Setup each app
echo -e "${BLUE}Setting up integrated apps...${NC}"
echo ""

# Auto-discover and set up all directories under app-store (makes the script future-proof)
for app_dir in "$APP_STORE_DIR"/*/; do
    app_name=$(basename "$app_dir")

    # Skip helper files
    if [[ "$app_name" == "COMPLETION_REPORT_12APPS.md" || "$app_name" == "INTEGRATION_COMPLETE.md" || "$app_name" == "docs/root/README.md" ]]; then
        continue
    fi

    # If a target app was provided, skip others
    if [[ -n "$TARGET_APP" && "$TARGET_APP" != "$app_name" ]]; then
        continue
    fi

    setup_app "$app_name"
done

# Also copy local workspace helper app (x3-app-store) so the AppLauncher can start it during dev
X3_WORKSPACE_APPSTORE_DIR="$SCRIPT_DIR/../../../x3-app-store"
if [ -d "$X3_WORKSPACE_APPSTORE_DIR" ]; then
    echo -e "${BLUE}→ Copying local x3-app-store workspace into Tauri app-data...${NC}"
    cp -a "$X3_WORKSPACE_APPSTORE_DIR" "$TAURI_DATA_DIR/app-store/" 2>/dev/null || true
    echo -e "${GREEN}  ✅ x3-app-store copied to Tauri app-store directory${NC}"
fi

echo ""
echo -e "${GREEN}╔════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║              Setup Complete! 🎉                ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════╝${NC}"
echo ""
echo "All apps are configured with X3 Treasury integration (50% split)"
echo ""
echo "Next steps:"
echo "  1. Launch X3 Desktop: cd .. && npm run tauri:dev"
echo "  2. Open App Store from Tools menu (Ctrl+Shift+A)"
echo "  3. Launch any app to start earning with treasury split"
echo ""
echo "Treasury addresses can be updated in:"
echo "  - src/config/treasury.config.ts (main config)"
echo "  - app-store/*/x3-treasury-config.* (per-app configs)"
echo ""
echo -e "${BLUE}Happy earnings! 💰${NC}"
