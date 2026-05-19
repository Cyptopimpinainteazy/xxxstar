/*
 * SHA-512 device implementation for CUDA
 *
 * Ed25519 verification requires SHA-512 to compute:
 *   h = SHA512(R || A || M) mod ℓ
 *
 * This is a straightforward CUDA port of the FIPS 180-4 SHA-512 spec.
 */

#ifndef SHA512_DEVICE_CUH
#define SHA512_DEVICE_CUH

#include <cuda_runtime.h>
#include <stdint.h>

/* SHA-512 round constants */
__device__ static const uint64_t sha512_K[80] = {
    0x428a2f98d728ae22ULL, 0x7137449123ef65cdULL, 0xb5c0fbcfec4d3b2fULL, 0xe9b5dba58189dbbcULL,
    0x3956c25bf348b538ULL, 0x59f111f1b605d019ULL, 0x923f82a4af194f9bULL, 0xab1c5ed5da6d8118ULL,
    0xd807aa98a3030242ULL, 0x12835b0145706fbeULL, 0x243185be4ee4b28cULL, 0x550c7dc3d5ffb4e2ULL,
    0x72be5d74f27b896fULL, 0x80deb1fe3b1696b1ULL, 0x9bdc06a725c71235ULL, 0xc19bf174cf692694ULL,
    0xe49b69c19ef14ad2ULL, 0xefbe4786384f25e3ULL, 0x0fc19dc68b8cd5b5ULL, 0x240ca1cc77ac9c65ULL,
    0x2de92c6f592b0275ULL, 0x4a7484aa6ea6e483ULL, 0x5cb0a9dcbd41fbd4ULL, 0x76f988da831153b5ULL,
    0x983e5152ee66dfabULL, 0xa831c66d2db43210ULL, 0xb00327c898fb213fULL, 0xbf597fc7beef0ee4ULL,
    0xc6e00bf33da88fc2ULL, 0xd5a79147930aa725ULL, 0x06ca6351e003826fULL, 0x142929670a0e6e70ULL,
    0x27b70a8546d22ffcULL, 0x2e1b21385c26c926ULL, 0x4d2c6dfc5ac42aedULL, 0x53380d139d95b3dfULL,
    0x650a73548baf63deULL, 0x766a0abb3c77b2a8ULL, 0x81c2c92e47edaee6ULL, 0x92722c851482353bULL,
    0xa2bfe8a14cf10364ULL, 0xa81a664bbc423001ULL, 0xc24b8b70d0f89791ULL, 0xc76c51a30654be30ULL,
    0xd192e819d6ef5218ULL, 0xd69906245565a910ULL, 0xf40e35855771202aULL, 0x106aa07032bbd1b8ULL,
    0x19a4c116b8d2d0c8ULL, 0x1e376c085141ab53ULL, 0x2748774cdf8eeb99ULL, 0x34b0bcb5e19b48a8ULL,
    0x391c0cb3c5c95a63ULL, 0x4ed8aa4ae3418acbULL, 0x5b9cca4f7763e373ULL, 0x682e6ff3d6b2b8a3ULL,
    0x748f82ee5defb2fcULL, 0x78a5636f43172f60ULL, 0x84c87814a1f0ab72ULL, 0x8cc702081a6439ecULL,
    0x90befffa23631e28ULL, 0xa4506cebde82bde9ULL, 0xbef9a3f7b2c67915ULL, 0xc67178f2e372532bULL,
    0xca273eceea26619cULL, 0xd186b8c721c0c207ULL, 0xeada7dd6cde0eb1eULL, 0xf57d4f7fee6ed178ULL,
    0x06f067aa72176fbaULL, 0x0a637dc5a2c898a6ULL, 0x113f9804bef90daeULL, 0x1b710b35131c471bULL,
    0x28db77f523047d84ULL, 0x32caab7b40c72493ULL, 0x3c9ebe0a15c9bebcULL, 0x431d67c49c100d4cULL,
    0x4cc5d4becb3e42b6ULL, 0x597f299cfc657e2aULL, 0x5fcb6fab3ad6faecULL, 0x6c44198c4a475817ULL
};

__device__ __forceinline__ uint64_t rotr64(uint64_t x, int n) {
    return (x >> n) | (x << (64 - n));
}

__device__ __forceinline__ uint64_t sha512_Ch(uint64_t x, uint64_t y, uint64_t z) {
    return (x & y) ^ (~x & z);
}

__device__ __forceinline__ uint64_t sha512_Maj(uint64_t x, uint64_t y, uint64_t z) {
    return (x & y) ^ (x & z) ^ (y & z);
}

__device__ __forceinline__ uint64_t sha512_Sigma0(uint64_t x) {
    return rotr64(x, 28) ^ rotr64(x, 34) ^ rotr64(x, 39);
}

__device__ __forceinline__ uint64_t sha512_Sigma1(uint64_t x) {
    return rotr64(x, 14) ^ rotr64(x, 18) ^ rotr64(x, 41);
}

__device__ __forceinline__ uint64_t sha512_sigma0(uint64_t x) {
    return rotr64(x, 1) ^ rotr64(x, 8) ^ (x >> 7);
}

__device__ __forceinline__ uint64_t sha512_sigma1(uint64_t x) {
    return rotr64(x, 19) ^ rotr64(x, 61) ^ (x >> 6);
}

__device__ __forceinline__ uint64_t load64_be(const unsigned char *p) {
    return ((uint64_t)p[0] << 56) | ((uint64_t)p[1] << 48) |
           ((uint64_t)p[2] << 40) | ((uint64_t)p[3] << 32) |
           ((uint64_t)p[4] << 24) | ((uint64_t)p[5] << 16) |
           ((uint64_t)p[6] <<  8) | ((uint64_t)p[7]);
}

__device__ __forceinline__ void store64_be(unsigned char *p, uint64_t x) {
    p[0] = (unsigned char)(x >> 56); p[1] = (unsigned char)(x >> 48);
    p[2] = (unsigned char)(x >> 40); p[3] = (unsigned char)(x >> 32);
    p[4] = (unsigned char)(x >> 24); p[5] = (unsigned char)(x >> 16);
    p[6] = (unsigned char)(x >>  8); p[7] = (unsigned char)(x);
}

/*
 * sha512_hash: compute SHA-512 of a message up to 128 bytes
 *
 * For Ed25519 verification, the message is always:
 *   R (32 bytes) || A (32 bytes) || M (variable)
 * Typically 64 + message_len. We support up to 128 bytes input
 * (i.e. messages up to 64 bytes), which suffices for Solana tx hashes.
 */
__device__ void sha512_hash(
    const unsigned char *msg,
    unsigned int msg_len,
    unsigned char digest[64]
) {
    uint64_t h[8] = {
        0x6a09e667f3bcc908ULL, 0xbb67ae8584caa73bULL,
        0x3c6ef372fe94f82bULL, 0xa54ff53a5f1d36f1ULL,
        0x510e527fade682d1ULL, 0x9b05688c2b3e6c1fULL,
        0x1f83d9abfb41bd6bULL, 0x5be0cd19137e2179ULL
    };

    /* Pad message into blocks of 128 bytes.
     * For messages <= 111 bytes, it fits in 1 block.
     * For 112-128 bytes, need 2 blocks. */
    unsigned char padded[256];
    for (unsigned int i = 0; i < msg_len; i++) padded[i] = msg[i];
    padded[msg_len] = 0x80;
    for (unsigned int i = msg_len + 1; i < 256; i++) padded[i] = 0;

    int num_blocks;
    if (msg_len < 112) {
        num_blocks = 1;
        /* Length in bits at end of block (128 bytes) */
        uint64_t bit_len = (uint64_t)msg_len * 8;
        store64_be(&padded[120], bit_len);
    } else {
        num_blocks = 2;
        uint64_t bit_len = (uint64_t)msg_len * 8;
        store64_be(&padded[248], bit_len);
    }

    for (int blk = 0; blk < num_blocks; blk++) {
        const unsigned char *block = &padded[blk * 128];
        uint64_t W[80];

        for (int t = 0; t < 16; t++) {
            W[t] = load64_be(&block[t * 8]);
        }
        for (int t = 16; t < 80; t++) {
            W[t] = sha512_sigma1(W[t-2]) + W[t-7] + sha512_sigma0(W[t-15]) + W[t-16];
        }

        uint64_t a = h[0], b = h[1], c = h[2], d = h[3];
        uint64_t e = h[4], f = h[5], g = h[6], hh = h[7];

        for (int t = 0; t < 80; t++) {
            uint64_t T1 = hh + sha512_Sigma1(e) + sha512_Ch(e, f, g) + sha512_K[t] + W[t];
            uint64_t T2 = sha512_Sigma0(a) + sha512_Maj(a, b, c);
            hh = g; g = f; f = e; e = d + T1;
            d = c; c = b; b = a; a = T1 + T2;
        }

        h[0] += a; h[1] += b; h[2] += c; h[3] += d;
        h[4] += e; h[5] += f; h[6] += g; h[7] += hh;
    }

    for (int i = 0; i < 8; i++) {
        store64_be(&digest[i * 8], h[i]);
    }
}

/* ───── sc_reduce: reduce 64-byte hash modulo ℓ ───── */
/* ℓ = 2^252 + 27742317777372353535851937790883648493 */
/* This is the standard Barrett reduction from ref10 */
__device__ void sc_reduce(unsigned char s[64]) {
    int64_t s0  = 2097151 & load_3(&s[0]);
    int64_t s1  = 2097151 & (load_4(&s[2]) >> 5);
    int64_t s2  = 2097151 & (load_3(&s[5]) >> 2);
    int64_t s3  = 2097151 & (load_4(&s[7]) >> 7);
    int64_t s4  = 2097151 & (load_4(&s[10]) >> 4);
    int64_t s5  = 2097151 & (load_3(&s[13]) >> 1);
    int64_t s6  = 2097151 & (load_4(&s[15]) >> 6);
    int64_t s7  = 2097151 & (load_3(&s[18]) >> 3);
    int64_t s8  = 2097151 & load_3(&s[21]);
    int64_t s9  = 2097151 & (load_4(&s[23]) >> 5);
    int64_t s10 = 2097151 & (load_3(&s[26]) >> 2);
    int64_t s11 = 2097151 & (load_4(&s[28]) >> 7);
    int64_t s12 = 2097151 & (load_4(&s[31]) >> 4);
    int64_t s13 = 2097151 & (load_3(&s[34]) >> 1);
    int64_t s14 = 2097151 & (load_4(&s[36]) >> 6);
    int64_t s15 = 2097151 & (load_3(&s[39]) >> 3);
    int64_t s16 = 2097151 & load_3(&s[42]);
    int64_t s17 = 2097151 & (load_4(&s[44]) >> 5);
    int64_t s18 = 2097151 & (load_3(&s[47]) >> 2);
    int64_t s19 = 2097151 & (load_4(&s[49]) >> 7);
    int64_t s20 = 2097151 & (load_4(&s[52]) >> 4);
    int64_t s21 = 2097151 & (load_3(&s[55]) >> 1);
    int64_t s22 = 2097151 & (load_4(&s[57]) >> 6);
    int64_t s23 = (load_4(&s[60]) >> 3);

    int64_t carry0, carry1, carry2, carry3, carry4, carry5;
    int64_t carry6, carry7, carry8, carry9, carry10, carry11;

    s11 += s23 * 666643;  s12 += s23 * 470296;  s13 += s23 * 654183;
    s14 -= s23 * 997805;  s15 += s23 * 136657;  s16 -= s23 * 683901;  s23 = 0;

    s10 += s22 * 666643;  s11 += s22 * 470296;  s12 += s22 * 654183;
    s13 -= s22 * 997805;  s14 += s22 * 136657;  s15 -= s22 * 683901;  s22 = 0;

    s9  += s21 * 666643;  s10 += s21 * 470296;  s11 += s21 * 654183;
    s12 -= s21 * 997805;  s13 += s21 * 136657;  s14 -= s21 * 683901;  s21 = 0;

    s8  += s20 * 666643;  s9  += s20 * 470296;  s10 += s20 * 654183;
    s11 -= s20 * 997805;  s12 += s20 * 136657;  s13 -= s20 * 683901;  s20 = 0;

    s7  += s19 * 666643;  s8  += s19 * 470296;  s9  += s19 * 654183;
    s10 -= s19 * 997805;  s11 += s19 * 136657;  s12 -= s19 * 683901;  s19 = 0;

    s6  += s18 * 666643;  s7  += s18 * 470296;  s8  += s18 * 654183;
    s9  -= s18 * 997805;  s10 += s18 * 136657;  s11 -= s18 * 683901;  s18 = 0;

    int64_t carry12, carry13, carry14, carry15, carry16;

    carry6  = (s6  + (1 << 20)) >> 21; s7  += carry6;  s6  -= carry6  << 21;
    carry8  = (s8  + (1 << 20)) >> 21; s9  += carry8;  s8  -= carry8  << 21;
    carry10 = (s10 + (1 << 20)) >> 21; s11 += carry10; s10 -= carry10 << 21;
    carry12 = (s12 + (1 << 20)) >> 21; s13 += carry12; s12 -= carry12 << 21;
    carry14 = (s14 + (1 << 20)) >> 21; s15 += carry14; s14 -= carry14 << 21;
    carry16 = (s16 + (1 << 20)) >> 21; s17 += carry16; s16 -= carry16 << 21;

    carry7  = (s7  + (1 << 20)) >> 21; s8  += carry7;  s7  -= carry7  << 21;
    carry9  = (s9  + (1 << 20)) >> 21; s10 += carry9;  s9  -= carry9  << 21;
    carry11 = (s11 + (1 << 20)) >> 21; s12 += carry11; s11 -= carry11 << 21;
    carry13 = (s13 + (1 << 20)) >> 21; s14 += carry13; s13 -= carry13 << 21;
    carry15 = (s15 + (1 << 20)) >> 21; s16 += carry15; s15 -= carry15 << 21;

    s5  += s17 * 666643;  s6  += s17 * 470296;  s7  += s17 * 654183;
    s8  -= s17 * 997805;  s9  += s17 * 136657;  s10 -= s17 * 683901;  s17 = 0;

    s4  += s16 * 666643;  s5  += s16 * 470296;  s6  += s16 * 654183;
    s7  -= s16 * 997805;  s8  += s16 * 136657;  s9  -= s16 * 683901;  s16 = 0;

    s3  += s15 * 666643;  s4  += s15 * 470296;  s5  += s15 * 654183;
    s6  -= s15 * 997805;  s7  += s15 * 136657;  s8  -= s15 * 683901;  s15 = 0;

    s2  += s14 * 666643;  s3  += s14 * 470296;  s4  += s14 * 654183;
    s5  -= s14 * 997805;  s6  += s14 * 136657;  s7  -= s14 * 683901;  s14 = 0;

    s1  += s13 * 666643;  s2  += s13 * 470296;  s3  += s13 * 654183;
    s4  -= s13 * 997805;  s5  += s13 * 136657;  s6  -= s13 * 683901;  s13 = 0;

    s0  += s12 * 666643;  s1  += s12 * 470296;  s2  += s12 * 654183;
    s3  -= s12 * 997805;  s4  += s12 * 136657;  s5  -= s12 * 683901;  s12 = 0;

    carry0 = (s0  + (1 << 20)) >> 21; s1  += carry0;  s0  -= carry0  << 21;
    carry2 = (s2  + (1 << 20)) >> 21; s3  += carry2;  s2  -= carry2  << 21;
    carry4 = (s4  + (1 << 20)) >> 21; s5  += carry4;  s4  -= carry4  << 21;
    carry6 = (s6  + (1 << 20)) >> 21; s7  += carry6;  s6  -= carry6  << 21;
    carry8 = (s8  + (1 << 20)) >> 21; s9  += carry8;  s8  -= carry8  << 21;
    carry10= (s10 + (1 << 20)) >> 21; s11 += carry10; s10 -= carry10 << 21;

    carry1 = (s1  + (1 << 20)) >> 21; s2  += carry1;  s1  -= carry1  << 21;
    carry3 = (s3  + (1 << 20)) >> 21; s4  += carry3;  s3  -= carry3  << 21;
    carry5 = (s5  + (1 << 20)) >> 21; s6  += carry5;  s5  -= carry5  << 21;
    carry7 = (s7  + (1 << 20)) >> 21; s8  += carry7;  s7  -= carry7  << 21;
    carry9 = (s9  + (1 << 20)) >> 21; s10 += carry9;  s9  -= carry9  << 21;
    carry11= (s11 + (1 << 20)) >> 21; s12 += carry11; s11 -= carry11 << 21;

    s0  += s12 * 666643;  s1  += s12 * 470296;  s2  += s12 * 654183;
    s3  -= s12 * 997805;  s4  += s12 * 136657;  s5  -= s12 * 683901;  s12 = 0;

    carry0 = s0  >> 21; s1  += carry0;  s0  -= carry0  << 21;
    carry1 = s1  >> 21; s2  += carry1;  s1  -= carry1  << 21;
    carry2 = s2  >> 21; s3  += carry2;  s2  -= carry2  << 21;
    carry3 = s3  >> 21; s4  += carry3;  s3  -= carry3  << 21;
    carry4 = s4  >> 21; s5  += carry4;  s4  -= carry4  << 21;
    carry5 = s5  >> 21; s6  += carry5;  s5  -= carry5  << 21;
    carry6 = s6  >> 21; s7  += carry6;  s6  -= carry6  << 21;
    carry7 = s7  >> 21; s8  += carry7;  s7  -= carry7  << 21;
    carry8 = s8  >> 21; s9  += carry8;  s8  -= carry8  << 21;
    carry9 = s9  >> 21; s10 += carry9;  s9  -= carry9  << 21;
    carry10= s10 >> 21; s11 += carry10; s10 -= carry10 << 21;
    carry11= s11 >> 21; s12 += carry11; s11 -= carry11 << 21;

    s0  += s12 * 666643;  s1  += s12 * 470296;  s2  += s12 * 654183;
    s3  -= s12 * 997805;  s4  += s12 * 136657;  s5  -= s12 * 683901;  s12 = 0;

    carry0 = s0  >> 21; s1  += carry0;  s0  -= carry0  << 21;
    carry1 = s1  >> 21; s2  += carry1;  s1  -= carry1  << 21;
    carry2 = s2  >> 21; s3  += carry2;  s2  -= carry2  << 21;
    carry3 = s3  >> 21; s4  += carry3;  s3  -= carry3  << 21;
    carry4 = s4  >> 21; s5  += carry4;  s4  -= carry4  << 21;
    carry5 = s5  >> 21; s6  += carry5;  s5  -= carry5  << 21;
    carry6 = s6  >> 21; s7  += carry6;  s6  -= carry6  << 21;
    carry7 = s7  >> 21; s8  += carry7;  s7  -= carry7  << 21;
    carry8 = s8  >> 21; s9  += carry8;  s8  -= carry8  << 21;
    carry9 = s9  >> 21; s10 += carry9;  s9  -= carry9  << 21;
    carry10= s10 >> 21; s11 += carry10; s10 -= carry10 << 21;

    /* Pack 12 reduced limbs (each < 2^21) into 32 bytes little-endian.
     * Direct ref10 bit-packing — no accumulator needed. */
    s[ 0] = (unsigned char)(s0  >>  0);
    s[ 1] = (unsigned char)(s0  >>  8);
    s[ 2] = (unsigned char)((s0 >> 16) | (s1 << 5));
    s[ 3] = (unsigned char)(s1  >>  3);
    s[ 4] = (unsigned char)(s1  >> 11);
    s[ 5] = (unsigned char)((s1 >> 19) | (s2 << 2));
    s[ 6] = (unsigned char)(s2  >>  6);
    s[ 7] = (unsigned char)((s2 >> 14) | (s3 << 7));
    s[ 8] = (unsigned char)(s3  >>  1);
    s[ 9] = (unsigned char)(s3  >>  9);
    s[10] = (unsigned char)((s3 >> 17) | (s4 << 4));
    s[11] = (unsigned char)(s4  >>  4);
    s[12] = (unsigned char)(s4  >> 12);
    s[13] = (unsigned char)((s4 >> 20) | (s5 << 1));
    s[14] = (unsigned char)(s5  >>  7);
    s[15] = (unsigned char)((s5 >> 15) | (s6 << 6));
    s[16] = (unsigned char)(s6  >>  2);
    s[17] = (unsigned char)(s6  >> 10);
    s[18] = (unsigned char)((s6 >> 18) | (s7 << 3));
    s[19] = (unsigned char)(s7  >>  5);
    s[20] = (unsigned char)(s7  >> 13);
    s[21] = (unsigned char)(s8  >>  0);
    s[22] = (unsigned char)(s8  >>  8);
    s[23] = (unsigned char)((s8 >> 16) | (s9 << 5));
    s[24] = (unsigned char)(s9  >>  3);
    s[25] = (unsigned char)(s9  >> 11);
    s[26] = (unsigned char)((s9 >> 19) | (s10 << 2));
    s[27] = (unsigned char)(s10 >>  6);
    s[28] = (unsigned char)((s10 >> 14) | (s11 << 7));
    s[29] = (unsigned char)(s11 >>  1);
    s[30] = (unsigned char)(s11 >>  9);
    s[31] = (unsigned char)(s11 >> 17);
    /* Zero upper half */
    for (int i = 32; i < 64; i++) s[i] = 0;
}

#endif /* SHA512_DEVICE_CUH */
