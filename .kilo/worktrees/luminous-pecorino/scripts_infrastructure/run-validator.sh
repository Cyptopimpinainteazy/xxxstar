#!/bin/bash
# Wrapper script to run GPU validator with proper environment

set -e
PROJECT_ROOT="/home/lojak/Desktop/x3-chain-master"
VENV_PYTHON="$PROJECT_ROOT/infra-structure/validator/.venv/bin/python"
GPU_LIBS_PATH="${HOME}/.venv-gpu-libs"

# Set GPU library paths - try both custom location and default build dir
export LD_LIBRARY_PATH="$GPU_LIBS_PATH:$PROJECT_ROOT/infra-structure/validator/kernels/build:${LD_LIBRARY_PATH:-}"
export PYTHONUNBUFFERED=1

# Run validator from the pkg directory
cd "$PROJECT_ROOT/infra-structure/validator"
exec "$VENV_PYTHON" -m cross_chain_gpu_validator.cli orchestrator
