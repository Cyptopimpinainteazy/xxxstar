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

The hashes below describe revision `922fbde16`. They are **not** a fixed
property of the project: any runtime-affecting change produces different WASM,
which is why `docs/reports/runtime-wasm-hashes.json` names the revision it was
recorded at, and why `scripts/mainnet_release_gate.py` **rebuilds and compares**
rather than trusting the file. Cutting a release means rebuilding from the
revision being released, confirming two builds of *that* revision agree, and
updating the record in the same change.

Earlier pairs of builds (compact `0x78feb683…`, then `0xb1f0348c…`) were taken at older
revision and is kept here only as the record of how this was established; the
current values are the `0xb1777a09…` pair.

**Compact (`x3_chain_runtime.compact.wasm`, 8,420,312 bytes)**

```
Version          : x3-chain-11 (x3-chain-1.tx1.au1)
Metadata         : V14
setCode          : 0xb1777a0973e0adfee402364c18815598487fe3e50a05da4f3034b795a4982328
authorizeUpgrade : 0xde33dad4f51852fe7aa4d8974e937b12e762ae1efee6cee48a10d26a85412aca
IPFS             : QmUNb2XxXXEfEjb9UHu7uC1FPwGTqFTL82vMURNT4vwHNz
BLAKE2_256       : 0xcce41a9e4b504522c569e9778d6253385451b31238ef01e4777e77f2b84ef561
```

**Compressed (`x3_chain_runtime.compact.compressed.wasm`, 1,443,216 bytes)**

```
setCode          : 0x5488df118b09819b34cfeb7dd9dfc1802dd03979627d12905ca7f2b3a0fa9095
authorizeUpgrade : 0xf684208d29a61e88dc9caad20f9764e6d437ea14f762370f1be90ebc86875dcc
IPFS             : QmefFnaknozLQJKx5i8ZefRgo5L6fmFeMQnVCCnv4VGSxR
BLAKE2_256       : 0x851d985d843e3f64ef855073c5fbe3b6a20e66bf9b53357b4b5e6a51a676e1f4
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
