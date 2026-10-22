use once_cell::sync::Lazy;
use semver::{Version, VersionReq};
use sha2::Digest;

use std::{
    fs,
    io::{Cursor, ErrorKind, Write},
    path::PathBuf,
    process::Command,
};

use std::time::Duration;
/// Use permissions extensions on unix
#[cfg(target_family = "unix")]
use std::{fs::Permissions, os::unix::fs::PermissionsExt};

mod error;
pub use error::SolcVmError;

mod platform;
pub use platform::{platform, Platform};

mod releases;
pub use releases::{all_releases, Releases};

#[cfg(feature = "blocking")]
pub use releases::blocking_all_releases;

/// Declare path to Solc Version Manager's home directory
/// On unix-based machines, if "~/.svm" already exists, then keep using it.
/// Otherwise, use $XDG_DATA_HOME or ~/.local/share/svm
pub static SVM_DATA_DIR: Lazy<PathBuf> = Lazy::new(|| {
    #[cfg(test)]
    {
        let dir = tempfile::tempdir().expect("could not create temp directory");
        dir.path().join(".svm")
    }
    #[cfg(not(test))]
    {
        resolve_data_dir()
    }
});

fn resolve_data_dir() -> PathBuf {
    let home_dir = dirs::home_dir()
        .expect("could not detect user home directory")
        .join(".svm");

    let data_dir = dirs::data_dir().expect("could not detect user data directory");
    if !home_dir.as_path().exists() && data_dir.as_path().exists() {
        data_dir.join("svm")
    } else {
        home_dir
    }
}

/// The timeout to use for requests to the source
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Version beyond which solc binaries are not fully static, hence need to be patched for NixOS.
static NIXOS_PATCH_REQ: Lazy<VersionReq> = Lazy::new(|| VersionReq::parse(">=0.7.6").unwrap());

// Installer type that copies binary data to the appropriate solc binary file:
// 1. create target file to copy binary data
// 2. copy data
struct Installer {
    // version of solc
    version: Version,
    // binary data of the solc executable
    binbytes: Vec<u8>,
}

impl Installer {
    /// Installs the solc version at the version specific destination and returns the path to the installed solc file.
    fn install(&self) -> Result<PathBuf, SolcVmError> {
        let version_path = version_path(self.version.to_string().as_str());
        self.install_binary_at(version_path.join(format!("solc-{}", self.version)))
    }

    fn install_binary_at(&self, solc_path: PathBuf) -> Result<PathBuf, SolcVmError> {
        // create solc file.
        let mut f = fs::File::create(&solc_path)?;

        #[cfg(target_family = "unix")]
        f.set_permissions(Permissions::from_mode(0o755))?;

        // copy contents over
        let mut content = Cursor::new(&self.binbytes);
        std::io::copy(&mut content, &mut f)?;

        if platform::is_nixos() && NIXOS_PATCH_REQ.matches(&self.version) {
            patch_for_nixos(solc_path)
        } else {
            Ok(solc_path)
        }
    }

    /// Extracts the solc archive at the version specified destination and returns the path to the
    /// installed solc binary.
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    fn install_zip(&self) -> Result<PathBuf, SolcVmError> {
        self.install_zip_at(&version_path(self.version.to_string().as_str()))
    }

    #[cfg(any(test, all(target_os = "windows", target_arch = "x86_64")))]
    fn install_zip_at(&self, version_path: &std::path::Path) -> Result<PathBuf, SolcVmError> {
        let solc_path = version_path.join(format!("solc-{}", self.version));

        // extract archive
        let mut content = Cursor::new(&self.binbytes);
        let mut archive = zip::ZipArchive::new(&mut content)?;
        archive.extract(version_path)?;

        // rename solc binary
        std::fs::rename(version_path.join("solc.exe"), solc_path.as_path())?;

        Ok(solc_path)
    }
}

/// Patch the given binary to use the dynamic linker provided by nixos
pub fn patch_for_nixos(bin: PathBuf) -> Result<PathBuf, SolcVmError> {
    let output = Command::new("nix-shell")
        .arg("-p")
        .arg("patchelf")
        .arg("--run")
        .arg(format!(
            "patchelf --set-interpreter \"$(cat $NIX_CC/nix-support/dynamic-linker)\" {}",
            bin.display()
        ))
        .output()
        .expect("Failed to execute command");

    match output.status.success() {
        true => Ok(bin),
        false => Err(SolcVmError::CouldNotPatchForNixOs(
            String::from_utf8(output.stdout).expect("Found invalid UTF-8 when parsing stdout"),
            String::from_utf8(output.stderr).expect("Found invalid UTF-8 when parsing stderr"),
        )),
    }
}

/// Derive path to a specific Solc version's binary.
pub fn version_path(version: &str) -> PathBuf {
    let mut version_path = SVM_DATA_DIR.to_path_buf();
    version_path.push(version);
    version_path
}

/// Derive path to SVM's global version file.
pub fn global_version_path() -> PathBuf {
    let mut global_version_path = SVM_DATA_DIR.to_path_buf();
    global_version_path.push(".global-version");
    global_version_path
}

/// Reads the currently set global version for Solc. Returns None if none has yet been set.
pub fn current_version() -> Result<Option<Version>, SolcVmError> {
    let v = fs::read_to_string(global_version_path().as_path())?;
    Ok(Version::parse(v.trim_end_matches('\n').to_string().as_str()).ok())
}

/// Sets the provided version as the global version for Solc.
pub fn use_version(version: &Version) -> Result<(), SolcVmError> {
    let mut v = fs::File::create(global_version_path().as_path())?;
    v.write_all(version.to_string().as_bytes())?;
    Ok(())
}

/// Unset the global version. This should be done if all versions are removed.
pub fn unset_global_version() -> Result<(), SolcVmError> {
    let mut v = fs::File::create(global_version_path().as_path())?;
    v.write_all("".as_bytes())?;
    Ok(())
}

/// Reads the list of Solc versions that have been installed in the machine. The version list is
/// sorted in ascending order.
pub fn installed_versions() -> Result<Vec<Version>, SolcVmError> {
    installed_versions_at(&SVM_DATA_DIR)
}

fn installed_versions_at(home_dir: &std::path::Path) -> Result<Vec<Version>, SolcVmError> {
    let mut versions = vec![];
    for v in fs::read_dir(home_dir)? {
        let v = v?;
        if v.file_type()?.is_dir() {
            versions.push(Version::parse(
                v.path()
                    .file_name()
                    .ok_or(SolcVmError::UnknownVersion)?
                    .to_str()
                    .ok_or(SolcVmError::UnknownVersion)?
                    .to_string()
                    .as_str(),
            )?);
        }
    }
    versions.sort();
    Ok(versions)
}

/// Blocking version of [`all_versions`]
#[cfg(feature = "blocking")]
pub fn blocking_all_versions() -> Result<Vec<Version>, SolcVmError> {
    Ok(releases::blocking_all_releases(platform::platform())?.into_versions())
}

/// Fetches the list of all the available versions of Solc. The list is platform dependent, so
/// different versions can be found for macosx vs linux.
pub async fn all_versions() -> Result<Vec<Version>, SolcVmError> {
    Ok(releases::all_releases(platform::platform())
        .await?
        .into_versions())
}

/// Blocking version of [`install`]
#[cfg(feature = "blocking")]
pub fn blocking_install(version: &Version) -> Result<PathBuf, SolcVmError> {
    setup_data_dir()?;

    let artifacts = releases::blocking_all_releases(platform::platform())?;
    let artifact = artifacts
        .get_artifact(version)
        .ok_or(SolcVmError::UnknownVersion)?;
    let download_url =
        releases::artifact_url(platform::platform(), version, artifact.to_string().as_str())?;

    let checksum = artifacts
        .get_checksum(version)
        .unwrap_or_else(|| panic!("checksum not available: {:?}", version.to_string()));

    let res = reqwest::blocking::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .expect("reqwest::Client::new()")
        .get(download_url.clone())
        .send()?;

    if !res.status().is_success() {
        return Err(SolcVmError::UnsuccessfulResponse(
            download_url,
            res.status(),
        ));
    }

    let binbytes = res.bytes()?;
    ensure_checksum(&binbytes, version, checksum)?;

    // lock file to indicate that installation of this solc version will be in progress.
    let lock_path = lock_file_path(version);
    // wait until lock file is released, possibly by another parallel thread trying to install the
    // same version of solc.
    let _lock = try_lock_file(lock_path)?;

    do_install(
        version.clone(),
        binbytes.to_vec(),
        artifact.to_string().as_str(),
    )
}

/// Installs the provided version of Solc in the machine.
///
/// Returns the path to the solc file.
pub async fn install(version: &Version) -> Result<PathBuf, SolcVmError> {
    setup_data_dir()?;

    let artifacts = releases::all_releases(platform::platform()).await?;
    let artifact = artifacts
        .releases
        .get(version)
        .ok_or(SolcVmError::UnknownVersion)?;
    let download_url =
        releases::artifact_url(platform::platform(), version, artifact.to_string().as_str())?;

    let checksum = artifacts
        .get_checksum(version)
        .unwrap_or_else(|| panic!("checksum not available: {:?}", version.to_string()));

    let res = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .expect("reqwest::Client::new()")
        .get(download_url.clone())
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(SolcVmError::UnsuccessfulResponse(
            download_url,
            res.status(),
        ));
    }

    let binbytes = res.bytes().await?;
    ensure_checksum(&binbytes, version, checksum)?;

    // lock file to indicate that installation of this solc version will be in progress.
    let lock_path = lock_file_path(version);
    // wait until lock file is released, possibly by another parallel thread trying to install the
    // same version of solc.
    let _lock = try_lock_file(lock_path)?;

    do_install(
        version.clone(),
        binbytes.to_vec(),
        artifact.to_string().as_str(),
    )
}

fn do_install(
    version: Version,
    binbytes: Vec<u8>,
    _artifact: &str,
) -> Result<PathBuf, SolcVmError> {
    let installer = {
        setup_version(version.to_string().as_str())?;

        Installer { version, binbytes }
    };

    // Solc versions <= 0.7.1 are .zip files for Windows only
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    if _artifact.ends_with(".zip") {
        return installer.install_zip();
    }

    installer.install()
}

/// Removes the provided version of Solc from the machine.
pub fn remove_version(version: &Version) -> Result<(), SolcVmError> {
    fs::remove_dir_all(version_path(version.to_string().as_str()))?;
    Ok(())
}

/// Setup SVM home directory.
pub fn setup_data_dir() -> Result<PathBuf, SolcVmError> {
    // create $XDG_DATA_HOME or ~/.local/share/svm, or fallback to ~/.svm
    let svm_dir = SVM_DATA_DIR.to_path_buf();
    if !svm_dir.as_path().exists() {
        // Create the directory, continuing if the directory came into existence after the check
        // for this if statement. This may happen if two copies of SVM run simultaneously (e.g CI).
        fs::create_dir_all(svm_dir.clone()).or_else(|err| match err.kind() {
            ErrorKind::AlreadyExists => Ok(()),
            _ => Err(err),
        })?;
    }
    // Check that the SVM directory is indeed a directory, and not e.g. a file.
    if !svm_dir.as_path().is_dir() {
        return Err(SolcVmError::IoError(std::io::Error::new(
            ErrorKind::AlreadyExists,
            "svm directory is not a directory",
        )));
    }
    // create $SVM/.global-version
    let mut global_version = SVM_DATA_DIR.to_path_buf();
    global_version.push(".global-version");
    if !global_version.as_path().exists() {
        fs::File::create(global_version.as_path())?;
    }
    Ok(svm_dir)
}

fn setup_version(version: &str) -> Result<(), SolcVmError> {
    let v = version_path(version);
    if !v.exists() {
        fs::create_dir_all(v.as_path())?
    }
    Ok(())
}

fn ensure_checksum(
    binbytes: impl AsRef<[u8]>,
    version: &Version,
    expected_checksum: Vec<u8>,
) -> Result<(), SolcVmError> {
    let mut hasher = sha2::Sha256::new();
    hasher.update(binbytes);
    let cs = &hasher.finalize()[..];
    // checksum does not match
    if cs != expected_checksum {
        return Err(SolcVmError::ChecksumMismatch {
            version: version.to_string(),
            expected: hex::encode(&expected_checksum),
            actual: hex::encode(cs),
        });
    }
    Ok(())
}

/// Creates the file and locks it exclusively, this will block if the file is currently locked
fn try_lock_file(lock_path: PathBuf) -> Result<LockFile, SolcVmError> {
    use fs2::FileExt;
    let _lock_file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)?;
    _lock_file.lock_exclusive()?;
    Ok(LockFile { _lock_file })
}

/// Closing the descriptor releases the lock. Keep the pathname so waiters and
/// subsequent installers always synchronize on the same inode.
struct LockFile {
    _lock_file: fs::File,
}

/// Returns the lockfile to use for a specific file
fn lock_file_path(version: &Version) -> PathBuf {
    SVM_DATA_DIR.join(format!(".lock-solc-{version}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        platform::Platform,
        releases::{all_releases, artifact_url},
    };
    use rand::seq::SliceRandom;
    use reqwest::Url;
    use std::process::{Command, Stdio};

    const LATEST: Version = Version::new(0, 8, 24);

    #[tokio::test]
    async fn test_data_dir_resolution() {
        let home_dir = dirs::home_dir().unwrap().join(".svm");
        let data_dir = dirs::data_dir();
        let resolved_dir = resolve_data_dir();
        if home_dir.as_path().exists() || data_dir.is_none() {
            assert_eq!(resolved_dir.as_path(), home_dir.as_path());
        } else {
            assert_eq!(resolved_dir.as_path(), data_dir.unwrap().join("svm"));
        }
    }

    #[tokio::test]
    async fn test_artifact_url() {
        let version = Version::new(0, 5, 0);
        let artifact = "solc-v0.5.0";
        assert_eq!(
            artifact_url(Platform::LinuxAarch64, &version, artifact).unwrap(),
            Url::parse(&format!(
                "https://github.com/nikitastupin/solc/raw/7687d6ce15553292adbb3e6c565eafea6e0caf85/linux/aarch64/{artifact}"
            ))
            .unwrap(),
        )
    }

    #[tokio::test]
    async fn test_install() {
        let versions = all_releases(platform())
            .await
            .unwrap()
            .releases
            .into_keys()
            .collect::<Vec<Version>>();
        let rand_version = versions.choose(&mut rand::thread_rng()).unwrap();
        assert!(install(rand_version).await.is_ok());
    }

    #[cfg(feature = "blocking")]
    #[test]
    fn blocking_test_install() {
        let versions = crate::releases::blocking_all_releases(platform::platform())
            .unwrap()
            .into_versions();
        let rand_version = versions.choose(&mut rand::thread_rng()).unwrap();
        assert!(blocking_install(rand_version).is_ok());
    }

    #[tokio::test]
    async fn test_version() {
        let version = "0.8.10".parse().unwrap();
        install(&version).await.unwrap();
        let solc_path = version_path(version.to_string().as_str()).join(format!("solc-{version}"));
        let output = Command::new(solc_path)
            .arg("--version")
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .output()
            .unwrap();

        assert!(String::from_utf8_lossy(&output.stdout)
            .as_ref()
            .contains("0.8.10"));
    }

    #[cfg(feature = "blocking")]
    #[test]
    fn blocking_test_version() {
        let version = "0.8.10".parse().unwrap();
        blocking_install(&version).unwrap();
        let solc_path = version_path(version.to_string().as_str()).join(format!("solc-{version}"));
        let output = Command::new(solc_path)
            .arg("--version")
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .output()
            .unwrap();

        assert!(String::from_utf8_lossy(&output.stdout)
            .as_ref()
            .contains("0.8.10"));
    }

    #[cfg(feature = "blocking")]
    #[test]
    fn can_install_parallel() {
        let version: Version = "0.8.10".parse().unwrap();
        let cloned_version = version.clone();
        let t = std::thread::spawn(move || blocking_install(&cloned_version));
        blocking_install(&version).unwrap();
        t.join().unwrap().unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn can_install_parallel_async() {
        let version: Version = "0.8.10".parse().unwrap();
        let cloned_version = version.clone();
        let t = tokio::task::spawn(async move { install(&cloned_version).await });
        install(&version).await.unwrap();
        t.await.unwrap().unwrap();
    }

    // ensures we can download the latest native solc for apple silicon
    #[tokio::test(flavor = "multi_thread")]
    async fn can_download_latest_native_apple_silicon() {
        let artifacts = all_releases(Platform::MacOsAarch64).await.unwrap();

        let artifact = artifacts.releases.get(&LATEST).unwrap();
        let download_url = artifact_url(
            Platform::MacOsAarch64,
            &LATEST,
            artifact.to_string().as_str(),
        )
        .unwrap();

        let checksum = artifacts.get_checksum(&LATEST).unwrap();

        let resp = reqwest::get(download_url).await.unwrap();
        assert!(resp.status().is_success());
        let binbytes = resp.bytes().await.unwrap();
        ensure_checksum(&binbytes, &LATEST, checksum).unwrap();
    }

    // ensures we can download the latest native solc for linux aarch64
    #[tokio::test(flavor = "multi_thread")]
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    async fn can_download_latest_linux_aarch64() {
        let artifacts = all_releases(Platform::LinuxAarch64).await.unwrap();

        let artifact = artifacts.releases.get(&LATEST).unwrap();
        let download_url = artifact_url(
            Platform::LinuxAarch64,
            &LATEST,
            artifact.to_string().as_str(),
        )
        .unwrap();

        let checksum = artifacts.get_checksum(&LATEST).unwrap();

        let resp = reqwest::get(download_url).await.unwrap();
        assert!(resp.status().is_success());
        let binbytes = resp.bytes().await.unwrap();
        ensure_checksum(&binbytes, &LATEST, checksum).unwrap();
    }

    #[tokio::test]
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    async fn can_install_windows_zip_release() {
        let version = "0.7.1".parse().unwrap();
        install(&version).await.unwrap();
        let solc_path =
            version_path(version.to_string().as_str()).join(&format!("solc-{}", version));
        let output = Command::new(&solc_path)
            .arg("--version")
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .output()
            .unwrap();

        assert!(String::from_utf8_lossy(&output.stdout)
            .as_ref()
            .contains("0.7.1"));
    }
}

#[cfg(test)]
mod zip_security_regressions {
    use super::*;

    #[test]
    fn zip_security_version_listing_ignores_persistent_installer_locks() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("0.8.20")).unwrap();
        fs::create_dir(dir.path().join("0.7.6")).unwrap();
        fs::write(dir.path().join(".global-version"), "0.8.20").unwrap();
        let lock = try_lock_file(dir.path().join(".lock-solc-0.8.20")).unwrap();
        drop(lock);
        assert_eq!(
            installed_versions_at(dir.path()).unwrap(),
            vec![Version::new(0, 7, 6), Version::new(0, 8, 20)]
        );
    }

    #[cfg(unix)]
    #[test]
    fn zip_security_unix_compiler_is_not_group_or_world_writable() {
        let dir = tempfile::tempdir().unwrap();
        let installer = Installer {
            version: Version::new(0, 8, 20),
            binbytes: b"installer-test-payload".to_vec(),
        };
        let path = installer
            .install_binary_at(dir.path().join("solc"))
            .unwrap();
        let mode = fs::metadata(path).unwrap().permissions().mode();
        assert_eq!(
            mode & 0o022,
            0,
            "installed compilers must not be writable by other users"
        );
        assert_ne!(mode & 0o111, 0, "the compiler must remain executable");
    }

    #[cfg(unix)]
    #[test]
    fn zip_security_waiting_installers_keep_mutual_exclusion() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("installer.lock");
        let first = try_lock_file(path.clone()).unwrap();
        // A waiter opens the current inode before the first holder releases it.
        let waiter = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        drop(first);
        fs2::FileExt::lock_exclusive(&waiter).unwrap();
        let contender = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        let error = fs2::FileExt::try_lock_exclusive(&contender)
            .expect_err("another installer must not acquire a different lock inode");
        assert_eq!(error.kind(), ErrorKind::WouldBlock);
    }

    #[test]
    fn zip_security_valid_compiler_archive_installs() {
        let dir = tempfile::tempdir().unwrap();
        let installer = Installer {
            version: Version::new(0, 8, 20),
            binbytes: include_bytes!("../tests/fixtures/valid.zip").to_vec(),
        };
        let path = installer.install_zip_at(dir.path()).unwrap();
        assert_eq!(path.file_name().unwrap(), "solc-0.8.20");
        assert_eq!(fs::read(path).unwrap(), b"archive-test-payload");
    }

    #[cfg(unix)]
    #[test]
    fn zip_security_rejects_external_symlink_writes_and_permissions() {
        let outer = tempfile::tempdir().unwrap();
        let destination = outer.path().join("install");
        let outside = outer.path().join("outside");
        fs::create_dir_all(&destination).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::set_permissions(&outside, Permissions::from_mode(0o700)).unwrap();
        let protected = outside.join("victim.txt");
        fs::write(&protected, b"protected").unwrap();
        let installer = Installer {
            version: Version::new(0, 8, 20),
            binbytes: include_bytes!("../tests/fixtures/external-symlink.zip").to_vec(),
        };
        let result = installer.install_zip_at(&destination);
        assert!(result.is_err(), "unsafe extraction must fail closed");
        assert_eq!(fs::read(protected).unwrap(), b"protected");
        assert_eq!(
            fs::metadata(outside).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert!(fs::symlink_metadata(destination.join("link")).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn zip_security_strips_privileged_executable_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let installer = Installer {
            version: Version::new(0, 8, 20),
            binbytes: include_bytes!("../tests/fixtures/privileged-mode.zip").to_vec(),
        };
        let path = installer.install_zip_at(dir.path()).unwrap();
        assert_eq!(fs::metadata(path).unwrap().permissions().mode() & 0o6000, 0);
    }
}
