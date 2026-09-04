// CUDA kernel for Keccak-256 hashing (single 32-byte input per lane).

#include <cuda_runtime.h>
#include <stdint.h>

__constant__ uint64_t k_round_constants[24] = {
    0x0000000000000001ULL, 0x0000000000008082ULL,
    0x800000000000808aULL, 0x8000000080008000ULL,
    0x000000000000808bULL, 0x0000000080000001ULL,
    0x8000000080008081ULL, 0x8000000000008009ULL,
    0x000000000000008aULL, 0x0000000000000088ULL,
    0x0000000080008009ULL, 0x000000008000000aULL,
    0x000000008000808bULL, 0x800000000000008bULL,
    0x8000000000008089ULL, 0x8000000000008003ULL,
    0x8000000000008002ULL, 0x8000000000000080ULL,
    0x000000000000800aULL, 0x800000008000000aULL,
    0x8000000080008081ULL, 0x8000000000008080ULL,
    0x0000000080000001ULL, 0x8000000080008008ULL
};

__constant__ int k_rotation[25] = {
    0,  1, 62, 28, 27,
    36, 44, 6, 55, 20,
    3, 10, 43, 25, 39,
    41, 45, 15, 21, 8,
    18, 2, 61, 56, 14
};

__constant__ int k_piln[25] = {
    0, 10, 20, 5, 15,
    16, 1, 11, 21, 6,
    7, 17, 2, 12, 22,
    23, 8, 18, 3, 13,
    14, 24, 9, 19, 4
};

namespace {
__device__ inline uint64_t rotl64(uint64_t x, int s) {
    return (x << s) | (x >> (64 - s));
}

__device__ void keccak_f(uint64_t state[25]) {

    for (int round = 0; round < 24; ++round) {
        uint64_t c[5];
        uint64_t d[5];
        for (int i = 0; i < 5; ++i) {
            c[i] = state[i] ^ state[i + 5] ^ state[i + 10] ^ state[i + 15] ^ state[i + 20];
        }
        for (int i = 0; i < 5; ++i) {
            d[i] = c[(i + 4) % 5] ^ rotl64(c[(i + 1) % 5], 1);
        }
        for (int i = 0; i < 25; i += 5) {
            for (int j = 0; j < 5; ++j) {
                state[i + j] ^= d[j];
            }
        }
        uint64_t b[25];
        for (int i = 0; i < 25; ++i) {
            b[k_piln[i]] = rotl64(state[i], k_rotation[i]);
        }
        for (int i = 0; i < 25; i += 5) {
            for (int j = 0; j < 5; ++j) {
                state[i + j] = b[i + j] ^ ((~b[i + ((j + 1) % 5)]) & b[i + ((j + 2) % 5)]);
            }
        }
        state[0] ^= k_round_constants[round];
    }
}

__device__ void keccak256_single(const unsigned char* input, unsigned char* output) {
    uint64_t state[25] = {0};
    unsigned char* state_bytes = reinterpret_cast<unsigned char*>(state);

    for (int i = 0; i < 32; ++i) {
        state_bytes[i] ^= input[i];
    }

    // pad10*1: domain separator 0x01 and final 0x80 for rate=136 bytes
    state_bytes[32] ^= 0x01;
    state_bytes[135] ^= 0x80;

    keccak_f(state);

    for (int i = 0; i < 32; ++i) {
        output[i] = state_bytes[i];
    }
}
} // namespace

__global__ void keccak256_batch_kernel(
    const unsigned char* messages,
    int count,
    unsigned char* digests
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= count) {
        return;
    }
    keccak256_single(messages + idx * 32, digests + idx * 32);
}

extern "C" int keccak256_batch_host(
    const unsigned char* messages,
    int count,
    unsigned char* digests
) {
    unsigned char* d_messages = nullptr;
    unsigned char* d_digests = nullptr;
    size_t msg_bytes = static_cast<size_t>(count) * 32;
    size_t digest_bytes = static_cast<size_t>(count) * 32;

    if (cudaMalloc(&d_messages, msg_bytes) != cudaSuccess) return 1;
    if (cudaMalloc(&d_digests, digest_bytes) != cudaSuccess) return 1;

    cudaMemcpy(d_messages, messages, msg_bytes, cudaMemcpyHostToDevice);

    int threads = 128;
    int blocks = (count + threads - 1) / threads;
    keccak256_batch_kernel<<<blocks, threads>>>(d_messages, count, d_digests);

    cudaMemcpy(digests, d_digests, digest_bytes, cudaMemcpyDeviceToHost);

    cudaFree(d_messages);
    cudaFree(d_digests);

    return 0;
}
