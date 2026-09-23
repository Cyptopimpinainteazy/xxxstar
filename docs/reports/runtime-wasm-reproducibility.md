# Runtime WASM reproducibility — verified 2026-09-19

Mainnet governance attests to a runtime hash; that hash is only meaningful if
anyone can rebuild the same WASM from the same source. This records a
reproducible build of `x3-chain-runtime` and the hashes it produced.

## How

```bash
# one pinned image, so every builder uses the same toolchain
docker pull paritytech/srtool:1.93.0-0.18.4   # = paritytech/srtool:1.93.0
docker run --rm \
  -e PACKAGE=x3-chain-runtime -e RUNTIME_DIR=runtime \
  -v "$PWD":/build paritytech/srtool:1.93.0-0.18.4 build
```

`scripts/run-srtool.sh build` wraps this (it also creates `runtime/target` with
the permissions the image's `builder` user needs — see that script).

| | |
| --- | --- |
| image | `paritytech/srtool:1.93.0-0.18.4` |
| image digest | `sha256:8638a668bd6d29111dc01953fbead6eb08c062e1cc62d3047a245a52b6edb3bf` |
| srtool | 0.18.4 |
| rustc in image | 1.93.0 |
| source | `master` @ the commit this file was added on |

## Result: reproducible at a given revision

The second run was a from-scratch rebuild (`runtime/target/srtool` removed
first), so this is not one artifact reported twice.

The values below were first established at `deab51f7f`, the revision that added
the cross-chain-gateway header anchor (`EvmHeaderAnchor`), re-attested unchanged at
`04fb44907` (the minimal-RLP EVM signature fix, which lives behind `std` and is
never instantiated by the runtime), and then **re-attested twice with different
bytes**:

* `5e0f86760` — the Bitcoin header path: proof of work is checked over Bitcoin's 80
  wire bytes instead of the SCALE encoding of a struct carrying a non-wire `height`
  field, negative/overflowing/zero targets are rejected as Bitcoin rejects them, and
  `spec_version` moves to 12.
* `9fe02ac37` — the cross-chain gateway derives withdrawal ids with
  domain-separated Blake2b-256 instead of XOR mixing, which collided (`"A" * 64` and
  `"B" * 64` produced the same id, and that id keys both the pallet's `Withdrawals`
  map and the relayer's processed set), and `spec_version` moves to 13.
* `93fe86c9bd` — the BTC SPV path gets a trust root: `anchor_btc_checkpoint`
  (root) commits this chain to a Bitcoin height/hash in write-once
  `BtcCheckpoints`, `BtcPoWLimitBits` carries Bitcoin's per-network `powLimit`, the
  single admission path `btc_admit_header` derives heights from the parent link and
  enforces Bitcoin's `nBits` and median-time-past rules, and both SPV entry points
  accept evidence only from a checkpoint-anchored chain. New storage and a new call,
  so `spec_version` moves to 14.
* `e74bb199f9` — the BTC tests now run on data a real Bitcoin Core v28.1.0 regtest
  node produced (header, merkle path and a confirmed transaction), and the SPV entry
  points document which transaction serialization they take. `spec_version` is
  unchanged at 14: **the bytes still move**, because pallet documentation is part of
  the runtime metadata, and metadata is in the WASM. That is the reason this gate
  watches the whole dependency graph rather than a hand-kept list of "runtime files".
* `2a4296b630` — the SPV trust root can be pinned in genesis: `X3SettlementEngine`'s
  `GenesisConfig` carries `btc_checkpoints`, validated as genesis is built and then
  admitted onto the anchored chain, so a testnet can start with the root of trust in its
  spec. `BtcBlockHeader` gains serde for the spec's benefit only. `spec_version` moves to
  15.
* `269d611d96` — formatting only, and the runtime still reports `x3-chain-15`; but the
  bytes moved by two. `cargo fmt` re-wrapped lines in the pallet, and an `assert!`'s
  message carries the `file:line` it was written at, so where a line sits is part of the
  artifact. Nothing about the format of a change is invisible to this gate, which is the
  point of measuring bytes rather than intentions.
* `d471adfec4` — validator key rotation is wired to the on-chain custody registry:
  `pallet_x3_custody` gains `KeyRotationPeriod` and `rotate_validator_key` grants
  `current_block + period` instead of inheriting an overdue due block; the node gains
  the `validator rotate` operator command. `spec_version` moves to 16.
* `93d97edd34` — the merge of the formatting fix with that key-rotation change. Both
  revisions were attested separately and neither record described the other's tree, which
  is what a single record for a single artifact means: at any moment the file describes
  **the revision it names**, and a merge of two attested revisions is a third revision
  that has to be rebuilt. Two from-scratch builds of the merge agree, and the compact
  artifact is 8,468,726 bytes.
* `e28a0907be` — the Bitcoin header path gains a batch submission
  (`x3SettlementEngine.submitBtcHeaders`, call_index 35, up to 100 headers per call, atomic so
  a batch refused partway applies nothing) and a `BtcHeaderOrigin` config item, which this
  runtime sets to `EnsureRoot`. `spec_version` moves to 17. Two from-scratch builds of
  `e28a0907be` agree, and the compact artifact was 8,475,400 bytes at that revision.
* `a7878e933e` — the BTC median-time-past rule stops being stricter than Bitcoin's: with fewer than
  eleven ancestors (the first headers above a checkpoint) the median is not computable from stored
  history, so the check is skipped instead of using the parent as a stand-in, which refused real
  headers. `spec_version` moves to 18. Two from-scratch builds of `a7878e933e` agree, and the compact
  artifact is 8,474,849 bytes.
* `5ec1b61576` — formatting only, and the hashes did not move: collapsing a multi-line `ensure!` into
  one line leaves the macro's `file:line` where it was, so this is the first revision in this list
  whose bytes are identical to its predecessor's. `recorded_revision` moved; nothing else did.
* `7ea1819169` — Bitcoin merkle verification in `x3-bitcoin-vault` becomes position-aware, and the
  pallet's tests now compare all three implementations on one real regtest block. The hashes are
  again identical to the previous revision's: the vault is a *dev*-dependency of the pallet, and the
  intent crate's change is a doc comment, so nothing the runtime instantiates moved. Two revisions
  in a row where only `recorded_revision` changes is worth noticing: it means the gate is measuring
  what it claims to measure rather than reacting to any edit in the graph.
* `0d296d236b` — the atomic kernel's `do_finalize_bundle` requires the bundle to be `Executing`
  instead of accepting a `Pending` one and leaving the refusal to `verify_bundle_consistency` two
  checks later, and five tests now cover the finalization entry point (the alarm TICKET-097 has to
  change). **The bytes move again**: the pallet is instantiated by the runtime, so this is not a
  dev-dependency revision. `spec_version` deliberately stays at 18 — every reachable call accepts
  and refuses exactly what it did before, a `Pending` bundle being refused both before and after,
  only earlier and with a different error code, and no state transition changes. Two from-scratch
  builds of `0d296d236b` agree: compact 8,474,849 bytes (same size as the previous revision,
  different bytes) and compressed 1,453,609.

Bytes changing is the intended behaviour: a revision that a mainnet governance
motion attests to has to be named, and the hash has to describe the artifact that
revision builds.

`recorded_at` and the top-level `runtime_version` are now written by
`./scripts/update-runtime-hashes.sh` from the same srtool block the hashes come
from (each runtime block also carries its own `version`/`metadata`, and stage 6b
compares them). Until 2026-09-22 those fields were carried over untouched: the
record described `x3-chain-11` for a wasm that reported `x3-chain-12`, and nothing
noticed.

They are **not** a fixed property of the project: any runtime-affecting change
produces different WASM, which is why `docs/reports/runtime-wasm-hashes.json`
names the revision it was recorded at, and why
`scripts/mainnet_release_gate.py` **rebuilds and compares** rather than trusting
the file. Cutting a release means rebuilding from the revision being released,
confirming two builds of *that* revision agree, and updating the record in the
same change.

Earlier pairs of builds (compact `0x78feb683…`, `0xb1f0348c…`, `0xb1777a09…`) were
taken at older revisions and are kept here only as the record of how this was
established. `setCode` for the compact artifact is `0xac13ee1b…`, quoted with the
other values below.

**Compact (`x3_chain_runtime.compact.wasm`, 8,474,849 bytes)**

```
Version          : x3-chain-18 (x3-chain-1.tx1.au1)
Metadata         : V14
setCode          : 0x678abec69f30bd5fd2a5afc296b482cb14fcc7738c8109b39fcde80b0efc9b36
authorizeUpgrade : 0x48640b791bff46e897b15e2de521ed271cb9c51d5ad82fb985de7f8a01b6be40
IPFS             : Qmf2FBS65ayur4Udb4hwQr7Dee7NoZDGZuKAY2V2ywKSwN
BLAKE2_256       : 0xd62665d003f586565013cfaa1e6a6abfa58796c201d3c8225f1e798e1f44338c
```

**Compressed (`x3_chain_runtime.compact.compressed.wasm`, 1,453,609 bytes)**

```
setCode          : 0xe55c4a202972f640a47b1960551bf83a757d2e1211a32fc7c43071d183c76344
authorizeUpgrade : 0x9b7ab954b0b5ab89668378bdcfa6dc5065b5e094e9b0cbc69a33a89005df0806
IPFS             : QmZka2CcEDNfCzofLMGeqdHVWMiUsJuvJtwH1UH6JWcfpP
BLAKE2_256       : 0xc6bbd66a4d1de9c48e1ab3009c3fc468a5869916d797b9457236d6e12225eb07
```

Both runs produced these values byte for byte. The compressed artifact is the
one whose `setCode`/`BLAKE2_256` a release would attest to.

## What this does and does not prove

* **Does:** the runtime source in this revision builds deterministically inside
  that image — nothing in the workspace's build scripts injects a timestamp,
  path or machine-specific value into the WASM.
* **Does:** the release gate enforces it. Stage 6b of
  `scripts/mainnet_release_gate.py` runs this build and compares every value
  against `docs/reports/runtime-wasm-hashes.json`, so `make mainnet-check` fails
  if the runtime no longer builds to the recorded bytes. It adds about ten
  minutes to that gate; that is the price of the claim.
* **Does not:** prove the hash is the one an *audited* release commits to.
  Updating the record is part of cutting a release, and an audit is a separate
  step.

## Re-attesting

Any change that alters the runtime's bytes invalidates the record, and the
release gate will say so. Re-recording it is one command:

```bash
./scripts/update-runtime-hashes.sh          # build twice, then rewrite the record
./scripts/update-runtime-hashes.sh --check  # build twice, write nothing
```

It clears srtool's target directory between the two builds, compares every
field, and refuses to write anything if they disagree — a disagreement is a
reproducibility failure, not a stale record. Run it in the same change that
alters the runtime, so the record and the code land together.
* Toolchain note: the image must be able to build this dependency graph. Both
  `1.75.0` (the previous pin) and `1.88.0` refuse with
  `enum-ordinalize@4.4.2 requires rustc 1.89`.
