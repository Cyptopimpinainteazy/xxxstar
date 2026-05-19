#!/usr/bin/env bash
set -euo pipefail

echo "X3 GPU Inventory"
echo "================="

if command -v nvidia-smi >/dev/null 2>&1; then
  nvidia-smi --query-gpu=index,name,memory.total,memory.free,driver_version --format=csv,noheader
else
  echo "nvidia-smi not found; listing /proc/driver/nvidia/gpus if available"
  if [ -d /proc/driver/nvidia/gpus ]; then
    find /proc/driver/nvidia/gpus -maxdepth 1 -mindepth 1 -type d | while read -r d; do
      echo "GPU: $d"
    done
  else
    echo "No NVIDIA GPU inventory source found."
  fi
fi

echo
if command -v lspci >/dev/null 2>&1; then
  echo "PCI devices relevant to GPU:" && lspci | grep -i nvidia || true
fi
