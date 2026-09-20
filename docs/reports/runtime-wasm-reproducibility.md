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

The hashes below describe revision `5775b1587`. They are **not** a fixed
property of the project: any runtime-affecting change produces different WASM,
which is why `docs/reports/runtime-wasm-hashes.json` names the revision it was
recorded at, and why `scripts/mainnet_release_gate.py` **rebuilds and compares**
rather than trusting the file. Cutting a release means rebuilding from the
revision being released, confirming two builds of *that* revision agree, and
updating the record in the same change.

Earlier pairs of builds (compact `0x78feb683…`, `0xb1f0348c…`, `0xb1777a09…`) were taken at older
revision and is kept here only as the record of how this was established; the
current values are the `0x47acc821…` pair (recorded at `5775b1587`).

**Compact (`x3_chain_runtime.compact.wasm`, 8,424,386 bytes)**

```
Version          : x3-chain-11 (x3-chain-1.tx1.au1)
Metadata         : V14
setCode          : 0x733d6faa7a12face9b6346165d06f5ebe96ddef29c169d50ce9f604c118ced4d
authorizeUpgrade : 0x4745cb20348fd82585402630fd6460348c668f40088a8b18d7c38b4e375d49ab
IPFS             : QmeocZ2WgT2BSv7FMNmWYgR7GfSw5AVkvpPPQ1bX9sAywM
BLAKE2_256       : 0x47acc8211bf5e787ac1f7e5a937f26e616e3d1a594b2cc1cb35d88fa11991ccf
```

**Compressed (`x3_chain_runtime.compact.compressed.wasm`, 1,442,745 bytes)**

```
setCode          : 0x9d7e04a43c45f5d887ac8d9b93a614abbecf71af52f7ba4a57ab19bbff4904d1
authorizeUpgrade : 0xe1c960e5633cdd9e29bf1f7200838a5d261fc8fd148a476acb9293477f87150c
IPFS             : QmYLcDxcLaB3MPf3xfSFETi8NFfDz6Y9sMo8UgHyxqvWUU
BLAKE2_256       : 0x380527faeabbd5e48ff14aba6cf1bf5e888a22b28bdec676a712909ea6e525ed
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
