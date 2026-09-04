#[cfg(target_os = "linux")]
pub fn lock_heap() -> Result<(), String> {
    use std::io;

    unsafe {
        // Prevent all current and future allocated memory from being paged to the swap area.
        if libc::mlockall(libc::MCL_CURRENT | libc::MCL_FUTURE) != 0 {
            let err = io::Error::last_os_error();
            return Err(format!("Failed to lock memory: {}", err));
        }

        // Disable core dumps
        let rlimit = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        if libc::setrlimit(libc::RLIMIT_CORE, &rlimit) != 0 {
            let err = io::Error::last_os_error();
            return Err(format!("Failed to disable core dumps: {}", err));
        }
    }

    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn lock_heap() -> Result<(), String> {
    // Scaffold for macos/windows support, if needed. For now just warn.
    println!("WARNING: Memory enclaving/mlock is currently only implemented fully on Linux.");
    Ok(())
}
