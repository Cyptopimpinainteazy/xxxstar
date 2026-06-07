fn main() {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let version = std::process::Command::new(rustc)
        .arg("--version")
        .output()
        .expect("failed to detect rustc version")
        .stdout;

    let full_version = String::from_utf8(version).unwrap();
    let full_version = full_version.trim();
    // Examples of version strings:
    //   'rustc 1.86.0 (05f9846f8 2025-03-31)'
    //   'rustc 1.90.0-nightly (e9182f195 2025-07-13)'
    //   'rustc 1.91.1 (ed61e7d7e 2025-11-07) (built from a source tarball)'

    const YEAR: u32 = 10000;
    const MONTH: u32 = 100;

    let version = full_version.split(' ').nth(1).unwrap();
    let is_nightly = full_version.contains("-nightly");
    let date = {
        let first_close_paren = full_version.find(')').unwrap();
        let date_start = full_version[..first_close_paren].rfind(' ').unwrap() + 1;
        let mut it = full_version[date_start..first_close_paren].split('-');
        let year: u32 = it.next().unwrap().parse().unwrap();
        let month: u32 = it.next().unwrap().parse().unwrap();
        let day: u32 = it.next().unwrap().parse().unwrap();
        year * YEAR + month * MONTH + day
    };
    let mut it = version.split('.');
    let major: u32 = it.next().unwrap().parse().unwrap();
    let minor: u32 = it.next().unwrap().parse().unwrap();

    println!("cargo::rustc-check-cfg=cfg(picosimd, values(\"avx512\", \"supports_safe_intrinsics\"))");

    let check_feature = |req_minor: u32, year: u32, month: u32, day: u32| -> bool {
        major > 1
            || (major == 1 && minor >= req_minor && !is_nightly)
            || (major == 1 && minor >= (req_minor + 1))
            || (major == 1 && is_nightly && date >= year * YEAR + month * MONTH + day)
    };

    if check_feature(87, 2025, 3, 8) {
        println!("cargo::rustc-cfg=picosimd=\"supports_safe_intrinsics\"");
    }

    if check_feature(89, 2025, 6, 19) {
        println!("cargo::rustc-cfg=picosimd=\"avx512\"");
    }
}
