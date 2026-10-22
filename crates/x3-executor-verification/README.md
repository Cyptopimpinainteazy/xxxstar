# Patched executor execution verification

This test-only workspace crate runs small real WAT programs through the production `sc-executor-wasmtime` API. It checks returned bytes, an out-of-bounds trap, and canonical NaN bits while requesting each of the four instantiation strategies. The executor may substitute a non-COW strategy on unsupported hosts; this suite does not certify four distinct effective strategies.

Run `cargo test --locked -p x3-executor-verification`. The `patched executor execution` default Local CI gate runs it as well. Full generated runtime WASM, historical upgrades, full upstream executor tests, architecture-specific advisory reproductions, and production deployment remain separate checks.
