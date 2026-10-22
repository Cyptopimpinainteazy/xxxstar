# Smoldot 2.2.0 decoder security compatibility patch

Source is the exact crates.io package, verified against the root Cargo.lock SHA256; provenance is recorded in patches/smoldot-provenance.json. Upstream GPL-3.0-or-later with Classpath exception is retained.

Only the manifest changes: ruzstd >=0.9.1 repairs RUSTSEC-2026-0344 without changing Smoldot APIs. The older 0.19.4 manifest also enables hex/ed25519-zebra alloc features explicitly, matching 2.2.0 and its actual alloc API use. No Rust production source is modified. Package API/decoder compile checks do not certify a running light client.

The nested workspace permits isolated package checks. Production resolution uses the root lockfile. The upstream crates.io package omits some test binary assets, so its complete upstream test suite is not claimed. The actual ruzstd block-size regression runs from patches/decoder-security-tests.

Unmodified upstream Rust sources reside under vendor/ and are byte-verified against the original crate archives. Manifest lib/bench paths point there. Project guards retain their existing third-party vendor policy; local test code stays outside vendor/.
