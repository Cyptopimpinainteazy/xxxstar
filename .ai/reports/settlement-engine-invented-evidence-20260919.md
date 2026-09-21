# Settlement engine: two places that verify against invented data — found 2026-09-19

Found while extending `fix/fake-metrics-honesty`'s hunt ("stop two crates inventing
security results") to the rest of the repository. Same family, higher stakes: this is
the **cross-chain proof validator**, so the invented values are what a settlement is
checked against.

Both are in `pallets/x3-settlement-engine/src/lib.rs`, at two sites each (EVM and SVM).

## 1. A proof missing its roots is verified against zero roots

```rust
// Extract state_root and merkle_root from merkle_proof (use first two entries)
let state_root = proof.merkle_proof.first().copied().unwrap_or_default();   // line 2276, 2442
let merkle_root = proof.merkle_proof.get(1).copied().unwrap_or_default();   // line 2277
```

`unwrap_or_default()` on a `[u8; 32]` is **thirty-two zero bytes**. A proof that
carries fewer than two `merkle_proof` entries is therefore verified against a zero
state root and a zero merkle root — values the proof never stated — and the result is
returned as `valid`.

The EVM site shows the author knew the length mattered and checked the wrong thing:

```rust
Ok(valid && !proof.merkle_proof.is_empty())      // non-empty, not "at least two"
```

so a **one-entry** proof passes that guard while its second root was invented.

Whether this is exploitable depends on `T::CrossChainValidator::verify_*`: if it
compares the given roots against a canonical header, it passes only when the header's
roots are zero (an unknown/empty header). That is the case worth checking first, and
the fix does not depend on the answer — a proof that does not carry what it is
verified against should be refused, not completed.

## 2. The block number and the slot are derived from `tx_hash`

```rust
// Extract block number from proof data (use lower 64 bits of tx_hash as proxy)
let block_number = u64::from_le_bytes(proof.tx_hash.as_bytes()[0..8].try_into().unwrap_or_default());
```

and the same for the SVM `slot` (line ~2432). `tx_hash` is proof data, so the value
used to look up the **canonical header** — the thing the proof is supposed to be
checked against — is chosen by whoever wrote the proof. A prover can grind a
`tx_hash` whose first eight bytes name any block, and the canonical-header check then
confirms a header the prover picked. The comment calls it a proxy, which is the
honest word for it: it is not the block number.

This one needs a decision rather than a patch, because the proof type does not carry
the field: either `EthereumProof`/`SolanaProof` gains `block_number`/`slot` (and the
validator checks the header it looked up *is* that block), or the validator trait
takes the height from somewhere trustworthy. The two `#[allow(dead_code)]`-hidden
helpers around these functions suggest the surface was never finished.

## Suggested order

1. Refuse a short proof at both sites (a check, not a patch: `merkle_proof.len() >= 2`
   before the roots are read, or a typed accessor that returns `None`).
2. Establish whether `verify_evm_proof`/`verify_svm_proof` accept zero roots against a
   missing header; if they do, that is the exploitable end of defect 1 and belongs
   with the pallet's test suite.
3. Decide defect 2 (proof type vs trait), then a test that a forged `tx_hash` cannot
   select a header.

## Verification of the sibling work, 2026-09-19

`origin/fix/fake-metrics-honesty` trial-merged into `origin/master` (`b6fff9dd1`)
cleanly and independently reproduces its claimed test results:

    cargo test -p x3-flash-finality  -> 18 passed, 0 failed
    cargo test -p x3-foundry-core    -> 42 passed, 0 failed

so that branch is merge-ready from the point of view of the crates it touches.

---

# Finding 3 (2026-09-19, same session) — the EVM/SVM proof is bound to nothing

Found while fixing finding 2 (the proof-supplied block height). Reading the path
as a whole shows what the header check actually establishes, and it is not what
the code reads like.

## What the path checks

`Pallet::verify_evm_receipt_proof` (EVM) and `verify_svm_proof` (SVM), both in
`pallets/x3-settlement-engine/src/lib.rs`:

1. `proof_type` is one of the EVM kinds;
2. `receipt_data` is non-empty and, structurally, an RLP list (`is_valid_receipt_rlp`);
3. `keccak256(receipt_data) == proof.tx_hash`;
4. the proof carries two roots (`proof_roots`, the previous fix);
5. `{chain_height, block_hash, state_root, merkle_root}` equal the corresponding
   fields of the stored header, via
   `pallet_cross_chain_validator::Pallet::verify_settlement_evm_header`, which
   compares against `LastEvmHeader` — a `StorageValue`, i.e. **one** header.

## What it does not check

Nothing links the receipt to that header. The `merkle_proof` entries are read as
the *roots* (`merkle_proof[0]` → `state_root`, `[1]` → `merkle_root`) and handed to
the validator; they are never walked as a path, so no receipt-trie (MPT) proof is
ever verified. `keccak(receipt)` has to equal `tx_hash`, and the header's
`merkle_root` has to equal `merkle_proof[1]`, but there is no relationship between
the two. The proof also carries no asset, amount or recipient, so it does not bind
to the intent it settles either.

## Consequence

Anyone can settle a leg against the **latest** validated header for a chain:

- copy that header's public `block_hash`, `state_root`, `merkle_root` into the
  proof, and state its `block_number` as `chain_height` (the previous fix made this
  an explicit claim rather than a 2^64 grind — see below);
- pair it with any structurally valid receipt RLP, using `keccak(receipt)` as
  `tx_hash`;
- the four fields match, so the header check passes and the settlement is accepted.

`runtime/src/lib.rs::RuntimeCrossChainValidator` wires the real pallet (not the
no-op), so this is the runtime's behaviour, not a test mock's.

**Honest note on the previous fix**: before it, the attacker additionally had to
make the first eight bytes of `tx_hash` equal the header's block number — a 2^64
grind over the receipt they choose. That was an accident of the derivation, not a
designed barrier, and it is exactly why relying on it is indefensible: the fix
replaces "the attacker must grind to state a height" with "the proof states the
height, and the validator is asked about it", which is what a real lookup needs.
Stating the height is a **prerequisite** for the fix below, not a mitigation.

## Required fix, in order

1. **Bind the receipt to the header.** Verify an MPT path from `keccak(receipt)` to
   the header's receipts root (this needs `merkle_proof` to carry RLP node bytes
   rather than `H256` values — a data-contract change), or an equivalent
   light-client check. Without this, step 2 changes nothing.
2. **Look the header up by height.** `LastEvmHeader`/`LastSvmHeader` are single
   values; the stated height should select a stored header (or a range that is
   looked up), so "the header for height N" is a fact rather than "the header that
   happens to be latest".
3. **Carry what is being settled.** The proof should state the asset, amount and
   recipient (or a hash of them) and the pallet should compare that to the intent,
   instead of crediting what the intent claims while proving only that *some*
   receipt exists.

## Acceptance criteria

- A forged proof — a valid receipt for another transaction, with the latest
  header's four fields copied and the height stated correctly — must be refused.
- A genuine proof (a receipt in the block the header is for, with its MPT path)
  must be accepted.
- A regression test asserting the four header fields alone are insufficient.

## Posture while finding 3 is open (2026-09-19, `fa478f2b9`)

Containment, not the fix. `Config::AllowUnboundExternalProofs` (`Get<bool>`, default
off) makes the runtime state whether its validator binds the receipt to the header:

- the chain runtime (`runtime/src/lib.rs`) sets it `false`, with `spec_version` 11, so
  `verify_evm_receipt_proof` and `verify_svm_proof` refuse before any check with
  `"external proof verification unavailable: nothing binds this receipt to the header
  it is checked against (the merkle path is read as the roots, never walked) … BTC SPV
  proofs are unaffected — theirs is verified. See TICKET-063"`;
- the test runtime sets it `true` (the mock validator stands in for one that binds),
  which keeps the pallet's lifecycle tests reachable;
- the runtime-level test `the_chain_refuses_an_unbound_external_settlement_proof`
  builds a proof of exactly the shape the pallet accepts and asserts the chain
  refuses it with that reason, and that the constant is the safe value.

The consequence to be explicit about: with the flag `false`, the chain cannot settle
an EVM or SVM leg at all. That is the intended trade until step 1 of the ordered fix
(the MPT path from `keccak(receipt)` to the header's receipts root) exists. The flag
must not be flipped to `true` without that, and a runtime that flips it is asserting
its validator does the binding.

## Canonical path (2026-09-19, `ebc22fa47`): the fix exists — wire it, do not write it

Searching for an existing implementation before writing the MPT walk turned up
three more, which changes the plan for step 1:

| implementation | what it establishes | wired to |
|---|---|---|
| `crates/x3-verification-router/src/evm_receipt.rs` | real MPT walk against a header's receipts root (`verify_merkle_patricia_proof`), `no_std`, `alloc`, `tiny_keccak` | the relayer (`ProductionEvmReceiptVerifier`) |
| `x3-lang/vm/src/bridge.rs` | a real MPT walk, receiving the receipts root from the finalized header | x3-lang bridge programs |
| `pallets/x3-settlement-engine` | the two roots compared to the stored header; **no path** | the settlement path (this report) |
| `crates/x3-crosschain-intent/src/proof/evm.rs` | a receipt parser that returns roots and hashes it never checked | nothing |

So TICKET-063 step 1 is **not** "write an Ethereum MPT verifier": it is "call the
one in `x3-verification-router` from the settlement path", which is possible because
that crate is `no_std`/`alloc`. Two things the pallet needs first:

- the proof type must carry the trie nodes (an RLP list of node byte strings, which
  is the shape `verify_merkle_patricia_proof` takes) and the receipt index (the
  trie key is `rlp(index)`); the current `merkle_proof: Vec<H256>` holds *roots* and
  cannot express a path;
- the stored header's `merkle_root` is a value an **authorized submitter** asserts
  (`pallets/cross-chain-validator::validate_evm_header` checks that the submitted
  proof's leaves recompute to it, and the module says the trust anchor is the
  submitter). For the receipts walk it must be the *receipts* root, which is a
  statement about the field's meaning, not a schema change. `EvmMerkleRoots`
  already stores a root **per height**, so step 2 (look the header up by height
  rather than comparing against `LastEvmHeader`) needs no new storage either.

And a finding of its own: the canonical verifier could not verify anything until
`ebc22fa47` — four independent defects (list-wrapped trie key, abbreviated header
indices, left-aligned big-endian decode, `None` for the leaf value), with a test
suite in which every assertion was a *failure*, so all four coexisted with a green
build. Wiring it into the settlement path without that fix would have swapped one
broken check for another. See TICKET-064 in the ledger.
