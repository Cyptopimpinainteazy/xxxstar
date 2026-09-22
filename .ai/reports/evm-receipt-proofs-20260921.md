# The EVM receipt path could not verify a real block (2026-09-21)

Merged as `#410` / `7e5924309` and `#411` / `a89dc63f3`.

## Starting point

Last turn's finding was "no live component produces a verified external proof". Building
one found three defects underneath it, in order.

## 1. The MPT walk refused every real block — `#410`

`x3-verification-router::verify_merkle_patricia_proof` had two independent bugs:

- a **branch node was required to be the last proof node** (`if i != last { BadProof }`).
  The path from a receipts root to a receipt is *branch → … → leaf*, so every proof for a
  block with more than one transaction was refused. The walk accepted exactly one shape: a
  single leaf at the root, i.e. a one-receipt block.
- **leaf-vs-extension was read from the wrong bit**: `first_nibble & 1` is the odd/even flag
  of the hex-prefix encoding; the leaf flag is the second bit (`2 * leaf + odd`). A leaf
  whose remaining path had an odd number of nibbles was walked as an extension.

Both are on fund paths: `pallet-x3-settlement-engine` gates `Finalized`/`Refunded` on this
walk, and `x3-crosschain-gateway` uses it for deposit credit.

### Evidence

`crates/x3-verification-router/tests/data/anvil_block1_receipts.hex` is `debug_getRawReceipts`
from a real anvil block (3 legacy transfers) — the node's consensus bytes, not a re-encoding.

```
the_producer_reproduces_the_root_the_chain_computed ... ok   # == header receiptsRoot
every_receipt_of_a_real_block_has_a_verifying_proof ... ok
a_real_blocks_proof_is_not_a_proof_for_another_index ... ok
```

The first is the one that settles it: a trie built from the block's receipts equals the
header's `receiptsRoot`, so the construction is Ethereum's rather than this crate's idea of
it. The other two each fail with `BadProof` under either defect — that is how they were found,
on real data rather than in a synthetic fixture. Tamper tests cover a truncated proof, a leaf
borrowed from another index, a proof under the wrong key and a proof against a modified root.

The producer (`receipts_trie_root`, `receipts_trie_proof`) now lives beside the verifier so the
two cannot drift into different conventions.

## 2. Typed receipts were refused before the walk — `#411`

Both verifiers required an RLP list prefix on the raw bytes. A typed receipt is
`type || rlp(payload)` (`0x02` for EIP-1559), so any leg sent by a wallet made since 2021 was
refused as malformed. The type byte stays in `receipt_data` — the trie leaf holds the whole
consensus encoding and `keccak256` over it is what the proof is checked against — and is
skipped only structurally. The four types sharing a legacy payload shape (`0x01`, `0x02`,
`0x03`, `0x04`) are listed once in the router as `TYPED_RECEIPT_TYPES`; an unknown type is
refused rather than walked with another type's assumptions.

Found while capturing the fixture in #410: anvil's default receipts are `0x02`-prefixed,
which is why that fixture had to pin legacy transfers.

## What is still missing for a live verified external settlement

1. **The trust root is the attester set.** `RuntimeCrossChainValidator` delegates to
   `pallet_cross_chain_validator::verify_settlement_evm_header`, which compares the proof's
   `(block_number, block_hash, state_root, merkle_root)` against `LastEvmHeader` — a single
   stored header, written by an account in `AuthorizedSubmitters`. The receipt's *inclusion*
   is now genuinely verified; the header those roots come from is an attestation.
2. **No client-side producer builds a `SettlementProof` from a chain.** The trie logic exists
   (`receipts_trie_root` / `receipts_trie_proof`), but nothing fetches a block, RLP-encodes its
   receipts in consensus form, fills `receipt_index` / `trie_proof` / `merkle_proof` and calls
   `submit_proof`. That is the next piece.
3. **`LastEvmHeader` is one slot**, so a proof is only satisfiable for the header most recently
   attested; the `confirmations` field in the proof is caller-supplied rather than derived from
   a chain head. Worth a design pass before mainnet.

## Pre-existing, unchanged

`cargo test -p x3-crosschain-gateway` fails 5 tests on master and on these branches, with
`VerificationFailed("no verifier implemented for this strategy: failing closed")` — the test
router registers no verifier for the strategy its envelopes declare. The gateway's deposit
credit path is therefore untested today.
