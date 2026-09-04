#!/usr/bin/env bash
set -euo pipefail

if ! command -v nvidia-smi >/dev/null 2>&1; then
  echo "GPU not detected. Install NVIDIA drivers or run on a host with GPU support."
  exit 1
fi

echo "Detected GPU devices:"
nvidia-smi --query-gpu=name,uuid,memory.total --format=csv,noheader

echo "GPU swarm worker placeholder"
echo "This script is a starting point for launching GPU-accelerated swarm workers."

echo "Use the API task list to claim GPU work and report progress."
