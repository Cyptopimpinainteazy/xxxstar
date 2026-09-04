// errno patch for WASM compatibility
// For all targets, provide a compatible errno API

#[cfg(not(target_arch = "wasm32"))]
extern crate libc;

/// The Errno struct representing an OS error code
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Errno(pub i32);

impl Errno {
    /// Create a new Errno from an error code
    pub fn new(code: i32) -> Self {
        Errno(code)
    }

    /// Get the last OS error
    #[cfg(not(target_arch = "wasm32"))]
    pub fn last_os_error() -> Self {
        Errno(errno().0)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn last_os_error() -> Self {
        Errno(0)
    }
}

impl From<i32> for Errno {
    fn from(err: i32) -> Self {
        Errno(err)
    }
}

impl From<Errno> for i32 {
    fn from(err: Errno) -> i32 {
        err.0
    }
}

impl std::fmt::Display for Errno {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "errno {}", self.0)
    }
}

impl std::error::Error for Errno {}

/// Get the current errno value
#[cfg(not(target_arch = "wasm32"))]
pub fn errno() -> Errno {
    unsafe { Errno(*libc::__errno_location()) }
}

#[cfg(target_arch = "wasm32")]
pub fn errno() -> Errno {
    Errno(0)
}

/// Set the errno value
#[cfg(not(target_arch = "wasm32"))]
pub fn set_errno(err: Errno) {
    unsafe {
        *libc::__errno_location() = err.0;
    }
}

#[cfg(target_arch = "wasm32")]
pub fn set_errno(_err: Errno) {
    // No-op for WASM
}
