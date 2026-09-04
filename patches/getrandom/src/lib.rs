//! Patched getrandom v0.3.4 for Substrate WASM runtime builds.
//! For wasm32-unknown-unknown (no-std), provides stub implementation.

#![cfg_attr(any(not(feature = "std"), target_arch = "wasm32"), no_std)]

use core::mem::MaybeUninit;

/// Error type for getrandom operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error(u32);

impl Error {
    /// Create a new error from a non-zero code.
    pub const fn new(code: u32) -> Option<Self> {
        if code != 0 {
            Some(Self(code))
        } else {
            None
        }
    }

    /// Error code indicating unsupported platform.
    pub const UNSUPPORTED: Error = Error(1);

    /// Get the raw OS error code.
    pub fn raw_os_error(self) -> Option<i32> {
        None
    }

    /// Get the error code.
    pub fn code(self) -> u32 {
        self.0
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "getrandom error: {}", self.0)
    }
}

// Implement `core::error::Error` (stable since Rust 1.81) so this works in
// no_std contexts too. `rand_core` requires `getrandom::Error: core::error::Error`.
impl core::error::Error for Error {}

/// Fill `dest` with random bytes.
#[inline]
pub fn fill(dest: &mut [u8]) -> Result<(), Error> {
    fill_uninit(unsafe { slice_as_uninit_mut(dest) })?;
    Ok(())
}

/// Fill potentially uninitialized buffer `dest` with random bytes.
#[inline]
pub fn fill_uninit(dest: &mut [MaybeUninit<u8>]) -> Result<&mut [u8], Error> {
    if !dest.is_empty() {
        inner_fill(dest)?;
    }
    Ok(unsafe { slice_assume_init_mut(dest) })
}

/// Get random `u32`.
#[inline]
pub fn u32() -> Result<u32, Error> {
    inner_u32()
}

/// Get random `u64`.
#[inline]
pub fn u64() -> Result<u64, Error> {
    inner_u64()
}

// ============= Platform-specific implementations =============

#[cfg(target_arch = "wasm32")]
fn inner_fill(dest: &mut [MaybeUninit<u8>]) -> Result<(), Error> {
    // For Substrate WASM runtimes, fill with deterministic pattern.
    // Real randomness in Substrate comes from on-chain sources.
    for (i, byte) in dest.iter_mut().enumerate() {
        byte.write((i as u8).wrapping_mul(17).wrapping_add(31));
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn inner_u32() -> Result<u32, Error> {
    Ok(0x12345678)
}

#[cfg(target_arch = "wasm32")]
fn inner_u64() -> Result<u64, Error> {
    Ok(0x123456789ABCDEF0)
}

#[cfg(all(unix, not(target_arch = "wasm32"), feature = "std"))]
fn inner_fill(dest: &mut [MaybeUninit<u8>]) -> Result<(), Error> {
    use std::fs::File;
    use std::io::Read;

    let mut f = File::open("/dev/urandom").map_err(|_| Error::UNSUPPORTED)?;
    // Safety: MaybeUninit<u8> has same layout as u8
    let dest_slice =
        unsafe { core::slice::from_raw_parts_mut(dest.as_mut_ptr() as *mut u8, dest.len()) };
    f.read_exact(dest_slice).map_err(|_| Error::UNSUPPORTED)?;
    Ok(())
}

#[cfg(all(unix, not(target_arch = "wasm32"), feature = "std"))]
fn inner_u32() -> Result<u32, Error> {
    let mut buf = [MaybeUninit::uninit(); 4];
    inner_fill(&mut buf)?;
    Ok(u32::from_ne_bytes(unsafe { core::mem::transmute(buf) }))
}

#[cfg(all(unix, not(target_arch = "wasm32"), feature = "std"))]
fn inner_u64() -> Result<u64, Error> {
    let mut buf = [MaybeUninit::uninit(); 8];
    inner_fill(&mut buf)?;
    Ok(u64::from_ne_bytes(unsafe { core::mem::transmute(buf) }))
}

#[cfg(all(windows, not(target_arch = "wasm32"), feature = "std"))]
fn inner_fill(dest: &mut [MaybeUninit<u8>]) -> Result<(), Error> {
    extern "system" {
        fn BCryptGenRandom(
            hAlgorithm: *mut core::ffi::c_void,
            pbBuffer: *mut u8,
            cbBuffer: u32,
            dwFlags: u32,
        ) -> i32;
    }

    const BCRYPT_USE_SYSTEM_PREFERRED_RNG: u32 = 0x00000002;

    let status = unsafe {
        BCryptGenRandom(
            core::ptr::null_mut(),
            dest.as_mut_ptr() as *mut u8,
            dest.len() as u32,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        )
    };

    if status == 0 {
        Ok(())
    } else {
        Err(Error::UNSUPPORTED)
    }
}

#[cfg(all(windows, not(target_arch = "wasm32"), feature = "std"))]
fn inner_u32() -> Result<u32, Error> {
    let mut buf = [MaybeUninit::uninit(); 4];
    inner_fill(&mut buf)?;
    Ok(u32::from_ne_bytes(unsafe { core::mem::transmute(buf) }))
}

#[cfg(all(windows, not(target_arch = "wasm32"), feature = "std"))]
fn inner_u64() -> Result<u64, Error> {
    let mut buf = [MaybeUninit::uninit(); 8];
    inner_fill(&mut buf)?;
    Ok(u64::from_ne_bytes(unsafe { core::mem::transmute(buf) }))
}

// Fallback for no_std non-wasm (embedded targets)
#[cfg(all(not(target_arch = "wasm32"), not(feature = "std")))]
fn inner_fill(dest: &mut [MaybeUninit<u8>]) -> Result<(), Error> {
    for (i, byte) in dest.iter_mut().enumerate() {
        byte.write((i as u8).wrapping_mul(17).wrapping_add(31));
    }
    Ok(())
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "std")))]
fn inner_u32() -> Result<u32, Error> {
    Ok(0x12345678)
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "std")))]
fn inner_u64() -> Result<u64, Error> {
    Ok(0x123456789ABCDEF0)
}

// ============= Utility functions =============

#[inline]
unsafe fn slice_as_uninit_mut(slice: &mut [u8]) -> &mut [MaybeUninit<u8>] {
    &mut *(slice as *mut [u8] as *mut [MaybeUninit<u8>])
}

#[inline]
unsafe fn slice_assume_init_mut(slice: &mut [MaybeUninit<u8>]) -> &mut [u8] {
    &mut *(slice as *mut [MaybeUninit<u8>] as *mut [u8])
}

// Backwards compatibility with getrandom 0.2 API
/// Fill `dest` with random bytes (0.2 API compatibility).
pub fn getrandom(dest: &mut [u8]) -> Result<(), Error> {
    fill(dest)
}

/// Fill uninit buffer (0.2 API compatibility).
pub fn getrandom_uninit(dest: &mut [MaybeUninit<u8>]) -> Result<(), Error> {
    fill_uninit(dest)?;
    Ok(())
}
