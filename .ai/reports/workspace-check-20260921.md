# Root workspace compile check — evidence

Date: 2026-09-21
Commit: current local `master` head

## Command

```text
CARGO_TARGET_DIR=/tmp/x3target190 \
OPENSSL_DIR=/usr OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu OPENSSL_INCLUDE_DIR=/usr/include \
SKIP_WASM_BUILD=1 \
rustup run 1.90.0 cargo check --workspace
```

## Result

PASS — `Finished dev profile` for the full native workspace, including the
runtime crate (`x3-chain-runtime`), node, pallets, crates, and tools.

`SKIP_WASM_BUILD=1` is required only because the runtime's WASM build script
otherwise fetches the `polkadot-sdk` git dependency, which cannot resolve
`github.com` in this environment. The native compilation has no such dependency
on the network; the check is green once that WASM-only fetch is skipped.

## Notes

- The build emitted only the known `pallet-x3-control` unused-manifest warning,
  unused patch warnings, and a `uint v0.4.1` future-incompat notice.
- No compile errors in any workspace member.
