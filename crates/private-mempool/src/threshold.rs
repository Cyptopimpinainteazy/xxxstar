//! Real Shamir secret sharing and Lagrange interpolation over the Ristretto
//! group, used to combine validators' decryption shares without any single
//! validator ever holding (or reconstructing) the committee's secret key.
//!
//! # Why Ristretto, not X25519
//!
//! `x25519-dalek`'s `StaticSecret` clamps its scalar bytes (clearing/setting
//! specific bits) before use. Clamping is not linear: if `a` and `b` are
//! clamped scalars, `a + b` is generally not the clamped scalar you'd get by
//! splitting a third clamped scalar via Shamir. That breaks the exact
//! property threshold decryption depends on — "any `t` shares combine to the
//! same secret regardless of which `t` you pick" — so the committee secret
//! and all per-validator shares here are plain Ristretto scalars (mod the
//! group order `L`), which *are* linear, and group elements are Ristretto
//! points rather than X25519 Montgomery-form public keys.
//!
//! # The scheme
//!
//! - The committee's secret is a scalar `s`; its public key is `s * G`
//!   (`G` the Ristretto basepoint).
//! - [`split_secret`] hands out `n` shares `(i, f(i))` of a degree-`(t-1)`
//!   random polynomial `f` with `f(0) = s`, so any `t` of them determine `s`.
//! - To decrypt, a validator holding share `(i, s_i)` never reconstructs `s`.
//!   It computes a *partial* decryption `s_i * ephemeral_pk` — a point, not
//!   a scalar — and only `t` of those partials are combined, via
//!   [`combine_points`], into `s * ephemeral_pk`: the same ECDH point the
//!   sender computed as `ephemeral_sk * (s * G)`. Fewer than `t` distinct
//!   validators can never produce that point.

use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::traits::Identity;
use rand::rngs::OsRng;

/// One committee member's share of a secret scalar.
///
/// Indices start at 1 — index 0 is reserved for the secret itself (the
/// polynomial's value at `x = 0`), so it can never collide with a real
/// share.
#[derive(Debug, Clone, Copy)]
pub struct SecretShare {
    pub index: u32,
    pub scalar: Scalar,
}

/// Split `secret` into `total` Shamir shares such that any `threshold` of
/// them reconstruct it (via [`lagrange_coefficient`] interpolation at
/// `x = 0`), and fewer than `threshold` reveal nothing about it.
///
/// # Panics
/// If `threshold` is 0 or exceeds `total`.
pub fn split_secret(secret: Scalar, threshold: u32, total: u32) -> Vec<SecretShare> {
    assert!(threshold >= 1, "threshold must be at least 1");
    assert!(
        threshold <= total,
        "threshold ({threshold}) cannot exceed the committee size ({total})"
    );

    let mut coefficients = Vec::with_capacity(threshold as usize);
    coefficients.push(secret);
    for _ in 1..threshold {
        coefficients.push(Scalar::random(&mut OsRng));
    }

    (1..=total)
        .map(|index| SecretShare {
            index,
            scalar: evaluate_polynomial(&coefficients, index),
        })
        .collect()
}

/// Evaluate a polynomial (lowest-degree coefficient first) at `x`.
///
/// Exposed so callers implementing their own dealer-side secret sharing
/// (e.g. a distributed key generation ceremony run by another crate) can
/// reuse the exact same arithmetic that [`split_secret`] and share
/// verification rely on.
pub fn evaluate_polynomial(coefficients: &[Scalar], x: u32) -> Scalar {
    let x = Scalar::from(x as u64);
    let mut result = Scalar::ZERO;
    let mut x_power = Scalar::ONE;
    for coefficient in coefficients {
        result += coefficient * x_power;
        x_power *= x;
    }
    result
}

/// The Lagrange basis coefficient `λ_i(0)` for interpolating the value at
/// `x = 0` from the points at `indices`, evaluated for index `i`.
///
/// `λ_i(0) = Π_{j ≠ i} (0 - x_j) / (x_i - x_j)`. Every index in `indices`
/// must be distinct and nonzero, and `i` must be one of them.
pub fn lagrange_coefficient(indices: &[u32], i: u32) -> Scalar {
    let xi = Scalar::from(i as u64);
    let mut coefficient = Scalar::ONE;
    for &j in indices {
        if j == i {
            continue;
        }
        let xj = Scalar::from(j as u64);
        let numerator = -xj; // 0 - x_j
        let denominator = xi - xj;
        coefficient *= numerator * denominator.invert();
    }
    coefficient
}

/// Combine `threshold`-or-more `(index, partial_point)` pairs into the
/// single point that Lagrange interpolation at `x = 0` implies, **without**
/// ever materializing the underlying scalar. This is what lets validators
/// jointly compute `secret * ephemeral_pk` while no individual validator
/// (nor the combiner) ever learns `secret`.
pub fn combine_points(shares: &[(u32, RistrettoPoint)]) -> RistrettoPoint {
    let indices: Vec<u32> = shares.iter().map(|(index, _)| *index).collect();
    shares
        .iter()
        .fold(RistrettoPoint::identity(), |acc, (index, point)| {
            acc + lagrange_coefficient(&indices, *index) * point
        })
}

/// Combine `threshold`-or-more `(index, scalar_share)` pairs directly into
/// the secret they share. Real threshold-decryption validators never call
/// this — see [`combine_points`] — but it is the ground truth [`split_secret`]
/// promises, so tests use it to check the scheme end-to-end.
pub fn combine_scalars(shares: &[(u32, Scalar)]) -> Scalar {
    let indices: Vec<u32> = shares.iter().map(|(index, _)| *index).collect();
    shares.iter().fold(Scalar::ZERO, |acc, (index, scalar)| {
        acc + lagrange_coefficient(&indices, *index) * scalar
    })
}

/// The Ristretto group public key `secret * G`, compressed to 32 bytes.
pub fn group_public_key(secret: &Scalar) -> [u8; 32] {
    (secret * RISTRETTO_BASEPOINT_POINT).compress().to_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn any_threshold_subset_reconstructs_the_same_secret() {
        let secret = Scalar::random(&mut OsRng);
        let shares = split_secret(secret, 3, 5);

        let subset_a: Vec<(u32, Scalar)> =
            shares[0..3].iter().map(|s| (s.index, s.scalar)).collect();
        let subset_b: Vec<(u32, Scalar)> =
            shares[2..5].iter().map(|s| (s.index, s.scalar)).collect();
        let subset_c: Vec<(u32, Scalar)> = vec![
            (shares[0].index, shares[0].scalar),
            (shares[2].index, shares[2].scalar),
            (shares[4].index, shares[4].scalar),
        ];

        assert_eq!(combine_scalars(&subset_a), secret);
        assert_eq!(combine_scalars(&subset_b), secret);
        assert_eq!(combine_scalars(&subset_c), secret);
    }

    #[test]
    fn below_threshold_does_not_reconstruct_the_secret() {
        let secret = Scalar::random(&mut OsRng);
        let shares = split_secret(secret, 3, 5);

        let too_few: Vec<(u32, Scalar)> =
            shares[0..2].iter().map(|s| (s.index, s.scalar)).collect();

        // combine_scalars has no way to know it was under-supplied — it just
        // interpolates a degree-1 curve through 2 points, which is generally
        // not the degree-2 polynomial's actual value at 0. Assert that this
        // wrong answer is in fact wrong (astronomically unlikely to collide).
        assert_ne!(combine_scalars(&too_few), secret);
    }

    #[test]
    fn combining_points_matches_combining_scalars_times_basepoint() {
        let secret = Scalar::random(&mut OsRng);
        let shares = split_secret(secret, 3, 4);
        let ephemeral = Scalar::random(&mut OsRng);
        let ephemeral_point = ephemeral * RISTRETTO_BASEPOINT_POINT;

        let partials: Vec<(u32, RistrettoPoint)> = shares[..3]
            .iter()
            .map(|s| (s.index, s.scalar * ephemeral_point))
            .collect();

        let combined = combine_points(&partials);
        let expected = secret * ephemeral_point;
        assert_eq!(combined.compress(), expected.compress());
    }

    #[test]
    fn different_subsets_of_partial_points_agree() {
        let secret = Scalar::random(&mut OsRng);
        let shares = split_secret(secret, 3, 5);
        let ephemeral = Scalar::random(&mut OsRng);
        let ephemeral_point = ephemeral * RISTRETTO_BASEPOINT_POINT;

        let partial_at = |i: usize| (shares[i].index, shares[i].scalar * ephemeral_point);

        let subset_a = vec![partial_at(0), partial_at(1), partial_at(2)];
        let subset_b = vec![partial_at(1), partial_at(3), partial_at(4)];

        assert_eq!(
            combine_points(&subset_a).compress(),
            combine_points(&subset_b).compress()
        );
    }

    #[test]
    fn group_public_key_matches_split_secret_constant_term() {
        let secret = Scalar::random(&mut OsRng);
        let shares = split_secret(secret, 2, 3);
        let reconstructed = combine_scalars(&[
            (shares[0].index, shares[0].scalar),
            (shares[1].index, shares[1].scalar),
        ]);
        assert_eq!(group_public_key(&reconstructed), group_public_key(&secret));
    }
}
