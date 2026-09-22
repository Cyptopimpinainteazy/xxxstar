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
