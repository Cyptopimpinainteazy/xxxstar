# The runtime hash record, re-attested for `aaece429d`

Date: 2026-09-26
Scope: `docs/reports/runtime-wasm-hashes.json`, gate `runtime hash freshness`
Matrix: `X3-LANG-004`

## What was red

The previous commit (`aaece429d`, the EVM/SVM packet guard) changed three files in the runtime's
dependency graph — `pallets/x3-kernel/src/packet_adapters.rs`,
`pallets/x3-kernel/src/wasm_adapters.rs`, `runtime/src/lib.rs` — without moving the record. The
commit message called that out ("re-attested separately"), and `local-ci` left it red:

```
$ python3 scripts/check-runtime-hash-freshness.py --base origin/master    # before
[runtime-hash] 3 changed file(s) can alter the runtime that mainnet governance attests to,
but docs/reports/runtime-wasm-hashes.json did not move
    pallets/x3-kernel/src/packet_adapters.rs
    pallets/x3-kernel/src/wasm_adapters.rs
    runtime/src/lib.rs
exit 1
```

This is not a bookkeeping nit: `scripts/mainnet_release_gate.py` stage 6b rebuilds the runtime and
fails when the bytes differ from the record, so the red gate was the early warning for a release
failure ten minutes into `make mainnet-check`.

## Why a rebuild was the only honest fix

The gate would also have gone green by moving `recorded_revision` alone, and that would have been a
false record: `pallets/x3-kernel/src/wasm_adapters.rs` is exactly what the wasm runtime instantiates
(`#[cfg(not(all(feature = "std", feature = "frontier")))] type EvmAdapter =
pallet_x3_kernel::wasm_adapters::WasmEvmAdapter;` in `runtime/src/lib.rs`), so the guard is compiled
into the blob. The measured bytes confirm it — the compact artifact moved 8,506,630 -> 8,508,157.

## Two blockers that had to be cleared first

1. **The srtool container could not read the checkout.** The repository root is mode `0700`
   (`drwx------ lojak lojak`), so the pinned image's `builder` user (uid 1001) failed with
   `/srtool/build: line 12: cd: /build: Permission denied` and the build stopped after 922-byte
   reports. Reproduced directly:
   `docker run --rm --user 1001:1001 -v "$PWD":/build alpine sh -c 'cd /build'` -> *Permission
   denied*. Fixed by restoring the root to `o+rx`, which is how it was when the last successful
   pair (`a7ff06572`) was built on 2026-09-24.
2. **The image starts with an empty cargo home**, so every build re-fetches polkadot-sdk. The host's
   warm `/home/lojak/.cargo/git` (1.3 GB, includes `polkadot-sdk-dee0edd6eefa0594`) was hardlinked to
   a world-readable `/tmp/x3-srtool-cargo-git` and passed as `SRTOOL_CARGO_GIT_CACHE`, which is the
   hook `scripts/run-srtool.sh` documents for this.

## The evidence

Three from-scratch srtool builds of `aaece429d` completed in the `/tmp/x3-rt-rebuild` worktree
(`runtime/target/srtool` removed between builds by `update-runtime-hashes.sh`). Parsed with the
release gate's own `_srtool_values`, they agree on **every** field:

| report | compact size | compact BLAKE2-256 | compressed size | compressed BLAKE2-256 |
|---|---|---|---|---|
| `srtool-20260925-205430.json` | 8508157 | `0x801fc4b6…22fb0a` | 1463840 | `0x8e799903…e5ee2a` |
| `srtool-20260925-211455.json` | 8508157 | `0x801fc4b6…22fb0a` | 1463840 | `0x8e799903…e5ee2a` |
| `srtool-20260925-212448.json` | 8508157 | `0x801fc4b6…22fb0a` | 1463840 | `0x8e799903…e5ee2a` |

Pairwise comparison of all fields (`size`, `set_code`, `authorize_upgrade`, `ipfs`, `blake2_256`,
`version`, `metadata`): **AGREE**. The record the build script wrote is byte-identical to the
committed one.

The independent cross-check is the document that landed with `aaece429d`:
`docs/reports/runtime-wasm-reproducibility.md` already claimed compact 8,508,157 and compressed
1,463,840 for that revision. The doc was right and the JSON record was the stale half.

## Result

```
$ python3 scripts/check-runtime-hash-freshness.py --base origin/master    # after
[runtime-hash] 5 file(s) changed, none in the runtime's dependency graph (129 packages) — nothing to do
exit 0

$ bash scripts/local-ci.sh --jobs 4
local-ci: all gates passed
```

`88/88` gates PASS at `9d7a0656f` (`.ai/runlogs/local-ci-20260926T035308Z-summary.json`), which is
the commit that carries the record.

## What this does not prove

* It does not prove the hash is the one an audited release commits to; that is stage 6b of
  `make mainnet-check` plus a separate audit.
* It does not close `X3-LANG-004`. The EVM/SVM payload convention is still undecided, so those two
  `submit_comit_v2` arms are *inert* (refused by name) rather than executed — the next task seed in
  `.ai/memory/agent-memory.md`.
* `runtime/src/lib.rs` is gated on `#[cfg(all(feature = "std", feature = "frontier"))]`, so the
  `NativeEvmAdapter`/`NativeSvmAdapter` guards there are **not** in the wasm; only the kernel
  sources moved the blob.
