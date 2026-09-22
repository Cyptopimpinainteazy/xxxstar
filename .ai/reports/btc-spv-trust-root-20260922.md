# The BTC SPV trust root — what was wrong, what changed, what proves it

Date: 2026-09-22
Scope: `pallets/x3-settlement-engine` (BTC SPV path), `runtime/src/lib.rs` (config + spec_version)

## The defect, measured

The standing record said "the header chain the SPV check reads has no trusted bootstrap".
Reading the code, that understated it. There were **two** doors into `BtcHeaders`, and both
were open:

1. `submit_btc_header` required `prev_exists || header.height == 0`. The `|| height == 0`
   branch meant no parent was needed at all. `nBits` is a field the submitter writes, so a
   chain could be started anywhere for the price of one hash — and every header above it
   inherited that fabricated root.
2. `submit_btc_proof` *inserted* the header its argument carried (`BtcHeaders::insert(...)`)
   with **no proof-of-work check, no parent check and no height check**. `confirmations` was
   then computed from the caller's own `height` field. A maker or taker could therefore hand
   the pallet a header they had just made up, choose the merkle root inside it, and have the
   SPV check verify their forged proof against their forged header.

The `verify_btc_pow` fix of 2026-09-22 (Bitcoin's 80 wire bytes + `CheckProofOfWork`
rejections) made the *hashing* honest. It did not give the chain anything to compare against,
because the submitter still chose the target.

## The change

- **`anchor_btc_checkpoint(header)`** (new, `call_index(34)`, root). Commits this chain to
  "Bitcoin block H has hash X": `BtcCheckpoints` (height → hash), write-once per height, with
  `BtcCheckpointAnchored` published so the commitment can be checked against a Bitcoin node.
  A second, different hash for the same height is `BtcCheckpointConflict`.
- **`BtcPoWLimitBits`** (new Config item). Mainnet and testnet `0x1d00ffff`, dev `0x207fffff`
  — Bitcoin's own `powLimit` per network. A target easier than the limit is
  `BtcPowLimitExceeded`, so neither an anchor nor an extension can be mined in one hash.
- **One admission path, `btc_admit_header`.** It checks: PoW; target inside the limit; the
  parent is stored *and anchored*; `height == parent.height + 1` (the height is **derived**,
  never read from the header); `nBits` is copied verbatim off a retarget boundary and moves by
  at most 4x on one (`BTC_RETARGET_INTERVAL_BLOCKS` 2016, `BTC_RETARGET_MAX_FACTOR` 4); the
  timestamp postdates the median of up to 11 ancestors (`BTC_MEDIAN_TIME_SPAN_BLOCKS`, Bitcoin's
  median-time-past). Only `anchor_btc_checkpoint` may start a chain.
- **`BtcHeaderMetaStore`** (new storage, hash → `BtcHeaderMeta { height, anchored }`), so the
  derived height and anchored-ness are readable by the SPV paths.
- **Both SPV entry points require an anchored header at the derived height**:
  `submit_btc_proof` (which no longer inserts anything) and
  `verify_btc_settlement_proof` (the external-proof path).
- `spec_version` 13 → 14. New storage, new call. No migration: the maps start empty and no
  deployed chain has BTC headers.

## Proof

**The reproduction, in one test:**

```
cargo test -p pallet-x3-settlement-engine the_raw_spv_verifier_accepts_the_fixture_the_pallet_refuses
```

It builds a forged block (mined at the test network's `powLimit`, real transaction hash as
merkle root, header never seen by the chain) and asserts all three states:

* `BtcSpvProof::verify` — the whole of what the pallet's SPV check is once the proof is
  decoded, and the whole of what it used to be — **returns true**. That is the pre-fix
  behaviour, reproduced on demand.
* `Pallet::verify_proof` returns `Ok(false)` for the same bytes.
* After `anchor_btc_checkpoint` admits that header, the same bytes return `Ok(true)` — so the
  change is the trust question, not the merkle arithmetic.

**Negative controls (all in `pallets/x3-settlement-engine/src/tests.rs`):**

| test | what it pins |
| --- | --- |
| `a_btc_header_chain_cannot_start_without_a_checkpoint` | `height == 0` is no longer an escape hatch (`BtcParentMissing`), and an easier-than-limit target is refused (`BtcPowLimitExceeded`) |
| `a_bitcoin_proof_cannot_use_a_header_this_chain_never_admitted` | a well-formed proof over a fabricated header is refused |
| `a_btc_checkpoint_height_cannot_be_re_pointed` | the anchor is a commitment; a competing hash for the same height is refused and storage is unchanged |
| `a_btc_extension_must_be_contiguous_with_its_parent` | a header claiming height 9,500,000 over a parent at 500 is refused |
| `a_btc_extension_cannot_change_difficulty_off_a_retarget_boundary` | any `nBits` change between retargets is refused |
| `a_btc_extension_must_postdate_the_median_of_its_ancestors` | a backdated child is refused |
| `submit_btc_proof_refuses_a_header_that_was_never_admitted` | the second door: `BtcHeaderNotAnchored` |
| `the_retarget_bound_bites_exactly_at_a_factor_of_four` | the boundary branch a fixture cannot reach (it needs mainnet difficulty to hit `height % 2016 == 0`), stated against the rule: 4x is allowed, 8x is not, verbatim elsewhere |

**Results:** `cargo test -p pallet-x3-settlement-engine` → 144 + 23 passed, 0 failed.
`cargo check -p pallet-x3-settlement-engine --features runtime-benchmarks` → clean.

## A second, pre-existing break found on the way

`--features runtime-benchmarks` did not compile on master: the `submit_proof` benchmark's
`SettlementProof` initializer was missing `receipt_index` and `trie_proof`, so the whole
benchmarking build was broken. Fixed here. The `submit_btc_header` and
`anchor_btc_checkpoint` benchmarks are deliberately **not** written: both verify proof of work
against the network's `powLimit`, and a benchmark body cannot produce that input without
paying mainnet's work — the old bench only "passed" because the pallet then accepted a header
whose `nBits` the caller chose. Their weights are fixed constants.

## What this does not claim

No Bitcoin chain of any kind was involved: the fixtures are mined locally at regtest
difficulty, and the anchored hashes are fixtures, not real Bitcoin checkpoints. An operator
must anchor a real one. Nothing pushes headers after the anchor (a bonded header relayer does
not exist), and the SPV path is fail-closed until an anchor is set — which is the intended
behaviour, not a regression.
