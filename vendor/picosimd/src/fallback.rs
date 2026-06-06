#![allow(clippy::should_implement_trait)]

use crate::indexes;

#[inline]
fn zip_map<T, const N: usize, U, F>(lhs: [T; N], rhs: [T; N], callback: F) -> [U; N]
where
    F: Fn(T, T) -> U,
    T: Copy,
{
    let mut n = 0;
    [(); N].map(|_| {
        let value = callback(lhs[n], rhs[n]);
        n += 1;
        value
    })
}

macro_rules! impl_fallback {
    (
        $type:ident,
        $lane_ty:ty,
        $lane_ty_unsigned:ty,
        $lane_count:expr
    ) => {
        #[repr(transparent)]
        #[derive(Copy, Clone)]
        pub struct $type(pub [$lane_ty; $lane_count]);

        impl $type {
            #[inline]
            pub fn zero() -> Self {
                Self([0; $lane_count])
            }

            #[inline]
            pub fn negative_one() -> Self {
                Self([-1; $lane_count])
            }

            #[inline]
            pub fn splat(value: $lane_ty) -> Self {
                Self([value; $lane_count])
            }

            #[inline]
            pub fn from_array(xs: [$lane_ty; $lane_count]) -> Self {
                Self(xs)
            }

            #[inline]
            pub fn from_array_ref(xs: &[$lane_ty; $lane_count]) -> Self {
                Self(*xs)
            }

            #[inline]
            pub fn to_array(self) -> [$lane_ty; $lane_count] {
                self.0
            }

            #[inline]
            pub fn as_slice(&self) -> &[$lane_ty; $lane_count] {
                &self.0
            }

            #[inline]
            pub fn as_slice_mut(&mut self) -> &mut [$lane_ty; $lane_count] {
                &mut self.0
            }

            #[inline]
            pub fn is_equal(self, rhs: Self) -> bool {
                self.0 == rhs.0
            }

            #[inline]
            pub fn is_zero(self) -> bool {
                self.is_equal(Self::zero())
            }

            #[inline]
            pub fn simd_eq(self, rhs: Self) -> Self {
                Self(zip_map(self.0, rhs.0, |lhs, rhs| if lhs == rhs { -1 } else { 0 }))
            }

            #[inline]
            pub fn simd_gt(self, rhs: Self) -> Self {
                Self(zip_map(self.0, rhs.0, |lhs, rhs| if lhs > rhs { -1 } else { 0 }))
            }

            #[inline]
            pub fn simd_lt(self, rhs: Self) -> Self {
                rhs.simd_gt(self)
            }

            #[inline]
            pub fn and_not(self, rhs: Self) -> Self {
                Self(zip_map(self.0, rhs.0, |lhs, rhs| lhs & (!rhs)))
            }

            #[inline]
            pub fn and(self, rhs: Self) -> Self {
                Self(zip_map(self.0, rhs.0, |lhs, rhs| lhs & rhs))
            }

            #[inline]
            pub fn or(self, rhs: Self) -> Self {
                Self(zip_map(self.0, rhs.0, |lhs, rhs| lhs | rhs))
            }

            #[inline]
            pub fn xor(self, rhs: Self) -> Self {
                Self(zip_map(self.0, rhs.0, |lhs, rhs| lhs ^ rhs))
            }

            #[inline]
            pub fn add(self, rhs: Self) -> Self {
                Self(zip_map(self.0, rhs.0, |lhs, rhs| lhs.wrapping_add(rhs)))
            }

            #[inline]
            pub fn sub(self, rhs: Self) -> Self {
                Self(zip_map(self.0, rhs.0, |lhs, rhs| lhs.wrapping_sub(rhs)))
            }

            #[inline]
            pub fn saturating_add(self, rhs: Self) -> Self {
                Self(zip_map(self.0, rhs.0, |lhs, rhs| lhs.saturating_add(rhs)))
            }

            #[inline]
            pub fn saturating_add_unsigned(self, rhs: Self) -> Self {
                Self(zip_map(self.0, rhs.0, |lhs, rhs| {
                    (lhs as $lane_ty_unsigned).saturating_add(rhs as $lane_ty_unsigned) as $lane_ty
                }))
            }

            #[inline]
            pub fn saturating_sub(self, rhs: Self) -> Self {
                Self(zip_map(self.0, rhs.0, |lhs, rhs| lhs.saturating_sub(rhs)))
            }

            #[inline]
            pub fn saturating_sub_unsigned(self, rhs: Self) -> Self {
                Self(zip_map(self.0, rhs.0, |lhs, rhs| {
                    (lhs as $lane_ty_unsigned).saturating_sub(rhs as $lane_ty_unsigned) as $lane_ty
                }))
            }

            #[inline]
            pub fn wrapping_reduce(self) -> $lane_ty {
                self.0.iter().copied().fold(0, |lhs, rhs| lhs.wrapping_add(rhs))
            }

            #[inline]
            pub fn bitwise_reduce(self) -> $lane_ty {
                self.0.iter().copied().fold(0, |lhs, rhs| lhs | rhs)
            }

            #[inline]
            pub fn horizontal_max_signed(self) -> $lane_ty {
                self.0.iter().copied().max().unwrap()
            }

            #[inline]
            pub fn horizontal_min_signed(self) -> $lane_ty {
                self.0.iter().copied().min().unwrap()
            }

            #[inline]
            pub fn horizontal_max_unsigned(self) -> $lane_ty_unsigned {
                self.0.iter().copied().map(|x| x as $lane_ty_unsigned).max().unwrap()
            }

            #[inline]
            pub fn horizontal_min_unsigned(self) -> $lane_ty_unsigned {
                self.0.iter().copied().map(|x| x as $lane_ty_unsigned).min().unwrap()
            }

            #[must_use]
            #[inline]
            pub fn set_dynamic(self, index: $lane_ty_unsigned, value: $lane_ty) -> Self {
                let mut copy = self;
                if index < $lane_count {
                    copy.0[index as usize] = value;
                }
                copy
            }

            #[must_use]
            #[inline]
            pub fn get_dynamic(self, index: $lane_ty_unsigned) -> $lane_ty {
                if index < $lane_count { self.0[index as usize] } else { 0 }
            }

            #[inline]
            pub fn min_unsigned(self, rhs: Self) -> Self {
                Self(zip_map(self.0, rhs.0, |lhs, rhs| {
                    (lhs as $lane_ty_unsigned).min(rhs as $lane_ty_unsigned) as $lane_ty
                }))
            }

            #[inline]
            pub fn max_unsigned(self, rhs: Self) -> Self {
                Self(zip_map(self.0, rhs.0, |lhs, rhs| {
                    (lhs as $lane_ty_unsigned).max(rhs as $lane_ty_unsigned) as $lane_ty
                }))
            }

            #[inline]
            pub fn min_signed(self, rhs: Self) -> Self {
                Self(zip_map(self.0, rhs.0, |lhs, rhs| lhs.min(rhs)))
            }

            #[inline]
            pub fn max_signed(self, rhs: Self) -> Self {
                Self(zip_map(self.0, rhs.0, |lhs, rhs| lhs.max(rhs)))
            }
        }

        #[cfg(feature = "ops")]
        impl core::fmt::Debug for $type {
            fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
                self.to_array().fmt(fmt)
            }
        }

        #[cfg(feature = "ops")]
        impl core::ops::Add for $type {
            type Output = Self;

            #[inline]
            fn add(self, rhs: Self) -> Self {
                Self::add(self, rhs)
            }
        }

        #[cfg(feature = "ops")]
        impl core::ops::Sub for $type {
            type Output = Self;

            #[inline]
            fn sub(self, rhs: Self) -> Self {
                Self::sub(self, rhs)
            }
        }

        #[cfg(feature = "ops")]
        impl core::ops::AddAssign for $type {
            #[inline]
            fn add_assign(&mut self, rhs: Self) {
                *self = *self + rhs;
            }
        }

        #[cfg(feature = "ops")]
        impl core::ops::SubAssign for $type {
            #[inline]
            fn sub_assign(&mut self, rhs: Self) {
                *self = *self - rhs;
            }
        }

        #[cfg(feature = "ops")]
        impl PartialEq for $type {
            fn eq(&self, rhs: &Self) -> bool {
                self.is_equal(*rhs)
            }
        }

        #[cfg(feature = "ops")]
        impl Eq for $type {}

        #[cfg(feature = "ops")]
        impl core::ops::BitAnd for $type {
            type Output = Self;

            #[inline]
            fn bitand(self, rhs: Self) -> Self {
                self.and(rhs)
            }
        }

        #[cfg(feature = "ops")]
        impl core::ops::BitOr for $type {
            type Output = Self;

            #[inline]
            fn bitor(self, rhs: Self) -> Self {
                self.or(rhs)
            }
        }

        #[cfg(feature = "ops")]
        impl core::ops::BitXor for $type {
            type Output = Self;

            #[inline]
            fn bitxor(self, rhs: Self) -> Self {
                self.xor(rhs)
            }
        }
    };
}

impl i8x32 {
    #[inline]
    pub fn from_i1x32_sext(packed_mask: i32) -> Self {
        Self::from_array(indexes::<32>().map(|n| if (packed_mask & (1 << n)) != 0 { -1 } else { 0 }))
    }

    #[inline]
    pub fn to_i16x32_sext(self) -> i16x32 {
        i16x32::from_array(self.to_array().map(i16::from))
    }

    #[inline]
    pub fn to_i32x32_sext(self) -> i32x32 {
        i32x32::from_array(self.to_array().map(i32::from))
    }

    #[inline]
    pub fn most_significant_bits(self) -> i32 {
        self.to_array()
            .map(|x| ((x as u8) >> 7) as i32)
            .into_iter()
            .enumerate()
            .map(|(index, x)| x << index)
            .fold(0, |a, b| a | b)
    }
}

impl i8x16 {
    #[inline]
    pub fn from_i1x16_sext(packed_mask: i16) -> Self {
        Self::from_array(indexes::<16>().map(|n| if (packed_mask & (1 << n)) != 0 { -1 } else { 0 }))
    }

    #[inline]
    pub fn to_i16x16_sext(self) -> i16x16 {
        i16x16(self.0.map(i16::from))
    }
}

impl i16x32 {
    #[inline]
    pub fn to_i32x32_sext(self) -> i32x32 {
        i32x32::from_array(self.to_array().map(i32::from))
    }
}

impl i32x8 {
    #[inline]
    pub fn from_i1x8_sext(packed_mask: i8) -> Self {
        Self::from_array(indexes::<8>().map(|n| if (packed_mask & (1 << n)) != 0 { -1 } else { 0 }))
    }
}

impl i32x32 {
    #[inline]
    pub fn from_i8x32_sext(value: i8x32) -> Self {
        Self::from_array(value.to_array().map(i32::from))
    }
}

impl_fallback!(i8x64, i8, u8, 64);
impl_fallback!(i8x32, i8, u8, 32);
impl_fallback!(i8x16, i8, u8, 16);
impl_fallback!(i16x32, i16, u16, 32);
impl_fallback!(i16x16, i16, u16, 16);
impl_fallback!(i16x8, i16, u16, 8);
impl_fallback!(i32x32, i32, u32, 32);
impl_fallback!(i32x16, i32, u32, 16);
impl_fallback!(i32x8, i32, u32, 8);
impl_fallback!(i32x4, i32, u32, 4);
impl_fallback!(i64x8, i64, u64, 8);
impl_fallback!(i64x4, i64, u64, 4);
impl_fallback!(i64x2, i64, u64, 2);

macro_rules! impl_clamp_to_i8_range {
    ($type:ty, $target_ty:ident) => {
        impl $type {
            #[inline]
            pub fn clamp_to_i8_range(self) -> $target_ty {
                $target_ty(self.0.map(|value| {
                    if value > 127 {
                        127
                    } else if value < -128 {
                        -128
                    } else {
                        value as i8
                    }
                }))
            }
        }
    };
}

impl_clamp_to_i8_range!(i16x16, i8x16);
impl_clamp_to_i8_range!(i16x32, i8x32);
impl_clamp_to_i8_range!(i32x32, i8x32);
