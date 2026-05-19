#!/bin/bash

# Start IPFS Node for X3 Social Network Media Storage

set -e

echo "🚀 X3 Social Network - IPFS Node Setup"
echo "========================================"

# Check if IPFS is installed
if ! command -v ipfs &> /dev/null; then
    echo "📦 Installing IPFS..."
    
    if command -v brew &> /dev/null; then
        brew install ipfs
    elif command -v apt &> /dev/null; then
        sudo apt-get update
        sudo apt-get install -y ipfs
    else
        echo "❌ Could not auto-install IPFS"
        echo "📖 Manual installation: https://docs.ipfs.io/install/command-line/"
        exit 1
    fi
    
    echo "✅ IPFS installed"
fi

# Initialize IPFS repo if needed
if [ ! -d ~/.ipfs ]; then
    echo "🔧 Initializing IPFS repository..."
    ipfs init --profile default
    echo "✅ IPFS repository initialized"
fi

# Start IPFS daemon
echo ""
echo "🌐 Starting IPFS daemon..."
echo "   (Running in background, use 'killall ipfs' to stop)"
echo ""

# Check if already running
if netstat -tuln 2>/dev/null | grep -q ":5001"; then
    echo "✅ IPFS API already listening on port 5001"
elif pgrep -f "ipfs daemon" > /dev/null; then
    echo "✅ IPFS daemon already running"
else
    # Start daemon in background with nohup
    nohup ipfs daemon > /tmp/ipfs.log 2>&1 &
    IPFS_PID=$!
    echo "✅ IPFS daemon started (PID: $IPFS_PID)"
    
    # Wait for startup
    sleep 3
fi

# Verify connectivity
echo ""
echo "🧪 Verifying IPFS connectivity..."

python3 << 'PYTHON_EOF'
import urllib.request
import json
import time

max_attempts = 5
attempt = 0

while attempt < max_attempts:
    try:
        # Check if IPFS API is responding
        response = urllib.request.urlopen('http://127.0.0.1:5001/api/v0/id', timeout=5)
        data = json.loads(response.read())
        
        print("✅ IPFS API responding")
        print(f"   Peer ID: {data['ID'][:16]}...")
        print(f"   Addresses: {len(data.get('Addresses', []))} found")
        break
    except Exception as e:
        attempt += 1
        if attempt < max_attempts:
            print(f"⏳ Waiting for IPFS to start... (attempt {attempt}/{max_attempts})")
            time.sleep(2)
        else:
            print(f"❌ Could not connect to IPFS API: {e}")
            print("   Make sure the daemon is running:")
            print("   $ ipfs daemon")
            exit(1)
PYTHON_EOF

if [ $? -ne 0 ]; then
    exit 1
fi

# Show gateway info
echo ""
echo "✅ IPFS Node Ready!"
echo ""
echo "🌐 IPFS Access Points:"
echo "   API:     http://127.0.0.1:5001"
echo "   Gateway: http://127.0.0.1:8080"
echo ""
echo "📝 X3 Configuration (.env):"
echo "   IPFS_API_URL=http://127.0.0.1:5001"
echo "   IPFS_GATEWAY=http://127.0.0.1:8080"
echo ""
echo "📊 Monitor IPFS Activity:"
echo "   $ ipfs stats"
echo ""
echo "📁 Upload a file to test:"
echo "   $ ipfs add path/to/file"
echo ""
echo "🚀 Ready to use IPFS for media storage in X3 Social!"
echo ""
echo "💡 To stop IPFS daemon:"
echo "   $ killall ipfs"
