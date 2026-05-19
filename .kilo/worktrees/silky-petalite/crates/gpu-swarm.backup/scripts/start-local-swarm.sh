#!/bin/bash
# start-local-swarm.sh
# Quickly spins up a local Coordinator and a single Node for testing the GPU Swarm.

set -e

# Base directory for config files
CONFIG_DIR=$(mktemp -d "/tmp/gpu-swarm.XXXXXX")
echo "Using temporary config directory: $CONFIG_DIR"

# Write default coordinator config
cat <<CONFIG > "$CONFIG_DIR/coordinator-config.toml"
[coordinator]
keypair_path = "$CONFIG_DIR/coordinator-keypair.json"
listen_addresses = ["/ip4/127.0.0.1/tcp/9100"]

[scheduler]
strategy = "ReputationWeighted"
max_queue_size = 10000

[verification]
min_verifiers = 1
verification_timeout_secs = 60

[reputation]
initial_score = 100
success_reward = 1
failure_penalty = 5
CONFIG

# Write default node config
cat <<CONFIG > "$CONFIG_DIR/node-config.toml"
[node]
keypair_path = "$CONFIG_DIR/node-keypair.json"
listen_addresses = ["/ip4/127.0.0.1/tcp/9102"]
min_stake = 1000
accepted_task_types = ["X3Bytecode", "MempoolSimulation", "ProofGeneration"]

[scheduler]
strategy = "BestFit"
max_concurrent_tasks = 4
task_timeout_secs = 300

[gpu]
# using fallback / dynamic
enabled_backends = ["vulkan", "cuda", "opencl"]
max_vram_bytes = 0 
CONFIG

# Set paths for the binaries (assumes ran from project root or inside crates/gpu-swarm)
WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
BIN_DIR="$WORKSPACE_ROOT/target/release"

if [ ! -f "$BIN_DIR/swarm-coordinator" ] || [ ! -f "$BIN_DIR/swarm-node" ]; then
    echo "Binaries not found in $BIN_DIR. Make sure to run 'cargo build --release -p gpu-swarm' first."
    exit 1
fi

echo "Starting Swarm Coordinator..."
export RUST_LOG=info
"$BIN_DIR/swarm-coordinator" --config "$CONFIG_DIR/coordinator-config.toml" &
COORD_PID=$!

# Give coordinator time to start P2P listener
sleep 2

echo "Starting Swarm Node..."
export GPU_BACKEND=vulkan # Override or unset as needed locally
"$BIN_DIR/swarm-node" --config "$CONFIG_DIR/node-config.toml" &
NODE_PID=$!

echo
echo "============================================================"
echo "Local GPU Swarm is running!"
echo "Coordinator PID : $COORD_PID"
echo "Node PID        : $NODE_PID"
echo "Local Admin UI  : http://127.0.0.1:9101 (from node)"
echo "Config Dir      : $CONFIG_DIR"
echo "============================================================"
echo
echo "Press Ctrl+C to stop both."

function cleanup {
    echo "Stopping Swarm..."
    kill $NODE_PID 2>/dev/null || true
    kill $COORD_PID 2>/dev/null || true
    rm -rf "$CONFIG_DIR"
    exit 0
}

trap cleanup SIGINT SIGTERM

wait
