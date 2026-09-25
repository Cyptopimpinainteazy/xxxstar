# The packet parser and the Merkle validator: two clean results, and why

2026-09-25. Sixth mutation pass. The previous five found a gas limit that was never applied, two
readers that allocated from an untrusted count, an interpreter that charged for fuel it never gave,
a validator that disagreed with its executor, and 191 test attributes nothing ran. This pass found
**nothing** in either target, and that is the finding: two parsers on the atomic path that were
written with the checks, and in one case with the tests for them.

The two targets are the ones named two cycles ago and not yet covered: `x3-packet-schema`, whose
`Packet::from_wire_format` is what `pallets/x3-kernel`'s packet adapters call on bytes a transaction
carries, and `cross-vm-bridge`'s Merkle proof validator, which is what a bridge settles against.

## `x3-packet-schema` — three gates, all present

Between a malformed packet and the per-domain decoder sit three checks, and all three are real:

* `PacketHeader::decode` consumes the header through `parity_scale_codec`, so a short header errors;
* `header.validate()` refuses a version that is not 1 and a zero domain mask;
* `payload_size` is checked against the bytes actually present (`cursor.len() < 1 + payload_size + 4`),
  which is the check the two X3 bytecode readers were missing — here the size field cannot outrun the
  buffer;
* and a CRC32 over header + type + payload, verified before any per-type decode.

`crates/x3-packet-schema/tests/wire_format_robustness.rs` pins all of it with six tests over packets
the crate's own `to_wire_format` builds first: the fixtures round-trip; every truncation is refused;
no single-byte mutation at five values panics, and any mutant that parses can be serialised again;
each header gate refuses *by name* (`Invalid packet version`, `Must target at least one domain`,
`Packet too short for payload + CRC`); a damaged body or type byte is caught by the checksum; and a
payload whose `args` vector claims `u32::MAX` elements — with a **valid CRC**, so it reaches the
decoder — errors instead of allocating. That last one is the shape that cost 95 GB two cycles ago,
and here the SCALE decoder refuses it.

## `cross-vm-bridge` — the guards are there, and they are tested

`verify_merkle_path` checks `is_empty`, then `len() < 80` (state root + block + index + leaf), then
that the remaining bytes are a multiple of 32, and only then does it slice at fixed offsets. The
sibling walk is bounded by the byte length and allocates nothing. The two refusal cases are pinned by
`test_verify_empty_merkle_proof` and `test_verify_too_short_merkle_proof` in the same file — which is
why the first pass over that file found the guard strings in the source and no test *names* for them:
the tests assert on the error variant, not on the message.

## What changed anyway

Neither crate was in the fast set. Both are root workspace members, so `test workspace` under
`--deep` ran them — the distinction last cycle's census drew between "not in the fast set" and "in no
gate at all". Two gates were added:

| gate | tests | run time |
| --- | --- | --- |
| `test x3-packet-schema` | 58 in-file + 6 new | 3 s |
| `test x3-cross-vm-bridge` | 152 | 3 s warm, ~60 s cold |

## Why the result differs from the previous five

The earlier passes found defects in code that had either no check (the bytecode readers' allocation),
a check in the wrong place (the SVM validator versus its executor), or a value taken from the wrong
source (the executor's gas limit, the interpreter's baseline). These two parsers had the checks in
the right places, and the bridge had tests for its guards. A mutation pass that finds nothing is not
a wasted pass: it is the only way to know which of those two situations a parser is in, and the
five defects it did find are what the technique is for.

## Tickets

1. **The remaining parsers of attacker bytes**, in the same order of suspicion: `x3-dns-server`'s own
   message handling, the `rbpf` SVM path's input handling (`crates/svm-integration/src/rbpf.rs`), and
   the bridge's `finality.rs` / `merkle_settlement_bridge.rs` proof decoders.
2. **A `cargo-fuzz` target over the two X3BC readers**, as `.ai/reports/x3-vm-gas-and-bytecode-20260925.md`
   already recommends — the deterministic sweep covers a fixed corpus; a fuzzer would not.
3. The four tickets from `.ai/reports/gate-census-20260925.md` are still open: the fuzz and Tauri
   workspaces' stale lockfiles (17 of 36 nested locks measured), `tests/loom-concurrency` with no
   lockfile at all, the solvency sidecar's minutes-long test, and the last five ungated crates.
