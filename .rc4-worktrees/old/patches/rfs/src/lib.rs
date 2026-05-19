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
