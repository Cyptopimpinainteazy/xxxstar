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

The hashes below describe the revision that added the cross-chain-gateway header
anchor (`EvmHeaderAnchor`), recorded at revision `0508659d5`. They are **not** a fixed
property of the project: any runtime-affecting change produces different WASM,
which is why `docs/reports/runtime-wasm-hashes.json` names the revision it was
recorded at, and why `scripts/mainnet_release_gate.py` **rebuilds and compares**
rather than trusting the file. Cutting a release means rebuilding from the
revision being released, confirming two builds of *that* revision agree, and
updating the record in the same change.

Earlier pairs of builds (compact `0x78feb683…`, `0xb1f0348c…`, `0xb1777a09…`) were taken at older
revision and is kept here only as the record of how this was established; the
current values are the `0xac13ee1b…` pair (recorded at `0508659d5`).

**Compact (`x3_chain_runtime.compact.wasm`, 8,435,073 bytes)**

```
Version          : x3-chain-11 (x3-chain-1.tx1.au1)
Metadata         : V14
setCode          : 0xac13ee1b4d375d257d808cd720253eef0b6f126c34aba35eab0e5ab04fa714aa
authorizeUpgrade : 0x23312ecb8c081cbbd838e8f8d9f4077bb789ef891dd25d345abfefe3acde8334
IPFS             : QmVsqseyoMvS7nnpTtuBQ9EBCB113FQfCBXP4XVLQJEyRg
BLAKE2_256       : 0x12696572aae6cc833630c13a1b7dd141cced8b6934f8d4ab872bf813274a5fc6
```

**Compressed (`x3_chain_runtime.compact.compressed.wasm`, 1,442,714 bytes)**

```
setCode          : 0x9088ef509c3ba8fc720156356bec7997a16a278ca024236ff0f35ac5f400b674
authorizeUpgrade : 0x626363657411d604831279cb41789351083e9ee0451f7c9d2a8013a23133ec56
IPFS             : QmS57QuggaLvhZ2h4rJCBWWhthRjoEWSooucR8aZbfzCNg
BLAKE2_256       : 0x0e6ef363215aaa098770718a85ee7d8f217852efefe6af329bf02915a25f0d88
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
