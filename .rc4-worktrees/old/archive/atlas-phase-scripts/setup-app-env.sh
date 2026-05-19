#!/bin/bash

# Setup environment variables for all X3 Desktop apps

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APPS=("explorer" "wallet" "dex" "x3-intelligence")

echo "Setting up environment variables for X3 Desktop apps..."

for app in "${APPS[@]}"; do
  app_dir="$REPO_ROOT/apps/$app"
  env_file="$app_dir/.env.local"
  
  if [ -d "$app_dir" ]; then
    echo "Configuring $app..."
    
    # Create .env.local if it doesn't exist
    if [ ! -f "$env_file" ]; then
      cat > "$env_file" << 'EOF'
# RPC Node Configuration
NEXT_PUBLIC_RPC_URL=http://127.0.0.1:9944
NEXT_PUBLIC_WS_URL=ws://127.0.0.1:9944

# API Gateway
NEXT_PUBLIC_API_URL=http://127.0.0.1:8080

# Network
NEXT_PUBLIC_CHAIN_ID=x3-testnet
NEXT_PUBLIC_NETWORK_NAME=X3 Chain

# Analytics
NEXT_PUBLIC_ANALYTICS_ENABLED=true

# Development
NEXT_PUBLIC_DEBUG=false
EOF
      echo "  ✓ Created $env_file"
    else
      echo "  ⚠ $env_file already exists"
    fi
  else
    echo "  ✗ Directory not found: $app_dir"
  fi
done

echo ""
echo "Environment setup complete!"
echo "You can now run: ./start-all-desktop-apps.sh"
