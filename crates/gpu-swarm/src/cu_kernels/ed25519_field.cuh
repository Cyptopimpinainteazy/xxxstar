/*
 * Ed25519 Field Arithmetic for CUDA
 *
 * Field: F_p where p = 2^255 - 19
 * Representation: 10 limbs of ~25.5 bits each (radix-2^25.5 aka 5x26 + 5x25)
 *   f[0], f[2], f[4], f[6], f[8]: 26-bit limbs
 *   f[1], f[3], f[5], f[7], f[9]: 25-bit limbs
 *
 * This is the standard "ref10" representation used by libsodium/SUPERCOP.
 * It avoids carry chains and allows lazy reduction.
 */

#ifndef ED25519_FIELD_CUH
#define ED25519_FIELD_CUH

#include <cuda_runtime.h>
#include <stdint.h>

/* fe: field element, 10 x int32 limbs */
typedef struct { int32_t v[10]; } fe;

/* Constant: p = 2^255 - 19 is implicit in reduction */

/* ───────────────────── helpers ──────────────────────── */
__device__ __forceinline__ int64_t load_3(const unsigned char *in) {
    return (int64_t)in[0] | ((int64_t)in[1] << 8) | ((int64_t)in[2] << 16);
}

__device__ __forceinline__ int64_t load_4(const unsigned char *in) {
    return (int64_t)in[0] | ((int64_t)in[1] << 8) |
           ((int64_t)in[2] << 16) | ((int64_t)in[3] << 24);
}

/* ───────────────────── fe_frombytes ─────────────────── */
/* Decode 32 bytes (little-endian) into field element */
__device__ void fe_frombytes(fe *h, const unsigned char *s) {
    int64_t h0 = load_4(s);
    int64_t h1 = load_3(s + 4) << 6;
    int64_t h2 = load_3(s + 7) << 5;
    int64_t h3 = load_3(s + 10) << 3;
    int64_t h4 = load_3(s + 13) << 2;
    int64_t h5 = load_4(s + 16);
    int64_t h6 = load_3(s + 20) << 7;
    int64_t h7 = load_3(s + 23) << 5;
    int64_t h8 = load_3(s + 26) << 4;
    int64_t h9 = (load_3(s + 29) & 0x7fffff) << 2;

    int64_t carry9 = (h9 + (1 << 24)) >> 25; h0 += carry9 * 19; h9 -= carry9 << 25;
    int64_t carry1 = (h1 + (1 << 24)) >> 25; h2 += carry1; h1 -= carry1 << 25;
    int64_t carry3 = (h3 + (1 << 24)) >> 25; h4 += carry3; h3 -= carry3 << 25;
    int64_t carry5 = (h5 + (1 << 24)) >> 25; h6 += carry5; h5 -= carry5 << 25;
    int64_t carry7 = (h7 + (1 << 24)) >> 25; h8 += carry7; h7 -= carry7 << 25;

    int64_t carry0 = (h0 + (1 << 25)) >> 26; h1 += carry0; h0 -= carry0 << 26;
    int64_t carry2 = (h2 + (1 << 25)) >> 26; h3 += carry2; h2 -= carry2 << 26;
    int64_t carry4 = (h4 + (1 << 25)) >> 26; h5 += carry4; h4 -= carry4 << 26;
    int64_t carry6 = (h6 + (1 << 25)) >> 26; h7 += carry6; h6 -= carry6 << 26;
    int64_t carry8 = (h8 + (1 << 25)) >> 26; h9 += carry8; h8 -= carry8 << 26;

    h->v[0] = (int32_t)h0; h->v[1] = (int32_t)h1;
    h->v[2] = (int32_t)h2; h->v[3] = (int32_t)h3;
    h->v[4] = (int32_t)h4; h->v[5] = (int32_t)h5;
    h->v[6] = (int32_t)h6; h->v[7] = (int32_t)h7;
    h->v[8] = (int32_t)h8; h->v[9] = (int32_t)h9;
}

/* ───────────────────── fe_tobytes ────────────────────── */
/* Encode field element to 32 bytes (little-endian), fully reduced */
__device__ void fe_tobytes(unsigned char *s, const fe *h) {
    int32_t h0 = h->v[0], h1 = h->v[1], h2 = h->v[2], h3 = h->v[3], h4 = h->v[4];
    int32_t h5 = h->v[5], h6 = h->v[6], h7 = h->v[7], h8 = h->v[8], h9 = h->v[9];
    int32_t q;
    int32_t carry0, carry1, carry2, carry3, carry4;
    int32_t carry5, carry6, carry7, carry8, carry9;

    q = (19 * h9 + ((int32_t)1 << 24)) >> 25;
    q = (h0 + q) >> 26;
    q = (h1 + q) >> 25;
    q = (h2 + q) >> 26;
    q = (h3 + q) >> 25;
    q = (h4 + q) >> 26;
    q = (h5 + q) >> 25;
    q = (h6 + q) >> 26;
    q = (h7 + q) >> 25;
    q = (h8 + q) >> 26;
    q = (h9 + q) >> 25;

    h0 += 19 * q;
    carry0 = h0 >> 26; h1 += carry0; h0 -= carry0 << 26;
    carry1 = h1 >> 25; h2 += carry1; h1 -= carry1 << 25;
    carry2 = h2 >> 26; h3 += carry2; h2 -= carry2 << 26;
    carry3 = h3 >> 25; h4 += carry3; h3 -= carry3 << 25;
    carry4 = h4 >> 26; h5 += carry4; h4 -= carry4 << 26;
    carry5 = h5 >> 25; h6 += carry5; h5 -= carry5 << 25;
    carry6 = h6 >> 26; h7 += carry6; h6 -= carry6 << 26;
    carry7 = h7 >> 25; h8 += carry7; h7 -= carry7 << 25;
    carry8 = h8 >> 26; h9 += carry8; h8 -= carry8 << 26;
    carry9 = h9 >> 25;              h9 -= carry9 << 25;
    /* carry9 is 0 or 1 at this point */

    s[0]  = (unsigned char)(h0 >>  0);
    s[1]  = (unsigned char)(h0 >>  8);
    s[2]  = (unsigned char)(h0 >> 16);
    s[3]  = (unsigned char)((h0 >> 24) | (h1 << 2));
    s[4]  = (unsigned char)(h1 >>  6);
    s[5]  = (unsigned char)(h1 >> 14);
    s[6]  = (unsigned char)((h1 >> 22) | (h2 << 3));
    s[7]  = (unsigned char)(h2 >>  5);
    s[8]  = (unsigned char)(h2 >> 13);
    s[9]  = (unsigned char)((h2 >> 21) | (h3 << 5));
    s[10] = (unsigned char)(h3 >>  3);
    s[11] = (unsigned char)(h3 >> 11);
    s[12] = (unsigned char)((h3 >> 19) | (h4 << 6));
    s[13] = (unsigned char)(h4 >>  2);
    s[14] = (unsigned char)(h4 >> 10);
    s[15] = (unsigned char)(h4 >> 18);
    s[16] = (unsigned char)(h5 >>  0);
    s[17] = (unsigned char)(h5 >>  8);
    s[18] = (unsigned char)(h5 >> 16);
    s[19] = (unsigned char)((h5 >> 24) | (h6 << 1));
    s[20] = (unsigned char)(h6 >>  7);
    s[21] = (unsigned char)(h6 >> 15);
    s[22] = (unsigned char)((h6 >> 23) | (h7 << 3));
    s[23] = (unsigned char)(h7 >>  5);
    s[24] = (unsigned char)(h7 >> 13);
    s[25] = (unsigned char)((h7 >> 21) | (h8 << 4));
    s[26] = (unsigned char)(h8 >>  4);
    s[27] = (unsigned char)(h8 >> 12);
    s[28] = (unsigned char)((h8 >> 20) | (h9 << 6));
    s[29] = (unsigned char)(h9 >>  2);
    s[30] = (unsigned char)(h9 >> 10);
    s[31] = (unsigned char)(h9 >> 18);
}

/* ───────────────────── basic arithmetic ─────────────── */
__device__ void fe_0(fe *h)  { for (int i = 0; i < 10; i++) h->v[i] = 0; }
__device__ void fe_1(fe *h)  { h->v[0] = 1; for (int i = 1; i < 10; i++) h->v[i] = 0; }
__device__ void fe_copy(fe *h, const fe *f) { for (int i = 0; i < 10; i++) h->v[i] = f->v[i]; }

__device__ void fe_add(fe *h, const fe *f, const fe *g) {
    for (int i = 0; i < 10; i++) h->v[i] = f->v[i] + g->v[i];
}

__device__ void fe_sub(fe *h, const fe *f, const fe *g) {
    for (int i = 0; i < 10; i++) h->v[i] = f->v[i] - g->v[i];
}

__device__ void fe_neg(fe *h, const fe *f) {
    for (int i = 0; i < 10; i++) h->v[i] = -f->v[i];
}

/* ───────────────────── fe_mul ──────────────────────── */
/* Schoolbook multiply with delayed carry, matching ref10 */
__device__ void fe_mul(fe *h, const fe *f, const fe *g) {
    int32_t f0 = f->v[0], f1 = f->v[1], f2 = f->v[2], f3 = f->v[3], f4 = f->v[4];
    int32_t f5 = f->v[5], f6 = f->v[6], f7 = f->v[7], f8 = f->v[8], f9 = f->v[9];
    int32_t g0 = g->v[0], g1 = g->v[1], g2 = g->v[2], g3 = g->v[3], g4 = g->v[4];
    int32_t g5 = g->v[5], g6 = g->v[6], g7 = g->v[7], g8 = g->v[8], g9 = g->v[9];
    int32_t g1_19 = 19*g1, g2_19 = 19*g2, g3_19 = 19*g3, g4_19 = 19*g4;
    int32_t g5_19 = 19*g5, g6_19 = 19*g6, g7_19 = 19*g7, g8_19 = 19*g8, g9_19 = 19*g9;
    int32_t f1_2 = 2*f1, f3_2 = 2*f3, f5_2 = 2*f5, f7_2 = 2*f7, f9_2 = 2*f9;

    int64_t h0 = (int64_t)f0*g0 + (int64_t)f1_2*g9_19 + (int64_t)f2*g8_19 + (int64_t)f3_2*g7_19 + (int64_t)f4*g6_19 + (int64_t)f5_2*g5_19 + (int64_t)f6*g4_19 + (int64_t)f7_2*g3_19 + (int64_t)f8*g2_19 + (int64_t)f9_2*g1_19;
    int64_t h1 = (int64_t)f0*g1 + (int64_t)f1*g0   + (int64_t)f2*g9_19 + (int64_t)f3*g8_19   + (int64_t)f4*g7_19 + (int64_t)f5*g6_19   + (int64_t)f6*g5_19 + (int64_t)f7*g4_19   + (int64_t)f8*g3_19 + (int64_t)f9*g2_19;
    int64_t h2 = (int64_t)f0*g2 + (int64_t)f1_2*g1  + (int64_t)f2*g0    + (int64_t)f3_2*g9_19 + (int64_t)f4*g8_19 + (int64_t)f5_2*g7_19 + (int64_t)f6*g6_19 + (int64_t)f7_2*g5_19 + (int64_t)f8*g4_19 + (int64_t)f9_2*g3_19;
    int64_t h3 = (int64_t)f0*g3 + (int64_t)f1*g2    + (int64_t)f2*g1    + (int64_t)f3*g0      + (int64_t)f4*g9_19 + (int64_t)f5*g8_19   + (int64_t)f6*g7_19 + (int64_t)f7*g6_19   + (int64_t)f8*g5_19 + (int64_t)f9*g4_19;
    int64_t h4 = (int64_t)f0*g4 + (int64_t)f1_2*g3  + (int64_t)f2*g2    + (int64_t)f3_2*g1    + (int64_t)f4*g0    + (int64_t)f5_2*g9_19 + (int64_t)f6*g8_19 + (int64_t)f7_2*g7_19 + (int64_t)f8*g6_19 + (int64_t)f9_2*g5_19;
    int64_t h5 = (int64_t)f0*g5 + (int64_t)f1*g4    + (int64_t)f2*g3    + (int64_t)f3*g2      + (int64_t)f4*g1    + (int64_t)f5*g0      + (int64_t)f6*g9_19 + (int64_t)f7*g8_19   + (int64_t)f8*g7_19 + (int64_t)f9*g6_19;
    int64_t h6 = (int64_t)f0*g6 + (int64_t)f1_2*g5  + (int64_t)f2*g4    + (int64_t)f3_2*g3    + (int64_t)f4*g2    + (int64_t)f5_2*g1    + (int64_t)f6*g0    + (int64_t)f7_2*g9_19 + (int64_t)f8*g8_19 + (int64_t)f9_2*g7_19;
    int64_t h7 = (int64_t)f0*g7 + (int64_t)f1*g6    + (int64_t)f2*g5    + (int64_t)f3*g4      + (int64_t)f4*g3    + (int64_t)f5*g2      + (int64_t)f6*g1    + (int64_t)f7*g0      + (int64_t)f8*g9_19 + (int64_t)f9*g8_19;
    int64_t h8 = (int64_t)f0*g8 + (int64_t)f1_2*g7  + (int64_t)f2*g6    + (int64_t)f3_2*g5    + (int64_t)f4*g4    + (int64_t)f5_2*g3    + (int64_t)f6*g2    + (int64_t)f7_2*g1    + (int64_t)f8*g0    + (int64_t)f9_2*g9_19;
    int64_t h9 = (int64_t)f0*g9 + (int64_t)f1*g8    + (int64_t)f2*g7    + (int64_t)f3*g6      + (int64_t)f4*g5    + (int64_t)f5*g4      + (int64_t)f6*g3    + (int64_t)f7*g2      + (int64_t)f8*g1    + (int64_t)f9*g0;

    int64_t c0, c1, c2, c3, c4, c5, c6, c7, c8, c9;
    c0 = (h0 + ((int64_t)1 << 25)) >> 26; h1 += c0; h0 -= c0 << 26;
    c4 = (h4 + ((int64_t)1 << 25)) >> 26; h5 += c4; h4 -= c4 << 26;
    c1 = (h1 + ((int64_t)1 << 24)) >> 25; h2 += c1; h1 -= c1 << 25;
    c5 = (h5 + ((int64_t)1 << 24)) >> 25; h6 += c5; h5 -= c5 << 25;
    c2 = (h2 + ((int64_t)1 << 25)) >> 26; h3 += c2; h2 -= c2 << 26;
    c6 = (h6 + ((int64_t)1 << 25)) >> 26; h7 += c6; h6 -= c6 << 26;
    c3 = (h3 + ((int64_t)1 << 24)) >> 25; h4 += c3; h3 -= c3 << 25;
    c7 = (h7 + ((int64_t)1 << 24)) >> 25; h8 += c7; h7 -= c7 << 25;
    c4 = (h4 + ((int64_t)1 << 25)) >> 26; h5 += c4; h4 -= c4 << 26;
    c8 = (h8 + ((int64_t)1 << 25)) >> 26; h9 += c8; h8 -= c8 << 26;
    c9 = (h9 + ((int64_t)1 << 24)) >> 25; h0 += c9 * 19; h9 -= c9 << 25;
    c0 = (h0 + ((int64_t)1 << 25)) >> 26; h1 += c0; h0 -= c0 << 26;

    h->v[0] = (int32_t)h0; h->v[1] = (int32_t)h1; h->v[2] = (int32_t)h2;
    h->v[3] = (int32_t)h3; h->v[4] = (int32_t)h4; h->v[5] = (int32_t)h5;
    h->v[6] = (int32_t)h6; h->v[7] = (int32_t)h7; h->v[8] = (int32_t)h8;
    h->v[9] = (int32_t)h9;
}

/* ───────────────────── fe_sq (square) ──────────────── */
__device__ void fe_sq(fe *h, const fe *f) {
    int32_t f0 = f->v[0], f1 = f->v[1], f2 = f->v[2], f3 = f->v[3], f4 = f->v[4];
    int32_t f5 = f->v[5], f6 = f->v[6], f7 = f->v[7], f8 = f->v[8], f9 = f->v[9];
    int32_t f0_2 = 2*f0, f1_2 = 2*f1, f2_2 = 2*f2, f3_2 = 2*f3, f4_2 = 2*f4;
    int32_t f5_2 = 2*f5, f6_2 = 2*f6, f7_2 = 2*f7;
    int32_t f5_38 = 38*f5, f6_19 = 19*f6, f7_38 = 38*f7, f8_19 = 19*f8, f9_38 = 38*f9;

    int64_t h0 = (int64_t)f0*f0     + (int64_t)f1_2*f9_38 + (int64_t)f2_2*f8_19 + (int64_t)f3_2*f7_38 + (int64_t)f4_2*f6_19 + (int64_t)f5*f5_38;
    int64_t h1 = (int64_t)f0_2*f1   + (int64_t)f2*f9_38   + (int64_t)f3_2*f8_19 + (int64_t)f4*f7_38   + (int64_t)f5_2*f6_19;
    int64_t h2 = (int64_t)f0_2*f2   + (int64_t)f1_2*f1    + (int64_t)f3_2*f9_38 + (int64_t)f4_2*f8_19 + (int64_t)f5_2*f7_38 + (int64_t)f6*f6_19;
    int64_t h3 = (int64_t)f0_2*f3   + (int64_t)f1_2*f2    + (int64_t)f4*f9_38   + (int64_t)f5_2*f8_19 + (int64_t)f6*f7_38;
    int64_t h4 = (int64_t)f0_2*f4   + (int64_t)f1_2*f3_2  + (int64_t)f2*f2      + (int64_t)f5_2*f9_38 + (int64_t)f6_2*f8_19 + (int64_t)f7*f7_38;
    int64_t h5 = (int64_t)f0_2*f5   + (int64_t)f1_2*f4    + (int64_t)f2_2*f3    + (int64_t)f6*f9_38   + (int64_t)f7_2*f8_19;
    int64_t h6 = (int64_t)f0_2*f6   + (int64_t)f1_2*f5_2  + (int64_t)f2_2*f4    + (int64_t)f3_2*f3    + (int64_t)f7_2*f9_38 + (int64_t)f8*f8_19;
    int64_t h7 = (int64_t)f0_2*f7   + (int64_t)f1_2*f6    + (int64_t)f2_2*f5    + (int64_t)f3_2*f4    + (int64_t)f8*f9_38;
    int64_t h8 = (int64_t)f0_2*f8   + (int64_t)f1_2*f7_2  + (int64_t)f2_2*f6    + (int64_t)f3_2*f5_2  + (int64_t)f4*f4    + (int64_t)f9*f9_38;
    int64_t h9 = (int64_t)f0_2*f9   + (int64_t)f1_2*f8    + (int64_t)f2_2*f7    + (int64_t)f3_2*f6    + (int64_t)f4_2*f5;

    int64_t c0, c1, c2, c3, c4, c5, c6, c7, c8, c9;
    c0 = (h0 + ((int64_t)1 << 25)) >> 26; h1 += c0; h0 -= c0 << 26;
    c4 = (h4 + ((int64_t)1 << 25)) >> 26; h5 += c4; h4 -= c4 << 26;
    c1 = (h1 + ((int64_t)1 << 24)) >> 25; h2 += c1; h1 -= c1 << 25;
    c5 = (h5 + ((int64_t)1 << 24)) >> 25; h6 += c5; h5 -= c5 << 25;
    c2 = (h2 + ((int64_t)1 << 25)) >> 26; h3 += c2; h2 -= c2 << 26;
    c6 = (h6 + ((int64_t)1 << 25)) >> 26; h7 += c6; h6 -= c6 << 26;
    c3 = (h3 + ((int64_t)1 << 24)) >> 25; h4 += c3; h3 -= c3 << 25;
    c7 = (h7 + ((int64_t)1 << 24)) >> 25; h8 += c7; h7 -= c7 << 25;
    c4 = (h4 + ((int64_t)1 << 25)) >> 26; h5 += c4; h4 -= c4 << 26;
    c8 = (h8 + ((int64_t)1 << 25)) >> 26; h9 += c8; h8 -= c8 << 26;
    c9 = (h9 + ((int64_t)1 << 24)) >> 25; h0 += c9 * 19; h9 -= c9 << 25;
    c0 = (h0 + ((int64_t)1 << 25)) >> 26; h1 += c0; h0 -= c0 << 26;

    h->v[0] = (int32_t)h0; h->v[1] = (int32_t)h1; h->v[2] = (int32_t)h2;
    h->v[3] = (int32_t)h3; h->v[4] = (int32_t)h4; h->v[5] = (int32_t)h5;
    h->v[6] = (int32_t)h6; h->v[7] = (int32_t)h7; h->v[8] = (int32_t)h8;
    h->v[9] = (int32_t)h9;
}

/* fe_sq2: h = 2 * f^2 (used by ge_madd etc.) */
/* Inlined with carry chain to keep output limbs bounded (avoids limb overflow in fe_mul). */
__device__ void fe_sq2(fe *h, const fe *f) {
    int32_t f0 = f->v[0], f1 = f->v[1], f2 = f->v[2], f3 = f->v[3], f4 = f->v[4];
    int32_t f5 = f->v[5], f6 = f->v[6], f7 = f->v[7], f8 = f->v[8], f9 = f->v[9];
    int32_t f0_2 = 2*f0, f1_2 = 2*f1, f2_2 = 2*f2, f3_2 = 2*f3, f4_2 = 2*f4;
    int32_t f5_2 = 2*f5, f6_2 = 2*f6, f7_2 = 2*f7;
    int32_t f5_38 = 38*f5, f6_19 = 19*f6, f7_38 = 38*f7, f8_19 = 19*f8, f9_38 = 38*f9;

    /* Compute 2*f^2 by doubling each accumulator before carrying */
    int64_t h0 = 2*((int64_t)f0*f0     + (int64_t)f1_2*f9_38 + (int64_t)f2_2*f8_19 + (int64_t)f3_2*f7_38 + (int64_t)f4_2*f6_19 + (int64_t)f5*f5_38);
    int64_t h1 = 2*((int64_t)f0_2*f1   + (int64_t)f2*f9_38   + (int64_t)f3_2*f8_19 + (int64_t)f4*f7_38   + (int64_t)f5_2*f6_19);
    int64_t h2 = 2*((int64_t)f0_2*f2   + (int64_t)f1_2*f1    + (int64_t)f3_2*f9_38 + (int64_t)f4_2*f8_19 + (int64_t)f5_2*f7_38 + (int64_t)f6*f6_19);
    int64_t h3 = 2*((int64_t)f0_2*f3   + (int64_t)f1_2*f2    + (int64_t)f4*f9_38   + (int64_t)f5_2*f8_19 + (int64_t)f6*f7_38);
    int64_t h4 = 2*((int64_t)f0_2*f4   + (int64_t)f1_2*f3_2  + (int64_t)f2*f2      + (int64_t)f5_2*f9_38 + (int64_t)f6_2*f8_19 + (int64_t)f7*f7_38);
    int64_t h5 = 2*((int64_t)f0_2*f5   + (int64_t)f1_2*f4    + (int64_t)f2_2*f3    + (int64_t)f6*f9_38   + (int64_t)f7_2*f8_19);
    int64_t h6 = 2*((int64_t)f0_2*f6   + (int64_t)f1_2*f5_2  + (int64_t)f2_2*f4    + (int64_t)f3_2*f3    + (int64_t)f7_2*f9_38 + (int64_t)f8*f8_19);
    int64_t h7 = 2*((int64_t)f0_2*f7   + (int64_t)f1_2*f6    + (int64_t)f2_2*f5    + (int64_t)f3_2*f4    + (int64_t)f8*f9_38);
    int64_t h8 = 2*((int64_t)f0_2*f8   + (int64_t)f1_2*f7_2  + (int64_t)f2_2*f6    + (int64_t)f3_2*f5_2  + (int64_t)f4*f4    + (int64_t)f9*f9_38);
    int64_t h9 = 2*((int64_t)f0_2*f9   + (int64_t)f1_2*f8    + (int64_t)f2_2*f7    + (int64_t)f3_2*f6    + (int64_t)f4_2*f5);

    int64_t c0, c1, c2, c3, c4, c5, c6, c7, c8, c9;
    c0 = (h0 + ((int64_t)1 << 25)) >> 26; h1 += c0; h0 -= c0 << 26;
    c4 = (h4 + ((int64_t)1 << 25)) >> 26; h5 += c4; h4 -= c4 << 26;
    c1 = (h1 + ((int64_t)1 << 24)) >> 25; h2 += c1; h1 -= c1 << 25;
    c5 = (h5 + ((int64_t)1 << 24)) >> 25; h6 += c5; h5 -= c5 << 25;
    c2 = (h2 + ((int64_t)1 << 25)) >> 26; h3 += c2; h2 -= c2 << 26;
    c6 = (h6 + ((int64_t)1 << 25)) >> 26; h7 += c6; h6 -= c6 << 26;
    c3 = (h3 + ((int64_t)1 << 24)) >> 25; h4 += c3; h3 -= c3 << 25;
    c7 = (h7 + ((int64_t)1 << 24)) >> 25; h8 += c7; h7 -= c7 << 25;
    c4 = (h4 + ((int64_t)1 << 25)) >> 26; h5 += c4; h4 -= c4 << 26;
    c8 = (h8 + ((int64_t)1 << 25)) >> 26; h9 += c8; h8 -= c8 << 26;
    c9 = (h9 + ((int64_t)1 << 24)) >> 25; h0 += c9 * 19; h9 -= c9 << 25;
    c0 = (h0 + ((int64_t)1 << 25)) >> 26; h1 += c0; h0 -= c0 << 26;

    h->v[0] = (int32_t)h0; h->v[1] = (int32_t)h1; h->v[2] = (int32_t)h2;
    h->v[3] = (int32_t)h3; h->v[4] = (int32_t)h4; h->v[5] = (int32_t)h5;
    h->v[6] = (int32_t)h6; h->v[7] = (int32_t)h7; h->v[8] = (int32_t)h8;
    h->v[9] = (int32_t)h9;
}

/* ───────────────────── fe_invert ───────────────────── */
/* h = f^(p-2) = f^(2^255 - 21) via addition chain */
__device__ void fe_invert(fe *out, const fe *z) {
    fe t0, t1, t2, t3;
    int i;

    fe_sq(&t0, z);                    /* t0 = z^2 */
    fe_sq(&t1, &t0);                  /* t1 = z^4 */
    fe_sq(&t1, &t1);                  /* t1 = z^8 */
    fe_mul(&t1, z, &t1);             /* t1 = z^9 */
    fe_mul(&t0, &t0, &t1);           /* t0 = z^11 */
    fe_sq(&t2, &t0);                  /* t2 = z^22 */
    fe_mul(&t1, &t1, &t2);           /* t1 = z^(2^5-1) */
    fe_sq(&t2, &t1);
    for (i = 1; i < 5; ++i) fe_sq(&t2, &t2);
    fe_mul(&t1, &t2, &t1);           /* t1 = z^(2^10-1) */
    fe_sq(&t2, &t1);
    for (i = 1; i < 10; ++i) fe_sq(&t2, &t2);
    fe_mul(&t2, &t2, &t1);           /* t2 = z^(2^20-1) */
    fe_sq(&t3, &t2);
    for (i = 1; i < 20; ++i) fe_sq(&t3, &t3);
    fe_mul(&t2, &t3, &t2);           /* t2 = z^(2^40-1) */
    fe_sq(&t2, &t2);
    for (i = 1; i < 10; ++i) fe_sq(&t2, &t2);
    fe_mul(&t1, &t2, &t1);           /* t1 = z^(2^50-1) */
    fe_sq(&t2, &t1);
    for (i = 1; i < 50; ++i) fe_sq(&t2, &t2);
    fe_mul(&t2, &t2, &t1);           /* t2 = z^(2^100-1) */
    fe_sq(&t3, &t2);
    for (i = 1; i < 100; ++i) fe_sq(&t3, &t3);
    fe_mul(&t2, &t3, &t2);           /* t2 = z^(2^200-1) */
    fe_sq(&t2, &t2);
    for (i = 1; i < 50; ++i) fe_sq(&t2, &t2);
    fe_mul(&t1, &t2, &t1);           /* t1 = z^(2^250-1) */
    fe_sq(&t1, &t1);
    for (i = 1; i < 5; ++i) fe_sq(&t1, &t1);
    fe_mul(out, &t1, &t0);           /* out = z^(2^255-21) */
}

/* ───────────────────── fe_pow22523 ─────────────────── */
/* h = f^((p-5)/8) = f^(2^252 - 3) — used in point decompression */
__device__ void fe_pow22523(fe *out, const fe *z) {
    fe t0, t1, t2;
    int i;

    fe_sq(&t0, z);
    fe_sq(&t1, &t0);
    fe_sq(&t1, &t1);
    fe_mul(&t1, z, &t1);
    fe_mul(&t0, &t0, &t1);
    fe_sq(&t0, &t0);
    fe_mul(&t0, &t1, &t0);
    fe_sq(&t1, &t0);
    for (i = 1; i < 5; ++i) fe_sq(&t1, &t1);
    fe_mul(&t0, &t1, &t0);
    fe_sq(&t1, &t0);
    for (i = 1; i < 10; ++i) fe_sq(&t1, &t1);
    fe_mul(&t1, &t1, &t0);
    fe_sq(&t2, &t1);
    for (i = 1; i < 20; ++i) fe_sq(&t2, &t2);
    fe_mul(&t1, &t2, &t1);
    fe_sq(&t1, &t1);
    for (i = 1; i < 10; ++i) fe_sq(&t1, &t1);
    fe_mul(&t0, &t1, &t0);
    fe_sq(&t1, &t0);
    for (i = 1; i < 50; ++i) fe_sq(&t1, &t1);
    fe_mul(&t1, &t1, &t0);
    fe_sq(&t2, &t1);
    for (i = 1; i < 100; ++i) fe_sq(&t2, &t2);
    fe_mul(&t1, &t2, &t1);
    fe_sq(&t1, &t1);
    for (i = 1; i < 50; ++i) fe_sq(&t1, &t1);
    fe_mul(&t0, &t1, &t0);
    fe_sq(&t0, &t0);
    fe_sq(&t0, &t0);
    fe_mul(out, &t0, z);
}

/* ───────────────────── fe_cmov (constant-time) ──────── */
__device__ void fe_cmov(fe *f, const fe *g, int b) {
    int32_t mask = -b;  /* b must be 0 or 1 */
    for (int i = 0; i < 10; i++) {
        f->v[i] ^= mask & (f->v[i] ^ g->v[i]);
    }
}

/* ───────────────────── fe_isnegative / fe_isnonzero ── */
__device__ int fe_isnegative(const fe *f) {
    unsigned char s[32];
    fe_tobytes(s, f);
    return s[0] & 1;
}

__device__ int fe_isnonzero(const fe *f) {
    unsigned char s[32];
    fe_tobytes(s, f);
    unsigned char r = 0;
    for (int i = 0; i < 32; i++) r |= s[i];
    return r != 0;
}

#endif /* ED25519_FIELD_CUH */
