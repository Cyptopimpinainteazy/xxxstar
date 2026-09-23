# The missing half of the BTC path: a header source

Date: 2026-09-22
Scope: design for TICKET-095. No code in this file.

## What exists, and where it stops

The chain can now be born anchored (spec_version 15): a spec pins a Bitcoin height/hash, the
header is admitted, and `BtcBestHeight` starts at the checkpoint. `submit_btc_header` extends the
chain from there, enforcing Bitcoin's rules.

What does not exist is anything that *pushes* headers. `submit_btc_header` is `ensure_root`, the
repo has no Bitcoin RPC client, and nothing watches Bitcoin. So the chain's view of Bitcoin is
frozen at the checkpoint: a deposit in any block after it can never be proven, because the header
for that block is never admitted. Anchoring made the path verifiable; it did not make it *live*.

## What the missing piece has to do

At minimum: notice new Bitcoin headers, and get them on chain in order, cheaply enough that an
operator can run it continuously, and verifiably enough that "the relayer is Byzantine" is a
bounded problem rather than "the relayer is the chain's Bitcoin view".

The good news is that the hard part is already built and adversarial:

* every header is checked against Bitcoin's proof of work under this network's `powLimit`;
* heights are derived from the parent link, never read from the header;
* `nBits` must be copied verbatim off a retarget boundary and move by at most 4x on one;
* timestamps must postdate the median of the last up-to-11 ancestors;
* **only headers on a checkpoint-anchored chain are usable as SPV evidence.**

So a malicious relayer cannot invent Bitcoin: the worst it can do is *withhold* headers (liveness)
or push a valid-but-non-canonical branch that satisfies the rules (which the 4x clamp and the
checkpoint make expensive). That is the right shape for the trust model, and it is why the relayer
does not need to be trusted for correctness — only for liveness.

## Smallest first slice

1. **Pallet: a batch call with a configurable submitter.**

   ```rust
   /// Account permitted to submit Bitcoin headers, or `None` for root only.
   #[pallet::constant]
   type BtcHeaderSubmitter: Get<Option<<Self as frame_system::Config>::AccountId>>;

   #[pallet::call_index(35)]
   pub fn submit_btc_headers(origin, headers: Vec<BtcBlockHeader>) -> DispatchResult
   ```

   The origin check is `ensure_root` **or** `ensure_signed == configured account`; `None` keeps
   today's behaviour (root only), so every existing chain is unchanged until it opts in. Each
   header goes through the existing `btc_admit_header`, and the call stops at the first refusal —
   a partially applied batch is worse than a rejected one, so the whole call is one storage layer
   (`with_storage_layer`) and returns the height it reached on failure.

   A bond and slashing for withholding belongs in the same place later; it is not needed to make
   the path live, and inventing the economics before there is a network to run it on would be
   guesswork.

2. **A relayer that is a script, not a node feature.** `scripts/btc/push-headers.py`:
   - reads `bitcoin-cli getblockhash/getblockheader` (the script in `scripts/btc/` already talks to
     a node and refuses to emit anything the node disagrees with);
   - keeps a cursor in a local file, verifies each new header links to the last one it pushed, and
     stops (loudly) on a gap;
   - submits `x3SettlementEngine.submitBtcHeaders(headers)` through a Substrate RPC.

   The submission half needs a signer. **Nothing in this repository can currently sign and submit
   an extrinsic on this host**: `packages/ts-sdk` has a real `signAndSend` client but no
   `node_modules` anywhere in the tree, and `scripts/mainnet/rc4_runtime_upgrade_rehearsal.sh`
   points at `packages/blockchain-connector/node_modules`, which is also absent. So the first
   deliverable of this slice is *making one signing path work end to end* (`npm ci` in
   `packages/ts-sdk`, or a small Rust binary using `subxt`/`x3-runtime-signer`), and proving it by
   submitting a remark on a local dev chain.

3. **Tests.**
   - `submit_btc_headers` from the configured account is accepted; from any other signed account
     is refused; with `BtcHeaderSubmitter = None` it is refused for everyone but root.
   - A batch that is valid up to header *k* and invalid at *k+1* applies nothing.
   - The relayer script refuses a chain with a gap and refuses a header the local Bitcoin node
     disagrees with (the pattern `scripts/btc/capture-regtest-spv.py` already uses).

## What this does not solve

* It does not make the chain *follow* Bitcoin on its own — someone still runs one relayer, and
  nothing slashes it for withholding. TICKET-095 stays open until there is more than one.
* It does not change the header-source audit gap: three SPV implementations still exist, and the
  canonical one is the pallet's.
* It does not touch the checkpoint: the anchor is still an operator's commitment, and the relayer
  extends from it.
