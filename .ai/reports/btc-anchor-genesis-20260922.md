# A chain born anchored — and the dev runtime that did not exist

Date: 2026-09-22
Scope: `pallets/x3-settlement-engine` (genesis), `node` (spec plumbing + a missing feature),
`runtime/src/lib.rs` (spec_version 15), `scripts/testnet/`

## The gap this closes

Spec_version 14 gave the BTC path a trust root (`anchor_btc_checkpoint` + `BtcCheckpoints`), and
left an awkward bring-up story: the path is fail-closed until a checkpoint exists, and creating one
was a **root call** — on a testnet, a manual step, doable by whoever holds the key first. The
readiness row said so: "anchoring is a manual operator act".

## What changed

* `X3SettlementEngine::GenesisConfig` gained **`btc_checkpoints`**. Each entry is validated as
  genesis is built — proof of work under this network's `powLimit`, and no two entries at one
  height — then pins `(height, hash)` in `BtcCheckpoints` and **admits the header** into
  `BtcHeaders` / `BtcHeaderMetaStore { height, anchored: true }`, raising `BtcBestHeight`. A bad
  entry makes the node refuse to start (with the header, the height and the reason) instead of
  launching a chain whose root of trust is a lie.
* `X3_BTC_CHECKPOINTS="<80-byte header hex>@<height>[,…]"` is the knob.
  `scripts/testnet/build-x3-testnet-spec.py` forwards the environment, so the generated spec's
  `genesis.runtimeGenesis.config.x3SettlementEngine.btcCheckpoints` carries it in plain JSON beside
  the rest of the genesis — reviewable in the same diff as the chain id.
* `BtcBlockHeader` gained serde derives, for the spec only: the proof path still hashes the 80 wire
  bytes, and the doc comment says so.
* `spec_version` 14 → 15, runtime re-attested.

## Proof

**Live, 12/12 — `scripts/testnet/btc-checkpoint-drill.sh` (new `--testnet` gate):**

| check | why it matters |
| --- | --- |
| `twox_128("System")` matches Substrate's known prefix | every storage key the drill computes is correct; a wrong key looks exactly like an empty value |
| the anchored spec lists the header, the plain spec lists none | the difference is the spec |
| genesis state: `BtcCheckpoints(h)`, `BtcHeaderMetaStore(hash)`, `BtcBestHeight` | the anchor is really in the chain's initial state, not just the JSON |
| a node from the anchored spec starts, serves RPC, and its storage matches | the live chain, not a rebuild of the same JSON |
| the anchored chain still produces blocks | an anchored genesis is not a chain that cannot author |
| a spec whose checkpoint is not a mined header is refused, with the pallet's reason | a wrong checkpoint cannot be shipped quietly |

**Unit, `cargo test -p pallet-x3-settlement-engine` = 152 + 23 passed**, including
`genesis_pins_a_bitcoin_checkpoint_and_admits_its_header` and three refusals (not a mined header,
easier than the network's `powLimit`, two entries at one height).

## The discovery the first drill run produced

The drill failed the first time, on the pallet's own message:

```
genesis BTC checkpoint 0xaf33…7127 at height 121 carries nBits 0x207fffff, an easier target than
this network's powLimit; Bitcoin would have refused that header
```

That was correct behaviour aimed at the wrong runtime. **The node had no `dev` feature.** It had a
runtime feature (`x3-chain-runtime/dev`) and a chain name (`--chain dev`), but nothing connected
them, so every `--chain dev` in this repository built a spec called dev and ran the **default**
runtime: no `Sudo`, and `powLimit` at mainnet's `0x1d00ffff`, which refuses every
regtest-difficulty header. Any dev-only behaviour — root calls, regtest proof of work — was
unavailable by construction, and nothing had noticed because nothing had tried to use it.

Fixed with `node` feature `dev = ["x3-chain-runtime/dev"]` plus the matching `sudo` genesis field in
`chain_spec.rs`. **A dev chain is `cargo build -p x3-chain-node --features dev`.** The gate encodes
this: it builds that binary, because with any other one the drill would be measuring the default
runtime's proof-of-work policy rather than the trust root.

## Not claimed

No public network has a checkpoint pinned; the header still arrives by an operator copying a hash
from elsewhere; nothing pushes headers after the anchor, so the chain does not follow Bitcoin; and a
regtest checkpoint only anchors on a chain whose `powLimit` is regtest's, which is why the drill
needs the dev runtime. TICKET-095 covers the header source.
