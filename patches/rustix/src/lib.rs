// rustix patch for WASM compatibility
// Re-export rustix with conditional compilation to avoid errno issues on WASM

#[cfg(not(target_arch = "wasm32"))]
pub mod rfs {
    use std::fs;
    use std::io::{Error, ErrorKind};
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    #[derive(Debug, Copy, Clone)]
    pub enum Access {
        EXEC_OK,
    }

    pub fn access(path: &Path, _access: Access) -> Result<(), Error> {
        let meta = fs::metadata(path)?;
        if meta.is_file() {
            let perms = meta.permissions();
            if (perms.mode() & 0o111) != 0 {
                Ok(())
            } else {
                Err(Error::new(ErrorKind::PermissionDenied, "not executable"))
            }
        } else {
            Err(Error::new(ErrorKind::NotFound, "not a file"))
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub mod fs {
    use std::fs;
    use std::io::{Error, ErrorKind};
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    #[derive(Debug, Copy, Clone)]
    pub enum Access {
        EXEC_OK,
    }

    pub fn access(path: &Path, _access: Access) -> Result<(), Error> {
        let meta = fs::metadata(path)?;
        if meta.is_file() {
            let perms = meta.permissions();
            if (perms.mode() & 0o111) != 0 {
                Ok(())
            } else {
                Err(Error::new(ErrorKind::PermissionDenied, "not executable"))
            }
        } else {
            Err(Error::new(ErrorKind::NotFound, "not a file"))
        }
    }

    // Minimal stub implementations for other fs functions
    pub fn openat() -> Result<(), ()> {
        Err(())
    }
    pub fn close() -> Result<(), ()> {
        Err(())
    }
    pub fn read() -> Result<(), ()> {
        Err(())
    }
    pub fn write() -> Result<(), ()> {
        Err(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub mod io {
    // Minimal stub implementations for native targets
    pub fn dup2_stdin() -> Result<(), ()> {
        Err(())
    }
    pub fn dup2_stdout() -> Result<(), ()> {
        Err(())
    }
    pub fn dup2_stderr() -> Result<(), ()> {
        Err(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub mod process {
    // Minimal stubs for native targets
    pub fn getpid() -> u32 {
        1
    }
    pub fn getppid() -> u32 {
        0
    }
}

#[cfg(target_arch = "wasm32")]
pub mod fs {
    // Stub implementations for WASM
    pub fn openat() -> Result<(), ()> {
        Err(())
    }
    pub fn close() -> Result<(), ()> {
        Err(())
    }
    pub fn read() -> Result<(), ()> {
        Err(())
    }
    pub fn write() -> Result<(), ()> {
        Err(())
    }
}

#[cfg(target_arch = "wasm32")]
pub mod io {
    // Stub implementations for WASM
    pub fn dup2_stdin() -> Result<(), ()> {
        Err(())
    }
    pub fn dup2_stdout() -> Result<(), ()> {
        Err(())
    }
    pub fn dup2_stderr() -> Result<(), ()> {
        Err(())
    }
}

#[cfg(target_arch = "wasm32")]
pub mod process {
    // Stub implementations for WASM
    pub fn getpid() -> u32 {
        1
    }
    pub fn getppid() -> u32 {
        0
    }
}
