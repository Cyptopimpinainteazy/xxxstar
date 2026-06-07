use crate::amd64::avx2::{i8x32, i16x16, i32x8, i64x4, si256};

macro_rules! impl_composite {
    ($ty:ident $inner_ty:ident $lane_ty:ident 2 $lane_count:expr) => {
        #[repr(C)]
        #[derive(Copy, Clone)]
        pub struct $ty($inner_ty, $inner_ty);

        impl $ty {
            #[target_feature(enable = "avx")]
            #[inline]
            pub fn zero() -> Self {
                Self($inner_ty::zero(), $inner_ty::zero())
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn negative_one() -> Self {
                Self($inner_ty::negative_one(), $inner_ty::negative_one())
            }

            #[target_feature(enable = "avx")]
            #[inline]
            pub fn splat(value: $lane_ty) -> Self {
                let value = $inner_ty::splat(value);
                Self(value, value)
            }

            #[inline]
            pub fn as_slice(&self) -> &[$lane_ty; $lane_count] {
                unsafe { &*core::ptr::addr_of!(self.0).cast::<[$lane_ty; $lane_count]>() }
            }

            #[inline]
            pub fn as_slice_mut(&mut self) -> &mut [$lane_ty; $lane_count] {
                unsafe { &mut *core::ptr::addr_of_mut!(self.0).cast::<[$lane_ty; $lane_count]>() }
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn bitwise_reduce(self) -> $lane_ty {
                self.0.or(self.1).bitwise_reduce()
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn simd_eq(self, rhs: Self) -> Self {
                Self(self.0.simd_eq(rhs.0), self.1.simd_eq(rhs.1))
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn simd_gt(self, rhs: Self) -> Self {
                Self(self.0.simd_gt(rhs.0), self.1.simd_gt(rhs.1))
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn is_equal(self, rhs: Self) -> bool {
                self.0.is_equal(rhs.0) & self.1.is_equal(rhs.1)
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn and(self, rhs: Self) -> Self {
                Self(self.0.and(rhs.0), self.1.and(rhs.1))
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn or(self, rhs: Self) -> Self {
                Self(self.0.or(rhs.0), self.1.or(rhs.1))
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn xor(self, rhs: Self) -> Self {
                Self(self.0.xor(rhs.0), self.1.xor(rhs.1))
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn and_not(self, rhs: Self) -> Self {
                Self(self.0.and_not(rhs.0), self.1.and_not(rhs.1))
            }

            #[target_feature(enable = "avx")]
            #[inline]
            pub unsafe fn store_unaligned(self, address: *mut u8) {
                unsafe {
                    self.0.as_si256().store_unaligned(address);
                    self.1
                        .as_si256()
                        .store_unaligned(address.byte_add(core::mem::size_of::<si256>()));
                }
            }

            #[target_feature(enable = "avx")]
            #[inline]
            pub unsafe fn load_unaligned(address: *const u8) -> Self {
                const _: () = {
                    assert!(core::mem::size_of::<si256>() * 2 == core::mem::size_of::<$ty>());
                };

                unsafe {
                    Self(
                        $inner_ty(si256::load_unaligned(address).0),
                        $inner_ty(si256::load_unaligned(address.byte_add(core::mem::size_of::<si256>())).0),
                    )
                }
            }

            #[target_feature(enable = "avx")]
            #[inline]
            pub fn from_array_ref(xs: &[$lane_ty; $lane_count]) -> Self {
                const _: () = {
                    assert!(core::mem::size_of::<[$lane_ty; $lane_count]>() == core::mem::size_of::<$ty>());
                };

                unsafe { Self::load_unaligned(xs.as_ptr().cast()) }
            }

            #[target_feature(enable = "avx")]
            #[inline]
            pub fn to_array(self) -> [$lane_ty; $lane_count] {
                const _: () = {
                    assert!(core::mem::size_of::<[$lane_ty; $lane_count]>() == core::mem::size_of::<$ty>());
                };

                let mut array: core::mem::MaybeUninit<[$lane_ty; $lane_count]> = core::mem::MaybeUninit::uninit();
                unsafe {
                    self.store_unaligned(array.as_mut_ptr().cast());
                    array.assume_init()
                }
            }

            #[inline]
            pub fn hi(self) -> $inner_ty {
                self.1
            }

            #[inline]
            pub fn lo(self) -> $inner_ty {
                self.0
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn add(self, rhs: Self) -> Self {
                Self(self.0.add(rhs.0), self.1.add(rhs.1))
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn sub(self, rhs: Self) -> Self {
                Self(self.0.sub(rhs.0), self.1.sub(rhs.1))
            }
        }

        impl_bitops!($ty);
        impl_common_ops!($ty);
    };

    ($ty:ident $inner_ty:ident $lane_ty:ident 4 $lane_count:expr) => {
        #[repr(C)]
        #[derive(Copy, Clone)]
        pub struct $ty($inner_ty, $inner_ty, $inner_ty, $inner_ty);

        impl $ty {
            #[target_feature(enable = "avx")]
            #[inline]
            pub fn zero() -> Self {
                Self($inner_ty::zero(), $inner_ty::zero(), $inner_ty::zero(), $inner_ty::zero())
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn negative_one() -> Self {
                Self(
                    $inner_ty::negative_one(),
                    $inner_ty::negative_one(),
                    $inner_ty::negative_one(),
                    $inner_ty::negative_one(),
                )
            }

            #[target_feature(enable = "avx")]
            #[inline]
            pub fn splat(value: $lane_ty) -> Self {
                let value = $inner_ty::splat(value);
                Self(value, value, value, value)
            }

            #[inline]
            pub fn as_slice(&self) -> &[$lane_ty; $lane_count] {
                unsafe { &*core::ptr::addr_of!(self.0).cast::<[$lane_ty; $lane_count]>() }
            }

            #[inline]
            pub fn as_slice_mut(&mut self) -> &mut [$lane_ty; $lane_count] {
                unsafe { &mut *core::ptr::addr_of_mut!(self.0).cast::<[$lane_ty; $lane_count]>() }
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn bitwise_reduce(self) -> i32 {
                self.0.or(self.1).or(self.2.or(self.3)).bitwise_reduce()
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn simd_eq(self, rhs: Self) -> Self {
                Self(
                    self.0.simd_eq(rhs.0),
                    self.1.simd_eq(rhs.1),
                    self.2.simd_eq(rhs.2),
                    self.3.simd_eq(rhs.3),
                )
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn simd_gt(self, rhs: Self) -> Self {
                Self(
                    self.0.simd_gt(rhs.0),
                    self.1.simd_gt(rhs.1),
                    self.2.simd_gt(rhs.2),
                    self.3.simd_gt(rhs.3),
                )
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn is_equal(self, rhs: Self) -> bool {
                self.0.is_equal(rhs.0) & self.1.is_equal(rhs.1) & self.2.is_equal(rhs.2) & self.3.is_equal(rhs.3)
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn and(self, rhs: Self) -> Self {
                Self(self.0.and(rhs.0), self.1.and(rhs.1), self.2.and(rhs.2), self.3.and(rhs.3))
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn or(self, rhs: Self) -> Self {
                Self(self.0.or(rhs.0), self.1.or(rhs.1), self.2.or(rhs.2), self.3.or(rhs.3))
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn xor(self, rhs: Self) -> Self {
                Self(self.0.xor(rhs.0), self.1.xor(rhs.1), self.2.xor(rhs.2), self.3.xor(rhs.3))
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn and_not(self, rhs: Self) -> Self {
                Self(
                    self.0.and_not(rhs.0),
                    self.1.and_not(rhs.1),
                    self.2.and_not(rhs.2),
                    self.3.and_not(rhs.3),
                )
            }

            #[target_feature(enable = "avx")]
            #[inline]
            pub unsafe fn store_unaligned(self, address: *mut u8) {
                unsafe {
                    self.0.as_si256().store_unaligned(address);
                    self.1
                        .as_si256()
                        .store_unaligned(address.byte_add(core::mem::size_of::<si256>()));
                    self.2
                        .as_si256()
                        .store_unaligned(address.byte_add(core::mem::size_of::<si256>() * 2));
                    self.3
                        .as_si256()
                        .store_unaligned(address.byte_add(core::mem::size_of::<si256>() * 3));
                }
            }

            #[target_feature(enable = "avx")]
            #[inline]
            pub unsafe fn load_unaligned(address: *const u8) -> Self {
                const _: () = {
                    assert!(core::mem::size_of::<si256>() * 4 == core::mem::size_of::<$ty>());
                };

                unsafe {
                    Self(
                        $inner_ty(si256::load_unaligned(address).0),
                        $inner_ty(si256::load_unaligned(address.byte_add(core::mem::size_of::<si256>())).0),
                        $inner_ty(si256::load_unaligned(address.byte_add(core::mem::size_of::<si256>() * 2)).0),
                        $inner_ty(si256::load_unaligned(address.byte_add(core::mem::size_of::<si256>() * 3)).0),
                    )
                }
            }

            #[target_feature(enable = "avx")]
            #[inline]
            pub fn from_array_ref(xs: &[$lane_ty; $lane_count]) -> Self {
                const _: () = {
                    assert!(core::mem::size_of::<[$lane_ty; $lane_count]>() == core::mem::size_of::<$ty>());
                };

                unsafe { Self::load_unaligned(xs.as_ptr().cast()) }
            }

            #[target_feature(enable = "avx")]
            #[inline]
            pub fn to_array(self) -> [$lane_ty; $lane_count] {
                const _: () = {
                    assert!(core::mem::size_of::<[$lane_ty; $lane_count]>() == core::mem::size_of::<$ty>());
                };

                let mut array: core::mem::MaybeUninit<[$lane_ty; $lane_count]> = core::mem::MaybeUninit::uninit();
                unsafe {
                    self.store_unaligned(array.as_mut_ptr().cast());
                    array.assume_init()
                }
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn add(self, rhs: Self) -> Self {
                Self(self.0.add(rhs.0), self.1.add(rhs.1), self.2.add(rhs.2), self.3.add(rhs.3))
            }

            #[target_feature(enable = "avx2")]
            #[inline]
            pub fn sub(self, rhs: Self) -> Self {
                Self(self.0.sub(rhs.0), self.1.sub(rhs.1), self.2.sub(rhs.2), self.3.sub(rhs.3))
            }
        }

        impl_bitops!($ty);
        impl_common_ops!($ty);
    };
}

impl_composite!(i8x64 i8x32 i8 2 64);
impl_composite!(i16x32 i16x16 i16 2 32);
impl_composite!(i32x16 i32x8 i32 2 16);
impl_composite!(i64x8 i64x4 i64 2 8);
impl_composite!(i32x32 i32x8 i32 4 32);

impl i8x32 {
    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn to_i16x32_sext(self) -> i16x32 {
        let rhs_lo: i16x16 = self.lo().to_i16x16_sext();
        let rhs_hi: i16x16 = self.hi().to_i16x16_sext();
        i16x32(rhs_lo, rhs_hi)
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn to_i32x32_sext(self) -> i32x32 {
        let x: i64x4 = self.as_i64x4();
        let x_0 = x.hi().as_i8x16().hi_i8x8_to_i32x8_sext();
        let x_1 = x.hi().as_i8x16().lo_i8x8_to_i32x8_sext();
        let x_2 = x.lo().as_i8x16().hi_i8x8_to_i32x8_sext();
        let x_3 = x.lo().as_i8x16().lo_i8x8_to_i32x8_sext();
        i32x32(x_3, x_2, x_1, x_0)
    }
}

impl i16x32 {
    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn wrapping_reduce(self) -> i16 {
        let sum = self.0.horizontal_reduce_i16x8x2(self.1); // 32 -> 16
        sum.wrapping_reduce()
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn clamp_to_i8_range(self) -> i8x32 {
        let value = self.0.clamp_to_i8_range_and_pack_i8x16x2(self.1).as_i64x4();
        crate::amd64::avx2::i64x4_pick!(value[0, 2, 1, 3]).as_i8x32()
    }
}

impl i32x16 {
    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn from_i1x16_sext(value: i16) -> Self {
        Self(i32x8::from_i1x8_sext(value as i8), i32x8::from_i1x8_sext((value >> 8) as i8))
    }

    #[must_use]
    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn set_dynamic(self, index: u32, value: i32) -> Self {
        Self(self.0.set_dynamic(index, value), self.1.set_dynamic(index.wrapping_sub(8), value))
    }

    #[must_use]
    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn get_dynamic(self, index: u32) -> i32 {
        self.0.get_dynamic(index) | self.1.get_dynamic(index.wrapping_sub(8))
    }
}

impl i32x32 {
    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn from_i8x32_sext(value: i8x32) -> Self {
        let lo = value.lo();
        let hi = value.hi();
        i32x32(
            lo.lo_i8x8_to_i32x8_sext(),
            lo.hi_i8x8_to_i32x8_sext(),
            hi.lo_i8x8_to_i32x8_sext(),
            hi.hi_i8x8_to_i32x8_sext(),
        )
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn truncate_to_i8x32(self) -> i8x32 {
        let ones = i32x8::splat(-1);
        let local_mask = const { i32::from_le_bytes([0, 4, 8, 12]) };
        let value = self
            .0
            .as_i128x2()
            .shuffle_i8x16(ones.set::<0>(local_mask).as_i8x32())
            .as_si256()
            .or(self
                .0
                .as_i128x2()
                .shuffle_i8x16(ones.set::<5>(local_mask).as_i8x32())
                .hi()
                .as_si256())
            .or(self.1.as_i128x2().shuffle_i8x16(ones.set::<2>(local_mask).as_i8x32()).as_si256())
            .or(self
                .1
                .as_i128x2()
                .shuffle_i8x16(ones.set::<7>(local_mask).as_i8x32())
                .hi()
                .as_si256())
            .or(si256::zero().set_hi(
                self.2
                    .as_i128x2()
                    .shuffle_i8x16(ones.set::<0>(local_mask).as_i8x32())
                    .lo()
                    .or(self.2.as_i128x2().shuffle_i8x16(ones.set::<5>(local_mask).as_i8x32()).hi())
                    .or(self.3.as_i128x2().shuffle_i8x16(ones.set::<2>(local_mask).as_i8x32()).lo())
                    .or(self.3.as_i128x2().shuffle_i8x16(ones.set::<7>(local_mask).as_i8x32()).hi()),
            ));

        value.as_i8x32()
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn clamp_to_i8_range(self) -> i8x32 {
        let value_1 = self.0.clamp_to_i16_range_and_pack_i16x8x2(self.2).as_i64x4();
        let value_2 = self.1.clamp_to_i16_range_and_pack_i16x8x2(self.3).as_i64x4();
        let value_1 = crate::amd64::avx2::i64x4_pick!(value_1[0, 2, 1, 3]).as_i16x16();
        let value_2 = crate::amd64::avx2::i64x4_pick!(value_2[0, 2, 1, 3]).as_i16x16();
        let value: i8x32 = value_1.clamp_to_i8_range_and_pack_i8x16x2(value_2);
        value

        // Alternative implementation: (TODO: check which one is faster)
        // let value_1: i16x16 = self.0.clamp_to_i16_range_and_pack_i16x8x2(self.1);
        // let value_2: i16x16 = self.2.clamp_to_i16_range_and_pack_i16x8x2(self.3);
        // let value: i8x32 = value_1.clamp_to_i8_range_and_pack_i8x16x2(value_2);
        // let value = value.as_i32x8();
        // crate::amd64::avx2::i32x8_pick!(value[0, 4, 1, 5, 2, 6, 3, 7]).as_i8x32()
    }
}

impl i8x64 {
    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn most_significant_bits(self) -> i64 {
        ((self.0.most_significant_bits() as u64) | ((self.1.most_significant_bits() as u64) << 32)) as i64
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0), self.1.saturating_add(rhs.1))
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn saturating_add_unsigned(self, rhs: Self) -> Self {
        Self(self.0.saturating_add_unsigned(rhs.0), self.1.saturating_add_unsigned(rhs.1))
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0), self.1.saturating_sub(rhs.1))
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    pub fn saturating_sub_unsigned(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub_unsigned(rhs.0), self.1.saturating_sub_unsigned(rhs.1))
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
            let array = test_array_i64x8();
            let value = i64x8::load_unaligned(array.as_ptr().cast());
            let mut array_out = [0_i64; 8];
            value.store_unaligned(array_out.as_mut_ptr().cast());
            assert_eq!(array_out, array);
            assert_eq!(value.to_array(), array);

            assert_eq!(&value.lo().to_array(), &array[..4]);
            assert_eq!(&value.hi().to_array(), &array[4..8]);

            #[cfg(feature = "ops")]
            {
                let mut value = i8x64::splat(3);
                value += i8x64::splat(7);
                assert_eq!(value.to_array()[0], 10);
            }

            assert!(i32x16::splat(0x12345678).is_equal(i32x16::splat(0x12345678)));
            assert!(!i32x16::splat(0x12345678).is_equal(i32x16::splat(0x12345678).add(i32x16::splat(1))));
            assert!(i32x32::splat(0x12345678).is_equal(i32x32::splat(0x12345678)));
            assert!(!i32x32::splat(0x12345678).is_equal(i32x32::splat(0x12345678).add(i32x32::splat(1))));
        }
    }

    #[test]
    fn i8x64_most_significant_bits() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            assert_eq!(
                i8x64::from_array_ref(&test_array_i8x64()).most_significant_bits(),
                test_array_i8x64()
                    .map(|x| ((x as u8) >> 7) as i64)
                    .into_iter()
                    .enumerate()
                    .map(|(index, x)| x << index)
                    .fold(0, |a, b| a | b)
            );
        }
    }

    #[test]
    fn i32x32_truncate_to_i8x32() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            assert_eq!(
                i32x32::from_array_ref(&test_array_i32x32()).truncate_to_i8x32().to_array(),
                test_array_i32x32().map(|value| value as i8)
            );
        }
    }

    #[test]
    fn i32x16_from_i1x16_sext() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            assert_eq!(i32x16::from_i1x16_sext(!0).to_array(), [-1; 16]);
            assert_eq!(i32x16::from_i1x16_sext(0).to_array(), [0; 16]);
            for n in 0..16 {
                let mask = (1_u16 << n) as i16;
                let value = i32x16::from_i1x16_sext(mask).to_array();
                for m in 0..16 {
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
    fn i16x32_and_i8x32() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            let value = i16x32::from_array_ref(&test_array_i16x32());
            assert_eq!(value.to_array(), test_array_i16x32());
            let out: i16x32 = value & i8x32::from_array([0_i8; 32]).to_i16x32_sext();
            assert!(out.to_array().iter().all(|&x| x == 0));

            for n in 0..32 {
                let mut mask = [0_i8; 32];
                mask[n] = 0xff_u8 as i8;
                let out: i16x32 = value & i8x32::from_array(mask).to_i16x32_sext();
                assert_eq!(out.to_array()[n], test_array_i16x32()[n]);
                for m in 0..32 {
                    if n != m {
                        assert_eq!(out.to_array()[m], 0);
                    }
                }
            }
        }
    }

    #[test]
    fn i32x32_and_i8x32() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            let value = i32x32::from_array_ref(&test_array_i32x32());
            assert_eq!(value.to_array(), test_array_i32x32());
            let out: i32x32 = value & i8x32::from_array([0_i8; 32]).to_i32x32_sext();
            assert!(out.to_array().iter().all(|&x| x == 0));

            for n in 0..32 {
                let mut mask = [0_i8; 32];
                mask[n] = 0xff_u8 as i8;
                let out: i32x32 = value & i8x32::from_array(mask).to_i32x32_sext();
                assert_eq!(out.to_array()[n], test_array_i32x32()[n], "failed for lane {n}");
                for m in 0..32 {
                    if n != m {
                        assert_eq!(out.to_array()[m], 0, "failed for lane {m}: expected a zero");
                    }
                }
            }
        }
    }

    #[test]
    fn i32x32_from_i8x32_sext() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            assert_eq!(
                i32x32::from_i8x32_sext(i8x32::from_array(test_array_i8x32())).to_array(),
                crate::fallback::i32x32::from_i8x32_sext(crate::fallback::i8x32::from_array(test_array_i8x32())).to_array()
            );
        }
    }

    #[test]
    fn i32x16_set_dynamic() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            for n in 0..16_u32 {
                let value = i32x16::from_array_ref(&test_array_i32x16());
                let mut expected = test_array_i32x16();
                expected[n as usize] = 58439;
                assert_eq!(value.set_dynamic(n, 58439).to_array(), expected, "failed for index {n}");
            }
        }
    }

    #[test]
    fn i32x16_get_dynamic() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            let a = test_array_i32x16();
            let b = i32x16::from_array_ref(&a);
            assert_eq!(indexes::<16>().map(|i| b.get_dynamic(i as u32)), a);
        }
    }

    #[test]
    fn i8x64_as_slice() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            let a = test_array_i8x64();
            let b = i8x64::from_array_ref(&a);
            assert_eq!(*b.as_slice(), a);
        }
    }

    #[test]
    fn i16x32_as_slice() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            let a = test_array_i16x32();
            let b = i16x32::from_array_ref(&a);
            assert_eq!(*b.as_slice(), a);
        }
    }

    #[test]
    fn i32x16_as_slice() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            let a = test_array_i32x16();
            let b = i32x16::from_array_ref(&a);
            assert_eq!(*b.as_slice(), a);
        }
    }

    #[test]
    fn i64x8_as_slice() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            let a = test_array_i64x8();
            let b = i64x8::from_array_ref(&a);
            assert_eq!(*b.as_slice(), a);
        }
    }

    #[test]
    fn i32x32_as_slice() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            let a = test_array_i32x32();
            let b = i32x32::from_array_ref(&a);
            assert_eq!(*b.as_slice(), a);
        }
    }

    #[test]
    fn i16x32_clamp_to_i8_range() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            let candidates = [
                test_array_i16x32(),
                test_array_i16x32().map(|x| x / 10),
                test_array_i16x32().map(|x| x / 100),
                test_array_i16x32().map(|x| x / 1000),
                [0_i16; 32],
                [-2; 32],
            ];
            for array in candidates {
                assert_eq!(
                    i16x32::from_array_ref(&array).clamp_to_i8_range().to_array(),
                    array.map(|value| {
                        if value > 127 {
                            127
                        } else if value < -128 {
                            -128
                        } else {
                            value as i8
                        }
                    })
                );
            }
        }
    }

    #[test]
    fn i32x32_clamp_to_i8_range() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        unsafe {
            let mut candidates = std::vec![[0_i32; 32], indexes::<32>().map(|n| n as i32)];
            for n in 0..32 {
                let mut array = [0_i32; 32];
                array[n] = -2;
                candidates.push(array);
            }
            candidates.extend([
                [-2; 32],
                test_array_i32x32(),
                test_array_i32x32().map(|x| x / 10),
                test_array_i32x32().map(|x| x / 100),
                test_array_i32x32().map(|x| x / 1000),
                test_array_i32x32().map(|x| x / 10000),
            ]);
            for array in candidates {
                assert_eq!(
                    i32x32::from_array_ref(&array).clamp_to_i8_range().to_array(),
                    array.map(|value| {
                        if value > 127 {
                            127
                        } else if value < -128 {
                            -128
                        } else {
                            value as i8
                        }
                    }),
                    "failed for input: {array:?}"
                );
            }
        }
    }
}
