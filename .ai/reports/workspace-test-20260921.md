# Full workspace test run — evidence (partial)

Date: 2026-09-21/22
Head: current local `master` (synced with `origin/master`)

## Command

```text
CARGO_TARGET_DIR=/tmp/x3target190 \
OPENSSL_DIR=/usr OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu OPENSSL_INCLUDE_DIR=/usr/include \
SKIP_WASM_BUILD=1 \
rustup run 1.90.0 cargo test --workspace --all-targets
```

## Result

The full workspace compiled and ran a large portion of the suite successfully
(e2e, gateway integration, settlement, pallets, property tests, etc.). The run
then produced no output for over 9 minutes after the `parallel-proposer`
`benches/authoring_overhead.rs` benchmark and was interrupted rather than
reported as a full pass.

This is a hang/slow benchmark in the root workspace test path, not in
`x3-lang`. The x3-lang workspace gate itself remains green (tests, clippy,
fmt), recorded in `x3lang-current-green-20260921.md`.

## Next action

Run only `parallel-proposer` benches/tests with a timeout to isolate the
non-terminating test, then either bound it or exclude it from the workspace
test path if it is a long-running benchmark rather than a correctness test.

## Isolation attempt (2026-09-22)

`cargo test -p parallel-proposer --all-targets` could not be isolated in the
default workspace target directory: cached artifacts were built with rustc
1.98.1 while the active toolchain is 1.90.0, producing `E0514`/`E0614` errors
before the tests ran. The hang therefore remains unisolated until a fresh
`CARGO_TARGET_DIR` build is used for that single crate.

Follow-up attempt (2026-09-22): a fresh `CARGO_TARGET_DIR` build of
`parallel-proposer` without `SKIP_WASM_BUILD=1` still reached the runtime WASM
build and failed on the missing `std` target, so the benchmark hang is still
not isolated. The workspace-level x3-lang gate remains green; this is a root
benchmark/test-path issue, not x3-lang.

Resolved (2026-09-22): the hang was the `authoring_overhead` benchmark being
compiled/run by `cargo test --workspace --all-targets`. It is now gated behind
a `bench` feature in `crates/parallel-proposer/Cargo.toml`
(`required-features = ["bench"]`), so the workspace test path skips it and
completes. Run it deliberately with
`cargo bench -p parallel-proposer --features bench --bench authoring_overhead`.

Full rerun (2026-09-22) with the benchmark gated off: the hang is gone, but
with `SKIP_WASM_BUILD=1` two `x3-chain-node` tests fail because they require
the embedded runtime WASM blob that the skip flag intentionally omits:
`default_keystore_path_matches_the_layout_a_running_node_uses` and
`full_node_http_rpc_...`. Those are environment-induced by the skip flag, not
x3-lang failures. A true full pass requires the canonical `cargo test
--workspace` without `SKIP_WASM_BUILD=1`.

Canonical attempt (2026-09-22) without `SKIP_WASM_BUILD`: the runtime WASM
build fails while resolving `crypto-common v0.1.6` for `wasm32v1-none`
(`can't find crate for std`). The workspace patch is `crypto-common v0.1.7`,
and the isolated wbuild sub-manifest did not inherit the patch set. This is a
runtime WASM build-environment issue, not an x3-lang code failure.
