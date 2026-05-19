/*
 * SHA-256 Batch Hashing — CUDA Kernel for PoH (Proof of History)
 *
 * Full FIPS 180-4 compliant SHA-256 implementation.
 * Each thread computes SHA-256 of a 32-byte input to produce a 32-byte output.
 *
 * For PoH chain: output[i] = SHA256(input[i]) where input is the prev hash.
 * Also supports independent batch hashing for signature pre-processing.
 *
 * Target: 10-20M hashes/sec on GTX 1070 (sm_61)
 *
 * Build:
 *   nvcc -arch=sm_61 -O2 -shared -Xcompiler -fPIC \
 *        sha256_batch.cu -o build/libsha256_batch.so
 */

#include <cuda_runtime.h>
#include <stdint.h>
#include <stdio.h>

/* SHA-256 round constants in constant memory (shared across warps) */
__constant__ uint32_t k_sha256[64] = {
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
    0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
    0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
    0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
    0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
    0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
};

/* ───── SHA-256 helper functions ───── */
__device__ __forceinline__ uint32_t rotr32(uint32_t x, int n) {
    return (x >> n) | (x << (32 - n));
}

__device__ __forceinline__ uint32_t sha256_Ch(uint32_t x, uint32_t y, uint32_t z) {
    return (x & y) ^ (~x & z);
}

__device__ __forceinline__ uint32_t sha256_Maj(uint32_t x, uint32_t y, uint32_t z) {
    return (x & y) ^ (x & z) ^ (y & z);
}

__device__ __forceinline__ uint32_t sha256_Sigma0(uint32_t x) {
    return rotr32(x, 2) ^ rotr32(x, 13) ^ rotr32(x, 22);
}

__device__ __forceinline__ uint32_t sha256_Sigma1(uint32_t x) {
    return rotr32(x, 6) ^ rotr32(x, 11) ^ rotr32(x, 25);
}

__device__ __forceinline__ uint32_t sha256_sigma0(uint32_t x) {
    return rotr32(x, 7) ^ rotr32(x, 18) ^ (x >> 3);
}

__device__ __forceinline__ uint32_t sha256_sigma1(uint32_t x) {
    return rotr32(x, 17) ^ rotr32(x, 19) ^ (x >> 10);
}

/*
 * sha256_transform_32: SHA-256 of exactly 32 bytes.
 *
 * Since the input is always 32 bytes:
 *   Block = input[0..31] || 0x80 || zeros || 0x00000100
 * This is exactly one 64-byte block, fully deterministic padding.
 */
__device__ void sha256_transform_32(const unsigned char *input, unsigned char *output) {
    /* Initial hash values */
    uint32_t h0 = 0x6a09e667, h1 = 0xbb67ae85;
    uint32_t h2 = 0x3c6ef372, h3 = 0xa54ff53a;
    uint32_t h4 = 0x510e527f, h5 = 0x9b05688c;
    uint32_t h6 = 0x1f83d9ab, h7 = 0x5be0cd19;

    /* Prepare message schedule W[0..63] */
    uint32_t W[64];

    /* W[0..7]: the 32-byte input as big-endian uint32s */
    #pragma unroll
    for (int i = 0; i < 8; i++) {
        const unsigned char *p = input + i * 4;
        W[i] = ((uint32_t)p[0] << 24) | ((uint32_t)p[1] << 16) |
               ((uint32_t)p[2] <<  8) | ((uint32_t)p[3]);
    }

    /* W[8] = 0x80000000 (padding bit) */
    W[8] = 0x80000000u;

    /* W[9..14] = 0 (padding zeros) */
    #pragma unroll
    for (int i = 9; i < 15; i++) W[i] = 0;

    /* W[15] = 256 (length in bits = 32 * 8) */
    W[15] = 256;

    /* Extend W[16..63] */
    #pragma unroll
    for (int i = 16; i < 64; i++) {
        W[i] = sha256_sigma1(W[i-2]) + W[i-7] + sha256_sigma0(W[i-15]) + W[i-16];
    }

    /* Compression */
    uint32_t a = h0, b = h1, c = h2, d = h3;
    uint32_t e = h4, f = h5, g = h6, hh = h7;

    #pragma unroll
    for (int i = 0; i < 64; i++) {
        uint32_t T1 = hh + sha256_Sigma1(e) + sha256_Ch(e, f, g) + k_sha256[i] + W[i];
        uint32_t T2 = sha256_Sigma0(a) + sha256_Maj(a, b, c);
        hh = g; g = f; f = e; e = d + T1;
        d = c; c = b; b = a; a = T1 + T2;
    }

    h0 += a; h1 += b; h2 += c; h3 += d;
    h4 += e; h5 += f; h6 += g; h7 += hh;

    /* Encode output as big-endian bytes */
    uint32_t hs[8] = {h0, h1, h2, h3, h4, h5, h6, h7};
    #pragma unroll
    for (int i = 0; i < 8; i++) {
        output[i*4 + 0] = (unsigned char)(hs[i] >> 24);
        output[i*4 + 1] = (unsigned char)(hs[i] >> 16);
        output[i*4 + 2] = (unsigned char)(hs[i] >>  8);
        output[i*4 + 3] = (unsigned char)(hs[i]);
    }
}

/* ───── Kernel 1: Independent batch SHA-256 ───── */
/* Each thread: output[idx] = SHA256(input[idx]), 32 bytes each */
__global__ void sha256_batch_kernel(
    const unsigned char* __restrict__ inputs,
    int count,
    unsigned char* __restrict__ outputs
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= count) return;

    sha256_transform_32(inputs + (size_t)idx * 32, outputs + (size_t)idx * 32);
}

/* ───── Kernel 2: PoH chain kernel ───── */
/*
 * Computes a PoH chain: hash[i] = SHA256(hash[i-1])
 * Since this is inherently sequential, we parallelize across
 * independent chains (e.g., for different slots or parallel PoH verification).
 *
 * Each thread computes `chain_length` sequential hashes.
 * Input: seeds[idx] (32 bytes per chain)
 * Output: results[idx] (32 bytes = final hash after chain_length iterations)
 */
__global__ void sha256_poh_chain_kernel(
    const unsigned char* __restrict__ seeds,
    int num_chains,
    int chain_length,
    unsigned char* __restrict__ results
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= num_chains) return;

    unsigned char current[32];
    unsigned char next[32];

    /* Load seed */
    const unsigned char *seed = seeds + (size_t)idx * 32;
    #pragma unroll
    for (int i = 0; i < 32; i++) current[i] = seed[i];

    /* Iterate chain */
    for (int step = 0; step < chain_length; step++) {
        sha256_transform_32(current, next);
        #pragma unroll
        for (int i = 0; i < 32; i++) current[i] = next[i];
    }

    /* Write result */
    unsigned char *out = results + (size_t)idx * 32;
    #pragma unroll
    for (int i = 0; i < 32; i++) out[i] = current[i];
}

/* ───── Host API ───── */

/* Batch SHA-256: process `count` independent 32-byte inputs */
extern "C" int sha256_batch_host(
    const unsigned char *inputs,
    int count,
    unsigned char *outputs
) {
    if (count <= 0) return 0;

    unsigned char *d_in = nullptr, *d_out = nullptr;
    size_t bytes = (size_t)count * 32;

    cudaError_t err;
    err = cudaMalloc(&d_in, bytes);
    if (err != cudaSuccess) {
        fprintf(stderr, "sha256_batch: cudaMalloc input failed: %s\n", cudaGetErrorString(err));
        return -1;
    }
    err = cudaMalloc(&d_out, bytes);
    if (err != cudaSuccess) {
        cudaFree(d_in);
        fprintf(stderr, "sha256_batch: cudaMalloc output failed: %s\n", cudaGetErrorString(err));
        return -1;
    }

    cudaMemcpy(d_in, inputs, bytes, cudaMemcpyHostToDevice);

    int threads = 256;
    int blocks = (count + threads - 1) / threads;
    sha256_batch_kernel<<<blocks, threads>>>(d_in, count, d_out);

    err = cudaGetLastError();
    if (err != cudaSuccess) {
        fprintf(stderr, "sha256_batch: kernel launch failed: %s\n", cudaGetErrorString(err));
        cudaFree(d_in); cudaFree(d_out);
        return -1;
    }

    cudaDeviceSynchronize();
    cudaMemcpy(outputs, d_out, bytes, cudaMemcpyDeviceToHost);

    cudaFree(d_in);
    cudaFree(d_out);
    return 0;
}

/* PoH chain: compute `num_chains` independent sequential hash chains */
extern "C" int sha256_poh_chain_host(
    const unsigned char *seeds,
    int num_chains,
    int chain_length,
    unsigned char *results
) {
    if (num_chains <= 0 || chain_length <= 0) return 0;

    unsigned char *d_seeds = nullptr, *d_results = nullptr;
    size_t bytes = (size_t)num_chains * 32;

    cudaMalloc(&d_seeds, bytes);
    cudaMalloc(&d_results, bytes);
    cudaMemcpy(d_seeds, seeds, bytes, cudaMemcpyHostToDevice);

    int threads = 128;
    int blocks = (num_chains + threads - 1) / threads;
    sha256_poh_chain_kernel<<<blocks, threads>>>(d_seeds, num_chains, chain_length, d_results);

    cudaError_t err = cudaGetLastError();
    if (err != cudaSuccess) {
        fprintf(stderr, "sha256_poh: kernel launch failed: %s\n", cudaGetErrorString(err));
        cudaFree(d_seeds); cudaFree(d_results);
        return -1;
    }

    cudaDeviceSynchronize();
    cudaMemcpy(results, d_results, bytes, cudaMemcpyDeviceToHost);

    cudaFree(d_seeds);
    cudaFree(d_results);
    return 0;
}

/* Multi-GPU batch SHA-256 */
extern "C" int sha256_batch_multi_gpu(
    const unsigned char *inputs,
    int count,
    unsigned char *outputs
) {
    int device_count = 0;
    cudaGetDeviceCount(&device_count);

    if (device_count <= 1) {
        return sha256_batch_host(inputs, count, outputs);
    }

    int per_gpu = (count + device_count - 1) / device_count;

    for (int dev = 0; dev < device_count; dev++) {
        int start = dev * per_gpu;
        int chunk = per_gpu;
        if (start + chunk > count) chunk = count - start;
        if (chunk <= 0) continue;

        cudaSetDevice(dev);

        unsigned char *d_in = nullptr, *d_out = nullptr;
        size_t bytes = (size_t)chunk * 32;

        cudaMalloc(&d_in, bytes);
        cudaMalloc(&d_out, bytes);
        cudaMemcpy(d_in, inputs + (size_t)start * 32, bytes, cudaMemcpyHostToDevice);

        int threads = 256;
        int blocks = (chunk + threads - 1) / threads;
        sha256_batch_kernel<<<blocks, threads>>>(d_in, chunk, d_out);

        cudaMemcpy(outputs + (size_t)start * 32, d_out, bytes, cudaMemcpyDeviceToHost);

        cudaFree(d_in);
        cudaFree(d_out);
    }

    for (int dev = 0; dev < device_count; dev++) {
        cudaSetDevice(dev);
        cudaDeviceSynchronize();
    }

    cudaSetDevice(0);
    return 0;
}
