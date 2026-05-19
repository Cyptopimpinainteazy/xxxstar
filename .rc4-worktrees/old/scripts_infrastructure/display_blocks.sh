#!/bin/bash
# Terminal block display wrapper for x3-chain
# Displays neon rainbow blocks when blocks are finalized

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Default to 8 blocks if not specified
NUM_BLOCKS="${1:-8}"

python3 "${SCRIPT_DIR}/block_visualizer.py" "$NUM_BLOCKS"
