#![no_std]
#![allow(unsafe_code)]
#![allow(clippy::undocumented_unsafe_blocks)]
#![allow(clippy::missing_safety_doc)]
#![allow(non_camel_case_types)]
#![forbid(unconditional_recursion)]

#[cfg(picosimd = "supports_safe_intrinsics")]
#[allow(unused_macros)]
macro_rules! maybe_unsafe {
    ($($t:tt)*) => { $($t)* }
}

#[cfg(not(picosimd = "supports_safe_intrinsics"))]
#[allow(unused_macros)]
macro_rules! maybe_unsafe {
    ($($t:tt)*) => { unsafe { $($t)* } }
}

#[cfg(feature = "std")]
extern crate std;

pub mod fallback;

#[cfg(target_arch = "x86_64")]
pub mod amd64;

#[cfg(not(target_arch = "x86_64"))]
pub mod amd64 {
    pub mod avx2 {
        pub use crate::fallback::{i8x32, i16x16, i32x8, i64x4};
    }

    pub mod avx2_composite {
        pub use crate::fallback::{i8x64, i16x32, i32x16, i32x32, i64x8};
    }

    pub mod sse {
        pub use crate::fallback::{i8x16, i16x8, i32x4, i64x2};
    }

    pub mod avx512 {
        pub use crate::fallback::{i8x64, i16x32, i32x16, i64x8};
    }
}

#[cfg(test)]
mod test_utils;

pub(crate) fn indexes<const COUNT: usize>() -> [usize; COUNT] {
    let mut n = 0;
    [(); COUNT].map(move |_| {
        n += 1;
        n - 1
    })
}
