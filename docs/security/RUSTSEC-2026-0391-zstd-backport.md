# zstd advisory remediation

## Exposure

RustSec published a coordinated set of zstd advisories on 2026-10-16. The
refreshed lock contained:

- `zstd 0.13.3` with `RUSTSEC-2026-0391`, where a reused stream context could
  expose plaintext left by an abandoned frame;
- `zstd-safe 5.0.2`, `6.0.6`, and `7.3.0` with `RUSTSEC-2026-0398` and
  `RUSTSEC-2026-0403`, plus the related unsound context, buffer, dictionary,
  seekable, and training fixes shipped in the same upstream release;
- `zstd-sys 2.1.0` with `RUSTSEC-2026-0410`, a wasm32 allocation-failure
  memory-corruption bug.

The fixes are published in `zstd-safe >= 8.2.0` and
`zstd-sys >= 2.1.2`. The released high-level `zstd` fix is `>= 0.14.3`, but
Actix HTTP, Solana, and Wasmtime in this graph require the compatible `0.13`
line.

## Vendored packages

`patches/zstd` vendors `zstd 0.13.3` as `0.13.4`, uses fixed
`zstd-safe 8.2.0`, and backports upstream commit
`74ace2dcdca62a2ce0f034895d6079458311e003`. Both `Encoder::with_context` and
`Decoder::with_context` reset a borrowed session with
`ResetDirective::SessionOnly`, discarding abandoned frame state while retaining
parameters and dictionary configuration.

`patches/zstd-v012` vendors `zstd 0.12.4` as `0.12.5` for
polkadot-sdk's compatible dependency line. That high-level release predates
the vulnerable `with_context` API, but it is rebuilt against `zstd-safe 8.2.0`;
its prepared-dictionary regression test compiles and exercises the lifetime
changes required by the fixed safe API. The current configured target graph
does not build this lock-only copy, but keeping it patched prevents
cargo-audit from finding a vulnerable stale package.

`patches/zip` vendors the Windows-only `zip 0.6.6` path as `0.6.7` and moves
its optional compressor from `zstd 0.11` to the patched `0.13.4`. This removes
the otherwise unreachable `zstd 0.11`/`zstd-safe 5` pair from `Cargo.lock`.

`patches/sp-maybe-compressed-blob` preserves the Substrate helper's upstream
implementation and test suite while replacing its vulnerable
`zstd 0.12`/`zstd-safe 6` dependency with `patches/zstd`.

No advisory ignore was added for these IDs. The fixed lock must stay clean
without suppressing `RUSTSEC-2026-0391`, `RUSTSEC-2026-0398`,
`RUSTSEC-2026-0403`, or `RUSTSEC-2026-0410`.

## Regression coverage

The vendored packages carry focused executable coverage:

- `patches/zstd/tests/zstd_backport.rs` proves encoder and decoder contexts
  cannot carry abandoned session data into a new frame.
- `patches/zstd-v012/tests/regression.rs` performs a prepared-dictionary
  compression/decompression round trip through `zstd-safe 8.2.0`.
- `patches/zip/tests/zstd_round_trip.rs` writes and reads a zstd-compressed ZIP
  entry through the patched high-level crate.
- `patches/sp-maybe-compressed-blob` retains the four upstream compression,
  decompression, bomb-limit, and magic-prefix tests.

The `vendored zstd patch regressions` local-CI gate runs all standalone patch
suites. `dependency audit`, `cargo deny check advisories`, and `advisory scope`
independently verify that the production lock no longer resolves any affected
versions.
