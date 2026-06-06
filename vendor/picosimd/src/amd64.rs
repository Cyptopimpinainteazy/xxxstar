macro_rules! impl_bitops {
    ($type:ident $as_full_width:ident $target_feature:expr) => {
        impl $type {
            #[target_feature(enable = $target_feature)]
            #[inline]
            pub fn and(self, rhs: Self) -> Self {
                Self((self.$as_full_width().and(rhs.$as_full_width())).0)
            }

            #[target_feature(enable = $target_feature)]
            #[inline]
            pub fn or(self, rhs: Self) -> Self {
                Self((self.$as_full_width().or(rhs.$as_full_width())).0)
            }

            #[target_feature(enable = $target_feature)]
            #[inline]
            pub fn xor(self, rhs: Self) -> Self {
                Self((self.$as_full_width().xor(rhs.$as_full_width())).0)
            }
        }

        impl_bitops!($type);
    };

    ($type:ident) => {
        #[cfg(feature = "ops")]
        impl PartialEq for $type {
            #[inline(always)]
            fn eq(&self, rhs: &Self) -> bool {
                unsafe { self.is_equal(*rhs) }
            }
        }

        #[cfg(feature = "ops")]
        impl Eq for $type {}

        #[cfg(feature = "ops")]
        impl core::ops::BitAnd for $type {
            type Output = Self;

            #[inline(always)]
            fn bitand(self, rhs: Self) -> Self {
                unsafe { self.and(rhs) }
            }
        }

        #[cfg(feature = "ops")]
        impl core::ops::BitOr for $type {
            type Output = Self;

            #[inline(always)]
            fn bitor(self, rhs: Self) -> Self {
                unsafe { self.or(rhs) }
            }
        }

        #[cfg(feature = "ops")]
        impl core::ops::BitXor for $type {
            type Output = Self;

            #[inline(always)]
            fn bitxor(self, rhs: Self) -> Self {
                unsafe { self.xor(rhs) }
            }
        }

        #[cfg(feature = "ops")]
        impl core::ops::BitAndAssign for $type {
            #[inline(always)]
            fn bitand_assign(&mut self, rhs: Self) {
                *self = *self & rhs;
            }
        }

        #[cfg(feature = "ops")]
        impl core::ops::BitOrAssign for $type {
            #[inline(always)]
            fn bitor_assign(&mut self, rhs: Self) {
                *self = *self | rhs;
            }
        }

        #[cfg(feature = "ops")]
        impl core::ops::BitXorAssign for $type {
            #[inline(always)]
            fn bitxor_assign(&mut self, rhs: Self) {
                *self = *self ^ rhs;
            }
        }
    };
}

macro_rules! impl_common_ops {
    ($type:ident) => {
        #[cfg(feature = "ops")]
        impl core::fmt::Debug for $type {
            fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
                unsafe { self.to_array().fmt(fmt) }
            }
        }

        #[cfg(feature = "ops")]
        impl core::ops::Add for $type {
            type Output = Self;

            #[inline(always)]
            fn add(self, rhs: Self) -> Self {
                unsafe { Self::add(self, rhs) }
            }
        }

        #[cfg(feature = "ops")]
        impl core::ops::Sub for $type {
            type Output = Self;

            #[inline(always)]
            fn sub(self, rhs: Self) -> Self {
                unsafe { Self::sub(self, rhs) }
            }
        }

        #[cfg(feature = "ops")]
        impl core::ops::AddAssign for $type {
            #[inline(always)]
            fn add_assign(&mut self, rhs: Self) {
                *self = *self + rhs;
            }
        }

        #[cfg(feature = "ops")]
        impl core::ops::SubAssign for $type {
            #[inline(always)]
            fn sub_assign(&mut self, rhs: Self) {
                *self = *self - rhs;
            }
        }
    };
}

macro_rules! impl_min_max {
    (
        $type:ident
        $min_unsigned:ident
        $max_unsigned:ident
        $min_signed:ident
        $max_signed:ident
        $target_feature_unsigned:expr,
        $target_feature_signed:expr
    ) => {
        impl $type {
            #[target_feature(enable = $target_feature_unsigned)]
            #[inline]
            pub fn min_unsigned(self, rhs: Self) -> Self {
                maybe_unsafe! { Self($min_unsigned(self.0, rhs.0)) }
            }

            #[target_feature(enable = $target_feature_unsigned)]
            #[inline]
            pub fn max_unsigned(self, rhs: Self) -> Self {
                maybe_unsafe! { Self($max_unsigned(self.0, rhs.0)) }
            }

            #[target_feature(enable = $target_feature_signed)]
            #[inline]
            pub fn min_signed(self, rhs: Self) -> Self {
                maybe_unsafe! { Self($min_signed(self.0, rhs.0)) }
            }

            #[target_feature(enable = $target_feature_signed)]
            #[inline]
            pub fn max_signed(self, rhs: Self) -> Self {
                maybe_unsafe! { Self($max_signed(self.0, rhs.0)) }
            }
        }
    };
}

pub mod avx2;
pub mod avx2_composite;
pub mod sse;

#[cfg(picosimd = "avx512")]
#[allow(clippy::incompatible_msrv)]
pub mod avx512;
