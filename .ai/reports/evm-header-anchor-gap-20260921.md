# The router's EVM receipt verifier has no anchor for the header it verifies against

Found while deciding whether to register `ProductionEvmReceiptVerifier` for `EvmReceiptProof`
routes in the cross-chain gateway (the open item left by `#413`). It should not be registered,
and the reason is worth stating precisely.

## What the verifier checks

`crates/x3-verification-router/src/evm_receipt.rs::DecodedProof::validate`:

1. the receipt's log matches the expected selector, recipient and amount;
2. `verify_merkle_patricia_proof(header.receipts_root, rlp(receipt_index), receipt_rlp, proof)`
   — the receipt is *in* the trie whose root the header names;
3. `confirmations = current_block_number - header.number >= min_confirmations`.

Steps 1 and 2 are sound: the walk is now verified against a real block (`#410`), and a receipt
cannot be forged into an attested root.

## The gap

`header`, `current_block_number` and `expected_chain_id` all arrive inside the proof payload
(`DecodedProof::decode`), and the verifier's own doc says so: "the receipt is verified to be
included in the block whose `receipts_root` the caller asserts".

- **The header is not anchored.** A prover builds a receipt trie containing a receipt they
  control, uses that trie's root as the header's `receipts_root`, and the walk succeeds against
  their own root. Nothing ties `receipts_root`, `header.number` or the block hash to the chain.
- **The confirmation depth is the prover's own number.** `current_block_number` is read from the
  payload, so a prover sets it to any height that satisfies `min_confirmations`. The check
  proves `payload_head - payload_height >= N`, not that the block is buried under N real blocks.

Both are the shape of the `TICKET-063`/`#389` class: a proof that is self-consistent and
self-attested. The walk makes the *inclusion* real; nothing makes the *header* real.

## Why this is not the same as the on-chain settlement path

`pallet-x3-settlement-engine::verify_evm_receipt_proof` compares the proof's
`(block_number, block_hash, state_root, merkle_root)` against `LastEvmHeader` — a header stored by
an account in `AuthorizedSubmitters` — *before* the walk runs. That path is anchored (to an
attester set, which is itself a trust decision, but not to the prover).

The router path has no such comparison, and off-chain there is no store to compare against.

## What registering it would have done

A gateway route with `verification_level: EvmReceiptProof` would have accepted a proof whose
receipt is real but whose block is whichever block the prover chose, and credited X3
representation against it. Failing closed with `MissingVerifier` is the better state until the
verifier takes an anchor.

## The shape of the fix

`ProductionEvmReceiptVerifier` should not be constructible without the header it will accept:
either

- an explicit expectation — `(receipts_root, block_number, chain_id)` supplied by whoever holds
  the chain's view (on-chain: `LastEvmHeader`; off-chain: a relayer's attested head), checked
  before the walk; or
- a callback into that store, so the verifier asks for the attested root by height instead of
  reading one out of the payload.

Confirmations then follow from the same anchor: `anchored_head - anchored_block >= N`, where
neither number came from the proof.

Until then, `EvmReceiptProof` routes must stay unregistered, and the router's EVM verifier should
say in its type that it expects an anchored header rather than "the caller asserts" one.
