#import "../style.typ": *
#import "../components.typ": *
#import "../data.typ": *

= Consensus, Transactions & State Integrity

This chapter cross-references the consensus/networking, transaction-lifecycle, and state/storage domain investigations into one integrity narrative, following a transaction from proposal through finality and highlighting exactly where documentation and types diverge from the real execution path.

== Genesis Through Finality

Genesis and chain-spec generation use a real CSPRNG (`secrets.token_hex(32)` in `scripts/testnet/build-x3-testnet-spec.py`) rather than fixed test seeds, with derived Aura (sr25519) and GRANDPA (ed25519) keys per validator — VERIFIED by static inspection. Block production (Aura) and finality (GRANDPA) are genuine, unmodified Substrate/Polkadot-SDK crates (`sc_consensus_aura::start_aura`, `sc_consensus_grandpa::run_grandpa_voter`), wired into every `construct_runtime!` variant — this is not represented by types with no backing; it is real, standard consensus machinery.

Equivocation handling is correctly proof-gated: `X3EquivocationReportSystem` calls `sp_consensus_grandpa::check_equivocation_proof` — a real cryptographic verification — before any slash from that path. The parallel `report_misbehavior` path does not share this discipline (CRIT-02, Chapter 5).

== Transaction Lifecycle, Traced End to End

Following one representative transaction — a cross-VM transfer through the router — from RPC to finality:

+ *Ingress*: `node/src/rpc.rs`'s `x3_submitCrossVmTransaction` rejects raw payloads via a magic-byte check and verifies `lock_proof[0..32] == blake2_256(operation.encode())` before constructing a real, fully-signed `UncheckedExtrinsic` with the standard `SignedExtra` chain (`CheckNonce`, `CheckWeight`, `ChargeTransactionPayment`, plus X3's own `InvariantCheck` and `AgentLaw` extensions).
+ *Pool*: submitted via the stock `sc_transaction_pool` — no custom admission/eviction logic was found, and the claimed `private-mempool` MEV-protection crate is not wired into this path at all (MED-05).
+ *Execution*: the cross-VM router pallet enforces replay protection via `NextNonce`/`UsedMessages` (VERIFIED live: `cargo test -p pallet-x3-cross-vm-router` passed 81/81), expiry refund (`cancel_expired_xvm_transfer`, tested), and the canonical-supply invariant on every debit/credit.
+ *Settlement*: `pallets/x3-settlement-engine`'s 23 property-based tests (VERIFIED live, proptest-based, not example-only) assert real invariants — no partial execution, bond release never exceeds reserved, settlement balance never exceeds total supply.
+ *Finality*: the same GRANDPA path described above finalizes the block containing all of the above atomically, as a single Substrate state-transition — genuinely atomic within the X3 runtime, though not across an external chain boundary (see Chapter 8's trust-boundary diagram).

== Where Types and Documentation Diverge From the Real Path

- The "12-step atomic execution model" in `CLAUDE.md` (parse → semantic check → ... → assert invariants) describes the x3-lang/IXL pipeline's intent; the actual atomicity guarantee a transaction receives on-chain comes from Substrate's transactional storage semantics in the settlement engine, which this audit verified directly — the 12-step model is a design document, not something this audit traced step-by-step in a single function.
- "PoAE" (Proof of Atomic Execution) in `pallets/x3-atomic-kernel` is a real, wired, tested on-chain audit record of bundle lifecycle state — but it is not an externally-verifiable cryptographic proof (LOW-04); a party that does not trust the X3 node's own storage cannot independently check it.
- State migrations for 10 pallets exist as files and are wired into the runtime's upgrade tuple, but contain no actual transformation logic (MED-01) — they would not survive a real storage-shape change without being rewritten from scratch first.

== State Reconstruction and Independent Verification

Standard Substrate trie-based state commitment is used unmodified; the only custom modification found was in vendored `patches/sp-state-machine` (`ReadOnlyExternalities`/`Basic`), which contains `unimplemented!()` calls for write-path methods that are — by design — never called from any X3-authored code (INFO-01, confirmed via repo-wide grep for callers). No X3-specific mechanism for independently verifying reconstructed state against a canonical source was found beyond the standard srtool reproducible-build path, which this audit found unusable in its current evidentiary state (HIGH-05).
