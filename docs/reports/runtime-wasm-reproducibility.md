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

## Result: two independent builds, identical output

The second run was a from-scratch rebuild (`runtime/target/srtool` removed
first), so this is not one artifact reported twice.

**Compact (`x3_chain_runtime.compact.wasm`, 8,419,266 bytes)**

```
Version          : x3-chain-11 (x3-chain-1.tx1.au1)
Metadata         : V14
setCode          : 0x78feb68360e5766aad0915f22c4b4ee049988c4c03bf248b53c910ba5ee397b9
authorizeUpgrade : 0x417006727b7dcff0d550c03d70bdd5b6420e02bda6393a5b8632ef87c62631af
IPFS             : Qma824b7PSzHTJ83aTQANLviuMonDCHk6C8ffLCkdtCQW9
BLAKE2_256       : 0xe58ded03cecee9687e4f53e5771a7eb8decb6513b595a1ddd0b234adf9a538d3
```

**Compressed (`x3_chain_runtime.compact.compressed.wasm`, 1,441,379 bytes, 82.88%)**

```
setCode          : 0xf4b7e11a6c397c8dfff25521958bc22df9c50739fa5e8ae109a8197da981998d
authorizeUpgrade : 0xc0eba551e55ff52682aea5ad992110c8c5f16e76ff5f4e141609b2e8d4bb255b
IPFS             : QmRnPZQ4Nvj52RtcHERgd96MPbpjYQ8c6VBkW3Ld9ZwhHP
BLAKE2_256       : 0xa252dd56da9d0db00549efc01e524dabc7351427d1738c794283a7ec5a67699f
```

Both runs produced these values byte for byte. The compressed artifact is the
one whose `setCode`/`BLAKE2_256` a release would attest to.

## What this does and does not prove

* **Does:** the runtime source in this revision builds deterministically inside
  that image — nothing in the workspace's build scripts injects a timestamp,
  path or machine-specific value into the WASM.
* **Does not:** prove the hash matches what an *audited* release commits to, and
  it is not yet wired into the release gate. `python3 scripts/mainnet_release_gate.py`
  stage 6 checks that srtool is *installed*; it does not rebuild and compare.
  Promoting this to a per-release gate step (≈10 minutes per run) would close
  that gap, and the two tags `1.93.0` and `1.93.0-0.18.4` are the same digest,
  so it can pin exactly what the hosted production-gate workflow uses.
* Toolchain note: the image must be able to build this dependency graph. Both
  `1.75.0` (the previous pin) and `1.88.0` refuse with
  `enum-ordinalize@4.4.2 requires rustc 1.89`.
