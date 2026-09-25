#!/bin/bash
# Build CUDA kernels for X3 Chain Solana GPU acceleration
# Target: NVIDIA GTX 1070 (sm_61) — 3 GPUs
#
# Kernels:
#   1. ed25519_batch    — Ed25519 signature batch verification
#   2. sha256_batch     — SHA-256 batch hashing + PoH chain
#   3. stream_pipeline  — Stream-pipelined SHA-256 with pinned memory
#
# Output: shared libraries in build/ for FFI loading (Python ctypes / Rust dlopen)

set -euo pipefail

KERNEL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="${KERNEL_DIR}/build"

mkdir -p "${BUILD_DIR}"

# Detect CUDA
if [[ -n "${CUDA_HOME:-}" ]] && [[ -x "${CUDA_HOME}/bin/nvcc" ]]; then
    NVCC="${CUDA_HOME}/bin/nvcc"
elif command -v nvcc >/dev/null 2>&1; then
    NVCC="$(command -v nvcc)"
else
    echo "ERROR: nvcc not found. Set CUDA_HOME or install CUDA toolkit." >&2
    exit 1
fi

echo "Using nvcc: ${NVCC}"
${NVCC} --version | head -1

# Detect GPU architecture
GPU_ARCH="${GPU_ARCH:-sm_61}"  # Default: GTX 1070

# Common flags
NVCC_FLAGS=(
    -arch="${GPU_ARCH}"
    -O2
    -shared
    -Xcompiler -fPIC
    --use_fast_math            # Fast transcendentals (safe for integer crypto)
    -lineinfo                   # Debug info without perf penalty
    -maxrregcount=64            # Limit registers for better occupancy on sm_61
)

echo ""
echo "═══════════════════════════════════════════════════════"
echo "  Building Ed25519 batch verification kernel..."
echo "═══════════════════════════════════════════════════════"
${NVCC} "${NVCC_FLAGS[@]}" \
    "${KERNEL_DIR}/ed25519_batch.cu" \
    -o "${BUILD_DIR}/libed25519_batch.so" \
    2>&1

echo "  → ${BUILD_DIR}/libed25519_batch.so"

echo ""
echo "═══════════════════════════════════════════════════════"
echo "  Building SHA-256 batch + PoH chain kernel..."
echo "═══════════════════════════════════════════════════════"
${NVCC} "${NVCC_FLAGS[@]}" \
    "${KERNEL_DIR}/sha256_batch.cu" \
    -o "${BUILD_DIR}/libsha256_batch.so" \
    2>&1

echo "  → ${BUILD_DIR}/libsha256_batch.so"

echo ""
echo "═══════════════════════════════════════════════════════"
echo "  Building Stream Pipeline kernel..."
echo "═══════════════════════════════════════════════════════"
${NVCC} "${NVCC_FLAGS[@]}" \
    "${KERNEL_DIR}/stream_pipeline.cu" \
    -o "${BUILD_DIR}/libstream_pipeline.so" \
    2>&1

echo "  → ${BUILD_DIR}/libstream_pipeline.so"

echo ""
echo "═══════════════════════════════════════════════════════"
echo "  Running ed25519 kernel unit test..."
echo "══════════════════════════════════════════════════════="
${NVCC} -arch="${GPU_ARCH}" -O2 -o "${BUILD_DIR}/ed25519_batch_test" \
    "${KERNEL_DIR}/ed25519_batch.cu" \
    "${KERNEL_DIR}/ed25519_batch_kernel_test.cu"
"${BUILD_DIR}/ed25519_batch_test"

echo ""
echo "══════════════════════════════════════════════════════="
echo "  Build Summary"
echo "══════════════════════════════════════════════════════="
ls -lh "${BUILD_DIR}"/*.so 2>/dev/null || echo "  (no .so files found)"
echo "" 
echo "  Architecture: ${GPU_ARCH}"
echo "  Kernels built: ed25519_batch, sha256_batch, stream_pipeline"
echo "═══════════════════════════════════════════════════════"
