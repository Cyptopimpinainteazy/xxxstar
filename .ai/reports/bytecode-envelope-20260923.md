# One envelope, one checksum, three implementations that disagreed

Date: 2026-09-23. Closes TICKET-108. Found while measuring two matrix rows (TICKET-101).

## What the ticket asked for, and what was actually there

The ticket said the X3BC header's version and checksum were written and never checked. Measuring it
found something worse: **three implementations of the checksum, and the only one that compared
anything was computing a different function.**

| site | algorithm | did it check? |
| --- | --- | --- |
| `x3-backend::bc_format::BytecodeModule::to_bytes` (the writer) | `sum = sum*31 + byte` (wrapping) | — it produces the value |
| `x3-backend::bc_format_helpers` (test fixtures) | CRC32, polynomial `0xEDB88320` | — |
| `x3-vm::verifier::verify_module_bytes` | CRC32 again | **yes**, but only when the header's value was non-zero |

So the verifier compared its CRC32 against a value the writer had produced with a multiply-and-add:
they disagree for essentially every module, and a module with a zeroed checksum field skipped the
check entirely. Reproduced before changing anything, with a test that is now the regression test:

```
test verifier::tests::a_written_module_passes_its_own_checksum ... FAILED
  a module written by the compiler must verify, got ParseError("ChecksumMismatch { expected: ..., found: ... }")
```

That is the VM refusing the compiler's own output at the gate meant to protect it — and accepting
bytecode that had no checksum at all.

## What is true now

* **One definition**, in `x3-common::bytecode`: `MAGIC`, `HEADER_LEN`, `CHECKSUM_OFFSET`, the packed
  `VERSION` / `MIN_SUPPORTED_VERSION` / `MAX_SUPPORTED_VERSION`, `checksum(body)`, and the two version
  predicates. `x3-common` is the crate both sides already depend on without `std`, which is why the
  definition can live in one place.
* **The writer** re-exports those constants and calls that checksum.
* **`BytecodeModule::from_bytes`** verifies the checksum it reads — a new `ChecksumMismatch { expected,
  found }` error — so the check happens on every parse, not in one reader that had its own idea.
* **`x3-vm`'s verifier** no longer computes anything: the CRC32 block is deleted, and the parse it
  already performs is the check. One place, always on.
* **`mini_x3`** (the no-std decoder `executor::execute` uses when `std` is off, i.e. the runtime's
  case) reads the twenty header bytes it used to `skip`, and refuses a version it cannot read, a
  `min_version` this loader does not satisfy, and a checksum mismatch.
* **Fixtures that hand-assembled envelopes** — two in `x3-vm`, one in `x3-integration`, one in
  `bc_format_helpers` — now write a real checksum. One of them had been declaring format version `1`
  rather than `1.0.0` (packed `0x0001_0000`), which the format's own rules call a different major; it
  only loaded because the header was skipped.

## Tests

```
x3-common            bytecode::tests::{the_checksum_is_order_sensitive_and_deterministic,
                                      version_bounds_match_the_packing}
x3-backend           bc_format::tests::{a_corrupted_body_fails_the_checksum,
                                        the_shared_version_predicates_agree_with_version_info}
                     (the last one compares the shared integer predicates against
                      VersionInfo::can_read/satisfies, so the two cannot drift)
x3-vm                verifier::tests::{a_written_module_passes_its_own_checksum,   <- the reproduction
                                       a_corrupted_body_fails_the_checksum}
x3-integration       mini_x3::tests::{test_a_corrupted_body_is_rejected,
                                      test_a_future_format_version_is_rejected,
                                      test_a_module_requiring_a_newer_loader_is_rejected}
```

```
cargo test -p x3-common -p x3-backend -p x3-x3-integration -p x3-compiler    all green
cargo test -p x3-vm                                                          150 passed
```

## What this did not fix, and one thing it broke and fixed again

`cargo check -p x3-common --no-default-features` fails — and did so **before** this change too,
measured at `cc19883faf`: a `String`-carrying enum with serde derives, in a crate that turns `std` off,
without `serde`'s `alloc` feature on that path. Every crate depending on `x3-common` with
`default-features = false` inherits it, which is why `scripts/check-no-default-features.sh` is red with
an empty known-list. The runtime's WASM build is unaffected (that graph enables `serde` with `alloc`);
it is the isolated configuration the gate checks. That is TICKET-109.

The same insertion that added the `bytecode` module also **moved `#[cfg(feature = "std")]` off
`pub mod signing;` and onto the new module**, un-gating `signing` for no-std builds. The runtime's WASM
build then failed inside srtool — the check that caught it — and seven crates' no-default-features
builds failed on signing as well. Restoring the attribute (and leaving `bytecode` ungated, which is
what it needs to be) puts both back. The first filing of TICKET-109 blamed the repository for what was
this agent's bug; it is corrected there and in the memory file, because a ticket that misdirects the
next reader is worse than no ticket.
