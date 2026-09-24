#!/bin/bash
# One-shot build of libed25519_batch.so — bypasses main build.sh interruptions
set -e
KDIR="/home/lojak/Desktop/x3-chain-master/crates/gpu-swarm/src/cu_kernels"
NVCC=/usr/local/cuda/bin/nvcc
LOG="$KDIR/build/ed25519_build.log"

echo "[$(date)] Starting ed25519_batch.so build" | tee "$LOG"
"$NVCC" -arch=sm_61 -O2 -shared -Xcompiler -fPIC \
    "$KDIR/ed25519_batch.cu" \
    -o "$KDIR/build/libed25519_batch.so" 2>&1 | tee -a "$LOG"
echo "[$(date)] BUILD COMPLETE — $(stat -c '%s' $KDIR/build/libed25519_batch.so) bytes" | tee -a "$LOG"
