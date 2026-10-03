//! secp256k1 ECDSA verification on the GPU (WGSL, so it runs over Vulkan,
//! Metal or DX12 without a CUDA toolkit).
//!
//! # Division of labour
//!
//! Everything that decides *whether an input is well-formed* stays on the
//! host, in the caller, using the consensus reference library: parsing the
//! compact signature (r, s < n), rejecting r = 0, s = 0 and high-S, and
//! parsing/decompressing the public key (on-curve, x/y < p, hybrid parity).
//! Those rules are where implementations quietly disagree, so they are not
//! re-implemented here. The kernel receives only well-formed
//! `(r, s, z, Qx, Qy)` and does the arithmetic:
//!
//! ```text
//! w  = s^-1 mod n            (Fermat: s^(n-2), Montgomery form)
//! u1 = z·w mod n,  u2 = r·w mod n     (z reduced mod n first)
//! R  = u1·G + u2·Q           (GLV split into four 128-bit scalars, 4-way Straus)
//! ok = R ≠ ∞  and  x(R) ≡ r  or  x(R) ≡ r + n  (when r + n < p)
//! ```
//!
//! The x(R) test is done without an inversion, as `r·Z == X (mod p)` in
//! homogeneous coordinates — libsecp256k1's `secp256k1_ecdsa_sig_verify` check
//! (it uses Jacobian coordinates, hence `r·Z²` there).
//!
//! The kernel still re-checks the preconditions it relies on (r, s in
//! [1, n), Qx, Qy < p, Q on the curve) and returns `false` if any fails, so a
//! host-side bug can only cause a rejection, never an acceptance.
//!
//! # Arithmetic
//!
//! 256-bit values are eight little-endian u32 limbs. WGSL has no 64-bit
//! integers in baseline, so 32×32→64 products are built from 16-bit halves.
//! Field elements use a reduction specialized to p = 2^256 − 2^32 − 977;
//! scalars mod n use Montgomery multiplication (CIOS). Points
//! use the complete addition formula of Renes–Costello–Batina (2016), which
//! is correct for every input pair — P = Q, P = −Q, the identity — with no
//! branches, because Q is chosen by whoever submits the transaction.
//!
//! GPU output is never consensus truth on its own: `x3-accel` compares it
//! with the CPU and fails closed on any mismatch.

use crate::{WgpuAccelError, WgpuBackend};

/// Words per job in the data buffer: r, s, z, Qx, Qy, 8 limbs each.
const JOB_WORDS: usize = 40;
/// Verifications per dispatch. Each thread runs a full scalar multiplication,
/// so this bounds one submission's GPU time well below driver watchdogs.
const MAX_JOBS_PER_DISPATCH: usize = 65_536;

/// One well-formed verification, every field a 32-byte big-endian integer.
///
/// The caller must already have applied the reference library's parsing
/// rules (see the module docs); `x3-accel` does this with `secp256k1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Secp256k1Prepared {
    pub r: [u8; 32],
    pub s: [u8; 32],
    /// The 32-byte message hash, not yet reduced mod n.
    pub z: [u8; 32],
    pub qx: [u8; 32],
    pub qy: [u8; 32],
}

fn push_be_limbs(out: &mut [u8], value: &[u8; 32]) {
    // Limb i (little-endian order) is big-endian bytes [28-4i, 32-4i).
    for limb in 0..8 {
        let start = 28 - 4 * limb;
        let word = u32::from_be_bytes(value[start..start + 4].try_into().expect("4 bytes"));
        out[4 * limb..4 * limb + 4].copy_from_slice(&word.to_le_bytes());
    }
}

impl WgpuBackend {
    /// Verify prepared ECDSA jobs; `true` means the signature is valid.
    pub fn secp256k1_verify_prepared(
        &self,
        jobs: &[Secp256k1Prepared],
    ) -> Result<Vec<bool>, WgpuAccelError> {
        let mut results = Vec::with_capacity(jobs.len());
        for chunk in jobs.chunks(MAX_JOBS_PER_DISPATCH) {
            let bytes = self.execute(
                &self.secp256k1_kernels().0,
                chunk.len(),
                chunk.len() * JOB_WORDS,
                |entries, data| {
                    entries.fill(0);
                    for (index, job) in chunk.iter().enumerate() {
                        let base = index * JOB_WORDS * 4;
                        for (field, value) in
                            [job.r, job.s, job.z, job.qx, job.qy].iter().enumerate()
                        {
                            let at = base + field * 32;
                            push_be_limbs(&mut data[at..at + 32], value);
                        }
                    }
                    let used = chunk.len() * JOB_WORDS * 4;
                    data[used..].fill(0);
                },
            )?;
            results.extend(bytes.chunks_exact(4).map(|word| match word {
                [1, 0, 0, 0] => Ok(true),
                [0, 0, 0, 0] => Ok(false),
                // Anything else means the kernel did not run to completion for
                // this job; never read it as a verdict.
                _ => Err(WgpuAccelError::BufferMapFailed(
                    "secp256k1 kernel wrote a non-boolean result".into(),
                )),
            }));
        }
        results.into_iter().collect()
    }

    /// Field and scalar primitives, for testing the arithmetic in isolation.
    ///
    /// For each `(a, b)` (little-endian limbs, each already reduced below the
    /// modulus it is used with) returns
    /// `[a·b mod p, a·b mod n, a^-1 mod n]`, also as little-endian limbs.
    #[doc(hidden)]
    pub fn secp256k1_selftest(
        &self,
        cases: &[([u32; 8], [u32; 8])],
    ) -> Result<Vec<[[u32; 8]; 3]>, WgpuAccelError> {
        let bytes = self.execute(
            &self.secp256k1_kernels().1,
            cases.len(),
            cases.len() * 16,
            |entries, data| {
                entries.fill(0);
                for (index, (a, b)) in cases.iter().enumerate() {
                    for (limb, word) in a.iter().chain(b.iter()).enumerate() {
                        let at = (index * 16 + limb) * 4;
                        data[at..at + 4].copy_from_slice(&word.to_le_bytes());
                    }
                }
                let used = cases.len() * 16 * 4;
                data[used..].fill(0);
            },
        )?;
        let words: Vec<u32> = bytes
            .chunks_exact(4)
            .map(|w| u32::from_le_bytes(w.try_into().expect("4 bytes")))
            .collect();
        Ok(words
            .chunks_exact(24)
            .map(|case| {
                let mut out = [[0u32; 8]; 3];
                for (i, value) in out.iter_mut().enumerate() {
                    value.copy_from_slice(&case[i * 8..i * 8 + 8]);
                }
                out
            })
            .collect())
    }
}

pub(crate) const SECP256K1_WGSL: &str = r#"
@group(0) @binding(0) var<storage, read> jobs: array<u32>;
@group(0) @binding(1) var<storage, read> entries: array<u32>;
@group(0) @binding(2) var<storage, read_write> results: array<u32>;

alias U256 = array<u32, 8>;

const P: U256 = U256(0xfffffc2fu, 0xfffffffeu, 0xffffffffu, 0xffffffffu, 0xffffffffu, 0xffffffffu, 0xffffffffu, 0xffffffffu);
const N: U256 = U256(0xd0364141u, 0xbfd25e8cu, 0xaf48a03bu, 0xbaaedce6u, 0xfffffffeu, 0xffffffffu, 0xffffffffu, 0xffffffffu);
const N_MINV: u32 = 0x5588b13fu;
const N_R2: U256 = U256(0x67d7d140u, 0x896cf214u, 0x0e7cf878u, 0x741496c2u, 0x5bcd07c6u, 0xe697f5e4u, 0x81c69bc5u, 0x9d671cd5u);
const N_MINUS_2: U256 = U256(0xd036413fu, 0xbfd25e8cu, 0xaf48a03bu, 0xbaaedce6u, 0xfffffffeu, 0xffffffffu, 0xffffffffu, 0xffffffffu);
const P_MINUS_N: U256 = U256(0x2fc9baeeu, 0x402da172u, 0x50b75fc4u, 0x45512319u, 0x00000001u, 0x00000000u, 0x00000000u, 0x00000000u);
const GX: U256 = U256(0x16f81798u, 0x59f2815bu, 0x2dce28d9u, 0x029bfcdbu, 0xce870b07u, 0x55a06295u, 0xf9dcbbacu, 0x79be667eu);
const GY: U256 = U256(0xfb10d4b8u, 0x9c47d08fu, 0xa6855419u, 0xfd17b448u, 0x0e1108a8u, 0x5da4fbfcu, 0x26a3c465u, 0x483ada77u);
// GLV endomorphism: λ·(x, y) = (β·x, y), with libsecp256k1's lattice
// constants for splitting a scalar k = k1 + k2·λ with |k1|, |k2| < 2^128.
// The *_M constants are in Montgomery form mod n.
const BETA: U256 = U256(0x719501eeu, 0xc1396c28u, 0x12f58995u, 0x9cf04975u, 0xac3434e9u, 0x6e64479eu, 0x657c0710u, 0x7ae96a2bu);
const LAMBDA_GX: U256 = U256(0x00b88fcbu, 0xa7bba044u, 0x7f15e98du, 0x87284406u, 0x96902325u, 0xab0102b6u, 0x9da01887u, 0xbcace2e9u);
const GLV_G1: U256 = U256(0x45dbb031u, 0xe893209au, 0x71e8ca7fu, 0x3daa8a14u, 0x9284eb15u, 0xe86c90e4u, 0xa7d46bcdu, 0x3086d221u);
const GLV_G2: U256 = U256(0x8ac47f71u, 0x1571b4aeu, 0x9df506c6u, 0x221208acu, 0x0abfe4c4u, 0x6f547fa9u, 0x010e8828u, 0xe4437ed6u);
const GLV_MINUS_B1_M: U256 = U256(0x0ad9263cu, 0xc50468d0u, 0xfaa6ed42u, 0x1b1c8205u, 0x8ac47f71u, 0x1571b4aeu, 0x9df506c6u, 0x221208acu);
const GLV_MINUS_B2_M: U256 = U256(0x6a144696u, 0x0cac5e50u, 0xf3ba5939u, 0x1e8a8dc5u, 0xba244fceu, 0x176cdf65u, 0x8e173580u, 0xc25575ebu);
const GLV_MINUS_LAMBDA_M: U256 = U256(0x06a3d4a3u, 0xcf54734fu, 0x2b820beeu, 0x8e1af539u, 0xad96826du, 0x8c5699f9u, 0x7aa729c6u, 0xacd7bfe8u);
const HALF_N: U256 = U256(0x681b20a0u, 0xdfe92f46u, 0x57a4501du, 0x5d576e73u, 0xffffffffu, 0xffffffffu, 0xffffffffu, 0x7fffffffu);
const ONE: U256 = U256(1u, 0u, 0u, 0u, 0u, 0u, 0u, 0u);
const ZERO: U256 = U256(0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u);
const SEVEN: U256 = U256(7u, 0u, 0u, 0u, 0u, 0u, 0u, 0u);

struct Carry {
    v: U256,
    c: u32,
}

// Homogeneous projective point (x = X/Z, y = Y/Z), coordinates mod p in normal form.
// The identity is (0 : 1 : 0).
struct Pt {
    x: U256,
    y: U256,
    z: U256,
}

// Full 64-bit product of two u32 as (lo, hi), from 16-bit halves.
fn mul32(a: u32, b: u32) -> vec2<u32> {
    let a0 = a & 0xffffu;
    let a1 = a >> 16u;
    let b0 = b & 0xffffu;
    let b1 = b >> 16u;
    let p00 = a0 * b0;
    let p01 = a0 * b1;
    let p10 = a1 * b0;
    let p11 = a1 * b1;
    let mid = (p00 >> 16u) + (p01 & 0xffffu) + (p10 & 0xffffu);
    let lo = (p00 & 0xffffu) | (mid << 16u);
    let hi = p11 + (p01 >> 16u) + (p10 >> 16u) + (mid >> 16u);
    return vec2<u32>(lo, hi);
}

fn is_zero(a_in: U256) -> bool {
    var a = a_in;
    var acc = 0u;
    for (var i = 0u; i < 8u; i = i + 1u) {
        acc = acc | a[i];
    }
    return acc == 0u;
}

fn eq(a_in: U256, b_in: U256) -> bool {
    var a = a_in;
    var b = b_in;
    var acc = 0u;
    for (var i = 0u; i < 8u; i = i + 1u) {
        acc = acc | (a[i] ^ b[i]);
    }
    return acc == 0u;
}

// a >= b
fn geq(a_in: U256, b_in: U256) -> bool {
    var a = a_in;
    var b = b_in;
    for (var i = 7i; i >= 0i; i = i - 1i) {
        if (a[i] != b[i]) {
            return a[i] > b[i];
        }
    }
    return true;
}

fn add_c(a_in: U256, b_in: U256) -> Carry {
    var a = a_in;
    var b = b_in;
    var r: U256;
    var c = 0u;
    for (var i = 0u; i < 8u; i = i + 1u) {
        let s = a[i] + b[i];
        let c1 = select(0u, 1u, s < a[i]);
        let s2 = s + c;
        let c2 = select(0u, 1u, s2 < s);
        r[i] = s2;
        c = c1 + c2;
    }
    return Carry(r, c);
}

fn sub_b(a_in: U256, b_in: U256) -> Carry {
    var a = a_in;
    var b = b_in;
    var r: U256;
    var borrow = 0u;
    for (var i = 0u; i < 8u; i = i + 1u) {
        let d = a[i] - b[i];
        let b1 = select(0u, 1u, a[i] < b[i]);
        let d2 = d - borrow;
        let b2 = select(0u, 1u, d < borrow);
        r[i] = d2;
        borrow = b1 | b2;
    }
    return Carry(r, borrow);
}

// (a + b) mod m for a, b < m.
fn add_mod(a: U256, b: U256, m: U256) -> U256 {
    let s = add_c(a, b);
    if (s.c != 0u || geq(s.v, m)) {
        return sub_b(s.v, m).v;
    }
    return s.v;
}

// (a - b) mod m for a, b < m.
fn sub_mod(a: U256, b: U256, m: U256) -> U256 {
    let d = sub_b(a, b);
    if (d.c != 0u) {
        return add_c(d.v, m).v;
    }
    return d.v;
}

// Montgomery product a·b·2^-256 mod m (CIOS), for a, b < m and odd m.
// minv = -m^-1 mod 2^32. The result is fully reduced (< m).
fn mont_mul(a_in: U256, b_in: U256, m_in: U256, minv: u32) -> U256 {
    // Naga only allows constant indices into by-value array parameters.
    var a = a_in;
    var b = b_in;
    var m = m_in;
    var t: array<u32, 10>;
    for (var i = 0u; i < 8u; i = i + 1u) {
        var c = 0u;
        for (var j = 0u; j < 8u; j = j + 1u) {
            let pr = mul32(a[j], b[i]);
            let s1 = t[j] + pr.x;
            let c1 = select(0u, 1u, s1 < pr.x);
            let s2 = s1 + c;
            let c2 = select(0u, 1u, s2 < c);
            t[j] = s2;
            c = pr.y + c1 + c2;
        }
        let s8 = t[8] + c;
        t[9] = select(0u, 1u, s8 < c);
        t[8] = s8;

        let q = t[0] * minv;
        let p0 = mul32(q, m[0]);
        let r0 = t[0] + p0.x;
        c = p0.y + select(0u, 1u, r0 < p0.x);
        for (var j = 1u; j < 8u; j = j + 1u) {
            let pr = mul32(q, m[j]);
            let s1 = t[j] + pr.x;
            let c1 = select(0u, 1u, s1 < pr.x);
            let s2 = s1 + c;
            let c2 = select(0u, 1u, s2 < c);
            t[j - 1u] = s2;
            c = pr.y + c1 + c2;
        }
        let s7 = t[8] + c;
        t[7] = s7;
        t[8] = t[9] + select(0u, 1u, s7 < c);
    }
    var r: U256;
    for (var i = 0u; i < 8u; i = i + 1u) {
        r[i] = t[i];
    }
    if (t[8] != 0u || geq(r, m)) {
        return sub_b(r, m).v;
    }
    return r;
}

// a·b mod p for a, b < p, in normal form. p = 2^256 - C with C = 2^32 + 977,
// so 2^256 ≡ C: the high half H of the 512-bit product folds back in as
// H·977 + H·2^32. Half the 32x32 products of a Montgomery multiplication.
fn fmul(a_in: U256, b_in: U256) -> U256 {
    var a = a_in;
    var b = b_in;
    var t: array<u32, 16>;
    for (var i = 0u; i < 8u; i = i + 1u) {
        var c = 0u;
        for (var j = 0u; j < 8u; j = j + 1u) {
            let pr = mul32(a[i], b[j]);
            let s1 = t[i + j] + pr.x;
            let c1 = select(0u, 1u, s1 < pr.x);
            let s2 = s1 + c;
            let c2 = select(0u, 1u, s2 < c);
            t[i + j] = s2;
            c = pr.y + c1 + c2;
        }
        t[i + 8u] = c;
    }

    // u = L + H·977 + (H << 32): below 2^289, ten limbs. Straight-line with
    // constant indices so every driver keeps it in registers (a loop with
    // branches here spilled to local memory on Pascal).
    var u: array<u32, 10>;
    var carry = 0u;
    var sum = 0u;
    var hi = 0u;
    var x = 0u;
    let h0 = mul32(t[8], 977u);
    let h1 = mul32(t[9], 977u);
    let h2 = mul32(t[10], 977u);
    let h3 = mul32(t[11], 977u);
    let h4 = mul32(t[12], 977u);
    let h5 = mul32(t[13], 977u);
    let h6 = mul32(t[14], 977u);
    let h7 = mul32(t[15], 977u);
    sum = carry;
    hi = 0u;
    x = t[0];
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    x = h0.x;
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    u[0] = sum;
    carry = hi;
    sum = carry;
    hi = 0u;
    x = t[1];
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    x = h1.x;
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    x = h0.y;
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    x = t[8];
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    u[1] = sum;
    carry = hi;
    sum = carry;
    hi = 0u;
    x = t[2];
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    x = h2.x;
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    x = h1.y;
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    x = t[9];
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    u[2] = sum;
    carry = hi;
    sum = carry;
    hi = 0u;
    x = t[3];
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    x = h3.x;
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    x = h2.y;
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    x = t[10];
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    u[3] = sum;
    carry = hi;
    sum = carry;
    hi = 0u;
    x = t[4];
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    x = h4.x;
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    x = h3.y;
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    x = t[11];
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    u[4] = sum;
    carry = hi;
    sum = carry;
    hi = 0u;
    x = t[5];
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    x = h5.x;
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    x = h4.y;
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    x = t[12];
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    u[5] = sum;
    carry = hi;
    sum = carry;
    hi = 0u;
    x = t[6];
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    x = h6.x;
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    x = h5.y;
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    x = t[13];
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    u[6] = sum;
    carry = hi;
    sum = carry;
    hi = 0u;
    x = t[7];
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    x = h7.x;
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    x = h6.y;
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    x = t[14];
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    u[7] = sum;
    carry = hi;
    sum = carry;
    hi = 0u;
    x = h7.y;
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    x = t[15];
    sum = sum + x;
    hi = hi + select(0u, 1u, sum < x);
    u[8] = sum;
    carry = hi;
    sum = carry;
    hi = 0u;
    u[9] = sum;
    carry = hi;

    // Fold the top again: T = u[8] + u[9]·2^32 < 2^33, T·C = T·977 + T·2^32.
    let t977_lo = mul32(u[8], 977u);
    let t977_hi = mul32(u[9], 977u);
    // T·977 as three limbs.
    let m0 = t977_lo.x;
    let m1a = t977_lo.y + t977_hi.x;
    let m1c = select(0u, 1u, m1a < t977_lo.y);
    let m2 = t977_hi.y + m1c;
    // Addend limbs: T·977 + (T << 32) = [m0, m1a + u8, m2 + u9 (+carry)].
    var add: U256;
    add[0] = m0;
    let a1 = m1a + u[8];
    add[1] = a1;
    add[2] = m2 + u[9] + select(0u, 1u, a1 < u[8]);
    var r: U256;
    for (var k = 0u; k < 8u; k = k + 1u) {
        r[k] = u[k];
    }
    var folded = add_c(r, add);
    // A carry out of 2^256 is worth C more; the value is now tiny, so once suffices.
    if (folded.c != 0u) {
        folded = add_c(folded.v, U256(977u, 1u, 0u, 0u, 0u, 0u, 0u, 0u));
    }
    if (geq(folded.v, P)) {
        return sub_b(folded.v, P).v;
    }
    return folded.v;
}

fn fsqr(a: U256) -> U256 {
    return fmul(a, a);
}

fn fadd(a: U256, b: U256) -> U256 {
    return add_mod(a, b, P);
}

fn fsub(a: U256, b: U256) -> U256 {
    return sub_mod(a, b, P);
}


// Montgomery form of a^-1 mod n, given a in Montgomery form (a ≠ 0).
fn inv_mont_n(a_m: U256) -> U256 {
    var exponent = N_MINUS_2;
    var acc = mont_mul(ONE, N_R2, N, N_MINV);
    for (var bit = 255i; bit >= 0i; bit = bit - 1i) {
        acc = mont_mul(acc, acc, N, N_MINV);
        if (((exponent[u32(bit) / 32u] >> (u32(bit) % 32u)) & 1u) == 1u) {
            acc = mont_mul(acc, a_m, N, N_MINV);
        }
    }
    return acc;
}

// 21·a = 3·b·a with b = 7, by additions.
fn mul_b3(a: U256) -> U256 {
    let a2 = fadd(a, a);
    let a4 = fadd(a2, a2);
    let a16 = fadd(fadd(a4, a4), fadd(a4, a4));
    return fadd(fadd(a16, a4), a);
}

// Complete addition for a = 0 curves (Renes–Costello–Batina 2016, Alg. 7).
// One formula, no branches, correct for P + Q, P + P, P + (−P) and the
// identity — so there are no special cases left to get wrong, and the scalar
// multiplication has a single call site (small code, fast driver compile).
fn cadd(p: Pt, q: Pt) -> Pt {
    var t0 = fmul(p.x, q.x);
    var t1 = fmul(p.y, q.y);
    var t2 = fmul(p.z, q.z);
    var t3 = fmul(fadd(p.x, p.y), fadd(q.x, q.y));
    var t4 = fadd(t0, t1);
    t3 = fsub(t3, t4);
    t4 = fmul(fadd(p.y, p.z), fadd(q.y, q.z));
    var x3 = fadd(t1, t2);
    t4 = fsub(t4, x3);
    x3 = fmul(fadd(p.x, p.z), fadd(q.x, q.z));
    var y3 = fadd(t0, t2);
    y3 = fsub(x3, y3);
    x3 = fadd(t0, t0);
    t0 = fadd(x3, t0);
    t2 = mul_b3(t2);
    var z3 = fadd(t1, t2);
    t1 = fsub(t1, t2);
    y3 = mul_b3(y3);
    x3 = fmul(t4, y3);
    t2 = fmul(t3, t1);
    x3 = fsub(t2, x3);
    y3 = fmul(y3, t0);
    t1 = fmul(t1, z3);
    y3 = fadd(t1, y3);
    t0 = fmul(t0, t3);
    z3 = fmul(z3, t4);
    z3 = fadd(z3, t0);
    return Pt(x3, y3, z3);
}

// Complete doubling for a = 0 (Renes–Costello–Batina 2016, Alg. 9): 6M + 2S.
// The identity (0 : 1 : 0) maps to itself.
fn cdbl(p: Pt) -> Pt {
    var t0 = fsqr(p.y);
    var z3 = fadd(t0, t0);
    z3 = fadd(z3, z3);
    z3 = fadd(z3, z3);
    var t1 = fmul(p.y, p.z);
    var t2 = fsqr(p.z);
    t2 = mul_b3(t2);
    var x3 = fmul(t2, z3);
    var y3 = fadd(t0, t2);
    z3 = fmul(t1, z3);
    t1 = fadd(t2, t2);
    t2 = fadd(t1, t2);
    t0 = fsub(t0, t2);
    y3 = fmul(t0, y3);
    y3 = fadd(x3, y3);
    t1 = fmul(p.x, p.y);
    x3 = fmul(t0, t1);
    x3 = fadd(x3, x3);
    return Pt(x3, y3, z3);
}

// round(k·g / 2^384) for k < 2^256: floor of the 512-bit product's top
// 128 bits plus its bit 383. Below 2^129.
fn mul_shift_384(k_in: U256, g_in: U256) -> U256 {
    var k = k_in;
    var g = g_in;
    var t: array<u32, 16>;
    for (var i = 0u; i < 8u; i = i + 1u) {
        var c = 0u;
        for (var j = 0u; j < 8u; j = j + 1u) {
            let pr = mul32(k[i], g[j]);
            let s1 = t[i + j] + pr.x;
            let c1 = select(0u, 1u, s1 < pr.x);
            let s2 = s1 + c;
            let c2 = select(0u, 1u, s2 < c);
            t[i + j] = s2;
            c = pr.y + c1 + c2;
        }
        t[i + 8u] = c;
    }
    let top = U256(t[12], t[13], t[14], t[15], 0u, 0u, 0u, 0u);
    return add_c(top, U256(t[11] >> 31u, 0u, 0u, 0u, 0u, 0u, 0u, 0u)).v;
}

// One half of a GLV split, as a magnitude below 2^128 and a sign.
struct Half {
    v: U256,
    neg: bool,
}

fn signed_half(r: U256) -> Half {
    // r > n/2 stands for the negative value r - n.
    if (geq(r, HALF_N) && !eq(r, HALF_N)) {
        return Half(sub_b(N, r).v, true);
    }
    return Half(r, false);
}

fn negate_y(p: Pt, neg: bool) -> Pt {
    if (neg) {
        return Pt(p.x, fsub(ZERO, p.y), p.z);
    }
    return p;
}

fn load(base: u32) -> U256 {
    var v: U256;
    for (var i = 0u; i < 8u; i = i + 1u) {
        v[i] = jobs[base + i];
    }
    return v;
}

fn verify(r: U256, s: U256, z_raw: U256, qx: U256, qy: U256) -> bool {
    // Preconditions the host already enforced; re-checked so a host bug can
    // only reject.
    if (is_zero(r) || is_zero(s) || geq(r, N) || geq(s, N) || geq(qx, P) || geq(qy, P)) {
        return false;
    }
    // y^2 == x^3 + 7
    if (!eq(fsqr(qy), fadd(fmul(fsqr(qx), qx), SEVEN))) {
        return false;
    }

    var z = z_raw;
    if (geq(z, N)) {
        z = sub_b(z, N).v;
    }
    let w_m = inv_mont_n(mont_mul(s, N_R2, N, N_MINV));
    // Normal × Montgomery = normal form.
    var u1 = mont_mul(z, w_m, N, N_MINV);
    var u2 = mont_mul(r, w_m, N, N_MINV);

    // GLV: u = k1 + k2·λ (mod n), so u1·G + u2·Q is a sum of four ~128-bit
    // multiples: k1·G + k2·(λG) + k3·Q + k4·(λQ), with λ·(x, y) = (β·x, y).
    // Negative halves negate their point instead.
    var scalars: array<U256, 4>;
    var points: array<Pt, 4>;
    let bases = array<Pt, 2>(Pt(GX, GY, ONE), Pt(qx, qy, ONE));
    let lambda_bases = array<Pt, 2>(Pt(LAMBDA_GX, GY, ONE), Pt(fmul(BETA, qx), qy, ONE));
    var us = array<U256, 2>(u1, u2);
    for (var which = 0u; which < 2u; which = which + 1u) {
        let k = us[which];
        let c1 = mul_shift_384(k, GLV_G1);
        let c2 = mul_shift_384(k, GLV_G2);
        let r2 = add_mod(mont_mul(c1, GLV_MINUS_B1_M, N, N_MINV), mont_mul(c2, GLV_MINUS_B2_M, N, N_MINV), N);
        let r1 = add_mod(k, mont_mul(r2, GLV_MINUS_LAMBDA_M, N, N_MINV), N);
        let h1 = signed_half(r1);
        let h2 = signed_half(r2);
        scalars[2u * which] = h1.v;
        scalars[2u * which + 1u] = h2.v;
        var b = bases;
        var lb = lambda_bases;
        points[2u * which] = negate_y(b[which], h1.neg);
        points[2u * which + 1u] = negate_y(lb[which], h2.neg);
    }

    // 4-way Straus, 1-bit window: table[m] = sum of points[i] for bits i of m.
    let identity = Pt(ZERO, ONE, ZERO);
    var table: array<Pt, 16>;
    table[0] = identity;
    for (var m = 1u; m < 16u; m = m + 1u) {
        table[m] = cadd(table[m & (m - 1u)], points[countTrailingZeros(m)]);
    }
    // 128 rounds of one doubling and one addition (the identity for a zero
    // digit), half the doublings of the unsplit scalars.
    var acc = identity;
    for (var bit = 127i; bit >= 0i; bit = bit - 1i) {
        acc = cdbl(acc);
        let word = u32(bit) / 32u;
        let shift = u32(bit) % 32u;
        var digit = 0u;
        for (var i = 0u; i < 4u; i = i + 1u) {
            digit = digit | (((scalars[i][word] >> shift) & 1u) << i);
        }
        acc = cadd(acc, table[digit]);
    }
    if (is_zero(acc.z)) {
        return false;
    }
    // x(R) = X/Z; compare r·Z with X instead of inverting.
    if (eq(fmul(r, acc.z), acc.x)) {
        return true;
    }
    // x(R) in [n, p) reduces to r when x(R) = r + n; possible only if r < p - n.
    if (!geq(r, P_MINUS_N)) {
        let rn = add_c(r, N).v;
        return eq(fmul(rn, acc.z), acc.x);
    }
    return false;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(num_workgroups) groups: vec3<u32>) {
    let idx = global_id.x + global_id.y * groups.x * 64u;
    if (idx >= entries[0]) {
        return;
    }
    let base = idx * 40u;
    let ok = verify(load(base), load(base + 8u), load(base + 16u), load(base + 24u), load(base + 32u));
    results[idx] = select(0u, 1u, ok);
}

@compute @workgroup_size(64)
fn selftest(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(num_workgroups) groups: vec3<u32>) {
    let idx = global_id.x + global_id.y * groups.x * 64u;
    if (idx >= entries[0]) {
        return;
    }
    let a = load(idx * 16u);
    let b = load(idx * 16u + 8u);
    // a·b mod p (specialized reduction) and mod n (Montgomery: mont(a·R², b) = a·b).
    var ab_p = fmul(a, b);
    var ab_n = mont_mul(mont_mul(a, N_R2, N, N_MINV), b, N, N_MINV);
    // a^-1 mod n back in normal form.
    var inv_n = mont_mul(inv_mont_n(mont_mul(a, N_R2, N, N_MINV)), ONE, N, N_MINV);
    let out = idx * 24u;
    for (var i = 0u; i < 8u; i = i + 1u) {
        results[out + i] = ab_p[i];
        results[out + 8u + i] = ab_n[i];
        results[out + 16u + i] = inv_n[i];
    }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    type U = [u32; 8];
    const P: U = [
        0xfffffc2f, 0xfffffffe, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff,
        0xffffffff,
    ];
    const N: U = [
        0xd0364141, 0xbfd25e8c, 0xaf48a03b, 0xbaaedce6, 0xfffffffe, 0xffffffff, 0xffffffff,
        0xffffffff,
    ];

    // Deliberately naive reference arithmetic: obviously correct, slow.
    fn geq(a: &U, b: &U) -> bool {
        for i in (0..8).rev() {
            if a[i] != b[i] {
                return a[i] > b[i];
            }
        }
        true
    }

    fn sub(a: &U, b: &U) -> U {
        let mut r = [0u32; 8];
        let mut borrow = 0i64;
        for i in 0..8 {
            let d = i64::from(a[i]) - i64::from(b[i]) - borrow;
            r[i] = d.rem_euclid(1 << 32) as u32;
            borrow = i64::from(d < 0);
        }
        r
    }

    fn add_mod(a: &U, b: &U, m: &U) -> U {
        let mut r = [0u32; 8];
        let mut carry = 0u64;
        for i in 0..8 {
            let s = u64::from(a[i]) + u64::from(b[i]) + carry;
            r[i] = s as u32;
            carry = s >> 32;
        }
        if carry == 1 || geq(&r, m) {
            sub(&r, m)
        } else {
            r
        }
    }

    fn mul_mod(a: &U, b: &U, m: &U) -> U {
        let mut r = [0u32; 8];
        for bit in (0..256).rev() {
            r = add_mod(&r, &r, m);
            if (b[bit / 32] >> (bit % 32)) & 1 == 1 {
                r = add_mod(&r, a, m);
            }
        }
        r
    }

    fn cases() -> Vec<(U, U)> {
        let mut state = 0x9e37_79b9_7f4a_7c15u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u32
        };
        // Below n (and so below p): valid for both moduli.
        let below_n = |mut v: U| {
            while geq(&v, &N) {
                v = sub(&v, &N);
            }
            v
        };
        let n_minus = |k: u32| sub(&N, &[k, 0, 0, 0, 0, 0, 0, 0]);
        let mut edges: Vec<U> = vec![
            [0; 8],
            [1, 0, 0, 0, 0, 0, 0, 0],
            [2, 0, 0, 0, 0, 0, 0, 0],
            n_minus(1),
            n_minus(2),
            [0, 0, 0, 0, 0, 0, 0, 0x8000_0000],
            [u32::MAX, u32::MAX, u32::MAX, u32::MAX, 0, 0, 0, 0],
            [0, 0, 0, 0, u32::MAX, u32::MAX, u32::MAX, 0x7fff_ffff],
        ];
        for _ in 0..24 {
            edges.push(below_n(std::array::from_fn(|_| next())));
        }
        // Field-only edges in [n, p): the specialized reduction's corner cases.
        let p_minus = |k: u32| sub(&P, &[k, 0, 0, 0, 0, 0, 0, 0]);
        edges.extend([
            p_minus(1),
            p_minus(2),
            p_minus(977),
            sub(&P, &[0, 1, 0, 0, 0, 0, 0, 0]),
            N,
            [
                u32::MAX,
                u32::MAX,
                u32::MAX,
                u32::MAX,
                u32::MAX,
                u32::MAX,
                u32::MAX,
                0xfffffffe,
            ],
        ]);
        let mut out = Vec::new();
        for (i, a) in edges.iter().enumerate() {
            out.push((*a, edges[(i * 7 + 3) % edges.len()]));
            out.push((*a, *a));
            out.push((*a, p_minus(1)));
        }
        out
    }

    #[test]
    fn gpu_field_and_scalar_primitives_match_reference() {
        let Ok(backend) = WgpuBackend::initialize() else {
            assert!(
                std::env::var("X3_REQUIRE_GPU").as_deref() != Ok("1"),
                "X3_REQUIRE_GPU=1 but no GPU adapter"
            );
            eprintln!("secp256k1 primitives: SKIPPED, no GPU adapter");
            return;
        };
        let cases = cases();
        let gpu = backend.secp256k1_selftest(&cases).unwrap();
        for ((a, b), [ab_p, ab_n, inv_n]) in cases.iter().zip(gpu) {
            assert_eq!(ab_p, mul_mod(a, b, &P), "a*b mod p, a={a:08x?} b={b:08x?}");
            // Mod-n results are only defined for inputs below n.
            if geq(a, &N) || geq(b, &N) {
                continue;
            }
            assert_eq!(ab_n, mul_mod(a, b, &N), "a*b mod n, a={a:08x?} b={b:08x?}");
            // The inverse is unique, so a·a^-1 ≡ 1 pins it exactly (0 maps to 0^(n-2) = 0).
            if *a == [0; 8] {
                assert_eq!(inv_n, [0; 8]);
            } else {
                assert_eq!(
                    mul_mod(a, &inv_n, &N),
                    [1, 0, 0, 0, 0, 0, 0, 0],
                    "a^-1 mod n, a={a:08x?}"
                );
            }
        }
    }
}
