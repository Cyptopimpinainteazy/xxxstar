//! The 256-bit unsigned arithmetic difficulty needs: targets, work and the retarget's scaling.

use core::cmp::Ordering;

/// A 256-bit unsigned integer, four little-endian 64-bit limbs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct U256([u64; 4]);

impl U256 {
    pub const ZERO: Self = Self([0; 4]);
    pub const ONE: Self = Self([1, 0, 0, 0]);
    pub const MAX: Self = Self([u64::MAX; 4]);

    pub const fn from_u64(value: u64) -> Self {
        Self([value, 0, 0, 0])
    }

    /// Read 32 bytes as a little-endian number (how Bitcoin reads a hash against a target).
    pub fn from_le_bytes(bytes: &[u8; 32]) -> Self {
        let mut limbs = [0u64; 4];
        for (i, limb) in limbs.iter_mut().enumerate() {
            let mut word = [0u8; 8];
            word.copy_from_slice(&bytes[i * 8..i * 8 + 8]);
            *limb = u64::from_le_bytes(word);
        }
        Self(limbs)
    }

    pub fn to_le_bytes(&self) -> [u8; 32] {
        let mut out = [0u8; 32];
        for (i, limb) in self.0.iter().enumerate() {
            out[i * 8..i * 8 + 8].copy_from_slice(&limb.to_le_bytes());
        }
        out
    }

    pub fn is_zero(&self) -> bool {
        self.0 == [0; 4]
    }

    pub fn low_u64(&self) -> u64 {
        self.0[0]
    }

    /// Bits needed to represent the value (0 for zero).
    pub fn bits(&self) -> u32 {
        for i in (0..4).rev() {
            if self.0[i] != 0 {
                return 64 * i as u32 + (64 - self.0[i].leading_zeros());
            }
        }
        0
    }

    pub fn not(&self) -> Self {
        Self([!self.0[0], !self.0[1], !self.0[2], !self.0[3]])
    }

    pub fn checked_add(&self, other: &Self) -> Option<Self> {
        let mut out = [0u64; 4];
        let mut carry = false;
        for (i, slot) in out.iter_mut().enumerate() {
            let (sum, c1) = self.0[i].overflowing_add(other.0[i]);
            let (sum, c2) = sum.overflowing_add(u64::from(carry));
            *slot = sum;
            carry = c1 || c2;
        }
        (!carry).then_some(Self(out))
    }

    pub fn saturating_add(&self, other: &Self) -> Self {
        self.checked_add(other).unwrap_or(Self::MAX)
    }

    fn wrapping_sub(&self, other: &Self) -> Self {
        let mut out = [0u64; 4];
        let mut borrow = false;
        for (i, slot) in out.iter_mut().enumerate() {
            let (diff, b1) = self.0[i].overflowing_sub(other.0[i]);
            let (diff, b2) = diff.overflowing_sub(u64::from(borrow));
            *slot = diff;
            borrow = b1 || b2;
        }
        Self(out)
    }

    pub fn checked_mul_u64(&self, factor: u64) -> Option<Self> {
        let mut out = [0u64; 4];
        let mut carry: u128 = 0;
        for (i, slot) in out.iter_mut().enumerate() {
            let product = u128::from(self.0[i]) * u128::from(factor) + carry;
            *slot = product as u64;
            carry = product >> 64;
        }
        (carry == 0).then_some(Self(out))
    }

    /// Division by a nonzero `u64`; division by zero answers zero.
    pub fn div_u64(&self, divisor: u64) -> Self {
        self.div_rem_u64(divisor).0
    }

    /// Quotient and remainder by a `u64`; division by zero answers `(0, 0)`.
    pub fn div_rem_u64(&self, divisor: u64) -> (Self, u64) {
        if divisor == 0 {
            return (Self::ZERO, 0);
        }
        let mut out = [0u64; 4];
        let mut remainder: u128 = 0;
        for i in (0..4).rev() {
            let current = (remainder << 64) | u128::from(self.0[i]);
            out[i] = (current / u128::from(divisor)) as u64;
            remainder = current % u128::from(divisor);
        }
        (Self(out), remainder as u64)
    }

    /// `floor(self * numerator / denominator)` without an intermediate wider than 256 bits;
    /// `None` when the result does not fit or the denominator is zero.
    pub fn mul_div_u64(&self, numerator: u64, denominator: u64) -> Option<Self> {
        if denominator == 0 {
            return None;
        }
        // self = q * d + r  =>  self * n / d = q * n + (r * n) / d, and r * n < 2^128.
        let (quotient, remainder) = self.div_rem_u64(denominator);
        let tail = (u128::from(remainder) * u128::from(numerator)) / u128::from(denominator);
        quotient.checked_mul_u64(numerator)?.checked_add(&Self([
            tail as u64,
            (tail >> 64) as u64,
            0,
            0,
        ]))
    }

    /// Long division; division by zero answers zero.
    pub fn div(&self, divisor: &Self) -> Self {
        if divisor.is_zero() {
            return Self::ZERO;
        }
        let mut quotient = Self::ZERO;
        let mut remainder = Self::ZERO;
        for bit in (0..self.bits()).rev() {
            remainder = remainder.shl(1);
            if self.bit(bit) {
                remainder.0[0] |= 1;
            }
            if remainder >= *divisor {
                remainder = remainder.wrapping_sub(divisor);
                quotient.0[(bit / 64) as usize] |= 1 << (bit % 64);
            }
        }
        quotient
    }

    fn bit(&self, index: u32) -> bool {
        self.0[(index / 64) as usize] >> (index % 64) & 1 == 1
    }

    pub fn shl(&self, shift: u32) -> Self {
        if shift >= 256 {
            return Self::ZERO;
        }
        let limbs = (shift / 64) as usize;
        let bits = shift % 64;
        let mut out = [0u64; 4];
        for i in (limbs..4).rev() {
            out[i] = self.0[i - limbs] << bits;
            if bits > 0 && i > limbs {
                out[i] |= self.0[i - limbs - 1] >> (64 - bits);
            }
        }
        Self(out)
    }

    pub fn shr(&self, shift: u32) -> Self {
        self.shr_const(shift)
    }

    pub const fn shr_const(&self, shift: u32) -> Self {
        if shift >= 256 {
            return Self::ZERO;
        }
        let limbs = (shift / 64) as usize;
        let bits = shift % 64;
        let mut out = [0u64; 4];
        let mut i = 0;
        while i + limbs < 4 {
            out[i] = self.0[i + limbs] >> bits;
            if bits > 0 && i + limbs + 1 < 4 {
                out[i] |= self.0[i + limbs + 1] << (64 - bits);
            }
            i += 1;
        }
        Self(out)
    }
}

impl Ord for U256 {
    fn cmp(&self, other: &Self) -> Ordering {
        for i in (0..4).rev() {
            match self.0[i].cmp(&other.0[i]) {
                Ordering::Equal => continue,
                unequal => return unequal,
            }
        }
        Ordering::Equal
    }
}

impl PartialOrd for U256 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shifts_round_trip_across_limbs() {
        let one = U256::ONE;
        for shift in [0, 1, 63, 64, 65, 127, 128, 200, 255] {
            assert_eq!(one.shl(shift).shr(shift), one, "shift {shift}");
            assert_eq!(one.shl(shift).bits(), shift + 1);
        }
        assert!(one.shl(256).is_zero());
    }

    #[test]
    fn division_matches_u128_where_both_apply() {
        let cases: [(u128, u128); 5] = [
            (u128::MAX, 3),
            (1 << 100, (1 << 37) + 5),
            (12345678901234567890, 987654321),
            (7, 9),
            (0, 1),
        ];
        for (a, b) in cases {
            let wide = |v: u128| U256([v as u64, (v >> 64) as u64, 0, 0]);
            assert_eq!(wide(a).div(&wide(b)), wide(a / b), "{a} / {b}");
        }
    }

    #[test]
    fn add_and_mul_report_overflow() {
        assert!(U256::MAX.checked_add(&U256::ONE).is_none());
        assert!(U256::MAX.checked_mul_u64(2).is_none());
        assert_eq!(
            U256::ONE.shl(200).checked_mul_u64(4),
            Some(U256::ONE.shl(202))
        );
        assert_eq!(U256::ONE.shl(202).div_u64(4), U256::ONE.shl(200));
    }

    #[test]
    fn mul_div_is_exact_where_the_product_would_not_fit() {
        // (2^255 - 1) * 10 / 40 = floor((2^255 - 1) / 4) = 2^253 - 1.
        let big = U256::MAX.shr(1);
        assert_eq!(big.mul_div_u64(10, 40), Some(U256::MAX.shr(3)));
        assert_eq!(big.mul_div_u64(3, 1), None);
        assert_eq!(
            U256::from_u64(7).mul_div_u64(3, 2),
            Some(U256::from_u64(10))
        );
        assert_eq!(U256::ONE.mul_div_u64(1, 0), None);
    }
}
