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

**Compact (`x3_chain_runtime.compact.wasm`, 8,456,248 bytes)**

```
Version          : x3-chain-14 (x3-chain-1.tx1.au1)
Metadata         : V14
setCode          : 0x5513646fae6177d06f1bbf0c8331c6edf357dace025391b6e56eea66566f870b
authorizeUpgrade : 0xfa813889751349ed521edfef33fcee526bccffd72ed6ef71506a73c76b7a41e2
IPFS             : QmZdnPkqPWtncHY1oDTHqM4mQmUk7LTSQnSpMLJ8pjUJnF
BLAKE2_256       : 0x65a114997691f5c2d02ed38427c76cf726094ce00728178c5e9d052fd1a9fcd4
```

**Compressed (`x3_chain_runtime.compact.compressed.wasm`, 1,449,722 bytes)**

```
setCode          : 0x47c9a97f47eac09cb4aac08a6f48a6fad0be6162464fd45b6505757223e68722
authorizeUpgrade : 0xcd1a80950ac5f9324bd719d50e942f82c07597be7e5b4cab32b779ec3dcb8f29
IPFS             : QmNsAaHtjMksUDupT8pqdYADAGMjJ4cK5EGJJ88micNeCG
BLAKE2_256       : 0x1b56ee67ad1732c034745a41c2a2fba32e95106fed325151ebe8e9bc6f396b5b
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
