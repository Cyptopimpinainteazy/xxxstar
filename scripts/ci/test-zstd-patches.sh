#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
target_dir="${CARGO_TARGET_DIR:-${repo_root}/target/vendor-zstd-tests}"

run_patch_tests() {
    cargo test \
        --manifest-path "$1" \
        --target-dir "$target_dir" \
        --locked \
        --offline \
        "${@:2}"
}

run_patch_tests "${repo_root}/patches/zstd/Cargo.toml"
run_patch_tests "${repo_root}/patches/zstd-v012/Cargo.toml"
run_patch_tests \
    "${repo_root}/patches/zip/Cargo.toml" \
    --config "patch.crates-io.zstd.path=\"${repo_root}/patches/zstd\""
run_patch_tests "${repo_root}/patches/sp-maybe-compressed-blob/Cargo.toml"
