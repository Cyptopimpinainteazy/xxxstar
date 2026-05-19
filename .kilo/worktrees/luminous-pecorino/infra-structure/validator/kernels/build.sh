#!/bin/bash
set -euo pipefail
# ---------------------------------------------------------------------------
# X3 Chain — Build all 5 GPU crypto kernels
#
# Targets:  GTX 1070 (sm_61) × 3  on Threadripper core node
#           GTX 1070 (sm_61) × 17 on secondary servers
#
# Outputs:
#   build/libsecp256k1_batch.so   — Optimized secp256k1 (Jacobian + Shamir)
#   build/libkeccak256_batch.so   — Keccak-256 batch hashing
#   build/libsha256_batch.so      — SHA-256 batch + PoH (symlink)
#   build/libed25519_batch.so     — Ed25519 batch verify (symlink)
#   build/libstream_pipeline.so   — Stream-pipelined SHA-256 (symlink)
# ---------------------------------------------------------------------------

KERNEL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="${KERNEL_DIR}/build"
GPU_SWARM_DIR="${KERNEL_DIR}/../../crates/gpu-swarm/src/cu_kernels"

mkdir -p "${OUTPUT_DIR}"

if ! command -v nvcc >/dev/null 2>&1; then
  echo "nvcc not found. Install CUDA toolkit before building kernels." >&2
  exit 1
fi

GPU_ARCH="${GPU_ARCH:-sm_61}"  # Default: GTX 1070
NVCC_FLAGS="-arch=${GPU_ARCH} -O2 -shared -Xcompiler -fPIC"

echo "=== Building GPU kernels (arch: ${GPU_ARCH}) ==="

# 1. Optimized secp256k1 with Jacobian coordinates + Shamir's trick
echo "[1/5] secp256k1 (optimized) ..."
nvcc ${NVCC_FLAGS} \
  -I "${KERNEL_DIR}/../third_party/secp256k1-cuda-ecc" \
  "${KERNEL_DIR}/secp256k1_optimized.cu" \
  -o "${OUTPUT_DIR}/libsecp256k1_batch.so"

# 2. Keccak-256
echo "[2/5] Keccak-256 ..."
nvcc ${NVCC_FLAGS} \
  "${KERNEL_DIR}/keccak256_batch.cu" \
  -o "${OUTPUT_DIR}/libkeccak256_batch.so"

# 3-5. SHA-256 / Ed25519 / Stream pipeline (from gpu-swarm crate)
if [[ -d "${GPU_SWARM_DIR}" ]]; then
  GPU_SWARM_BUILD="${GPU_SWARM_DIR}/build"
  mkdir -p "${GPU_SWARM_BUILD}"

  echo "[3/5] SHA-256 batch + PoH ..."
  nvcc ${NVCC_FLAGS} \
    "${GPU_SWARM_DIR}/sha256_batch.cu" \
    -o "${GPU_SWARM_BUILD}/libsha256_batch.so"

  echo "[4/5] Ed25519 batch verify ..."
  nvcc ${NVCC_FLAGS} \
    -I "${GPU_SWARM_DIR}" \
    "${GPU_SWARM_DIR}/ed25519_batch.cu" \
    -o "${GPU_SWARM_BUILD}/libed25519_batch.so"

  echo "[5/5] Stream pipeline ..."
  nvcc ${NVCC_FLAGS} \
    "${GPU_SWARM_DIR}/stream_pipeline.cu" \
    -o "${GPU_SWARM_BUILD}/libstream_pipeline.so"

  # Symlink into unified build/ for easy deployment
  for lib in libsha256_batch.so libed25519_batch.so libstream_pipeline.so; do
    ln -sf "${GPU_SWARM_BUILD}/${lib}" "${OUTPUT_DIR}/${lib}" 2>/dev/null || \
      cp "${GPU_SWARM_BUILD}/${lib}" "${OUTPUT_DIR}/${lib}"
  done
else
  echo "[3-5/5] gpu-swarm crate not found at ${GPU_SWARM_DIR}, skipping SHA/Ed25519/Pipeline"
fi

echo ""
echo "=== Built kernels ==="
ls -lh "${OUTPUT_DIR}"/*.so 2>/dev/null || echo "(no .so files found)"
echo ""
echo "Set X3_CUDA_LIB_DIR=${OUTPUT_DIR} to use these kernels."
