# The allocation-from-input sweep, and adversarial coverage for cross-chain evidence

2026-09-25. The previous cycle found two unbounded `Vec::with_capacity(<parsed count>)` allocations
in the X3 bytecode readers, one of them on the chain's own reader
(`.ai/reports/x3-vm-gas-and-bytecode-20260925.md`). This cycle swept the rest of the tree for the
same shape and added the adversarial coverage the receipt decoder was missing.

## The sweep

Every `with_capacity(<identifier>)` outside test code was listed and each decode-path hit was read
against one question: **is the count a parsed field, or the length of something already allocated?**
Only the first is dangerous.

| site | count comes from | verdict |
| --- | --- | --- |
| `crates/x3-verification-router/src/evm_receipt.rs:945` | a `u32` parsed from the payload | **safe** — validated against the remaining input first (`if s.len() < 4 + len { return Err(TooShort) }`), which is exactly the check the X3 readers were missing |
| `runtime/src/fraud_proofs/witness_v1.rs:322` | `tx_ids.len()` | safe — the length of a slice that already exists |
| `runtime/src/fraud_proofs/scheduler_v1.rs:106` | `witness.tx_ids.len()` | safe — same |
| `crates/x3-vm/src/bytecode.rs:150,155,162` | parsed counts | safe — every one is checked against `MAX_CONST_POOL` / `MAX_FUNCTIONS` / `MAX_INSTRUCTIONS` before the allocation |
| `crates/cross-vm-coordinator`, `x3-parallel-executor`, `x3-gulfstream`, `contention-predictor`, `x3-evolution`, `quantum-swarm`, `gpu-swarm`, `mini_evm` | configured limits or slice lengths | safe |

So the two defects were the ones that mattered, and the rest of the tree does this correctly — the
`evm_receipt` decoder being the clearest example of the pattern to copy.

## What the sweep did find: a second bytecode format nothing uses

`crates/x3-vm/src/bytecode.rs` (383 lines) defines its own `BytecodeModule`, `Opcode`, `parse` and
`validate` for a **different container** — its own header layout, a `u16` version, a `Vec<u64>`
constant pool. Evidence that it is dead:

* nothing in the workspace references `x3_vm::bytecode::*` — not this crate, not `x3-integration`,
  not `pallet-x3-kernel`, not the compiler, not any test;
* it has a `parse` and **no writer**, so nothing in the tree can even produce that format;
* this crate already re-exports the real one under the same name
  (`pub use x3_backend::bc_format::BytecodeModule`), so `x3_vm::BytecodeModule` and
  `x3_vm::bytecode::BytecodeModule` are **two different types with one name**.

That is the shape that produced the previous cycle's divergence: `x3-backend`'s reader and
`mini_x3` read the same bytes and disagreed about a string constant, and every string constant
executed differently on chain than off it.

**I did not delete it.** It is public API of a crate several workspace members depend on
(`atomic-swap-orchestrator`, `x3-bot`, `x3-gpu-validator-swarm`, `x3-bridge-adapters`, …), and the
repository's own rule is that files are not deleted silently. It is recorded here as a ticket
instead, with the evidence needed to act on it. Labelling the module in place would have been the
next best thing, but a doc comment in `crates/x3-vm/src/` moves the runtime hash record and costs a
25-minute re-attestation to certify that a comment changed nothing — which is not a trade worth
making for wording that the deletion ticket supersedes.

## Adversarial coverage for the cross-chain evidence decoder

`evm_receipt` is a trust boundary: `DecodedProof::decode` parses bytes a relayer supplies, and
`validate` decides whether an EVM leg happened. It had 19 unit tests and one real-block fixture, but
nothing that damaged the bytes. `crates/x3-verification-router/tests/decoder_robustness.rs` adds six,
built on a proof produced by the module's own `receipts_trie_root` / `receipts_trie_proof` /
`encode_proof_payload`, with the first test asserting it decodes *and* validates so the sweeps cannot
pass vacuously:

* every truncation of the body is refused;
* every single-byte mutation at five replacement values must not panic the decoder or the inclusion
  walk;
* the walk is bound to `header.receipts_root` — flipping any of its 32 bytes must break inclusion;
* trailing bytes must not change what the proof says (the sections are self-delimiting, so this is
  pinned as a decision rather than left as a surprise).

No defect found: this decoder validates lengths before allocating and fails closed on damage. That
is worth having on record as much as a bug would be.

### The wire format has two layers, and that is easy to get wrong

`encode_proof_payload` writes a 16-byte preamble (the prover's claimed head height and its own
minimum confirmations) and then four length-prefixed sections. `DecodedProof::decode` does **not**
read the preamble — `ProductionEvmReceiptVerifier::verify` consumes it (`let body = &proof.payload[16..]`)
and passes the rest. The two functions are therefore not a matched pair:
`decode(&encode_proof_payload(..))` returns `TooShort`, because the preamble's first four bytes are
read as a section length.

I wrote this file the wrong way first and the test told me so; the boundary is now pinned by
`the_preamble_belongs_to_the_verifier_not_to_the_decoder`, which asserts the head and minimum are in
the preamble, that `decode` refuses the whole payload, and that it accepts the body.

## Ticket: the duplicate bytecode format

**Action:** delete `crates/x3-vm/src/bytecode.rs` and its `pub mod bytecode;` declaration, or — if it
is being kept for a future format — rename it so `BytecodeModule` is not exported twice from one
crate, and document that it has no producer.

**Acceptance criteria:** `cargo tree -i x3-vm` consumers still build; no path in `crates/`,
`pallets/` or `runtime/` references `x3_vm::bytecode`; `grep -rn 'pub mod bytecode' crates/x3-vm`
returns nothing (if deleted); and the crate's 158 tests still pass.

**Why it matters rather than being tidiness:** a third reader of a *different* container, exported
under the same type name as the canonical one, is the next place a divergence can hide.
