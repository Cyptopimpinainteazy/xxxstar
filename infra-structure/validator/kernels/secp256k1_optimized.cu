// Optimized secp256k1 ECDSA batch verification — Jacobian + Shamir's trick.
//
// Key optimizations over the naive kernel:
//   1. Jacobian projective coordinates: eliminates expensive field inversions
//      from every point_add/double (only 1 inversion at the very end).
//   2. Shamir's trick: computes u1*G + u2*Q in a single 256-iteration loop
//      instead of two separate 256-bit scalar multiplications.
//   3. Multi-GPU support: automatic workload partitioning across devices.
//
// Theoretical speedup: ~30-45x over the naive affine implementation.
// Target: ≥50,000 sig/sec per GTX 1070 (sm_61).
//
// API-compatible drop-in replacement for secp256k1_batch.cu.

#include <cuda_runtime.h>
#include <stdint.h>
#include <stdio.h>
#include "secp256k1.cuh"

// ============================================================================
// Jacobian point representation: (X, Y, Z) ↔ affine (X/Z², Y/Z³)
// Infinity is represented by Z = 0.
// ============================================================================
struct JacPoint {
    BigInt X;
    BigInt Y;
    BigInt Z;
};

// ============================================================================
// Constant memory: secp256k1 generator G (affine) and field prime p
// ============================================================================
__constant__ ECPoint c_generator;
__constant__ BigInt  c_prime;

// ============================================================================
// Helper: construct BigInt from 8 little-endian uint32 words
// ============================================================================
namespace opt {

__host__ __device__ inline BigInt make_bigint(uint32_t w0, uint32_t w1, uint32_t w2,
                                              uint32_t w3, uint32_t w4, uint32_t w5,
                                              uint32_t w6, uint32_t w7) {
    BigInt x;
    x.data[0] = w0; x.data[1] = w1; x.data[2] = w2; x.data[3] = w3;
    x.data[4] = w4; x.data[5] = w5; x.data[6] = w6; x.data[7] = w7;
    return x;
}

__host__ __device__ inline void bytes_to_bigint_be(const unsigned char* src, BigInt &out) {
    for (int i = 0; i < 8; ++i) {
        int off = 28 - i * 4;
        out.data[i] =
            (static_cast<uint32_t>(src[off])     << 24) |
            (static_cast<uint32_t>(src[off + 1]) << 16) |
            (static_cast<uint32_t>(src[off + 2]) <<  8) |
            (static_cast<uint32_t>(src[off + 3]));
    }
}

__device__ inline void bigint_to_bytes_be(const BigInt &in, unsigned char* dst) {
    for (int i = 0; i < 8; ++i) {
        int off = 28 - i * 4;
        dst[off]     = static_cast<unsigned char>((in.data[i] >> 24) & 0xFF);
        dst[off + 1] = static_cast<unsigned char>((in.data[i] >> 16) & 0xFF);
        dst[off + 2] = static_cast<unsigned char>((in.data[i] >>  8) & 0xFF);
        dst[off + 3] = static_cast<unsigned char>( in.data[i]        & 0xFF);
    }
}

} // namespace opt

// ============================================================================
// Jacobian point operations
// ============================================================================

__device__ __forceinline__ void jac_set_inf(JacPoint &P) {
    init_bigint(P.X, 0);
    init_bigint(P.Y, 1);
    init_bigint(P.Z, 0);
}

__device__ __forceinline__ bool jac_is_inf(const JacPoint &P) {
    return is_zero(P.Z);
}

__device__ __forceinline__ void jac_copy(JacPoint &dst, const JacPoint &src) {
    copy_bigint(dst.X, src.X);
    copy_bigint(dst.Y, src.Y);
    copy_bigint(dst.Z, src.Z);
}

__device__ __forceinline__ void jac_from_affine(JacPoint &J, const ECPoint &A) {
    if (A.infinity) {
        jac_set_inf(J);
        return;
    }
    copy_bigint(J.X, A.x);
    copy_bigint(J.Y, A.y);
    init_bigint(J.Z, 1);
}

// Convert Jacobian → affine.  Costs one field inversion (via Fermat).
__device__ void jac_to_affine(ECPoint &A, const JacPoint &J, const BigInt &p) {
    if (jac_is_inf(J)) {
        A.infinity = true;
        return;
    }
    BigInt z_inv, z_inv2, z_inv3;
    mod_inverse(z_inv, J.Z, p);
    mul_mod(z_inv2, z_inv, z_inv, p);      // Z⁻²
    mul_mod(z_inv3, z_inv2, z_inv, p);      // Z⁻³
    mul_mod(A.x, J.X, z_inv2, p);           // x = X · Z⁻²
    mul_mod(A.y, J.Y, z_inv3, p);           // y = Y · Z⁻³
    A.infinity = false;
}

// ----------------------------------------------------------------------------
// Point doubling in Jacobian coordinates (secp256k1: a = 0)
//
// Source: https://www.hyperelliptic.org/EFD/g1p/auto-shortw-jacobian-0.html
//   A = Y1²
//   B = 4·X1·A
//   C = 8·A²
//   D = 3·X1²          (for a=0)
//   X3 = D² − 2·B
//   Y3 = D·(B − X3) − C
//   Z3 = 2·Y1·Z1
//
// Cost: 4 field multiplications + 4 squarings ≈ 10 mul_mod calls.
// (We treat squaring = mul_mod for implementation simplicity.)
// ----------------------------------------------------------------------------
__device__ void jac_double(JacPoint &R, const JacPoint &P, const BigInt &p) {
    if (jac_is_inf(P) || is_zero(P.Y)) {
        jac_set_inf(R);
        return;
    }

    BigInt A, B, C, D, X3, Y3, Z3, tmp;

    // A = Y1²
    mul_mod(A, P.Y, P.Y, p);

    // B = 4 · X1 · A
    mul_mod(tmp, P.X, A, p);
    add_mod(B, tmp, tmp, p);
    add_mod(B, B, B, p);                    // 4 · X1 · Y1²

    // C = 8 · A²
    mul_mod(C, A, A, p);
    add_mod(C, C, C, p);
    add_mod(C, C, C, p);
    add_mod(C, C, C, p);                    // 8 · Y1⁴

    // D = 3 · X1²
    mul_mod(D, P.X, P.X, p);
    BigInt D2;
    add_mod(D2, D, D, p);
    add_mod(D, D2, D, p);                   // 3 · X1²  (note: add D+D+D)

    // X3 = D² − 2·B
    mul_mod(X3, D, D, p);
    BigInt B2;
    add_mod(B2, B, B, p);
    sub_mod(X3, X3, B2, p);

    // Y3 = D · (B − X3) − C
    sub_mod(tmp, B, X3, p);
    mul_mod(Y3, D, tmp, p);
    sub_mod(Y3, Y3, C, p);

    // Z3 = 2 · Y1 · Z1
    mul_mod(Z3, P.Y, P.Z, p);
    add_mod(Z3, Z3, Z3, p);

    copy_bigint(R.X, X3);
    copy_bigint(R.Y, Y3);
    copy_bigint(R.Z, Z3);
}

// ----------------------------------------------------------------------------
// Point addition in Jacobian coordinates
//
// Source: https://www.hyperelliptic.org/EFD/g1p/auto-shortw-jacobian-0.html
//   Z1Z1 = Z1²            Z2Z2 = Z2²
//   U1 = X1·Z2Z2          U2 = X2·Z1Z1
//   S1 = Y1·Z2·Z2Z2       S2 = Y2·Z1·Z1Z1
//   H = U2 − U1           I = (2H)²
//   J = H·I               r = 2(S2 − S1)
//   V = U1·I
//   X3 = r² − J − 2V
//   Y3 = r(V − X3) − 2·S1·J
//   Z3 = ((Z1+Z2)² − Z1Z1 − Z2Z2) · H
//
// Cost: ~16 mul_mod calls.
// Handles identity and P==Q / P==-Q cases.
// ----------------------------------------------------------------------------
__device__ void jac_add(JacPoint &R, const JacPoint &P, const JacPoint &Q, const BigInt &p) {
    if (jac_is_inf(P)) { jac_copy(R, Q); return; }
    if (jac_is_inf(Q)) { jac_copy(R, P); return; }

    BigInt Z1Z1, Z2Z2, U1, U2, S1, S2, H, I, J, r, V, X3, Y3, Z3, tmp;

    // Z1Z1, Z2Z2
    mul_mod(Z1Z1, P.Z, P.Z, p);
    mul_mod(Z2Z2, Q.Z, Q.Z, p);

    // U1 = X1 · Z2²,  U2 = X2 · Z1²
    mul_mod(U1, P.X, Z2Z2, p);
    mul_mod(U2, Q.X, Z1Z1, p);

    // S1 = Y1 · Z2³,  S2 = Y2 · Z1³
    mul_mod(tmp, Q.Z, Z2Z2, p);     // Z2³
    mul_mod(S1, P.Y, tmp, p);
    mul_mod(tmp, P.Z, Z1Z1, p);     // Z1³
    mul_mod(S2, Q.Y, tmp, p);

    // H = U2 − U1
    sub_mod(H, U2, U1, p);

    // Handle degenerate cases: P == ±Q
    if (is_zero(H)) {
        BigInt S_diff;
        sub_mod(S_diff, S2, S1, p);
        if (is_zero(S_diff)) {
            // P == Q → use doubling
            jac_double(R, P, p);
        } else {
            // P == −Q → result is infinity
            jac_set_inf(R);
        }
        return;
    }

    // I = (2H)²
    BigInt H2;
    add_mod(H2, H, H, p);
    mul_mod(I, H2, H2, p);

    // J = H · I
    mul_mod(J, H, I, p);

    // r = 2(S2 − S1)
    sub_mod(r, S2, S1, p);
    add_mod(r, r, r, p);

    // V = U1 · I
    mul_mod(V, U1, I, p);

    // X3 = r² − J − 2V
    mul_mod(X3, r, r, p);
    sub_mod(X3, X3, J, p);
    BigInt V2;
    add_mod(V2, V, V, p);
    sub_mod(X3, X3, V2, p);

    // Y3 = r · (V − X3) − 2·S1·J
    sub_mod(tmp, V, X3, p);
    mul_mod(Y3, r, tmp, p);
    mul_mod(tmp, S1, J, p);
    BigInt SJ2;
    add_mod(SJ2, tmp, tmp, p);
    sub_mod(Y3, Y3, SJ2, p);

    // Z3 = ((Z1+Z2)² − Z1Z1 − Z2Z2) · H
    BigInt Zsum;
    add_mod(Zsum, P.Z, Q.Z, p);
    mul_mod(Z3, Zsum, Zsum, p);
    sub_mod(Z3, Z3, Z1Z1, p);
    sub_mod(Z3, Z3, Z2Z2, p);
    mul_mod(Z3, Z3, H, p);

    copy_bigint(R.X, X3);
    copy_bigint(R.Y, Y3);
    copy_bigint(R.Z, Z3);
}

// ============================================================================
// Shamir's trick kernel.
//
// Computes R = u1·G + u2·Q in a single 256-iteration loop:
//   Precompute table[4]: { O, Q, G, G+Q }
//   For each bit from 255 down to 0:
//     R ← 2·R
//     d = (bit(u1,i) << 1) | bit(u2,i)      ∈ {0,1,2,3}
//     if d > 0: R ← R + table[d]
//
// Cost per signature: 256 Jacobian doubles + ~192 Jacobian adds + 1 inversion
// ≈ 5,900 field multiplications (vs ~266,000 for naive affine).
// ============================================================================
__global__ void secp256k1_shamir_kernel(
    const unsigned char* __restrict__ u1_bytes,
    const unsigned char* __restrict__ u2_bytes,
    const unsigned char* __restrict__ pubkey_bytes,
    int count,
    unsigned char* __restrict__ out_x
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= count) return;

    // Read scalar inputs
    BigInt u1, u2;
    opt::bytes_to_bigint_be(u1_bytes + idx * 32, u1);
    opt::bytes_to_bigint_be(u2_bytes + idx * 32, u2);

    // Read public key Q (affine)
    ECPoint Q_affine;
    opt::bytes_to_bigint_be(pubkey_bytes + idx * 64,      Q_affine.x);
    opt::bytes_to_bigint_be(pubkey_bytes + idx * 64 + 32, Q_affine.y);
    Q_affine.infinity = false;

    // Build Shamir table in Jacobian coordinates
    //   table[0] = O  (identity)
    //   table[1] = Q
    //   table[2] = G
    //   table[3] = G + Q
    JacPoint table[4];
    jac_set_inf(table[0]);
    jac_from_affine(table[1], Q_affine);
    jac_from_affine(table[2], c_generator);
    jac_add(table[3], table[2], table[1], c_prime);

    // Main Shamir double-and-add loop (MSB → LSB)
    JacPoint R;
    jac_set_inf(R);

    for (int i = 255; i >= 0; --i) {
        // R ← 2R
        JacPoint dbl;
        jac_double(dbl, R, c_prime);

        int b1 = get_bit(u1, i) ? 1 : 0;
        int b0 = get_bit(u2, i) ? 1 : 0;
        int d  = (b1 << 1) | b0;

        if (d > 0) {
            jac_add(R, dbl, table[d], c_prime);
        } else {
            jac_copy(R, dbl);
        }
    }

    // Convert result to affine x-coordinate
    ECPoint result;
    jac_to_affine(result, R, c_prime);

    if (result.infinity) {
        for (int j = 0; j < 32; ++j) out_x[idx * 32 + j] = 0;
    } else {
        opt::bigint_to_bytes_be(result.x, out_x + idx * 32);
    }
}

// ============================================================================
// Host-callable API (C linkage, same signature as naive kernel)
// ============================================================================
static void setup_constants() {
    // secp256k1 prime: p = 2^256 − 2^32 − 977
    BigInt prime = opt::make_bigint(
        0xFFFFFC2F, 0xFFFFFFFE, 0xFFFFFFFF, 0xFFFFFFFF,
        0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF);
    cudaMemcpyToSymbol(c_prime, &prime, sizeof(BigInt));
    cudaMemcpyToSymbol(const_p, &prime, sizeof(BigInt));   // for secp256k1.cuh helpers

    // Generator G
    ECPoint G;
    G.x = opt::make_bigint(
        0x16F81798, 0x59F2815B, 0x2DCE28D9, 0x029BFCDB,
        0xCE870B07, 0x55A06295, 0xF9DCBBAC, 0x79BE667E);
    G.y = opt::make_bigint(
        0xFB10D4B8, 0x9C47D08F, 0xA6855419, 0xFD17B448,
        0x0E1108A8, 0x5DA4FBFC, 0x26A3C465, 0x483ADA77);
    G.infinity = false;
    cudaMemcpyToSymbol(c_generator, &G, sizeof(ECPoint));
}

extern "C" int secp256k1_ecdsa_verify_host(
    const unsigned char* u1_bytes,
    const unsigned char* u2_bytes,
    const unsigned char* pubkey_bytes,
    int count,
    unsigned char* out_x
) {
    if (count <= 0) return 0;

    setup_constants();

    size_t u_sz  = static_cast<size_t>(count) * 32;
    size_t pk_sz = static_cast<size_t>(count) * 64;
    size_t o_sz  = static_cast<size_t>(count) * 32;

    unsigned char *d_u1 = nullptr, *d_u2 = nullptr;
    unsigned char *d_pk = nullptr,  *d_out = nullptr;

    if (cudaMalloc(&d_u1,  u_sz)  != cudaSuccess) return 1;
    if (cudaMalloc(&d_u2,  u_sz)  != cudaSuccess) { cudaFree(d_u1); return 1; }
    if (cudaMalloc(&d_pk,  pk_sz) != cudaSuccess) { cudaFree(d_u1); cudaFree(d_u2); return 1; }
    if (cudaMalloc(&d_out, o_sz)  != cudaSuccess) { cudaFree(d_u1); cudaFree(d_u2); cudaFree(d_pk); return 1; }

    cudaMemcpy(d_u1, u1_bytes,     u_sz,  cudaMemcpyHostToDevice);
    cudaMemcpy(d_u2, u2_bytes,     u_sz,  cudaMemcpyHostToDevice);
    cudaMemcpy(d_pk, pubkey_bytes, pk_sz,  cudaMemcpyHostToDevice);

    // GTX 1070 sweet spot: 128 threads/block to reduce register pressure
    int threads = 128;
    int blocks  = (count + threads - 1) / threads;
    secp256k1_shamir_kernel<<<blocks, threads>>>(d_u1, d_u2, d_pk, count, d_out);

    cudaError_t err = cudaDeviceSynchronize();

    cudaMemcpy(out_x, d_out, o_sz, cudaMemcpyDeviceToHost);

    cudaFree(d_u1);
    cudaFree(d_u2);
    cudaFree(d_pk);
    cudaFree(d_out);

    return (err == cudaSuccess) ? 0 : 2;
}

// ============================================================================
// Multi-GPU variant: splits batch across all CUDA devices
// ============================================================================
extern "C" int secp256k1_ecdsa_verify_multi_gpu(
    const unsigned char* u1_bytes,
    const unsigned char* u2_bytes,
    const unsigned char* pubkey_bytes,
    int count,
    unsigned char* out_x
) {
    int device_count = 0;
    cudaGetDeviceCount(&device_count);
    if (device_count <= 1) {
        return secp256k1_ecdsa_verify_host(u1_bytes, u2_bytes, pubkey_bytes, count, out_x);
    }

    // Partition across devices
    int per_device = count / device_count;
    int remainder  = count % device_count;

    struct DeviceWork {
        unsigned char *d_u1, *d_u2, *d_pk, *d_out;
        int offset, n;
        cudaStream_t stream;
    };

    DeviceWork* work = new DeviceWork[device_count];
    int offset = 0;

    for (int dev = 0; dev < device_count; ++dev) {
        cudaSetDevice(dev);

        int n = per_device + (dev < remainder ? 1 : 0);
        work[dev].offset = offset;
        work[dev].n = n;

        if (n == 0) {
            work[dev].d_u1 = work[dev].d_u2 = work[dev].d_pk = work[dev].d_out = nullptr;
            continue;
        }

        setup_constants();

        size_t u_sz  = static_cast<size_t>(n) * 32;
        size_t pk_sz = static_cast<size_t>(n) * 64;
        size_t o_sz  = static_cast<size_t>(n) * 32;

        cudaStreamCreate(&work[dev].stream);
        cudaMalloc(&work[dev].d_u1,  u_sz);
        cudaMalloc(&work[dev].d_u2,  u_sz);
        cudaMalloc(&work[dev].d_pk,  pk_sz);
        cudaMalloc(&work[dev].d_out, o_sz);

        cudaMemcpyAsync(work[dev].d_u1, u1_bytes     + offset * 32, u_sz,  cudaMemcpyHostToDevice, work[dev].stream);
        cudaMemcpyAsync(work[dev].d_u2, u2_bytes     + offset * 32, u_sz,  cudaMemcpyHostToDevice, work[dev].stream);
        cudaMemcpyAsync(work[dev].d_pk, pubkey_bytes  + offset * 64, pk_sz, cudaMemcpyHostToDevice, work[dev].stream);

        int threads = 128;
        int blocks  = (n + threads - 1) / threads;
        secp256k1_shamir_kernel<<<blocks, threads, 0, work[dev].stream>>>(
            work[dev].d_u1, work[dev].d_u2, work[dev].d_pk, n, work[dev].d_out);

        cudaMemcpyAsync(out_x + offset * 32, work[dev].d_out, o_sz, cudaMemcpyDeviceToHost, work[dev].stream);

        offset += n;
    }

    // Synchronize all devices
    int rc = 0;
    for (int dev = 0; dev < device_count; ++dev) {
        if (work[dev].n == 0) continue;
        cudaSetDevice(dev);
        if (cudaStreamSynchronize(work[dev].stream) != cudaSuccess) rc = 2;
        cudaFree(work[dev].d_u1);
        cudaFree(work[dev].d_u2);
        cudaFree(work[dev].d_pk);
        cudaFree(work[dev].d_out);
        cudaStreamDestroy(work[dev].stream);
    }
    delete[] work;

    cudaSetDevice(0);
    return rc;
}
