# Toolchain

- Rust workspace is pinned by rust-toolchain.toml to Rust 1.90.0, minimal profile, rustfmt/clippy/rust-src, targets wasm32-unknown-unknown and wasm32v1-none.
- Cargo workspace is large and intentionally excludes desktop Tauri sources, X3-contracts, adapters, and standalone SVM program crates; inspect root Cargo.toml before assuming workspace membership.
- JavaScript package manager is pnpm 10.15.1, with many nested npm projects. Root package.json delegates tests/builds to app/package workspaces.
- Main languages/configs: Rust, TypeScript/JavaScript, Python, Bash, YAML, TOML, Markdown.
- Substrate/Polkadot SDK dependencies are pinned or patched in root Cargo.toml; avoid broad dependency upgrades without targeted validation.