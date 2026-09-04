# Offline Cargo Vendor Setup

This repository uses a local vendored crates source for offline Cargo builds.

## Purpose

When `cargo check --workspace --offline` is required, Cargo must resolve dependencies from a local registry mirror instead of downloading from crates.io.

## When configured

The root `.cargo/config.toml` contains:

```toml
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
```

## How to populate the vendor directory

Run this on a machine with network access:

```bash
cargo vendor
```

This will populate the repository `vendor/` directory with all crate sources needed by the lockfile.

## Offline build

After `vendor/` is populated, use:

```bash
SKIP_WASM_BUILD=1 cargo check --workspace --offline
```

If Cargo still fails offline, verify that:

- `vendor/` exists and contains crate directories
- `Cargo.lock` is up to date
- `.cargo/config.toml` has the vendored source override
