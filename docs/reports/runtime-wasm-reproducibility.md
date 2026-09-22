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
never instantiated by the runtime), and **re-attested with different bytes at
`5e0f86760`**: that revision fixes the Bitcoin header path — proof of work is
checked over Bitcoin's 80 wire bytes instead of the SCALE encoding of a struct
carrying a non-wire `height` field, and negative, overflowing and zero targets are
rejected as Bitcoin rejects them — and bumps `spec_version` to 12. The runtime
bytes changed, which is the intended behaviour: a revision that a mainnet
governance motion attests to has to be named, and the hash has to describe the
artifact that revision builds.

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

**Compact (`x3_chain_runtime.compact.wasm`, 8,433,833 bytes)**

```
Version          : x3-chain-12 (x3-chain-1.tx1.au1)
Metadata         : V14
setCode          : 0xba54025879f965ba59045c86790e68d17a305751dd4fd58dd24f72811e0b32a4
authorizeUpgrade : 0x19eedc53456633712b4c7e7b0228a8a746c5f3ba06f4b2021b40148a38b6c868
IPFS             : QmZASRxZoEgy4mrJeKpULzEPmyX84JUfdnSPcAnCCc3oJL
BLAKE2_256       : 0xbbb17006c0948307a775e8d3779e5a25a232deb4dce5ef8c3a17b1c70377ce61
```

**Compressed (`x3_chain_runtime.compact.compressed.wasm`, 1,444,249 bytes)**

```
setCode          : 0x9328da7077d22aaaa00a544282946fe8819d3d388c4bdc327a60cd19388b35de
authorizeUpgrade : 0xa9c8a93d620b4b44fb5b20751ea9100a06fdc9dc90f0d5eb90f644b50cfc92d3
IPFS             : QmZNZoHVHkvFRUrUCWhFfb8QfT3rsn7qeeLeAwbMgxzzmr
BLAKE2_256       : 0x0c36f055f2904497f2770d3eed5c2ac3fe48b67f1ec0f1aa3f23eb101d8a37a8
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
