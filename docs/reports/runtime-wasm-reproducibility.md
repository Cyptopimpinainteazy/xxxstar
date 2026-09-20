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

The hashes below describe revision `f7007ee4d`. They are **not** a fixed
property of the project: any runtime-affecting change produces different WASM,
which is why `docs/reports/runtime-wasm-hashes.json` names the revision it was
recorded at, and why `scripts/mainnet_release_gate.py` **rebuilds and compares**
rather than trusting the file. Cutting a release means rebuilding from the
revision being released, confirming two builds of *that* revision agree, and
updating the record in the same change.

Earlier pairs of builds (compact `0x78feb683…`, `0xb1f0348c…`, `0xb1777a09…`) were taken at older
revision and is kept here only as the record of how this was established; the
current values are the `0xf0e1e6e7…` pair.

**Compact (`x3_chain_runtime.compact.wasm`, 8,420,214 bytes)**

```
Version          : x3-chain-11 (x3-chain-1.tx1.au1)
Metadata         : V14
setCode          : 0xf0e1e6e72542170aa22567a0791b346c5eba80579e0af07af3ff2b6e209fc86c
authorizeUpgrade : 0x877de18ba73356356b59c203d53453f6eb6df5f31aab4623b042ff21ff751b48
IPFS             : QmdtaKNfMc58FG1tSwnVZ6deR2su3Zw1DgdAMEtaujtCeL
BLAKE2_256       : 0xa90fa39255a0dbf9a6aeab96c595aea25ffd32ff94258b01798b85d9e4e5082e
```

**Compressed (`x3_chain_runtime.compact.compressed.wasm`, 1,442,110 bytes)**

```
setCode          : 0x8e83291590bb93227fb6b5ecdf0823d7d9e6369de0dfd341730205c1ac733770
authorizeUpgrade : 0x333052240df6b5d53eadf7fd11076ab09b0fbaa409663ad464c0c595c14c7d29
IPFS             : Qmcto7SvzXqWVJEJqbT8QsyQJJG2xMiiMrCZhNwV9o2U7X
BLAKE2_256       : 0x124b5b9389459b3960ff972ce46176f8f45ee1b3853e541e33e2616cbfc1a22e
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
