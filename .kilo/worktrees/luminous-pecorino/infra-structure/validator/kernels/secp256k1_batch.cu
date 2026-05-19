// CUDA kernel for secp256k1 ECDSA verification helpers.
// Uses ECC primitives from the MIT-licensed Secp256k1-CUDA-ecc project.

#include <cuda_runtime.h>
#include <stdint.h>
#include "secp256k1.cuh"

namespace {
__host__ __device__ inline BigInt make_bigint(uint32_t w0, uint32_t w1, uint32_t w2,
                                              uint32_t w3, uint32_t w4, uint32_t w5,
                                              uint32_t w6, uint32_t w7) {
    BigInt x;
    x.data[0] = w0;
    x.data[1] = w1;
    x.data[2] = w2;
    x.data[3] = w3;
    x.data[4] = w4;
    x.data[5] = w5;
    x.data[6] = w6;
    x.data[7] = w7;
    return x;
}

__host__ __device__ inline void bytes_to_bigint_be(const unsigned char* src, BigInt &out) {
    for (int i = 0; i < 8; ++i) {
        int offset = 28 - i * 4;
        out.data[i] =
            (static_cast<uint32_t>(src[offset]) << 24) |
            (static_cast<uint32_t>(src[offset + 1]) << 16) |
            (static_cast<uint32_t>(src[offset + 2]) << 8) |
            (static_cast<uint32_t>(src[offset + 3]));
    }
}

__device__ inline void bigint_to_bytes_be(const BigInt &in, unsigned char* dst) {
    for (int i = 0; i < 8; ++i) {
        int offset = 28 - i * 4;
        dst[offset] = static_cast<unsigned char>((in.data[i] >> 24) & 0xFF);
        dst[offset + 1] = static_cast<unsigned char>((in.data[i] >> 16) & 0xFF);
        dst[offset + 2] = static_cast<unsigned char>((in.data[i] >> 8) & 0xFF);
        dst[offset + 3] = static_cast<unsigned char>(in.data[i] & 0xFF);
    }
}

__device__ void scalar_mul(ECPoint &R, const BigInt &k, const ECPoint &P, const BigInt &p) {
    ECPoint result;
    point_set_infinity(result);
    ECPoint addend;
    point_copy(addend, P);

    for (int i = 0; i < 256; ++i) {
        if (get_bit(k, i)) {
            ECPoint tmp;
            point_add(tmp, result, addend, p);
            point_copy(result, tmp);
        }
        ECPoint doubled;
        double_point(doubled, addend, p);
        point_copy(addend, doubled);
    }
    point_copy(R, result);
}
} // namespace

__global__ void secp256k1_ecdsa_batch_kernel(
    const unsigned char* u1_bytes,
    const unsigned char* u2_bytes,
    const unsigned char* pubkey_bytes,
    ECPoint generator,
    BigInt prime,
    int count,
    unsigned char* out_x
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= count) {
        return;
    }

    BigInt u1;
    BigInt u2;
    bytes_to_bigint_be(u1_bytes + idx * 32, u1);
    bytes_to_bigint_be(u2_bytes + idx * 32, u2);

    ECPoint pubkey;
    bytes_to_bigint_be(pubkey_bytes + idx * 64, pubkey.x);
    bytes_to_bigint_be(pubkey_bytes + idx * 64 + 32, pubkey.y);
    pubkey.infinity = false;

    ECPoint p1;
    ECPoint p2;
    scalar_mul(p1, u1, generator, prime);
    scalar_mul(p2, u2, pubkey, prime);

    ECPoint sum;
    point_add(sum, p1, p2, prime);

    if (sum.infinity) {
        for (int i = 0; i < 32; ++i) {
            out_x[idx * 32 + i] = 0;
        }
        return;
    }

    bigint_to_bytes_be(sum.x, out_x + idx * 32);
}

extern "C" int secp256k1_ecdsa_verify_host(
    const unsigned char* u1_bytes,
    const unsigned char* u2_bytes,
    const unsigned char* pubkey_bytes,
    int count,
    unsigned char* out_x
) {
    // secp256k1 prime p = 2^256 - 2^32 - 977
    BigInt prime = make_bigint(
        0xFFFFFC2F, 0xFFFFFFFE, 0xFFFFFFFF, 0xFFFFFFFF,
        0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF);

    // Set constant memory for field prime in secp256k1.cuh
    cudaMemcpyToSymbol(const_p, &prime, sizeof(BigInt));

    // Generator point
    ECPoint generator;
    generator.x = make_bigint(
        0x16F81798, 0x59F2815B, 0x2DCE28D9, 0x029BFCDB,
        0xCE870B07, 0x55A06295, 0xF9DCBBAC, 0x79BE667E);
    generator.y = make_bigint(
        0xFB10D4B8, 0x9C47D08F, 0xA6855419, 0xFD17B448,
        0x0E1108A8, 0x5DA4FBFC, 0x26A3C465, 0x483ADA77);
    generator.infinity = false;

    unsigned char* d_u1 = nullptr;
    unsigned char* d_u2 = nullptr;
    unsigned char* d_pubkeys = nullptr;
    unsigned char* d_out = nullptr;

    size_t u_bytes = static_cast<size_t>(count) * 32;
    size_t pk_bytes = static_cast<size_t>(count) * 64;
    size_t out_bytes = static_cast<size_t>(count) * 32;

    if (cudaMalloc(&d_u1, u_bytes) != cudaSuccess) return 1;
    if (cudaMalloc(&d_u2, u_bytes) != cudaSuccess) return 1;
    if (cudaMalloc(&d_pubkeys, pk_bytes) != cudaSuccess) return 1;
    if (cudaMalloc(&d_out, out_bytes) != cudaSuccess) return 1;

    cudaMemcpy(d_u1, u1_bytes, u_bytes, cudaMemcpyHostToDevice);
    cudaMemcpy(d_u2, u2_bytes, u_bytes, cudaMemcpyHostToDevice);
    cudaMemcpy(d_pubkeys, pubkey_bytes, pk_bytes, cudaMemcpyHostToDevice);

    int threads = 128;
    int blocks = (count + threads - 1) / threads;
    secp256k1_ecdsa_batch_kernel<<<blocks, threads>>>(
        d_u1, d_u2, d_pubkeys, generator, prime, count, d_out);

    cudaMemcpy(out_x, d_out, out_bytes, cudaMemcpyDeviceToHost);

    cudaFree(d_u1);
    cudaFree(d_u2);
    cudaFree(d_pubkeys);
    cudaFree(d_out);

    return 0;
}
