//! Utility functions for getrandom

use core::mem::MaybeUninit;

/// Cast a slice of bytes to a slice of MaybeUninit<u8>
#[inline]
pub unsafe fn slice_as_uninit_mut(slice: &mut [u8]) -> &mut [MaybeUninit<u8>] {
    &mut *(slice as *mut [u8] as *mut [MaybeUninit<u8>])
}

/// Assume a slice of MaybeUninit<u8> is initialized
#[inline]
pub unsafe fn slice_assume_init_mut(slice: &mut [MaybeUninit<u8>]) -> &mut [u8] {
    &mut *(slice as *mut [MaybeUninit<u8>] as *mut [u8])
}

/// Get a random u32 from two u16 values
#[inline]
pub fn inner_u32() -> u32 {
    0x12345678
}

/// Get a random u64 from two u32 values
#[inline]
pub fn inner_u64() -> u64 {
    0x123456789ABCDEF0
}
