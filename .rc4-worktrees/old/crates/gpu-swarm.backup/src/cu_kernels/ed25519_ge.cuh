/*
 * Ed25519 Group Element operations for CUDA
 *
 * Curve: -x^2 + y^2 = 1 + d*x^2*y^2  (twisted Edwards, a = -1)
 *   where d = -121665/121666 mod p
 *
 * Representations:
 *   ge_p2:     (X:Y:Z)       projective             Y/Z, X/Z
 *   ge_p3:     (X:Y:Z:T)     extended               T = X*Y/Z
 *   ge_p1p1:   (X:Y:Z:T)     completed              X/T, Y/Z
 *   ge_precomp: (ypx:ymx:xy2d) for fixed-base lookup
 *   ge_cached:  (YpX:YmX:Z:T2d) for variable-base addition
 *
 * Based on the ref10 implementation from SUPERCOP/libsodium.
 */

#ifndef ED25519_GE_CUH
#define ED25519_GE_CUH

#include "ed25519_field.cuh"

/* ───── Curve constant d = -121665/121666 mod p ───── */
__device__ void fe_d(fe *out) {
    /* d = -121665/121666 mod p
     * = 37095705934669439343138083508754565189542113879843219016388785533085940283555 */
    out->v[0] = -10913610; out->v[1] =  13857413;
    out->v[2] = -15372611; out->v[3] =   6949391;
    out->v[4] =    114729; out->v[5] =  -8787816;
    out->v[6] =  -6275908; out->v[7] =  -3247719;
    out->v[8] = -18696448; out->v[9] =  -12055116;
}

/* 2*d */
__device__ void fe_d2(fe *out) {
    out->v[0] = -21827239; out->v[1] = -5839606;
    out->v[2] = -30745221; out->v[3] = 13898782;
    out->v[4] =    229458; out->v[5] = 15978800;
    out->v[6] = -12551817; out->v[7] = -6495438;
    out->v[8] =  29715968; out->v[9] =  9444199;
}

/* sqrt(-1) mod p */
__device__ void fe_sqrtm1(fe *out) {
    out->v[0] = -32595792; out->v[1] = -7943725;
    out->v[2] =  9377950;  out->v[3] =  3500415;
    out->v[4] = 12389472;  out->v[5] = -272473;
    out->v[6] = -25146209; out->v[7] = -2005654;
    out->v[8] =  326686;   out->v[9] =  11406482;
}

/* ───── Point types ───── */
typedef struct { fe X; fe Y; fe Z; }             ge_p2;
typedef struct { fe X; fe Y; fe Z; fe T; }       ge_p3;
typedef struct { fe X; fe Y; fe Z; fe T; }       ge_p1p1;
typedef struct { fe yplusx; fe yminusx; fe xy2d; } ge_precomp;
typedef struct { fe YplusX; fe YminusX; fe Z; fe T2d; } ge_cached;

/* ───── Conversions ───── */
__device__ void ge_p1p1_to_p2(ge_p2 *r, const ge_p1p1 *p) {
    fe_mul(&r->X, &p->X, &p->T);
    fe_mul(&r->Y, &p->Y, &p->Z);
    fe_mul(&r->Z, &p->Z, &p->T);
}

__device__ void ge_p1p1_to_p3(ge_p3 *r, const ge_p1p1 *p) {
    fe_mul(&r->X, &p->X, &p->T);
    fe_mul(&r->Y, &p->Y, &p->Z);
    fe_mul(&r->Z, &p->Z, &p->T);
    fe_mul(&r->T, &p->X, &p->Y);
}

__device__ void ge_p2_to_p3(ge_p3 *r, const ge_p2 *p) {
    fe_copy(&r->X, &p->X); fe_copy(&r->Y, &p->Y); fe_copy(&r->Z, &p->Z);
    fe_mul(&r->T, &p->X, &p->Y);
}

/* ───── ge_p2_dbl → ge_p1p1 ───── */
__device__ void ge_p2_dbl(ge_p1p1 *r, const ge_p2 *p) {
    fe t0;
    fe_sq(&r->X, &p->X);
    fe_sq(&r->Z, &p->Y);
    fe_sq2(&r->T, &p->Z);
    fe_add(&r->Y, &p->X, &p->Y);
    fe_sq(&t0, &r->Y);
    fe_add(&r->Y, &r->Z, &r->X);
    fe_sub(&r->Z, &r->Z, &r->X);
    fe_sub(&r->X, &t0, &r->Y);
    fe_sub(&r->T, &r->T, &r->Z);
}

/* ───── ge_p3_dbl → ge_p1p1 ───── */
__device__ void ge_p3_dbl(ge_p1p1 *r, const ge_p3 *p) {
    ge_p2 q;
    q.X = p->X; q.Y = p->Y; q.Z = p->Z;
    ge_p2_dbl(r, &q);
}

/* ───── ge_p3_to_cached ───── */
__device__ void ge_p3_to_cached(ge_cached *r, const ge_p3 *p) {
    fe d2;
    fe_d2(&d2);
    fe_add(&r->YplusX, &p->Y, &p->X);
    fe_sub(&r->YminusX, &p->Y, &p->X);
    fe_copy(&r->Z, &p->Z);
    fe_mul(&r->T2d, &p->T, &d2);
}

/* ───── ge_add: p3 + cached → p1p1 ───── */
__device__ void ge_add(ge_p1p1 *r, const ge_p3 *p, const ge_cached *q) {
    fe t0;
    fe_add(&r->X, &p->Y, &p->X);
    fe_sub(&r->Y, &p->Y, &p->X);
    fe_mul(&r->Z, &r->X, &q->YplusX);
    fe_mul(&r->Y, &r->Y, &q->YminusX);
    fe_mul(&r->T, &q->T2d, &p->T);
    fe_mul(&r->X, &p->Z, &q->Z);
    fe_add(&t0, &r->X, &r->X);
    fe_sub(&r->X, &r->Z, &r->Y);
    fe_add(&r->Y, &r->Z, &r->Y);
    fe_add(&r->Z, &t0, &r->T);
    fe_sub(&r->T, &t0, &r->T);
}

/* ───── ge_sub: p3 - cached → p1p1 ───── */
__device__ void ge_sub(ge_p1p1 *r, const ge_p3 *p, const ge_cached *q) {
    fe t0;
    fe_add(&r->X, &p->Y, &p->X);
    fe_sub(&r->Y, &p->Y, &p->X);
    fe_mul(&r->Z, &r->X, &q->YminusX);
    fe_mul(&r->Y, &r->Y, &q->YplusX);
    fe_mul(&r->T, &q->T2d, &p->T);
    fe_mul(&r->X, &p->Z, &q->Z);
    fe_add(&t0, &r->X, &r->X);
    fe_sub(&r->X, &r->Z, &r->Y);
    fe_add(&r->Y, &r->Z, &r->Y);
    fe_sub(&r->Z, &t0, &r->T);
    fe_add(&r->T, &t0, &r->T);
}

/* ───── ge_madd: p3 + precomp → p1p1 ───── */
__device__ void ge_madd(ge_p1p1 *r, const ge_p3 *p, const ge_precomp *q) {
    fe t0;
    fe_add(&r->X, &p->Y, &p->X);
    fe_sub(&r->Y, &p->Y, &p->X);
    fe_mul(&r->Z, &r->X, &q->yplusx);
    fe_mul(&r->Y, &r->Y, &q->yminusx);
    fe_mul(&r->T, &q->xy2d, &p->T);
    fe_add(&t0, &p->Z, &p->Z);
    fe_sub(&r->X, &r->Z, &r->Y);
    fe_add(&r->Y, &r->Z, &r->Y);
    fe_add(&r->Z, &t0, &r->T);
    fe_sub(&r->T, &t0, &r->T);
}

/* ───── ge_msub: p3 - precomp → p1p1 ───── */
__device__ void ge_msub(ge_p1p1 *r, const ge_p3 *p, const ge_precomp *q) {
    fe t0;
    fe_add(&r->X, &p->Y, &p->X);
    fe_sub(&r->Y, &p->Y, &p->X);
    fe_mul(&r->Z, &r->X, &q->yminusx);
    fe_mul(&r->Y, &r->Y, &q->yplusx);
    fe_mul(&r->T, &q->xy2d, &p->T);
    fe_add(&t0, &p->Z, &p->Z);
    fe_sub(&r->X, &r->Z, &r->Y);
    fe_add(&r->Y, &r->Z, &r->Y);
    fe_sub(&r->Z, &t0, &r->T);
    fe_add(&r->T, &t0, &r->T);
}

/* ───── Identity / zero point ───── */
__device__ void ge_p3_0(ge_p3 *h) {
    fe_0(&h->X);
    fe_1(&h->Y);
    fe_1(&h->Z);
    fe_0(&h->T);
}

__device__ void ge_p2_0(ge_p2 *h) {
    fe_0(&h->X);
    fe_1(&h->Y);
    fe_1(&h->Z);
}

/* ───── ge_p3_is_neutral ───── */
__device__ int ge_p3_is_neutral(const ge_p3 *p) {
    fe check;
    fe_sub(&check, &p->Y, &p->Z);
    if (fe_isnonzero(&check)) return 0;
    if (fe_isnonzero(&p->X)) return 0;
    return 1;
}

/* ───── ge_frombytes_negate_vartime: decode point from 32 bytes ───── */
/* Returns 0 on success, -1 on failure */
__device__ int ge_frombytes_negate_vartime(ge_p3 *h, const unsigned char *s) {
    fe u, v, v3, vxx, check;
    fe d_val, sqrtm1;

    fe_frombytes(&h->Y, s);
    fe_1(&h->Z);

    /* u = y^2 - 1 */
    fe_sq(&u, &h->Y);
    fe_d(&d_val);
    fe_mul(&v, &u, &d_val);   /* v = d*y^2 */
    fe_sub(&u, &u, &h->Z);     /* u = y^2 - 1 */
    fe_add(&v, &v, &h->Z);     /* v = d*y^2 + 1 */

    /* x = (u*v^3) * (u*v^7)^((p-5)/8) */
    fe_sq(&v3, &v);
    fe_mul(&v3, &v3, &v);       /* v3 = v^3 */
    fe_sq(&h->X, &v3);
    fe_mul(&h->X, &h->X, &v);  /* h->X = v^7 */
    fe_mul(&h->X, &h->X, &u);  /* h->X = u*v^7 */
    fe_pow22523(&h->X, &h->X);  /* h->X = (u*v^7)^((p-5)/8) */
    fe_mul(&h->X, &h->X, &v3);
    fe_mul(&h->X, &h->X, &u);  /* h->X = u*v^3 * (u*v^7)^((p-5)/8) */

    fe_sq(&vxx, &h->X);
    fe_mul(&vxx, &vxx, &v);     /* vxx = v * x^2 */
    fe_sub(&check, &vxx, &u);
    if (fe_isnonzero(&check) == 0) {
        /* x^2*v = u, ok */
    } else {
        fe_add(&check, &vxx, &u);
        if (fe_isnonzero(&check)) return -1;
        fe_sqrtm1(&sqrtm1);
        fe_mul(&h->X, &h->X, &sqrtm1);
    }

    /* "negate_vartime": return -point, so negate when sign AGREES with sign_bit.
     * This gives isneg(result_X) == 1 - sign_bit = sign of -correct_x. */
    if (fe_isnegative(&h->X) == ((s[31] >> 7) & 1)) {
        fe_neg(&h->X, &h->X);
    }

    fe_mul(&h->T, &h->X, &h->Y);
    return 0;
}

/* ───── Scalar multiplication: double-and-add (variable time, 1 thread per sig) ───── */
/* Computes R = [scalar] * P using a simple double-and-add */
__device__ void ge_scalarmult_vartime(ge_p3 *R, const unsigned char scalar[32], const ge_p3 *P) {
    ge_p3_0(R);
    ge_cached Pcached;
    ge_p3_to_cached(&Pcached, P);

    /* Find highest set bit */
    int top = 255;
    while (top >= 0) {
        int byte_idx = top >> 3;
        int bit_idx = top & 7;
        if ((scalar[byte_idx] >> bit_idx) & 1) break;
        top--;
    }

    for (int i = top; i >= 0; i--) {
        ge_p1p1 t;
        /* Always double first: R = 2*R */
        ge_p3_dbl(&t, R);
        ge_p1p1_to_p3(R, &t);

        int byte_idx = i >> 3;
        int bit_idx = i & 7;
        int bit = (scalar[byte_idx] >> bit_idx) & 1;

        if (bit) {
            /* R was just set to 2*oldR; now add P: R = 2*oldR + P */
            ge_add(&t, R, &Pcached);
            ge_p1p1_to_p3(R, &t);
        }
    }
}

/* ───── Double scalar multiplication: [a]*A + [b]*B ───── */
/* This is the core of Ed25519 verification: check [s]*B == R + [h]*A */
/* Uses Straus/Shamir interleaved double-and-add */
__device__ void ge_double_scalarmult_vartime(
    ge_p2 *R,
    const unsigned char a[32],
    const ge_p3 *A,
    const unsigned char b[32],
    const ge_p3 *B
) {
    ge_cached Acached, Bcached;
    ge_p3_to_cached(&Acached, A);
    ge_p3_to_cached(&Bcached, B);

    ge_p3 result;
    ge_p3_0(&result);

    /* Find highest set bit across both scalars */
    int top = 255;
    while (top >= 0) {
        int byte_idx = top >> 3;
        int bit_idx = top & 7;
        int abit = (a[byte_idx] >> bit_idx) & 1;
        int bbit = (b[byte_idx] >> bit_idx) & 1;
        if (abit || bbit) break;
        top--;
    }

    for (int i = top; i >= 0; i--) {
        ge_p1p1 t;
        ge_p3_dbl(&t, &result);
        ge_p1p1_to_p3(&result, &t);

        int byte_idx = i >> 3;
        int bit_idx = i & 7;
        int abit = (a[byte_idx] >> bit_idx) & 1;
        int bbit = (b[byte_idx] >> bit_idx) & 1;

        if (abit && bbit) {
            /* result += A + B */
            ge_add(&t, &result, &Acached);
            ge_p1p1_to_p3(&result, &t);
            ge_add(&t, &result, &Bcached);
            ge_p1p1_to_p3(&result, &t);
        } else if (abit) {
            ge_add(&t, &result, &Acached);
            ge_p1p1_to_p3(&result, &t);
        } else if (bbit) {
            ge_add(&t, &result, &Bcached);
            ge_p1p1_to_p3(&result, &t);
        }
    }

    /* Convert to projective (X:Y:Z) */
    R->X = result.X;
    R->Y = result.Y;
    R->Z = result.Z;
}

/* ───── ge_tobytes: encode ge_p2 point to 32 bytes ───── */
__device__ void ge_tobytes(unsigned char *s, const ge_p2 *h) {
    fe recip, x, y;

    fe_invert(&recip, &h->Z);
    fe_mul(&x, &h->X, &recip);
    fe_mul(&y, &h->Y, &recip);
    fe_tobytes(s, &y);
    s[31] ^= fe_isnegative(&x) << 7;
}

#endif /* ED25519_GE_CUH */
