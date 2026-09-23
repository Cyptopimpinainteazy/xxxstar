# The node finalizes with a certificate it observed, not one the chain was told

Date: 2026-09-23. Closes TICKET-107.

## The hole

`record_flash_finality_anchor` is an unsigned call that stores **the first non-zero certificate for a
height**, and `FinalityCertAnchors` cannot be overwritten afterwards. `AtomicGatewayService::finalize_bundle`
read the certificate for the current finalized height out of that map and signed
`finalize_atomic_bundle` with it, committing the receipt root to it. So a peer that called the anchor
first could choose the certificate the honest node would **sign** — a fabricated certificate, in a
proof the chain would show to external verifiers. The node was the last step of the attack; the chain
was only the messenger.

Nothing in the runtime can tell a planted anchor from an honest one: the certificate is a hash of
either a flash-finality certificate or the GRANDPA-finalized block hash, and the runtime sees neither.

## The fix

The knowledge the chain lacks, the node has — so the trust decision moved there:

* `node/src/finality_certs.rs` adds `ObservedFinalityCerts`, a small bounded map (`block → cert`) shared
  between the finality tasks and the atomic service, and `decide_finalization_cert(observed, anchored)`:

  | observed | anchored | decision |
  | --- | --- | --- |
  | `Some(o)` | `Some(a)`, `a == o` | `Finalize(o)` |
  | `Some(o)` | `Some(a)`, `a != o` | `Poisoned { observed: o, anchored: a }` — refuse, log both |
  | `Some(o)` | `None` | `Wait` — ours has not landed yet |
  | `None` | anything | `Wait` — never finalize on a value this node did not produce |

* `run_flash_finality_voter` records the certificate it writes under `x3ff:`; `run_grandpa_finality_anchor`
  records the certificate it anchors. Both already compute exactly this value; they now also publish it.
* `finalize_bundle` queries the anchor at the **finalized** block hash (it used `best_hash` before,
  while asking about the finalized height) and refuses to sign on disagreement.

**What the attacker gains now is a stall, not a signature.** A planted anchor for height *N* makes
every honest node refuse at *N*; finality advances, and the service's next poll takes the fresh anchor
at *N+1*. The refusal names the block and both hashes, so an operator sees a poisoning attempt instead
of a mysterious abort.

## Evidence

```
cargo test -p x3-chain-node --lib finality_certs   5 passed
   a_node_certificate_the_chain_agrees_with_is_used
   a_planted_anchor_is_refused_rather_than_signed
   an_unanchored_certificate_waits
   an_anchor_without_an_observation_waits
   observations_keep_the_first_value_and_stay_bounded
cargo check -p x3-chain-node                      ok
cargo clippy -p x3-chain-node --all-targets -- -D warnings   clean
```

The decision is a pure function of two values, which is why it can be tested without a node: the
tests above are the acceptance criteria, and the wiring is a few lines either side of them.

## What is still true

The anchor call remains unsigned and first-write-wins, so a peer can still consume a height's anchor
and stall that height. That is a liveness cost, not a safety one — no node signs an anchor it did not
observe — and closing it entirely means making the anchor an authenticated, per-authority call, which
is a runtime change to argue on its merits rather than a patch to slip in here.
