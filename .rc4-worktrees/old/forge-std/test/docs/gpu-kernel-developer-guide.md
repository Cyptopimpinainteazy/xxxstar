# X3 Chain — GPU Kernel Developer Guide

## Overview

This document covers the architecture, optimization techniques, and development workflow for the CUDA GPU kernels used in the X3 Chain multi-chain validator.

## Kernel Inventory

| Kernel | File | Output .so | Purpose |
|--------|------|-----------|---------|
| secp256k1 verify | `secp256k1_optimized.cu` | `libsecp256k1_batch.so` | ECDSA signature verification (EVM/Cosmos) |
| Keccak-256 | `keccak256_batch.cu` | `libkeccak256_batch.so` | Keccak hash for EVM transaction hashing |
| SHA-256 | `sha256_batch.cu` (gpu-swarm) | `libsha256_batch.so` | SHA-256 for SVM/Cosmos/Substrate |
| Ed25519 | `ed25519_batch.cu` (gpu-swarm) | `libed25519_batch.so` | Signature verify for SVM/Substrate |
| PoH pipeline | `stream_pipeline.cu` (gpu-swarm) | `libstream_pipeline.so` | Proof-of-History chain verify (SVM) |

## secp256k1 Optimization Deep-Dive

### Problem: Naive Affine Implementation

The original `secp256k1_batch.cu` used affine point arithmetic:
- Each point addition required a **modular inverse** (256-bit extended GCD)
- ~266,000 field multiplications per signature verification
- Result: ~3,700 signatures/sec on GTX 1070

### Solution: Jacobian Projective + Shamir's Trick

The optimized `secp256k1_optimized.cu` uses two key techniques:

#### 1. Jacobian Projective Coordinates

Points stored as (X, Y, Z) where the affine point is (X/Z², Y/Z³).

- **Point doubling**: ~8 field muls, 0 inversions
- **Point addition**: ~12 field muls, 0 inversions  
- **Only 1 inversion** at the very end (to convert back to affine)

```
// Old: affine add (1 inversion per add)
inv = mod_inverse(x2 - x1)    // EXPENSIVE: ~256 field muls
slope = (y2 - y1) * inv
x3 = slope² - x1 - x2
y3 = slope * (x1 - x3) - y1

// New: Jacobian add (0 inversions)
U1 = X1 * Z2²;  U2 = X2 * Z1²
S1 = Y1 * Z2³;  S2 = Y2 * Z1³
H = U2 - U1;    R = S2 - S1
X3 = R² - H³ - 2*U1*H²
Y3 = R*(U1*H² - X3) - S1*H³
Z3 = H * Z1 * Z2
```

#### 2. Shamir's Trick (Joint Scalar Multiplication)

ECDSA verify computes `u1*G + u2*Q`. Instead of two separate 256-bit scalar multiplications:

```
// Old: 2 × 256 iterations = 512 double-and-add loops
R1 = u1 * G    // 256 iterations
R2 = u2 * Q    // 256 iterations
result = R1 + R2

// New: 1 × 256 iterations using precomputed table
table[0] = IDENTITY
table[1] = Q
table[2] = G  
table[3] = G + Q
for bit 255..0:
    R = jac_double(R)
    idx = (u2_bit << 1) | u1_bit
    if idx > 0: R = jac_add(R, table[idx])
```

#### Performance Impact

| Metric | Naive Affine | Jacobian + Shamir | Improvement |
|--------|-------------|-------------------|-------------|
| Inversions per sig | ~512 | 1 | 512× fewer |
| Field muls per sig | ~266,000 | ~5,900 | 45× fewer |
| GPU ops/sec (GTX 1070) | 3,700 | 115,617 | 31× |
| CPU ops/sec | 2,538 | N/A | — |
| GPU vs CPU speedup | 1.5× | 45.6× | — |

### Multi-GPU Support

```c
extern "C" int secp256k1_ecdsa_verify_multi_gpu(
    const uint8_t* u1_scalars,   // count × 32 bytes
    const uint8_t* u2_scalars,   // count × 32 bytes
    const uint8_t* pubkeys,      // count × 64 bytes (x,y uncompressed)
    int count,
    uint8_t* out_x               // count × 32 bytes result
);
```

Work is split across available GPUs with `cudaSetDevice()`. Each GPU gets a CUDA stream for async H2D → kernel → D2H pipelining.

## Building Kernels

```bash
cd cross-chain-gpu-validator/kernels
bash build.sh
```

To target a different GPU architecture:
```bash
export GPU_ARCH=sm_86  # RTX 3090
bash build.sh
```

## Adding a New Kernel

1. **Write CUDA kernel** in `cross-chain-gpu-validator/kernels/`
2. **Add to build.sh** — include the `nvcc` compilation line
3. **Add Rust FFI** in `crates/x3-vm/src/gpu_hostcalls.rs`:
   - Define FFI function type
   - Add library struct and loader
   - Add handler function
4. **Add opcode** in `crates/x3-backend/src/opcode.rs` (next available 0xD_ value)
5. **Add VM dispatch** in `crates/x3-vm/src/vm.rs`
6. **Add instruction size** in `crates/x3-bench/src/pipeline.rs`
7. **Add kernel profile** in `kernel_profiles.py` for chain family mapping
8. **Write tests** in `cross-chain-gpu-validator/tests/`
9. **Benchmark** using `tests/p4_benchmarks/crypto_bench.py`

## Constant Memory

secp256k1 uses CUDA `__constant__` memory for the prime and generator:
```c
__constant__ uint32_t c_prime[8];       // secp256k1 field prime p
__constant__ uint32_t c_generator[16];  // G.x (8) + G.y (8)
```

These are set once via `setup_constants()` and are read-only broadcast to all SMs.

## Thread Configuration

| Kernel | Block Size | Grid Size | Shared Mem |
|--------|-----------|-----------|------------|
| secp256k1 | 256 | ⌈count/256⌉ | 0 |
| Keccak-256 | 256 | ⌈count/256⌉ | 0 |
| SHA-256 | 256 | ⌈count/256⌉ | 0 |
| Ed25519 | 128 | ⌈count/128⌉ | 0 |
| PoH stream | 1 | 1 | 0 (sequential by design) |

## Debugging

```bash
# Check kernel launch errors
cuda-gdb ./test_binary

# Memory checking
compute-sanitizer --tool memcheck ./test_binary

# Profiling
nsys profile -o report ./test_binary
ncu --target-processes all ./test_binary
```
