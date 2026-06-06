use core::arch::x86_64::*;

use crate::amd64::avx2::{i16x16, si256};

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct si512(pub __m512i);

impl si512 {
    #[target_feature(enable = "avx512f")]
    #[inline]
    pub fn zero() -> Self {
        maybe_unsafe! { Self(_mm512_setzero_si512()) }
    }

    #[target_feature(enable = "avx512f")]
    #[inline]
    pub fn negative_one() -> Self {
        i32x16::splat(-1).as_si512()
    }

    #[inline]
    pub fn as_i8x64(self) -> i8x64 {
        i8x64(self.0)
    }

    #[inline]
    pub fn as_i16x32(self) -> i16x32 {
        i16x32(self.0)
    }

    #[inline]
    pub fn as_i32x16(self) -> i32x16 {
        i32x16(self.0)
    }

    #[inline]
    pub fn as_i64x8(self) -> i64x8 {
        i64x8(self.0)
    }

    #[target_feature(enable = "avx512f")]
    #[inline]
    pub unsafe fn store_unaligned(self, address: *mut u8) {
        unsafe { _mm512_storeu_si512(address.cast(), self.0) }
    }

    #[target_feature(enable = "avx512f")]
    #[inline]
    pub unsafe fn store_aligned(self, address: *mut u8) {
        unsafe { _mm512_store_si512(address.cast(), self.0) }
    }

    #[target_feature(enable = "avx512f")]
    #[inline]
    pub unsafe fn load_unaligned(address: *const u8) -> Self {
        unsafe { Self(_mm512_loadu_si512(address.cast())) }
    }

    #[target_feature(enable = "avx512f")]
    #[inline]
    pub unsafe fn load_aligned(address: *const u8) -> Self {
        unsafe { Self(_mm512_load_si512(address.cast())) }
    }

    #[target_feature(enable = "avx512f")]
    #[inline]
    pub fn from_bytes(xs: [u8; 64]) -> Self {
        maybe_unsafe! {
            Self(_mm512_set_epi8(
                xs[63] as i8,
                xs[62] as i8,
                xs[61] as i8,
                xs[60] as i8,
                xs[59] as i8,
                xs[58] as i8,
                xs[57] as i8,
                xs[56] as i8,
                xs[55] as i8,
                xs[54] as i8,
                xs[53] as i8,
                xs[52] as i8,
                xs[51] as i8,
                xs[50] as i8,
                xs[49] as i8,
                xs[48] as i8,
                xs[47] as i8,
                xs[46] as i8,
                xs[45] as i8,
                xs[44] as i8,
                xs[43] as i8,
                xs[42] as i8,
                xs[41] as i8,
                xs[40] as i8,
                xs[39] as i8,
                xs[38] as i8,
                xs[37] as i8,
                xs[36] as i8,
                xs[35] as i8,
                xs[34] as i8,
                xs[33] as i8,
                xs[32] as i8,
                xs[31] as i8,
                xs[30] as i8,
                xs[29] as i8,
                xs[28] as i8,
                xs[27] as i8,
                xs[26] as i8,
                xs[25] as i8,
                xs[24] as i8,
                xs[23] as i8,
                xs[22] as i8,
                xs[21] as i8,
                xs[20] as i8,
                xs[19] as i8,
                xs[18] as i8,
                xs[17] as i8,
                xs[16] as i8,
                xs[15] as i8,
                xs[14] as i8,
                xs[13] as i8,
                xs[12] as i8,
                xs[11] as i8,
                xs[10] as i8,
                xs[9] as i8,
                xs[8] as i8,
                xs[7] as i8,
                xs[6] as i8,
                xs[5] as i8,
                xs[4] as i8,
                xs[3] as i8,
                xs[2] as i8,
                xs[1] as i8,
                xs[0] as i8,
            ))
        }
    }

    #[target_feature(enable = "avx512f")]
    #[inline]
    pub fn lo(self) -> si256 {
        maybe_unsafe! { si256(_mm512_castsi512_si256(self.0)) }
    }

    #[target_feature(enable = "avx512f")]
    #[inline]
    pub fn hi(self) -> si256 {
        maybe_unsafe! { si256(_mm512_extracti64x4_epi64(self.0, 1)) }
    }

    #[target_feature(enable = "avx512f")]
    #[inline]
    pub fn as_bytes(self) -> [u8; 64] {
        let mut array: core::mem::MaybeUninit<[u8; 64]> = core::mem::MaybeUninit::uninit();
        unsafe {
            self.store_unaligned(array.as_mut_ptr().cast());
            array.assume_init()
        }
    }

    #[target_feature(enable = "avx512f")]
    #[inline]
    pub fn is_equal(self, rhs: Self) -> bool {
        self.as_i64x8().simd_eq(rhs.as_i64x8()) == 0xff
    }

    #[target_feature(enable = "avx512f")]
    #[inline]
    pub fn and_not(self, rhs: Self) -> Self {
        maybe_unsafe! { Self(_mm512_andnot_si512(rhs.0, self.0)) }
    }

    #[target_feature(enable = "avx512f")]
    #[inline]
    pub fn and(self, rhs: Self) -> Self {
        maybe_unsafe! { Self(_mm512_and_si512(self.0, rhs.0)) }
    }

    #[target_feature(enable = "avx512f")]
    #[inline]
    pub fn or(self, rhs: Self) -> Self {
        maybe_unsafe! { Self(_mm512_or_si512(self.0, rhs.0)) }
    }

    #[target_feature(enable = "avx512f")]
    #[inline]
    pub fn xor(self, rhs: Self) -> Self {
        maybe_unsafe! { Self(_mm512_xor_si512(self.0, rhs.0)) }
    }
}

impl_bitops!(si512);

macro_rules! impl_m512 {
    (
        $type:ident
        $add:ident
        $sub:ident
        $cmpeq:ident
        $cmpgt:ident
        $cmpmask:ident
        $lane_ty:ident
        $feature:tt
        $lane_count:expr
    ) => {
        #[repr(transparent)]
        #[derive(Copy, Clone)]
        pub struct $type(pub __m512i);

        impl $type {
            #[inline]
            pub fn as_si512(self) -> si512 {
                si512(self.0)
            }

            #[inline]
            pub fn as_i8x64(self) -> i8x64 {
                self.as_si512().as_i8x64()
            }

            #[inline]
            pub fn as_i16x32(self) -> i16x32 {
                self.as_si512().as_i16x32()
            }

            #[inline]
            pub fn as_i32x16(self) -> i32x16 {
                self.as_si512().as_i32x16()
            }

            #[inline]
            pub fn as_i64x8(self) -> i64x8 {
                self.as_si512().as_i64x8()
            }

            #[target_feature(enable = "avx512f")]
            #[inline]
            pub fn as_bytes(self) -> [u8; 64] {
                self.as_si512().as_bytes()
            }

            #[target_feature(enable = "avx512f")]
            #[inline]
            pub fn zero() -> Self {
                Self(si512::zero().0)
            }

            #[target_feature(enable = "avx512f")]
            #[inline]
            pub fn negative_one() -> Self {
                Self(si512::negative_one().0)
            }

            #[target_feature(enable = $feature)]
            #[inline]
            pub fn simd_eq(self, rhs: Self) -> $cmpmask {
                maybe_unsafe! { $cmpeq(self.0, rhs.0) }
            }

            #[target_feature(enable = $feature)]
            #[inline]
            pub fn simd_gt(self, rhs: Self) -> $cmpmask {
                maybe_unsafe! { $cmpgt(self.0, rhs.0) }
            }

            #[target_feature(enable = $feature)]
            #[inline]
            pub fn simd_lt(self, rhs: Self) -> $cmpmask {
                rhs.simd_gt(self)
            }

            #[target_feature(enable = "avx512f")]
            #[inline]
            pub fn is_equal(self, rhs: Self) -> bool {
                self.as_si512().is_equal(rhs.as_si512())
            }

            #[target_feature(enable = "avx512f")]
            #[inline]
            pub fn and_not(self, rhs: Self) -> Self {
                Self(self.as_si512().and_not(rhs.as_si512()).0)
            }

            #[target_feature(enable = "avx512f")]
            #[inline]
            pub fn to_array(self) -> [$lane_ty; $lane_count] {
                let mut array: core::mem::MaybeUninit<[$lane_ty; $lane_count]> = core::mem::MaybeUninit::uninit();
                unsafe {
                    self.as_si512().store_unaligned(array.as_mut_ptr().cast());
                    array.assume_init()
                }
            }

            #[target_feature(enable = $feature)]
            #[inline]
            pub fn add(self, rhs: Self) -> Self {
                maybe_unsafe! { Self($add(self.0, rhs.0)) }
            }

            #[target_feature(enable = $feature)]
            #[inline]
            pub fn sub(self, rhs: Self) -> Self {
                maybe_unsafe! { Self($sub(self.0, rhs.0)) }
            }
        }

        impl_bitops!($type as_si512 "avx512f");
        impl_common_ops!($type);
    };
}

impl_m512!(i8x64 _mm512_add_epi8 _mm512_sub_epi8 _mm512_cmpeq_epi8_mask _mm512_cmpgt_epi8_mask u64 i8 "avx512bw" 64);
impl_m512!(i16x32 _mm512_add_epi16 _mm512_sub_epi16 _mm512_cmpeq_epi16_mask _mm512_cmpgt_epi16_mask u32 i16 "avx512bw" 32);
impl_m512!(i32x16 _mm512_add_epi32 _mm512_sub_epi32 _mm512_cmpeq_epi32_mask _mm512_cmpgt_epi32_mask u16 i32 "avx512f" 16);
impl_m512!(i64x8 _mm512_add_epi64 _mm512_sub_epi64 _mm512_cmpeq_epi64_mask _mm512_cmpgt_epi64_mask u8 i64 "avx512f" 8);

impl i8x64 {
    #[target_feature(enable = "avx512f")]
    #[inline]
    pub fn splat(value: i8) -> Self {
        maybe_unsafe! { Self(_mm512_set1_epi8(value)) }
    }
}

impl i16x32 {
    #[target_feature(enable = "avx512f")]
    #[inline]
    pub fn splat(value: i16) -> Self {
        maybe_unsafe! { Self(_mm512_set1_epi16(value)) }
    }
}

impl i32x16 {
    #[target_feature(enable = "avx512f")]
    #[inline]
    pub fn splat(value: i32) -> Self {
        maybe_unsafe! { Self(_mm512_set1_epi32(value)) }
    }
}

impl i64x8 {
    #[target_feature(enable = "avx512f")]
    #[inline]
    pub fn splat(value: i64) -> Self {
        maybe_unsafe! { Self(_mm512_set1_epi64(value)) }
    }
}

// -----------------------------
// AVX interoperability and extensions
// -----------------------------

impl i16x16 {
    #[target_feature(enable = "avx512bw,avx512vl")]
    #[inline]
    pub fn unbounded_shr(self, shifts: Self) -> Self {
        maybe_unsafe! { Self(_mm256_srlv_epi16(self.0, shifts.0)) }
    }

    #[target_feature(enable = "avx512bw,avx512vl")]
    #[inline]
    pub fn unbounded_shl(self, shifts: Self) -> Self {
        maybe_unsafe! { Self(_mm256_sllv_epi16(self.0, shifts.0)) }
    }
}
