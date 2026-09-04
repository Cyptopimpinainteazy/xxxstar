# CLAUDE Notes for external-chains crate

- This crate lives in a Substrate workspace and is compiled both with `std` and potentially `no_std`. Avoid pulling in `std`-only dependencies unless guarded by `#[cfg(feature="std")]` and `Cargo.toml` feature flags.
- When adding new error variants, also add corresponding constructor/helper methods and update all `.map_err` usages to use them. This prevents frequent Vec<u8> mismatches when converting strings.
- HTTP logic is abstracted into `rpc_http.rs`; keep platform-specific implementations there to ease testing and compilation across targets.
- Adding new external crates requires editing `Cargo.toml` and enabling features for the `std` feature if needed.
