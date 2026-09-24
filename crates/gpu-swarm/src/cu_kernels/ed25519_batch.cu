/*
 * Ed25519 Batch Signature Verification — CUDA Kernel
 *
 * Each thread verifies one Ed25519 signature independently.
 * The verification equation:
 *   [s]B == R + [h]A
 * where:
 *   (R, s) = signature (R is 32 bytes, s is 32 bytes)
 *   A      = public key (32-byte compressed point)
 *   M      = message (up to 64 bytes, typically 32 = SHA256 of tx)
 *   h      = SHA-512(R || A || M) mod ℓ
 *   B      = Ed25519 base point
 *   ℓ      = group order = 2^252 + 27742317777372353535851937790883648493
 *
 * We verify by computing [s]B - [h]A and checking == R.
 * Equivalently: ge_double_scalarmult([h], -A, [s], B) and compare output to R.
 *
 * Target: 500k+ sig/sec on 3× GTX 1070 (sm_61)
 * Approach: 1 thread = 1 signature verification
 *   (Ed25519 verification involves ~256 point doublings + ~128 point adds,
 *    which is compute-heavy enough that 1 thread per sig is the right granularity)
 *
 * Build:
 *   nvcc -arch=sm_61 -O2 -shared -Xcompiler -fPIC \
 *        ed25519_batch.cu -o build/libed25519_batch.so
 */

#include <cuda_runtime.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include "ed25519_field.cuh"
#include "ed25519_ge.cuh"
#include "sha512_device.cuh"

/* ───── Ed25519 base point B (compressed: y-coordinate) ───── */
/* B = (x, 4/5) where x is positive, encoded in 32 bytes LE */
__device__ static const unsigned char ed25519_basepoint_bytes[32] = {
    0x58, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
    0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
    0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
    0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66
};

/*
 * Input layout per signature (flat buffer):
 *   [0..31]    signature R (32 bytes, compressed point)
 *   [32..63]   signature s (32 bytes, scalar LE)
 *   [64..95]   public key A (32 bytes, compressed point)
 *   [96..127]  message M (32 bytes — SHA256 hash of transaction)
 *
 * Total: 128 bytes per signature entry
 * Output: 1 byte per signature (1 = valid, 0 = invalid)
 */
#define SIG_ENTRY_SIZE 128

__global__ void ed25519_verify_batch_kernel(
    const unsigned char* __restrict__ entries,
    int count,
    unsigned char* __restrict__ results
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= count) return;

    const unsigned char *entry = entries + (size_t)idx * SIG_ENTRY_SIZE;
    const unsigned char *sig_R = entry;        /* 32 bytes */
    const unsigned char *sig_s = entry + 32;   /* 32 bytes */
    const unsigned char *pubkey = entry + 64;  /* 32 bytes */
    const unsigned char *msg = entry + 96;     /* 32 bytes */

    /* 1. Check s < ℓ (scalar must be reduced) */
    /* ℓ = 2^252 + 27742317777372353535851937790883648493
     * Top byte of s should have bit 252 clear at most.
     * Quick check: s[31] must have top 4 bits clear (< 0x10) */
    if (sig_s[31] & 0xf0) {
        results[idx] = 0;
        return;
    }

    /* 2. Decode public key A */
    ge_p3 A;
    if (ge_frombytes_negate_vartime(&A, pubkey) != 0) {
        results[idx] = 0;
        return;
    }
    /* A is now -A (negated) — this is what we want for verification */

    /* 3. Decode R from signature */
    ge_p3 R_point;
    if (ge_frombytes_negate_vartime(&R_point, sig_R) != 0) {
        results[idx] = 0;
        return;
    }
    /* ge_frombytes_negate_vartime returns -R; negate to get R for comparison. */
    fe_neg(&R_point.X, &R_point.X);
    fe_neg(&R_point.T, &R_point.T);

    /* 4. Compute h = SHA-512(R || A || M) mod ℓ */
    unsigned char hash_input[96];
    for (int i = 0; i < 32; i++) hash_input[i]      = sig_R[i];
    for (int i = 0; i < 32; i++) hash_input[32 + i]  = pubkey[i];
    for (int i = 0; i < 32; i++) hash_input[64 + i]  = msg[i];

    unsigned char h_full[64];
    sha512_hash(hash_input, 96, h_full);

    /* sc_reduce takes the raw SHA-512 bytes as-is (same convention as libsodium/nacl).
     * No byte reversal — both our sha512_hash (store64_be) and libsodium output big-endian
     * SHA-512, and sc_reduce reads them with load_3/load_4 the same way on both sides. */
    sc_reduce(h_full);  /* h = h_full mod ℓ, result in h_full[0..31] */

    unsigned char *h_bytes = h_full;

    /* 5. Compute check_point = [s]B + [h](-A) = [s]B - [h]A
     *    (recall A was decoded as -A by ge_frombytes_negate_vartime) */
    ge_p3 B;
    ge_frombytes_negate_vartime(&B, ed25519_basepoint_bytes);
    /* B is also negated, so negate it back */
    fe_neg(&B.X, &B.X);
    fe_neg(&B.T, &B.T);

    ge_p2 check_point;
    ge_double_scalarmult_vartime(&check_point, h_bytes, &A, sig_s, &B);

    /* 6. Encode check_point to bytes and compare with R */
    unsigned char check_bytes[32];
    ge_tobytes(check_bytes, &check_point);

    int valid = 1;
    for (int i = 0; i < 32; i++) {
        if (check_bytes[i] != sig_R[i]) {
            valid = 0;
            break;
        }
    }

    results[idx] = (unsigned char)valid;
}

/* ───── Host API ───── */

extern "C" int ed25519_verify_batch_host(
    const unsigned char *entries,   /* count * 128 bytes */
    int count,
    unsigned char *results          /* count bytes: 1=valid, 0=invalid */
) {
    if (count <= 0) return 0;

    unsigned char *d_entries = nullptr;
    unsigned char *d_results = nullptr;
    size_t entry_bytes = (size_t)count * SIG_ENTRY_SIZE;
    size_t result_bytes = (size_t)count;

    cudaError_t err;

    err = cudaMalloc(&d_entries, entry_bytes);
    if (err != cudaSuccess) {
        fprintf(stderr, "ed25519: cudaMalloc entries failed: %s\n", cudaGetErrorString(err));
        return -1;
    }
    err = cudaMalloc(&d_results, result_bytes);
    if (err != cudaSuccess) {
        cudaFree(d_entries);
        fprintf(stderr, "ed25519: cudaMalloc results failed: %s\n", cudaGetErrorString(err));
        return -1;
    }

    err = cudaMemcpy(d_entries, entries, entry_bytes, cudaMemcpyHostToDevice);
    if (err != cudaSuccess) {
        cudaFree(d_entries); cudaFree(d_results);
        return -1;
    }

    /* Clear results */
    cudaMemset(d_results, 0, result_bytes);

    /* Launch: 128 threads per block (good for sm_61 occupancy) */
    int threads_per_block = 128;
    int blocks = (count + threads_per_block - 1) / threads_per_block;

    ed25519_verify_batch_kernel<<<blocks, threads_per_block>>>(
        d_entries, count, d_results
    );

    err = cudaGetLastError();
    if (err != cudaSuccess) {
        fprintf(stderr, "ed25519: kernel launch failed: %s\n", cudaGetErrorString(err));
        cudaFree(d_entries); cudaFree(d_results);
        return -1;
    }

    err = cudaDeviceSynchronize();
    if (err != cudaSuccess) {
        fprintf(stderr, "ed25519: sync failed: %s\n", cudaGetErrorString(err));
        cudaFree(d_entries); cudaFree(d_results);
        return -1;
    }

    cudaMemcpy(results, d_results, result_bytes, cudaMemcpyDeviceToHost);

    cudaFree(d_entries);
    cudaFree(d_results);

    return 0;
}

/* ───── Kernel helper: scalar reduction ───── */
__global__ void sc_reduce_kernel(unsigned char *s) {
    sc_reduce(s);
}

/* ───── Test helper: scalar reduction (host wrapper) ───── */
extern "C" int ed25519_sc_reduce_host(
    const unsigned char *hash64, /* 64-byte hash input */
    unsigned char *out32        /* 32-byte reduced scalar output */
) {
    unsigned char *d_hash = nullptr;
    cudaError_t err = cudaMalloc(&d_hash, 64);
    if (err != cudaSuccess) {
        return -1;
    }

    err = cudaMemcpy(d_hash, hash64, 64, cudaMemcpyHostToDevice);
    if (err != cudaSuccess) {
        cudaFree(d_hash);
        return -1;
    }

    sc_reduce_kernel<<<1, 1>>>(d_hash);
    err = cudaGetLastError();
    if (err != cudaSuccess) {
        cudaFree(d_hash);
        return -1;
    }

    err = cudaMemcpy(out32, d_hash, 32, cudaMemcpyDeviceToHost);
    cudaFree(d_hash);
    if (err != cudaSuccess) {
        return -1;
    }

    return 0;
}

/* ───── Multi-GPU batch verification ───── */
extern "C" int ed25519_verify_batch_multi_gpu(
    const unsigned char *entries,
    int count,
    unsigned char *results
) {
    int device_count = 0;
    cudaGetDeviceCount(&device_count);

    if (device_count <= 1) {
        return ed25519_verify_batch_host(entries, count, results);
    }

    /* Split work evenly across GPUs */
    int per_gpu = (count + device_count - 1) / device_count;

    /* We'll use streams on different devices */
    for (int dev = 0; dev < device_count; dev++) {
        int start = dev * per_gpu;
        int chunk = per_gpu;
        if (start + chunk > count) chunk = count - start;
        if (chunk <= 0) continue;

        cudaSetDevice(dev);

        unsigned char *d_entries = nullptr;
        unsigned char *d_results = nullptr;
        size_t entry_bytes = (size_t)chunk * SIG_ENTRY_SIZE;
        size_t result_bytes = (size_t)chunk;

        cudaMalloc(&d_entries, entry_bytes);
        cudaMalloc(&d_results, result_bytes);
        cudaMemcpy(d_entries, entries + (size_t)start * SIG_ENTRY_SIZE, entry_bytes, cudaMemcpyHostToDevice);
        cudaMemset(d_results, 0, result_bytes);

        int threads = 128;
        int blocks = (chunk + threads - 1) / threads;
        ed25519_verify_batch_kernel<<<blocks, threads>>>(d_entries, chunk, d_results);

        cudaMemcpy(results + start, d_results, result_bytes, cudaMemcpyDeviceToHost);

        cudaFree(d_entries);
        cudaFree(d_results);
    }

    /* Sync all devices */
    for (int dev = 0; dev < device_count; dev++) {
        cudaSetDevice(dev);
        cudaDeviceSynchronize();
    }

    cudaSetDevice(0);
    return 0;
}

/* ───── Device capability check ───── */
extern "C" void ed25519_print_gpu_info() {
    int device_count = 0;
    cudaGetDeviceCount(&device_count);

    printf("╔═══════════════════════════════════════════════════════╗\n");
    printf("║     Ed25519 GPU Batch Verifier — Device Info          ║\n");
    printf("╠═══════════════════════════════════════════════════════╣\n");
    printf("║ CUDA devices: %d                                      ║\n", device_count);
    for (int i = 0; i < device_count; i++) {
        cudaDeviceProp props;
        cudaGetDeviceProperties(&props, i);
        printf("║ [%d] %s  SM %d.%d  %.1f GB  %d cores ║\n",
               i, props.name, props.major, props.minor,
               (float)props.totalGlobalMem / (1024*1024*1024),
               props.multiProcessorCount * 128);
    }
    printf("╚═══════════════════════════════════════════════════════╝\n");
}
