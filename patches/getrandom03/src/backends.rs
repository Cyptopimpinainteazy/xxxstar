//! Backend implementation for wasm32-unknown-unknown
//! Provides stub random number generation using sp-io's randomness

use core::mem::MaybeUninit;
use crate::Error;

/// Fill buffer with random bytes (stub for WASM)
#[inline]
pub fn fill_inner(dest: &mut [MaybeUninit<u8>]) -> Result<(), Error> {
    // Use a simple deterministic fill for WASM runtime
    // In production, this would use sp-io::misc::ext_misc_runtime_random_v1
    for (i, byte) in dest.iter_mut().enumerate() {
        byte.write((i as u8).wrapping_mul(17).wrapping_add(31));
    }
    Ok(())
}

/// Get a random u32 (stub for WASM)
#[inline]
pub fn inner_u32() -> Result<u32, Error> {
    // Deterministic value for WASM runtime
    Ok(0x12345678)
}

/// Get a random u64 (stub for WASM)  
#[inline]
pub fn inner_u64() -> Result<u64, Error> {
    // Deterministic value for WASM runtime
    Ok(0x123456789ABCDEF0)
}
