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

The hashes below describe revision `c4b3a1621`. They are **not** a fixed
property of the project: any runtime-affecting change produces different WASM,
which is why `docs/reports/runtime-wasm-hashes.json` names the revision it was
recorded at, and why `scripts/mainnet_release_gate.py` **rebuilds and compares**
rather than trusting the file. Cutting a release means rebuilding from the
revision being released, confirming two builds of *that* revision agree, and
updating the record in the same change.

The first pair of builds (compact `0x78feb683…`) was taken at an earlier
revision and is kept here only as the record of how this was established; the
current values are the `0xb1f0348c…` pair.

**Compact (`x3_chain_runtime.compact.wasm`, 8,419,783 bytes)**

```
Version          : x3-chain-11 (x3-chain-1.tx1.au1)
Metadata         : V14
setCode          : 0xb1f0348cbe38d893a6ed3aafccf46c9be882fbeb6065d84693f58f0945c90ce3
authorizeUpgrade : 0x9dff843475473cc9cffce4baea61279a2490ef4a7cd82188e07dea38d24d16c6
IPFS             : QmSoEEz3vrqZmnYGL8M9Rsr2r2BYV7EcHd79ABxWCzpmHE
BLAKE2_256       : 0x154d73b853bcbbe2cf1802cd5bd2b818b5200c42e1ae6d8db3faad8429cae37d
```

**Compressed (`x3_chain_runtime.compact.compressed.wasm`, 1,443,099 bytes)**

```
setCode          : 0xa3de51b578473f55cadcfc80d559c60aa14f45e447330ecb74b44cec3d42849c
authorizeUpgrade : 0xfe31159a26fd644df98b697c0f6405e3aa8e49805357f3cab1d35dded67ed9bd
IPFS             : QmXraoEUG4u6qbW5b3TnDMTNPuimfLEkaMqxnkEgEpQBUz
BLAKE2_256       : 0xf3726b93445966095ddbee28d277b18db1aa0d44c98dc04f148fa29a3d3dd27c
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
* Toolchain note: the image must be able to build this dependency graph. Both
  `1.75.0` (the previous pin) and `1.88.0` refuse with
  `enum-ordinalize@4.4.2 requires rustc 1.89`.
