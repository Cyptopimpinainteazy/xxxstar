use core::arch::x86_64::*;

use crate::amd64::sse::{i8x16, i16x8, i32x4, i64x2, si128};

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct si256(pub __m256i);

impl si256 {
    #[target_feature(enable = "avx")]
    #[inline]
    pub fn undefined() -> Self {
        maybe_unsafe! { Self(_mm256_undefined_si256()) }
    }

    #[target_feature(enable = "avx")]
    #[inline]
    pub fn zero() -> Self {
        maybe_unsafe! { Self(_mm256_setzero_si256()) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn negative_one() -> Self {
        let undef = Self::undefined().as_i32x8();
        undef.simd_eq(undef).as_si256()

        // Alternative:
        //  i32x8::splat(-1).as_si256()
    }

    #[target_feature(enable = "avx")]
    #[inline]
    pub unsafe fn store_unaligned(self, address: *mut u8) {
        unsafe { _mm256_storeu_si256(address.cast(), self.0) }
    }

    #[target_feature(enable = "avx")]
    #[inline]
    pub unsafe fn store_aligned(self, address: *mut u8) {
        unsafe { _mm256_store_si256(address.cast(), self.0) }
    }

    #[target_feature(enable = "avx")]
    #[inline]
    pub unsafe fn load_unaligned(address: *const u8) -> Self {
        unsafe { Self(_mm256_loadu_si256(address.cast())) }
    }

    #[target_feature(enable = "avx")]
    #[inline]
    pub unsafe fn load_aligned(address: *const u8) -> Self {
        unsafe { Self(_mm256_load_si256(address.cast())) }
    }

    #[target_feature(enable = "avx")]
    #[inline]
    pub fn from_bytes(xs: [u8; 32]) -> Self {
        maybe_unsafe! {
            Self(_mm256_setr_epi8(
                xs[0] as i8,
                xs[1] as i8,
                xs[2] as i8,
                xs[3] as i8,
                xs[4] as i8,
                xs[5] as i8,
                xs[6] as i8,
                xs[7] as i8,
                xs[8] as i8,
                xs[9] as i8,
                xs[10] as i8,
                xs[11] as i8,
                xs[12] as i8,
                xs[13] as i8,
                xs[14] as i8,
                xs[15] as i8,
                xs[16] as i8,
                xs[17] as i8,
                xs[18] as i8,
                xs[19] as i8,
                xs[20] as i8,
                xs[21] as i8,
                xs[22] as i8,
                xs[23] as i8,
                xs[24] as i8,
                xs[25] as i8,
                xs[26] as i8,
                xs[27] as i8,
                xs[28] as i8,
                xs[29] as i8,
                xs[30] as i8,
                xs[31] as i8,
            ))
        }
    }

    #[target_feature(enable = "avx")]
    #[inline]
    pub fn to_bytes(self) -> [u8; 32] {
        let mut array: core::mem::MaybeUninit<[u8; 32]> = core::mem::MaybeUninit::uninit();
        unsafe {
            self.store_unaligned(array.as_mut_ptr().cast());
            array.assume_init()
        }
    }

    #[target_feature(enable = "avx")]
    #[inline]
    pub fn lo(self) -> si128 {
        maybe_unsafe! { si128(_mm256_castsi256_si128(self.0)) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn hi(self) -> si128 {
        maybe_unsafe! { si128(_mm256_extracti128_si256(self.0, 1)) }
    }

    #[inline]
    pub fn as_i128x2(self) -> i128x2 {
        i128x2(self.0)
    }

    #[inline]
    pub fn as_i8x32(self) -> i8x32 {
        i8x32(self.0)
    }

    #[inline]
    pub fn as_i16x16(self) -> i16x16 {
        i16x16(self.0)
    }

    #[inline]
    pub fn as_i32x8(self) -> i32x8 {
        i32x8(self.0)
    }

    #[inline]
    pub fn as_i64x4(self) -> i64x4 {
        i64x4(self.0)
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn is_equal(self, rhs: Self) -> bool {
        let cmp = self.as_i32x8().simd_eq(rhs.as_i32x8());
        maybe_unsafe! { _mm256_testc_si256(cmp.0, i32x8::splat(-1).0) != 0 }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn and_not(self, rhs: Self) -> Self {
        maybe_unsafe! { Self(_mm256_andnot_si256(rhs.0, self.0)) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn and(self, rhs: Self) -> Self {
        maybe_unsafe! { Self(_mm256_and_si256(self.0, rhs.0)) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn or(self, rhs: Self) -> Self {
        maybe_unsafe! { Self(_mm256_or_si256(self.0, rhs.0)) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn xor(self, rhs: Self) -> Self {
        maybe_unsafe! { Self(_mm256_xor_si256(self.0, rhs.0)) }
    }

    #[must_use]
    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn set_lo(self, value: si128) -> si256 {
        maybe_unsafe! { Self(_mm256_inserti128_si256(self.0, value.0, 0)) }
    }

    #[must_use]
    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn set_hi(self, value: si128) -> si256 {
        maybe_unsafe! { Self(_mm256_inserti128_si256(self.0, value.0, 1)) }
    }
}

impl_bitops!(si256);

macro_rules! impl_m256 {
    (
        $type:ident
        $add:ident
        $sub:ident
        $cmpeq:ident
        $cmpgt:ident
        $lane_ty:ident
        $lane_ty_unsigned:ident
        $lane_count:expr
    ) => {
        #[repr(transparent)]
        #[derive(Copy, Clone)]
        pub struct $type(pub __m256i);

        impl $type {
            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn negative_one() -> Self {
                Self(si256::negative_one().0)
            }

            #[inline]
            pub fn as_si256(self) -> si256 {
                si256(self.0)
            }

            #[inline]
            pub fn as_i128x2(self) -> i128x2 {
                self.as_si256().as_i128x2()
            }

            #[inline]
            pub fn as_i8x32(self) -> i8x32 {
                self.as_si256().as_i8x32()
            }

            #[inline]
            pub fn as_i16x16(self) -> i16x16 {
                self.as_si256().as_i16x16()
            }

            #[inline]
            pub fn as_i32x8(self) -> i32x8 {
                self.as_si256().as_i32x8()
            }

            #[inline]
            pub fn as_i64x4(self) -> i64x4 {
                self.as_si256().as_i64x4()
            }

            #[inline]
            pub fn as_slice(&self) -> &[$lane_ty; $lane_count] {
                unsafe {
                    &*core::ptr::addr_of!(self.0).cast::<[$lane_ty; $lane_count]>()
                }
            }

            #[inline]
            pub fn as_slice_mut(&mut self) -> &mut [$lane_ty; $lane_count] {
                unsafe {
                    &mut *core::ptr::addr_of_mut!(self.0).cast::<[$lane_ty; $lane_count]>()
                }
            }

            #[target_feature(enable = "avx")]
            #[inline]
            pub fn to_bytes(self) -> [u8; 32] {
                self.as_si256().to_bytes()
            }

            #[target_feature(enable = "avx")]
            #[inline]
            pub unsafe fn load_unaligned(address: *const u8) -> Self {
                unsafe {
                    Self(si256::load_unaligned(address).0)
                }
            }

            #[target_feature(enable = "avx")]
            #[inline]
            pub fn from_array_ref(xs: &[$lane_ty; $lane_count]) -> Self {
                const _: () = {
                    assert!(core::mem::size_of::<[$lane_ty; $lane_count]>() == core::mem::size_of::<$type>());
                };

                unsafe {
                    Self::load_unaligned(xs.as_ptr().cast())
                }
            }

            #[target_feature(enable = "avx")]
            #[inline]
            pub fn zero() -> Self {
                Self(si256::zero().0)
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn simd_eq(self, rhs: Self) -> Self {
                maybe_unsafe! { Self($cmpeq(self.0, rhs.0)) }
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn simd_gt(self, rhs: Self) -> Self {
                maybe_unsafe! { Self($cmpgt(self.0, rhs.0)) }
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn simd_lt(self, rhs: Self) -> Self {
                rhs.simd_gt(self)
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn is_equal(self, rhs: Self) -> bool {
                self.as_si256().is_equal(rhs.as_si256())
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn and_not(self, rhs: Self) -> Self {
                Self(self.as_si256().and_not(rhs.as_si256()).0)
            }

            #[target_feature(enable = "avx")]
            #[inline]
            pub fn to_array(self) -> [$lane_ty; $lane_count] {
                let mut array: core::mem::MaybeUninit<[$lane_ty; $lane_count]> = core::mem::MaybeUninit::uninit();
                unsafe {
                    self.as_si256().store_unaligned(array.as_mut_ptr().cast());
                    array.assume_init()
                }
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn add(self, rhs: Self) -> Self {
                maybe_unsafe! { Self($add(self.0, rhs.0)) }
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn sub(self, rhs: Self) -> Self {
                maybe_unsafe! { Self($sub(self.0, rhs.0)) }
            }

            #[must_use]
            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn set_dynamic(self, index: $lane_ty_unsigned, value: $lane_ty) -> Self {
                let index = index as $lane_ty;
                let indexes = Self::lane_indexes();
                let value = Self::splat(value).as_si256().as_i8x32();
                let mask = indexes.simd_eq(Self::splat(index)).as_si256().as_i8x32();
                Self::from(self.as_i8x32().conditional_assign(value, mask).as_si256())
            }

            #[must_use]
            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn get_dynamic(self, index: $lane_ty_unsigned) -> $lane_ty {
                self.and(Self::lane_indexes().simd_eq(Self::splat(index as $lane_ty))).bitwise_reduce()
            }
        }

        impl From<si256> for $type {
            #[inline]
            fn from(value: si256) -> Self {
                Self(value.0)
            }
        }

        impl_bitops!($type as_si256 "avx2");
        impl_common_ops!($type);
    };
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct i128x2(pub __m256i);

impl i128x2 {
    #[target_feature(enable = "avx")]
    #[inline]
    pub fn zero() -> Self {
        Self(si256::zero().0)
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn splat(value: si128) -> Self {
        maybe_unsafe! { Self(_mm256_broadcastsi128_si256(value.0)) }
    }

    #[inline]
    pub fn as_si256(self) -> si256 {
        si256(self.0)
    }

    #[inline]
    pub fn as_i8x32(self) -> i8x32 {
        i8x32(self.0)
    }

    #[inline]
    pub fn as_i16x16(self) -> i16x16 {
        i16x16(self.0)
    }

    #[inline]
    pub fn as_i32x8(self) -> i32x8 {
        i32x8(self.0)
    }

    #[inline]
    pub fn as_i64x4(self) -> i64x4 {
        i64x4(self.0)
    }

    #[target_feature(enable = "avx")]
    #[inline]
    pub fn lo(self) -> si128 {
        self.as_si256().lo()
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn hi(self) -> si128 {
        self.as_si256().hi()
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn is_equal(self, rhs: Self) -> bool {
        self.as_si256().is_equal(rhs.as_si256())
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn shuffle_i8x16(self, mask: i8x32) -> Self {
        maybe_unsafe! {
            Self(_mm256_shuffle_epi8(self.0, mask.0))
        }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn shuffle_i32x4<const LANES: i32>(self) -> Self {
        assert_eq!(LANES & (!0xff), 0);
        maybe_unsafe! {
            Self(_mm256_shuffle_epi32(self.0, LANES))
        }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn unbounded_shl_by_bytes<const AMOUNT: i32>(self) -> i128x2 {
        maybe_unsafe! { Self(_mm256_slli_si256(self.0, AMOUNT)) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn unbounded_shr_by_bytes<const AMOUNT: i32>(self) -> i128x2 {
        maybe_unsafe! { Self(_mm256_srli_si256(self.0, AMOUNT)) }
    }
}

impl_m256!(i8x32 _mm256_add_epi8 _mm256_sub_epi8 _mm256_cmpeq_epi8 _mm256_cmpgt_epi8 i8 u8 32);
impl_m256!(i16x16 _mm256_add_epi16 _mm256_sub_epi16 _mm256_cmpeq_epi16 _mm256_cmpgt_epi16 i16 u16 16);
impl_m256!(i32x8 _mm256_add_epi32 _mm256_sub_epi32 _mm256_cmpeq_epi32 _mm256_cmpgt_epi32 i32 u32 8);
impl_m256!(i64x4 _mm256_add_epi64 _mm256_sub_epi64 _mm256_cmpeq_epi64 _mm256_cmpgt_epi64 i64 u64 4);
impl_bitops!(i128x2 as_si256 "avx2");

impl_min_max!(i8x32 _mm256_min_epu8 _mm256_max_epu8 _mm256_min_epi8 _mm256_max_epi8 "avx2", "avx2");
impl_min_max!(i16x16 _mm256_min_epu16 _mm256_max_epu16 _mm256_min_epi16 _mm256_max_epi16 "avx2" ,"avx2");
impl_min_max!(i32x8 _mm256_min_epu32 _mm256_max_epu32 _mm256_min_epi32 _mm256_max_epi32 "avx2", "avx2");

macro_rules! impl_i8x32_horizontal_minmax {
    ($name:ident, $value:expr, $out_ty:ty) => {{
        let value = $value;
        let mut value: i8x16 = value.hi().$name(value.lo()); // 32 -> 16
        value = value.as_si128().swap_halves().as_i8x16().$name(value); // 16 -> 8
        value = crate::amd64::sse::i32x4_pick!(value, [3, 2, 0, 0]).as_i8x16().$name(value); // 8 -> 4
        value = value.as_si128().unbounded_shr_by_bytes::<2>().as_i8x16().$name(value); // 4 -> 2
        value = value.as_si128().unbounded_shr_by_bytes::<1>().as_i8x16().$name(value); // 2 -> 1
        value.as_i32x4().get_0th() as $out_ty
    }};
}

impl i8x32 {
    #[target_feature(enable = "avx")]
    #[inline]
    pub fn splat(value: i8) -> Self {
        maybe_unsafe! { Self(_mm256_set1_epi8(value)) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn from_i1x32_sext(value: i32) -> Self {
        let value = !value;
        let mask = i32x8::from_array([1 << 0, 1 << 1, 1 << 2, 1 << 3, 1 << 4, 1 << 5, 1 << 6, 1 << 7]);
        let v0: i32x8 = i32x8::splat(value).and(mask);
        let v1: i32x8 = i32x8::splat((value as u32 >> 8) as i32).and(mask);
        let v2: i32x8 = i32x8::splat((value as u32 >> 16) as i32).and(mask);
        let v3: i32x8 = i32x8::splat((value as u32 >> 24) as i32).and(mask);
        i32x8::clamp_to_i8_range_and_pack(v0, v1, v2, v3).simd_eq(i8x32::zero())

        // Alternative implementation:
        //  let value = i32x8::splat(value).unbounded_shrv(i32x8::lane_indexes());
        //  let v0 = value.truncate_to_i8x8();
        //  let v1 = value.unbounded_shri::<8>().truncate_to_i8x8();
        //  let v2 = value.unbounded_shri::<16>().truncate_to_i8x8();
        //  let v3 = value.unbounded_shri::<24>().truncate_to_i8x8();
        //  i8x32::zero().sub(i64x4::from_array([v0, v1, v2, v3]).as_i8x32().and(i8x32::splat(1)))
    }

    #[target_feature(enable = "avx")]
    #[inline]
    pub fn from_array(xs: [i8; 32]) -> Self {
        let xs: [u8; 32] = unsafe { core::mem::transmute(xs) };
        si256::from_bytes(xs).as_i8x32()
    }

    #[target_feature(enable = "avx")]
    #[inline]
    pub fn lane_indexes() -> Self {
        Self::from_array([
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
        ])
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn bitwise_reduce_into_0th_lane(self) -> i8x16 {
        let mut sum = self.lo().or(self.hi()).as_si128(); // 32 -> 16
        sum = sum.or(sum.unbounded_shr_by_bytes::<8>()); // 16 -> 8
        sum = sum.or(sum.unbounded_shr_by_bytes::<4>()); // 8 -> 4
        sum = sum.or(sum.unbounded_shr_by_bytes::<2>()); // 4 -> 2
        sum = sum.or(sum.unbounded_shr_by_bytes::<1>()); // 2 -> 1
        sum.as_i8x16()
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn bitwise_reduce(self) -> i8 {
        self.bitwise_reduce_into_0th_lane().get::<0>()
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn horizontal_max_signed(self) -> i8 {
        impl_i8x32_horizontal_minmax!(max_signed, self, i8)
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn horizontal_max_unsigned(self) -> u8 {
        impl_i8x32_horizontal_minmax!(max_unsigned, self, u8)
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn horizontal_min_signed(self) -> i8 {
        impl_i8x32_horizontal_minmax!(min_signed, self, i8)
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn horizontal_min_unsigned(self) -> u8 {
        impl_i8x32_horizontal_minmax!(min_unsigned, self, u8)
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn most_significant_bits(self) -> i32 {
        maybe_unsafe! { _mm256_movemask_epi8(self.0) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn mul_by_sign_of(self, rhs: Self) -> Self {
        maybe_unsafe! { Self(_mm256_sign_epi8(self.0, rhs.0)) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn conditional_assign(self, rhs: Self, should_pick_rhs: Self) -> Self {
        maybe_unsafe! { Self(_mm256_blendv_epi8(self.0, rhs.0, should_pick_rhs.0)) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn saturating_add(self, rhs: Self) -> Self {
        maybe_unsafe! { Self(_mm256_adds_epi8(self.0, rhs.0)) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn saturating_add_unsigned(self, rhs: Self) -> Self {
        maybe_unsafe! { Self(_mm256_adds_epu8(self.0, rhs.0)) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn saturating_sub(self, rhs: Self) -> Self {
        maybe_unsafe! { Self(_mm256_subs_epi8(self.0, rhs.0)) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn saturating_sub_unsigned(self, rhs: Self) -> Self {
        maybe_unsafe! { Self(_mm256_subs_epu8(self.0, rhs.0)) }
    }

    #[must_use]
    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn get<const INDEX: i32>(self) -> i8 {
        let value: i32 = maybe_unsafe! { _mm256_extract_epi8(self.0, INDEX) };
        value as i8
    }

    #[must_use]
    #[target_feature(enable = "avx")]
    #[inline]
    pub fn set<const INDEX: i32>(self, value: i8) -> Self {
        maybe_unsafe! { Self(_mm256_insert_epi8(self.0, value, INDEX)) }
    }

    #[target_feature(enable = "avx")]
    #[inline]
    pub fn lo(self) -> i8x16 {
        i8x16(self.as_si256().lo().0)
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn hi(self) -> i8x16 {
        i8x16(self.as_si256().hi().0)
    }
}

impl i16x16 {
    #[target_feature(enable = "avx")]
    #[inline]
    pub fn splat(value: i16) -> Self {
        maybe_unsafe! { Self(_mm256_set1_epi16(value)) }
    }

    #[target_feature(enable = "avx")]
    #[inline]
    pub fn from_array(xs: [i16; 16]) -> Self {
        maybe_unsafe! {
            Self(_mm256_setr_epi16(
                xs[0], xs[1], xs[2], xs[3], xs[4], xs[5], xs[6], xs[7], xs[8], xs[9], xs[10], xs[11], xs[12], xs[13], xs[14], xs[15],
            ))
        }
    }

    #[target_feature(enable = "avx")]
    #[inline]
    pub fn lane_indexes() -> Self {
        Self::from_array([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15])
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn horizontal_reduce_i16x8x2(self, rhs: Self) -> Self {
        maybe_unsafe! {
            Self(_mm256_hadd_epi16(self.0, rhs.0))
        }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn clamp_to_i8_range_and_pack_i8x16x2(self, rhs: Self) -> i8x32 {
        maybe_unsafe! {
            i8x32(_mm256_packs_epi16(self.0, rhs.0))
        }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn clamp_to_u8_range_and_pack_i8x16x2(self, rhs: Self) -> i8x32 {
        maybe_unsafe! {
            i8x32(_mm256_packus_epi16(self.0, rhs.0))
        }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn clamp_to_i8_range(self) -> i8x16 {
        let value = self.clamp_to_i8_range_and_pack_i8x16x2(Self::zero()).as_i64x4();
        i64x4_pick!(value[0, 2, 1, 1]).as_i8x32().lo()
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn clamp_to_u8_range(self) -> i8x16 {
        let value = self.clamp_to_u8_range_and_pack_i8x16x2(Self::zero()).as_i64x4();
        i64x4_pick!(value[0, 2, 1, 1]).as_i8x32().lo()
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn wrapping_reduce(self) -> i16 {
        let mut sum = self.horizontal_reduce_i16x8x2(self); // 16 -> 8
        sum = sum.horizontal_reduce_i16x8x2(sum); // 8 -> 4
        sum = sum.horizontal_reduce_i16x8x2(sum); // 4 -> 2
        sum.lo().add(sum.hi()).get::<0>()
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn bitwise_reduce_into_0th_lane(self) -> i16x8 {
        let mut sum = self.lo().or(self.hi()).as_si128(); // 16 -> 8
        sum = sum.or(sum.unbounded_shr_by_bytes::<8>()); // 8 -> 4
        sum = sum.or(sum.unbounded_shr_by_bytes::<4>()); // 4 -> 2
        sum = sum.or(sum.unbounded_shr_by_bytes::<2>()); // 2 -> 1
        sum.as_i16x8()
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn bitwise_reduce(self) -> i16 {
        self.bitwise_reduce_into_0th_lane().get::<0>()
    }

    #[must_use]
    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn get<const INDEX: i32>(self) -> i16 {
        let value: i32 = maybe_unsafe! { _mm256_extract_epi16(self.0, INDEX) };
        value as i16
    }

    #[must_use]
    #[target_feature(enable = "avx")]
    #[inline]
    pub fn set<const INDEX: i32>(self, value: i16) -> Self {
        maybe_unsafe! { Self(_mm256_insert_epi16(self.0, value, INDEX)) }
    }

    #[target_feature(enable = "avx")]
    #[inline]
    pub fn lo(self) -> i16x8 {
        self.as_si256().lo().as_i16x8()
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn hi(self) -> i16x8 {
        self.as_si256().hi().as_i16x8()
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn unbounded_shri<const SHIFT: i32>(self) -> Self {
        maybe_unsafe! { Self(_mm256_srli_epi16(self.0, SHIFT)) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn unbounded_shli<const SHIFT: i32>(self) -> Self {
        maybe_unsafe! { Self(_mm256_slli_epi16(self.0, SHIFT)) }
    }
}

impl i32x8 {
    #[target_feature(enable = "avx")]
    #[inline]
    pub fn splat(value: i32) -> Self {
        maybe_unsafe! { Self(_mm256_set1_epi32(value)) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn from_i1x8_sext(value: i8) -> Self {
        Self::zero().sub(
            Self::splat(value as u32 as i32)
                .unbounded_shrv(Self::lane_indexes())
                .and(Self::splat(1)),
        )
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn from_i8x8_sext(value: i64) -> Self {
        i64x2::splat(value).as_i8x16().lo_i8x8_to_i32x8_sext()
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn pick(self, lanes: i32x8) -> Self {
        maybe_unsafe! {
            Self(_mm256_permutevar8x32_epi32(self.0, lanes.0))
        }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn truncate_to_i16x8(self) -> i16x8 {
        // Initial state (i8x32):
        //   V U T S | R Q P O || N M L K | J I H G ||| F E D C | B A 9 8 || 7 6 5 4 | 3 2 1 0
        // We only care about:
        //   _ _ T S | _ _ P O || _ _ L K | _ _ H G ||| _ _ D C | _ _ 9 8 || _ _ 5 4 | _ _ 1 0
        // Shuffle into:
        //   _ _ _ _ | _ _ _ _ || T S P O | L K H G ||| _ _ _ _ | _ _ _ _ || D C 9 8 | 5 4 1 0
        let local_mask = const { i64::from_le_bytes([0, 1, 4, 5, 8, 9, 12, 13]) };
        let mask = i64x4::splat(local_mask);
        let value = self.as_i128x2().shuffle_i8x16(mask.as_i8x32());
        value
            .hi()
            .unbounded_shl_by_bytes::<8>()
            .or(value.lo().with_upper_i64_clear())
            .as_i16x8()
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn truncate_to_i8x8(self) -> i64 {
        // Initial state (i8x32):
        //   V U T S | R Q P O || N M L K | J I H G ||| F E D C | B A 9 8 || 7 6 5 4 | 3 2 1 0
        // We only care about:
        //   _ _ _ S | _ _ _ O || _ _ _ K | _ _ _ G ||| _ _ _ C | _ _ _ 8 || _ _ _ 4 | _ _ _ 0
        // Shuffle into:
        //   _ _ _ _ | _ _ _ _ || _ _ _ _ | S O K G ||| _ _ _ _ | _ _ _ _ || _ _ _ _ | C 8 4 0
        let local_mask = const { i32::from_le_bytes([0, 4, 8, 12]) };
        let mask = i32x8::splat(-1).set::<0>(local_mask).set::<4>(local_mask).as_i8x32();
        let value = self.as_i128x2().shuffle_i8x16(mask);
        value.lo().lo() | (value.hi().lo() << 32)
        // Alternative implementation:
        //   (value.lo().or(value.hi().unbounded_shl_by_bytes::<4>())).lo()
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn clamp_to_i16_range_and_pack_i16x8x2(self, rhs: Self) -> i16x16 {
        maybe_unsafe! {
            i16x16(_mm256_packs_epi32(self.0, rhs.0))
        }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn clamp_to_u16_range_and_pack_i16x8x2(self, rhs: Self) -> i16x16 {
        maybe_unsafe! {
            i16x16(_mm256_packus_epi32(self.0, rhs.0))
        }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn clamp_to_i16_range(self) -> i16x8 {
        let value = self.clamp_to_i16_range_and_pack_i16x8x2(Self::zero()).as_i64x4();
        i64x4_pick!(value[0, 2, 1, 1]).as_i16x16().lo()
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn clamp_to_i8_range_and_pack(v0: Self, v1: Self, v2: Self, v3: Self) -> i8x32 {
        let a: i16x16 = v0.clamp_to_i16_range_and_pack_i16x8x2(v1);
        let b: i16x16 = v2.clamp_to_i16_range_and_pack_i16x8x2(v3);
        let v: i8x32 = a.clamp_to_i8_range_and_pack_i8x16x2(b);
        let v = v.as_i32x8();
        i32x8_pick!(v[0, 4, 1, 5, 2, 6, 3, 7]).as_i8x32()
    }

    #[target_feature(enable = "avx")]
    #[inline]
    pub fn from_array(xs: [i32; 8]) -> Self {
        maybe_unsafe! { Self(_mm256_setr_epi32(xs[0], xs[1], xs[2], xs[3], xs[4], xs[5], xs[6], xs[7])) }
    }

    #[target_feature(enable = "avx")]
    #[inline]
    pub fn lane_indexes() -> Self {
        Self::from_array([0, 1, 2, 3, 4, 5, 6, 7])
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn bitwise_reduce_into_0th_lane(self) -> i32x4 {
        let mut sum = self.lo().or(self.hi()).as_si128(); // 8 -> 4
        sum = sum.or(sum.unbounded_shr_by_bytes::<8>()); // 4 -> 2
        sum = sum.or(sum.unbounded_shr_by_bytes::<4>()); // 2 -> 1
        sum.as_i32x4()
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn bitwise_reduce(self) -> i32 {
        self.bitwise_reduce_into_0th_lane().get::<0>()
    }

    #[must_use]
    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn get<const INDEX: i32>(self) -> i32 {
        maybe_unsafe! { _mm256_extract_epi32(self.0, INDEX) }
    }

    #[must_use]
    #[target_feature(enable = "avx")]
    #[inline]
    pub fn set<const INDEX: i32>(self, value: i32) -> Self {
        maybe_unsafe! { Self(_mm256_insert_epi32(self.0, value, INDEX)) }
    }

    #[target_feature(enable = "avx")]
    #[inline]
    pub fn lo(self) -> i32x4 {
        self.as_si256().lo().as_i32x4()
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn hi(self) -> i32x4 {
        self.as_si256().hi().as_i32x4()
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn unbounded_shrv(self, shifts: Self) -> Self {
        maybe_unsafe! { Self(_mm256_srlv_epi32(self.0, shifts.0)) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn unbounded_shlv(self, shifts: Self) -> Self {
        maybe_unsafe! { Self(_mm256_sllv_epi32(self.0, shifts.0)) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn unbounded_shri<const SHIFT: i32>(self) -> Self {
        maybe_unsafe! { Self(_mm256_srli_epi32(self.0, SHIFT)) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn unbounded_shli<const SHIFT: i32>(self) -> Self {
        maybe_unsafe! { Self(_mm256_slli_epi32(self.0, SHIFT)) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub unsafe fn masked_store_raw(self, dst: *mut i32, mask: Self) {
        unsafe { _mm256_maskstore_epi32(dst, mask.0, self.0) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn masked_store(self, dst: &mut [i32; 8], mask: Self) {
        unsafe { self.masked_store_raw(dst.as_mut_ptr(), mask) }
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! _i32x8_pick {
    ($value:ident[$l0:expr, $l1:expr, $l2:expr, $l3:expr, $l4:expr, $l5:expr, $l6:expr, $l7:expr]) => {
        $crate::amd64::avx2::i32x8_pick!($value, [$l0, $l1, $l2, $l3, $l4, $l5, $l6, $l7])
    };

    ($value:expr, [$l0:expr, $l1:expr, $l2:expr, $l3:expr, $l4:expr, $l5:expr, $l6:expr, $l7:expr]) => {{
        let _: () = {
            assert!($l0 < 8);
            assert!($l1 < 8);
            assert!($l2 < 8);
            assert!($l3 < 8);
            assert!($l4 < 8);
            assert!($l5 < 8);
            assert!($l6 < 8);
            assert!($l7 < 8);
        };

        let l0: i32 = $l0;
        let l1: i32 = $l1;
        let l2: i32 = $l2;
        let l3: i32 = $l3;
        let l4: i32 = $l4;
        let l5: i32 = $l5;
        let l6: i32 = $l6;
        let l7: i32 = $l7;

        let input: $crate::amd64::avx2::i32x8 = $value;
        let output = if l0 < 4 && l1 < 4 && l2 < 4 && l3 < 4 && l0 + 4 == l4 && l1 + 4 == l5 && l2 + 4 == l6 && l3 + 4 == l7 {
            // This is faster but it cannot work for arbitrary permutations.
            input
                .as_i128x2()
                .shuffle_i32x4::<{ (($l0) | ($l1 << 2) | ($l2 << 4) | ($l3 << 6)) & 0xff }>()
                .as_i32x8()
        } else {
            input.pick($crate::amd64::avx2::i32x8::from_array([l0, l1, l2, l3, l4, l5, l6, l7]))
        };
        output
    }};
}

pub use _i32x8_pick as i32x8_pick;

#[doc(hidden)]
#[macro_export]
macro_rules! _i64x4_pick {
    ($value:ident[$l0:expr, $l1:expr, $l2:expr, $l3:expr]) => {
        $crate::amd64::avx2::i64x4_pick!($value, [$l0, $l1, $l2, $l3])
    };

    ($value:expr, [$l0:expr, $l1:expr, $l2:expr, $l3:expr]) => {{
        let _: () = {
            assert!($l0 < 4);
            assert!($l1 < 4);
            assert!($l2 < 4);
            assert!($l3 < 4);
        };

        let input: $crate::amd64::avx2::i64x4 = $value;
        let output = input._pick::<{ $l0 | ($l1 << 2) | ($l2 << 4) | ($l3 << 6) }>();
        output
    }};
}

pub use _i64x4_pick as i64x4_pick;

impl i64x4 {
    #[target_feature(enable = "avx")]
    #[inline]
    pub fn splat(value: i64) -> Self {
        maybe_unsafe! { Self(_mm256_set1_epi64x(value)) }
    }

    #[target_feature(enable = "avx")]
    #[inline]
    pub fn from_array(xs: [i64; 4]) -> Self {
        maybe_unsafe! { Self(_mm256_setr_epi64x(xs[0], xs[1], xs[2], xs[3])) }
    }

    #[target_feature(enable = "avx")]
    #[inline]
    pub fn lane_indexes() -> Self {
        Self::from_array([0, 1, 2, 3])
    }

    #[doc(hidden)]
    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn _pick<const LANES: i32>(self) -> Self {
        assert_eq!(LANES as u32 & (!0xff_u32), 0);

        maybe_unsafe! {
            Self(_mm256_permute4x64_epi64::<LANES>(self.0))
        }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn bitwise_reduce_into_0th_lane(self) -> i64x2 {
        let mut sum = self.lo().or(self.hi()).as_si128(); // 4 -> 2
        sum = sum.or(sum.unbounded_shr_by_bytes::<8>()); // 2 -> 1
        sum.as_i64x2()
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn bitwise_reduce(self) -> i64 {
        self.bitwise_reduce_into_0th_lane().get::<0>()
    }

    #[must_use]
    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn get<const INDEX: i32>(self) -> i64 {
        maybe_unsafe! { _mm256_extract_epi64(self.0, INDEX) }
    }

    #[must_use]
    #[target_feature(enable = "avx")]
    #[inline]
    pub fn set<const INDEX: i32>(self, value: i64) -> Self {
        maybe_unsafe! { Self(_mm256_insert_epi64(self.0, value, INDEX)) }
    }

    #[target_feature(enable = "avx")]
    #[inline]
    pub fn lo(self) -> i64x2 {
        self.as_si256().lo().as_i64x2()
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn hi(self) -> i64x2 {
        self.as_si256().hi().as_i64x2()
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn unbounded_shrv(self, shifts: Self) -> Self {
        maybe_unsafe! { Self(_mm256_srlv_epi64(self.0, shifts.0)) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn unbounded_shlv(self, shifts: Self) -> Self {
        maybe_unsafe! { Self(_mm256_sllv_epi64(self.0, shifts.0)) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn unbounded_shri<const SHIFT: i32>(self) -> Self {
        maybe_unsafe! { Self(_mm256_srli_epi64(self.0, SHIFT)) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn unbounded_shli<const SHIFT: i32>(self) -> Self {
        maybe_unsafe! { Self(_mm256_slli_epi64(self.0, SHIFT)) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub unsafe fn masked_store_raw(self, dst: *mut i64, mask: Self) {
        unsafe { _mm256_maskstore_epi64(dst, mask.0, self.0) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn masked_store(self, dst: &mut [i64; 4], mask: Self) {
        unsafe { self.masked_store_raw(dst.as_mut_ptr(), mask) }
    }
}

// -----------------------------
// SSE interoperability and extensions
// -----------------------------

impl si128 {
    #[target_feature(enable = "avx")]
    #[inline]
    pub fn as_si256(self) -> si256 {
        maybe_unsafe! { si256(_mm256_castsi128_si256(self.0)) }
    }

    #[target_feature(enable = "avx")]
    #[inline]
    pub fn to_si256_zext(self) -> si256 {
        maybe_unsafe! { si256(_mm256_zextsi128_si256(self.0)) }
    }
}

impl i8x16 {
    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn from_i1x16_sext(value: i16) -> Self {
        let value = i16x16::splat(!value);
        let mask = i16x16::from_array([
            1 << 0,
            1 << 1,
            1 << 2,
            1 << 3,
            1 << 4,
            1 << 5,
            1 << 6,
            1 << 7,
            1 << 8,
            1 << 9,
            1 << 10,
            1 << 11,
            1 << 12,
            1 << 13,
            1 << 14,
            1 << 15,
        ]);
        let value: i16x16 = value.and(mask).simd_eq(i16x16::zero());
        let value: i8x16 = value.clamp_to_i8_range();
        value

        // Alternative implementation:
        //  let value = i16x16::splat(value);
        //  let lo = i32x8::from_array([0, 1, 2, 3, 8, 9, 10, 11]);
        //  let hi = i32x8::from_array([4, 5, 6, 7, 12, 13, 14, 15]);
        //  let lo = value.lo().to_i32x8_sext().unbounded_shrv(lo).and(i32x8::splat(1)).simd_eq(i32x8::splat(1));
        //  let hi = value.hi().to_i32x8_sext().unbounded_shrv(hi).and(i32x8::splat(1)).simd_eq(i32x8::splat(1));
        //  let value: i16x16 = lo.clamp_to_i16_range_and_pack_i16x8x2(hi);
        //  let value = value.lo().clamp_to_i8_range_and_pack(value.hi());
        //  value

        // Alternative implementation:
        //  let value = i16x16::splat(value);
        //  let lo: i32x8 = value.lo().to_i32x8_sext().unbounded_shrv(i32x8::lane_indexes()).and(i32x8::splat(1)).simd_eq(i32x8::splat(1));
        //  let hi: i32x8 = value.hi().to_i32x8_sext().unbounded_shrv(i32x8::lane_indexes().add(i32x8::splat(8))).and(i32x8::splat(1)).simd_eq(i32x8::splat(1));
        //  let lo = lo.truncate_to_i8x8();
        //  let hi = hi.truncate_to_i8x8();
        //  i64x2::from_array([lo, hi]).as_i8x16()
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn to_i16x16_sext(self) -> i16x16 {
        maybe_unsafe! { i16x16(_mm256_cvtepi8_epi16(self.0)) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn lo_i8x8_to_i32x8_sext(self) -> i32x8 {
        maybe_unsafe! {
            i32x8(_mm256_cvtepi8_epi32(self.0))
        }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn hi_i8x8_to_i32x8_sext(self) -> i32x8 {
        self.as_si128().unbounded_shr_by_bytes::<8>().as_i8x16().lo_i8x8_to_i32x8_sext()
    }
}

impl i16x8 {
    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn to_i32x8_sext(self) -> i32x8 {
        maybe_unsafe! { i32x8(_mm256_cvtepi16_epi32(self.0)) }
    }
}

impl i32x4 {
    #[target_feature(enable = "avx2")]
    #[inline]
    pub unsafe fn masked_store_raw(self, dst: *mut i32, mask: Self) {
        unsafe { _mm_maskstore_epi32(dst, mask.0, self.0) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn masked_store(self, dst: &mut [i32; 4], mask: Self) {
        unsafe { self.masked_store_raw(dst.as_mut_ptr(), mask) }
    }
}

impl i64x2 {
    #[target_feature(enable = "avx2")]
    #[inline]
    pub unsafe fn masked_store_raw(self, dst: *mut i64, mask: Self) {
        unsafe { _mm_maskstore_epi64(dst, mask.0, self.0) }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn masked_store(self, dst: &mut [i64; 2], mask: Self) {
        unsafe { self.masked_store_raw(dst.as_mut_ptr(), mask) }
    }
}

#[cfg(all(feature = "std", test))]
mod tests {
    use super::*;
    use crate::test_utils::*;

    #[test]
    fn basic() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            assert!(i8x32::zero().to_bytes().iter().all(|&byte| byte == 0));
            assert!(i8x32::splat(0).to_bytes().iter().all(|&byte| byte == 0));
            assert!(i8x32::splat(1).to_bytes().iter().all(|&byte| byte == 1));
            assert_eq!(i32x8::splat(0x12345678).as_i8x32().get::<0>(), 0x78);
            assert_eq!(i32x8::splat(0x12345678).as_i8x32().get::<3>(), 0x12);
            assert_eq!(i32x8::splat(0x12345678).as_i8x32().get::<4>(), 0x78);
            assert_eq!(i32x8::splat(0x12345678).set::<0>(0xff).to_bytes()[0], 0xff);
            assert_eq!(i32x8::splat(0x12345678).set::<0>(0xff).to_array()[0], 0xff);
            assert!(i32x8::splat(0x12345678).is_equal(i32x8::splat(0x12345678)));
            assert!(!i32x8::splat(0x12345678).is_equal(i32x8::splat(0x12345678).set::<0>(0xff)));
            assert!(!i8x32::zero().set::<0>(-1).is_equal(i8x32::zero()));

            let mut bytes = [0; 32];
            bytes[0] = 0x77;
            assert_eq!(i8x32::from_array(bytes).get::<0>(), 0x77);

            let array = [
                0x5668ba8fd8dff557_u64 as i64,
                0x27d0383ac5820461_u64 as i64,
                0x6fecdef042b0ecb5_u64 as i64,
                0xea03afd32c503a5a_u64 as i64,
            ];
            let value = i64x4::load_unaligned(array.as_ptr().cast());
            assert_eq!(
                value.as_si256().lo().as_i64x2().to_array(),
                i64x2::from_array([array[0], array[1]]).to_array()
            );
            assert_eq!(value.as_si256().lo().as_i64x2().get::<0>() as u64, 0x5668ba8fd8dff557);

            assert_eq!(i8x32::splat(1).and(i8x32::zero().set::<0>(0xff_u8 as i8)).get::<0>(), 1);
            assert_eq!(i8x32::splat(1).and(i8x32::zero().set::<0>(0xff_u8 as i8)).get::<1>(), 0);
            assert_eq!(i8x32::splat(1).and_not(i8x32::zero().set::<0>(0xff_u8 as i8)).get::<0>(), 0);
            assert_eq!(i8x32::splat(1).and_not(i8x32::zero().set::<0>(0xff_u8 as i8)).get::<1>(), 1);

            #[cfg(feature = "ops")]
            {
                let mut value = i8x32::splat(3);
                value += i8x32::splat(7);
                assert_eq!(value.to_array(), [10; 32]);
            }

            assert_eq!(i32x4::from_array(test_array_i32x4()).get_0th(), test_array_i32x4()[0])
        }
    }

    #[test]
    fn i8x32_most_significant_bits() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            assert_eq!(i8x32::splat(-1).most_significant_bits(), 0xffffffff_u32 as i32);
            assert_eq!(i8x32::splat(0).most_significant_bits(), 0);
            assert_eq!(
                i8x32::from_array(test_array_i8x32()).most_significant_bits(),
                crate::fallback::i8x32::from_array(test_array_i8x32()).most_significant_bits(),
            );
        }
    }

    #[test]
    fn wrapping_reduce_i16x16() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        let mut rng = oorandom::Rand64::new(389476348975);
        unsafe {
            for _ in 0..32 {
                let mut array = [0_i16; 16];
                for _ in 0..32 {
                    array[rng.rand_range(0..16) as usize] = rng.rand_i64() as i16;
                    assert_eq!(
                        i16x16::from_array(array).wrapping_reduce(),
                        array.iter().copied().fold(0_i16, |a, b| a.wrapping_add(b))
                    );
                }
            }
        }
    }

    #[test]
    fn bitwise_reduce_i8x32() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        let a = 0b10101010_u8 as i8;
        let b = 0b01010101_u8 as i8;
        unsafe {
            for n in 0..32 {
                for m in 0..32 {
                    let mut array = [0_i8; 32];
                    array[n] = a;
                    array[m] = b;
                    let value = i8x32::from_array(array);
                    assert_eq!(
                        std::format!("{:08b}", value.bitwise_reduce()),
                        std::format!("{:08b}", if n == m { b } else { a | b }),
                        "failed for n = {n}, m = {m}"
                    );
                }
            }
        }
    }

    #[test]
    fn bitwise_reduce_i16x16() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        let a = 0b1010101010101010_u16 as i16;
        let b = 0b0101010101010101_u16 as i16;
        unsafe {
            for n in 0..16 {
                for m in 0..16 {
                    let mut array = [0_i16; 16];
                    array[n] = a;
                    array[m] = b;
                    let value = i16x16::from_array(array);
                    assert_eq!(
                        std::format!("{:016b}", value.bitwise_reduce()),
                        std::format!("{:016b}", if n == m { b } else { a | b }),
                        "failed for n = {n}, m = {m}"
                    );
                }
            }
        }
    }

    #[test]
    fn bitwise_reduce_i32x8() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        let a = 0b10101010101010101010101010101010_u32 as i32;
        let b = 0b01010101010101010101010101010101_u32 as i32;
        unsafe {
            for n in 0..8 {
                for m in 0..8 {
                    let mut array = [0_i32; 8];
                    array[n] = a;
                    array[m] = b;
                    let value = i32x8::from_array(array);
                    assert_eq!(
                        std::format!("{:032b}", value.bitwise_reduce()),
                        std::format!("{:032b}", if n == m { b } else { a | b }),
                        "failed for n = {n}, m = {m}"
                    );
                }
            }
        }
    }

    #[test]
    fn bitwise_reduce_i64x4() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        let a = 0b1010101010101010101010101010101010101010101010101010101010101010_u64 as i64;
        let b = 0b010101010101010101010101010101011010101010101010101010101010101_u64 as i64;
        unsafe {
            for n in 0..4 {
                for m in 0..4 {
                    let mut array = [0_i64; 4];
                    array[n] = a;
                    array[m] = b;
                    let value = i64x4::from_array(array);
                    assert_eq!(
                        std::format!("{:064b}", value.bitwise_reduce()),
                        std::format!("{:064b}", if n == m { b } else { a | b }),
                        "failed for n = {n}, m = {m}"
                    );
                }
            }
        }
    }

    #[test]
    fn i32x8_from_i1x8_sext() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            assert_eq!(i32x8::from_i1x8_sext(!0).to_array(), [-1; 8]);
            assert_eq!(i32x8::from_i1x8_sext(0).to_array(), [0; 8]);
            for packed_mask in test_array_i8x32() {
                assert_eq!(
                    i32x8::from_i1x8_sext(packed_mask).to_array(),
                    crate::fallback::i32x8::from_i1x8_sext(packed_mask).to_array()
                );
            }
            for n in 0..8 {
                let mask = (1_u8 << n) as i8;
                let value = i32x8::from_i1x8_sext(mask).to_array();
                for m in 0..8 {
                    if n == m {
                        assert_eq!(value[m], 0xffffffff_u32 as i32);
                    } else {
                        assert_eq!(value[m], 0);
                    }
                }
            }
        }
    }

    #[test]
    fn i8x32_from_i1x32_sext() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            assert!(i8x32::from_i1x32_sext(0).to_array().into_iter().all(|x| x == 0));
            assert!(i8x32::from_i1x32_sext(-1).to_array().into_iter().all(|x| x == -1));
            for packed_mask in test_array_i32x32() {
                assert_eq!(
                    i8x32::from_i1x32_sext(packed_mask).to_array(),
                    crate::fallback::i8x32::from_i1x32_sext(packed_mask).to_array()
                );
            }
            for n in 0..32 {
                let mask = (1_u32 << n) as i32;
                let value = i8x32::from_i1x32_sext(mask).to_array();
                for m in 0..32 {
                    if n == m {
                        assert_eq!(value[m], 0xff_u8 as i8);
                    } else {
                        assert_eq!(value[m], 0);
                    }
                }
            }
        }
    }

    #[test]
    fn i16x16_and_i8x16() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            let value = i16x16::from_array(test_array_i16x16());
            assert_eq!(value.to_array(), test_array_i16x16());
            let out: i16x16 = value.and(i8x16::from_array([0_i8; 16]).to_i16x16_sext());
            assert!(out.to_array().iter().all(|&x| x == 0));

            for n in 0..16 {
                let mut mask = [0_i8; 16];
                mask[n] = 0xff_u8 as i8;
                let out: i16x16 = value & i8x16::from_array(mask).to_i16x16_sext();
                assert_eq!(out.to_array()[n], test_array_i16x16()[n]);
                for m in 0..16 {
                    if n != m {
                        assert_eq!(out.to_array()[m], 0);
                    }
                }
            }
        }
    }

    #[test]
    fn i64x4_pick_macro() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            let xs = test_array_i64x4();
            let ys = i64x4::from_array(xs);
            assert_eq!(i64x4_pick!(ys[0, 0, 0, 0]).to_array(), [xs[0], xs[0], xs[0], xs[0]]);
            assert_eq!(i64x4_pick!(ys[1, 1, 1, 1]).to_array(), [xs[1], xs[1], xs[1], xs[1]]);
            assert_eq!(i64x4_pick!(ys[0, 1, 2, 3]).to_array(), [xs[0], xs[1], xs[2], xs[3]]);
            assert_eq!(i64x4_pick!(ys[3, 2, 1, 0]).to_array(), [xs[3], xs[2], xs[1], xs[0]]);
        }
    }

    #[test]
    fn i32x8_pick_macro() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            let xs = test_array_i32x8();
            let ys = i32x8::from_array(xs);
            assert_eq!(ys.to_array(), xs);

            // Fast path.
            assert_eq!(
                i32x8_pick!(ys[0, 0, 0, 0, 4, 4, 4, 4]).to_array(),
                [xs[0], xs[0], xs[0], xs[0], xs[4], xs[4], xs[4], xs[4]]
            );
            assert_eq!(
                i32x8_pick!(ys[1, 1, 1, 1, 5, 5, 5, 5]).to_array(),
                [xs[1], xs[1], xs[1], xs[1], xs[5], xs[5], xs[5], xs[5]]
            );
            assert_eq!(
                i32x8_pick!(ys[3, 3, 3, 3, 7, 7, 7, 7]).to_array(),
                [xs[3], xs[3], xs[3], xs[3], xs[7], xs[7], xs[7], xs[7]]
            );
            assert_eq!(
                i32x8_pick!(ys[0, 1, 2, 3, 4, 5, 6, 7]).to_array(),
                [xs[0], xs[1], xs[2], xs[3], xs[4], xs[5], xs[6], xs[7]]
            );

            // Normal path.
            assert_eq!(
                i32x8_pick!(ys[0, 0, 0, 0, 0, 0, 0, 0]).to_array(),
                [xs[0], xs[0], xs[0], xs[0], xs[0], xs[0], xs[0], xs[0]]
            );
            assert_eq!(
                i32x8_pick!(ys[1, 1, 1, 1, 1, 1, 1, 1]).to_array(),
                [xs[1], xs[1], xs[1], xs[1], xs[1], xs[1], xs[1], xs[1]]
            );
            assert_eq!(
                i32x8_pick!(ys[0, 0, 0, 0, 5, 5, 5, 5]).to_array(),
                [xs[0], xs[0], xs[0], xs[0], xs[5], xs[5], xs[5], xs[5]]
            );
            assert_eq!(
                i32x8_pick!(ys[0, 0, 0, 0, 7, 7, 7, 7]).to_array(),
                [xs[0], xs[0], xs[0], xs[0], xs[7], xs[7], xs[7], xs[7]]
            );
            assert_eq!(
                i32x8_pick!(ys[4, 4, 4, 4, 0, 0, 0, 0]).to_array(),
                [xs[4], xs[4], xs[4], xs[4], xs[0], xs[0], xs[0], xs[0]]
            );
            assert_eq!(
                i32x8_pick!(ys[7, 6, 5, 4, 3, 2, 1, 0]).to_array(),
                [xs[7], xs[6], xs[5], xs[4], xs[3], xs[2], xs[1], xs[0]]
            );
            assert_eq!(
                i32x8_pick!(ys[0, 7, 0, 7, 0, 7, 0, 7]).to_array(),
                [xs[0], xs[7], xs[0], xs[7], xs[0], xs[7], xs[0], xs[7]]
            );
            assert_eq!(
                i32x8_pick!(ys[7, 0, 7, 0, 7, 0, 7, 0]).to_array(),
                [xs[7], xs[0], xs[7], xs[0], xs[7], xs[0], xs[7], xs[0]]
            );
            assert_eq!(
                i32x8_pick!(ys[7, 7, 7, 7, 0, 0, 0, 0]).to_array(),
                [xs[7], xs[7], xs[7], xs[7], xs[0], xs[0], xs[0], xs[0]]
            );
        }
    }

    #[test]
    fn i32x8_pick() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        let mut rng = oorandom::Rand64::new(389476348975);
        unsafe {
            let xs = test_array_i32x8();
            let ys = i32x8::from_array(xs);
            for _ in 0..128 {
                let i0 = rng.rand_range(0..8) as i32;
                let i1 = rng.rand_range(0..8) as i32;
                let i2 = rng.rand_range(0..8) as i32;
                let i3 = rng.rand_range(0..8) as i32;
                let i4 = rng.rand_range(0..8) as i32;
                let i5 = rng.rand_range(0..8) as i32;
                let i6 = rng.rand_range(0..8) as i32;
                let i7 = rng.rand_range(0..8) as i32;
                assert_eq!(
                    ys.pick(i32x8::from_array([i0, i1, i2, i3, i4, i5, i6, i7])).to_array(),
                    [
                        xs[i0 as usize],
                        xs[i1 as usize],
                        xs[i2 as usize],
                        xs[i3 as usize],
                        xs[i4 as usize],
                        xs[i5 as usize],
                        xs[i6 as usize],
                        xs[i7 as usize]
                    ]
                );
            }

            assert_eq!(
                ys.pick(i32x8::from_array([8, 9, 10, 11, 12, 13, 14, 15])).to_array(),
                [xs[0], xs[1], xs[2], xs[3], xs[4], xs[5], xs[6], xs[7]]
            );
        }
    }

    #[test]
    fn i32x8_truncate_to_i16x8() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            assert_eq!(
                i32x8::from_array(test_array_i32x8()).truncate_to_i16x8().to_array(),
                test_array_i32x8().map(|value| value as i16)
            );
        }
    }

    #[test]
    fn i32x8_truncate_to_i8x8() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            assert_eq!(
                i32x8::from_array(test_array_i32x8()).truncate_to_i8x8().to_le_bytes(),
                test_array_i32x8().map(|value| value as u8)
            );
        }
    }

    #[test]
    fn i64x4_set_dynamic() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            for n in 0..4_u64 {
                let value = i64x4::from_array(test_array_i64x4());
                let mut expected = test_array_i64x4();
                expected[n as usize] = 5843934634634;
                assert_eq!(value.set_dynamic(n, 5843934634634).to_array(), expected, "failed for index {n}");
            }
        }
    }

    #[test]
    fn i32x8_set_dynamic() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            for n in 0..8_u32 {
                let value = i32x8::from_array(test_array_i32x8());
                let mut expected = test_array_i32x8();
                expected[n as usize] = 58439;
                assert_eq!(value.set_dynamic(n, 58439).to_array(), expected, "failed for index {n}");
            }
        }
    }

    #[test]
    fn i16x16_set_dynamic() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            for n in 0..16_u16 {
                let value = i16x16::from_array(test_array_i16x16());
                let mut expected = test_array_i16x16();
                expected[n as usize] = 8439;
                assert_eq!(value.set_dynamic(n, 8439).to_array(), expected, "failed for index {n}");
            }
        }
    }

    #[test]
    fn i8x32_set_dynamic() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            for n in 0..64_u8 {
                let value = i8x32::from_array(test_array_i8x32());
                let mut expected = test_array_i8x32();
                if n < 32 {
                    expected[n as usize] = 39;
                }
                assert_eq!(value.set_dynamic(n, 39).to_array(), expected, "failed for index {n}");
            }
        }
    }

    #[test]
    fn i64x4_get_dynamic() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            let a = test_array_i64x4();
            let b = i64x4::from_array(a);
            assert_eq!(indexes::<4>().map(|i| b.get_dynamic(i as u64)), a);
            for n in 4..32 {
                assert_eq!(b.get_dynamic(n), 0);
            }
        }
    }

    #[test]
    fn i32x8_get_dynamic() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            let a = test_array_i32x8();
            let b = i32x8::from_array(a);
            assert_eq!(indexes::<8>().map(|i| b.get_dynamic(i as u32)), a);
        }
    }

    #[test]
    fn i16x16_get_dynamic() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            let a = test_array_i16x16();
            let b = i16x16::from_array(a);
            assert_eq!(indexes::<16>().map(|i| b.get_dynamic(i as u16)), a);
        }
    }

    #[test]
    fn i8x32_get_dynamic() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            let a = test_array_i8x32();
            let b = i8x32::from_array(a);
            assert_eq!(indexes::<32>().map(|i| b.get_dynamic(i as u8)), a);
        }
    }

    #[test]
    fn i16x16_as_slice() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            let mut a = test_array_i16x16();
            let mut b = i16x16::from_array(a);
            assert_eq!(*b.as_slice(), a);

            a[3] = 6439;
            b.as_slice_mut()[3] = 6439;
            assert_eq!(*b.as_slice(), a);
        }
    }

    #[test]
    fn masked_store_i32x8() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            let mut dst = test_array_i32x8();
            let value = i32x8::from_array(test_array_i32x8()).add(i32x8::splat(123));

            value.masked_store(&mut dst, i32x8::zero());
            assert_eq!(dst, test_array_i32x8());
            value.masked_store(&mut dst, i32x8::splat(-1));
            assert_eq!(dst, value.to_array());

            dst = test_array_i32x8();
            value.masked_store(&mut dst, i32x8::zero().set::<0>(-1).set::<1>(1));
            assert_eq!(dst[0], value.get::<0>());
            assert_eq!(dst[1..], test_array_i32x8()[1..]);
        }
    }

    #[test]
    fn i16x16_clamp_to_u8_range() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            for array in [
                test_array_i16x16(),
                test_array_i16x16().map(|x| x / 10),
                test_array_i16x16().map(|x| x / 100),
                [0_i16; 16],
                [-2; 16],
            ] {
                assert_eq!(
                    i16x16::from_array(array).clamp_to_u8_range().to_array(),
                    array.map(|value| {
                        (if value > 0xff {
                            0xff_u8
                        } else if value < 0 {
                            0
                        } else {
                            value as u8
                        }) as i8
                    })
                );
            }
        }
    }

    #[test]
    fn i16x16_clamp_to_i8_range() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            for array in [
                test_array_i16x16(),
                test_array_i16x16().map(|x| x / 10),
                test_array_i16x16().map(|x| x / 100),
                test_array_i16x16().map(|x| x / 1000),
                [0_i16; 16],
                [-2; 16],
            ] {
                assert_eq!(
                    i16x16::from_array(array).clamp_to_i8_range().to_array(),
                    crate::fallback::i16x16::from_array(array).clamp_to_i8_range().to_array()
                );
            }
        }
    }

    #[test]
    fn i32x8_clamp_to_i16_range() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            for array in [
                test_array_i32x8(),
                test_array_i32x8().map(|x| x / 10),
                test_array_i32x8().map(|x| x / 100),
                test_array_i32x8().map(|x| x / 1000),
                test_array_i32x8().map(|x| x / 10000),
                [0_i32; 8],
                [-2; 8],
            ] {
                assert_eq!(
                    i32x8::from_array(array).clamp_to_i16_range().to_array(),
                    array.map(|value| {
                        if value > i16::MAX as i32 {
                            i16::MAX
                        } else if value < i16::MIN as i32 {
                            i16::MIN
                        } else {
                            value as i16
                        }
                    })
                );
            }
        }
    }

    #[test]
    fn i8x16_from_i1x16_sext() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            assert_eq!(i8x16::from_i1x16_sext(!0).to_array(), [-1; 16]);
            assert_eq!(i8x16::from_i1x16_sext(0).to_array(), [0; 16]);
            for packed_mask in test_array_i16x32() {
                assert_eq!(
                    i8x16::from_i1x16_sext(packed_mask).to_array(),
                    crate::fallback::i8x16::from_i1x16_sext(packed_mask).to_array()
                );
            }
            for n in 0..16 {
                let mask = (1_u16 << n) as i16;
                let value = i8x16::from_i1x16_sext(mask).to_array();
                for m in 0..16 {
                    if n == m {
                        assert_eq!(value[m], 0xff_u8 as i8, "failed for n={n} m={m}");
                    } else {
                        assert_eq!(value[m], 0, "failed for n={n} m={m}");
                    }
                }
            }
        }
    }

    #[test]
    fn i8x32_horizontal_max_signed() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            assert_eq!(
                i8x32::from_array([
                    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32
                ])
                .horizontal_max_signed(),
                32
            );
            assert_eq!(
                i8x32::from_array(test_array_i8x32()).horizontal_max_signed(),
                test_array_i8x32().iter().copied().max().unwrap()
            );
        }
    }
}
