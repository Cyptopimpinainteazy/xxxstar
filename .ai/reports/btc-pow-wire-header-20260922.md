# The Bitcoin header check was not checking Bitcoin

Date: 2026-09-22. Base: `origin/master` = `3c7d3b349`, merged as `1ced8fb78`.

Found while auditing the weakest P0 rows of `FEATURE_MATRIX.toml`, which is where
the last two readiness defects came from. The row said "BTC proof verification was
placeholder"; half of that was true, and it was the half that matters.

## 1. The block hash was the hash of a struct, not of a block

`pallets/x3-settlement-engine/src/lib.rs`:

```rust
fn compute_btc_block_hash(header: &BtcBlockHeader) -> H256 {
    let data = header.encode();          // SCALE encoding of the pallet's struct
    ...
}
```

`BtcBlockHeader` carries `height: u64`, which exists only so the pallet can store a
height beside the header — Bitcoin's header is 80 bytes and has no height. Hashing
the SCALE encoding hashed 88 bytes with an eight-byte suffix no block has, so the
value was not the block's hash and every proof-of-work comparison against it
compared an unrelated number.

`verify_btc_settlement_proof` even computed the check and discarded it:

```rust
let _ = block_hash_matches;   // "SCALE and wire encodings differ in field ordering"
```

That comment was the bug, written down.

## 2. An overflowing target was "any hash passes"

```rust
if size > 32 {
    // Target is larger than 256 bits, so any hash passes
    return Ok(true);
}
```

Bitcoin's `CheckProofOfWork` rejects a target that does not fit in 256 bits. This
accepted the header. With `bits = 0xff7fffff` a caller with root could put a header
into `BtcHeaders` with no proof of work at all.

## 3. Negative targets were read as positive

The mantissa was taken as `bits & 0x00ffffff`, so the sign bit Bitcoin treats as a
negative (invalid) target was folded into the value.

## The fix

* `btc_header_wire_bytes` builds Bitcoin's 80 bytes — version, previous hash,
  merkle root, timestamp, bits, nonce, each little-endian — and
  `compute_btc_block_hash` hashes those.
* `btc_target_le` decodes `nBits` with Bitcoin's `SetCompact` semantics and
  returns the target little-endian, rejecting the three targets
  `CheckProofOfWork` rejects: negative, overflowing (including the `size == 34`
  case), and zero. `size <= 3` is decoded instead of refused.
* the settlement path now **requires** `proof.block_hash` to equal the hash of the
  header the proof carries, instead of computing it and dropping it.
* `spec_version` 11 → 12: `BtcHeaders` keys are this hash, so headers filed by an
  earlier build would sit under different keys. No migration is needed — every
  deployed chain has an empty `BtcHeaders` map, because the BTC path is not live
  on any network yet.

## Proof

```
cargo test -p pallet-x3-settlement-engine --lib          # 135 passed
bash scripts/local-ci.sh --variants --only clippy-runtime-rc1,test-settlement-engine,no-default-features-crates
  PASS clippy runtime rc1 396s · PASS test settlement-engine 11s ·
  PASS no-default-features crates 1497s (68-crate no_std sweep)
```

New tests: `btc_block_hash_is_double_sha256_over_the_eighty_wire_bytes` (states the
wire layout in the test, submits a mined header, requires the storage key to equal
that hash and to differ from the SCALE hash),
`btc_submit_header_refuses_targets_bitcoin_refuses` (overflow/negative/zero),
`btc_submit_header_refuses_a_header_that_misses_its_target`,
`btc_submit_header_accepts_a_header_that_meets_its_target`,
`btc_settlement_proof_naming_a_different_block_is_refused`.

Negative control — only the pallet fix reverted:

```
test tests::btc_submit_header_refuses_targets_bitcoin_refuses ... FAILED
test tests::btc_block_hash_is_double_sha256_over_the_eighty_wire_bytes ... FAILED
test tests::btc_settlement_proof_naming_a_different_block_is_refused ... FAILED
test result: FAILED. 41 passed; 3 failed
```

The first failure is the hole itself: the overflowing target was accepted. Two
existing fixtures had to be corrected (they named arbitrary block hashes,
`0xAB`/`0xDD`) — strengthened, not weakened, to name the header's real hash.

## The attested bytes moved

```
./scripts/update-runtime-hashes.sh             # two builds agree
compact    8433833 bytes  0xbbb17006c0948307a775e8d3779e5a25a232deb4dce5ef8c3a17b1c70377ce61
compressed 1444249 bytes  0x0c36f055f2904497f2770d3eed5c2ac3fe48b67f1ec0f1aa3f23eb101d8a37a8
recorded_revision 04fb44907 -> 5e0f86760
```

…and so did the record's metadata, which had been lying since the last bump: it
said `runtime_version: "x3-chain-11 (…)"` while the artifact reports
`x3-chain-12 (…)`, because `update-runtime-hashes.sh` rewrote only the five hash
fields and carried every other field over. `_srtool_values` now parses `Version`
and `Metadata` from the same block as the hashes, stage 6b compares them, and the
script writes them (refusing to write at all if srtool printed no version line).

## Readiness records corrected in the same change

* `[btc_fortress_gateway]` pointed at `crates/x3-gateway` — the REST/GraphQL data
  service, whose only Bitcoin reference is an HTLC type import in one handler. It
  now points at the settlement engine's `btc_gateway` + header chain, the vault and
  the bridge's HTLC/SPV modules, cites six real tests, and moves 25 → 45 with
  blockers that are true (no live BTC run of any kind, no trusted header bootstrap,
  no signer quorum, mainnet flag off, no audit).
* `X3-XCHAIN-001/002` no longer claim "proof verification was placeholder" without
  qualification; they name the components, the reproduction tests and the three
  SPV implementations (only the pallet's is on the runtime path).
* `scripts/check-readiness-consistency.sh` extracted `required_tests` names with
  `grep -oE '"[a-zA-Z0-9_]+"'`, so any citation containing a separator
  (`script.sh::case`) matched nothing and was **never checked** — the same escape
  that let the CRITICAL-TOK-1 fictional names sit in the registry. It now captures
  whole quoted strings and resolves the tail after `::`; planting
  `"test-live-lifecycle.sh::a_case_nobody_wrote"` produces a VIOLATION, removing it
  returns PASS.
